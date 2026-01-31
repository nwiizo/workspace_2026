//! KeePass KDBX Password Cracker
//!
//! Supports KDBX 3.x format with AES-256 encryption.
//!
//! # Example
//!
//! ```rust,ignore
//! use web_security_toolkit::keepass::{KdbxFile, crack_kdbx};
//!
//! let kdbx = KdbxFile::parse(&data)?;
//! if let Some(password) = crack_kdbx(&kdbx, &wordlist) {
//!     println!("Password found: {}", password);
//! }
//! ```

use aes::Aes256;
use cbc::{
    cipher::{BlockDecryptMut, KeyIvInit},
    Decryptor,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// KeePass-related errors
#[derive(Error, Debug)]
pub enum KeePassError {
    #[error("Invalid KDBX signature")]
    InvalidSignature,
    #[error("Unsupported KDBX version: {0}.{1}")]
    UnsupportedVersion(u16, u16),
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("IO error: {0}")]
    IoError(String),
}

/// KDBX file header information
#[derive(Debug, Clone)]
pub struct KdbxHeader {
    /// Major version
    pub major_version: u16,
    /// Minor version
    pub minor_version: u16,
    /// Cipher ID (AES-256 = 31c1f2e6-bf71-4350-be58-05216afc5aff)
    pub cipher_id: [u8; 16],
    /// Compression flags (0 = none, 1 = gzip)
    pub compression: u32,
    /// Master seed (32 bytes)
    pub master_seed: Vec<u8>,
    /// Transform seed (32 bytes for AES-KDF)
    pub transform_seed: Vec<u8>,
    /// Transform rounds
    pub transform_rounds: u64,
    /// Encryption IV (16 bytes for AES)
    pub encryption_iv: Vec<u8>,
    /// Protected stream key
    pub protected_stream_key: Vec<u8>,
    /// Stream start bytes (first 32 bytes after decryption)
    pub stream_start_bytes: Vec<u8>,
    /// Inner random stream ID
    pub inner_random_stream_id: u32,
}

/// Parsed KDBX file
#[derive(Debug)]
pub struct KdbxFile {
    /// Header information
    pub header: KdbxHeader,
    /// Encrypted payload
    pub encrypted_data: Vec<u8>,
    /// Header hash (for verification)
    pub header_bytes: Vec<u8>,
}

impl KdbxFile {
    /// Parse a KDBX file from bytes
    pub fn parse(data: &[u8]) -> Result<Self, KeePassError> {
        if data.len() < 12 {
            return Err(KeePassError::InvalidHeader("File too small".to_string()));
        }

        // Check signatures
        let sig1 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let sig2 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        // KDBX signature: 0x9AA2D903, 0xB54BFB67
        if sig1 != 0x9AA2D903 || sig2 != 0xB54BFB67 {
            return Err(KeePassError::InvalidSignature);
        }

        // Version
        let minor = u16::from_le_bytes([data[8], data[9]]);
        let major = u16::from_le_bytes([data[10], data[11]]);

        // Only support KDBX 3.x
        if major != 3 {
            return Err(KeePassError::UnsupportedVersion(major, minor));
        }

        let mut header = KdbxHeader {
            major_version: major,
            minor_version: minor,
            cipher_id: [0; 16],
            compression: 0,
            master_seed: Vec::new(),
            transform_seed: Vec::new(),
            transform_rounds: 0,
            encryption_iv: Vec::new(),
            protected_stream_key: Vec::new(),
            stream_start_bytes: Vec::new(),
            inner_random_stream_id: 0,
        };

        // Parse header fields
        let mut pos = 12;
        loop {
            if pos + 3 > data.len() {
                return Err(KeePassError::InvalidHeader("Truncated header".to_string()));
            }

            let field_id = data[pos];
            let field_size = u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize;
            pos += 3;

            if pos + field_size > data.len() {
                return Err(KeePassError::InvalidHeader(
                    "Truncated field data".to_string(),
                ));
            }

            let field_data = &data[pos..pos + field_size];
            pos += field_size;

            match field_id {
                0 => break, // End of header
                1 => {
                    // Comment (ignored)
                }
                2 => {
                    // Cipher ID
                    if field_size == 16 {
                        header.cipher_id.copy_from_slice(field_data);
                    }
                }
                3 => {
                    // Compression flags
                    if field_size >= 4 {
                        header.compression = u32::from_le_bytes([
                            field_data[0],
                            field_data[1],
                            field_data[2],
                            field_data[3],
                        ]);
                    }
                }
                4 => {
                    // Master seed
                    header.master_seed = field_data.to_vec();
                }
                5 => {
                    // Transform seed
                    header.transform_seed = field_data.to_vec();
                }
                6 => {
                    // Transform rounds
                    if field_size >= 8 {
                        header.transform_rounds = u64::from_le_bytes([
                            field_data[0],
                            field_data[1],
                            field_data[2],
                            field_data[3],
                            field_data[4],
                            field_data[5],
                            field_data[6],
                            field_data[7],
                        ]);
                    }
                }
                7 => {
                    // Encryption IV
                    header.encryption_iv = field_data.to_vec();
                }
                8 => {
                    // Protected stream key
                    header.protected_stream_key = field_data.to_vec();
                }
                9 => {
                    // Stream start bytes
                    header.stream_start_bytes = field_data.to_vec();
                }
                10 => {
                    // Inner random stream ID
                    if field_size >= 4 {
                        header.inner_random_stream_id = u32::from_le_bytes([
                            field_data[0],
                            field_data[1],
                            field_data[2],
                            field_data[3],
                        ]);
                    }
                }
                _ => {
                    // Unknown field, skip
                }
            }
        }

        // Validate required fields
        if header.master_seed.len() != 32 {
            return Err(KeePassError::InvalidHeader(
                "Invalid master seed".to_string(),
            ));
        }
        if header.transform_seed.len() != 32 {
            return Err(KeePassError::InvalidHeader(
                "Invalid transform seed".to_string(),
            ));
        }
        if header.encryption_iv.len() != 16 {
            return Err(KeePassError::InvalidHeader(
                "Invalid encryption IV".to_string(),
            ));
        }
        if header.stream_start_bytes.len() != 32 {
            return Err(KeePassError::InvalidHeader(
                "Invalid stream start bytes".to_string(),
            ));
        }

        let header_bytes = data[..pos].to_vec();
        let encrypted_data = data[pos..].to_vec();

        Ok(Self {
            header,
            encrypted_data,
            header_bytes,
        })
    }

    /// Get human-readable info about the KDBX file
    pub fn info(&self) -> String {
        format!(
            "KDBX {}.{}\n\
             Transform rounds: {}\n\
             Compression: {}\n\
             Encrypted data size: {} bytes",
            self.header.major_version,
            self.header.minor_version,
            self.header.transform_rounds,
            if self.header.compression == 1 {
                "gzip"
            } else {
                "none"
            },
            self.encrypted_data.len()
        )
    }
}

/// Derive the composite key from password only
fn derive_composite_key(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let password_hash = hasher.finalize();

    let mut hasher2 = Sha256::new();
    hasher2.update(password_hash);
    hasher2.finalize().into()
}

/// Derive the composite key from password and key file
fn derive_composite_key_with_keyfile(password: &str, keyfile_data: &[u8]) -> [u8; 32] {
    // Hash password
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let password_hash = hasher.finalize();

    // Hash key file contents
    let mut keyfile_hasher = Sha256::new();
    keyfile_hasher.update(keyfile_data);
    let keyfile_hash = keyfile_hasher.finalize();

    // Combine password hash and keyfile hash
    let mut combined_hasher = Sha256::new();
    combined_hasher.update(password_hash);
    combined_hasher.update(keyfile_hash);

    combined_hasher.finalize().into()
}

/// Transform the key using AES-KDF
fn transform_key(key: &[u8; 32], seed: &[u8], rounds: u64) -> [u8; 32] {
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};

    let cipher = aes::Aes256::new_from_slice(seed).expect("Invalid seed size");
    let mut transformed = *key;

    // Split into two 16-byte blocks
    let (left, right) = transformed.split_at_mut(16);

    // Create GenericArray from slices
    let mut block1 = GenericArray::clone_from_slice(left);
    let mut block2 = GenericArray::clone_from_slice(right);

    for _ in 0..rounds {
        cipher.encrypt_block(&mut block1);
        cipher.encrypt_block(&mut block2);
    }

    left.copy_from_slice(&block1);
    right.copy_from_slice(&block2);

    // Final hash
    let mut hasher = Sha256::new();
    hasher.update(transformed);
    hasher.finalize().into()
}

/// Derive the master key from composite key
fn derive_master_key(transformed_key: &[u8; 32], master_seed: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(master_seed);
    hasher.update(transformed_key);
    hasher.finalize().into()
}

/// Try to decrypt the KDBX file with a password
pub fn try_password(kdbx: &KdbxFile, password: &str) -> bool {
    try_password_internal(kdbx, password, None)
}

/// Try to decrypt the KDBX file with a password and key file
pub fn try_password_with_keyfile(kdbx: &KdbxFile, password: &str, keyfile_data: &[u8]) -> bool {
    try_password_internal(kdbx, password, Some(keyfile_data))
}

/// Internal function to try decryption
fn try_password_internal(kdbx: &KdbxFile, password: &str, keyfile_data: Option<&[u8]>) -> bool {
    // Step 1: Derive composite key from password (and optionally keyfile)
    let composite_key = match keyfile_data {
        Some(data) => derive_composite_key_with_keyfile(password, data),
        None => derive_composite_key(password),
    };

    // Step 2: Transform the key
    let transformed_key = transform_key(
        &composite_key,
        &kdbx.header.transform_seed,
        kdbx.header.transform_rounds,
    );

    // Step 3: Derive master key
    let master_key = derive_master_key(&transformed_key, &kdbx.header.master_seed);

    // Step 4: Decrypt first block and check stream start bytes
    type Aes256CbcDec = Decryptor<Aes256>;

    let Ok(cipher) = Aes256CbcDec::new_from_slices(&master_key, &kdbx.header.encryption_iv) else {
        return false;
    };

    // We only need to decrypt enough to check the stream start bytes
    let mut buffer = kdbx.encrypted_data.clone();
    if buffer.len() < 32 {
        return false;
    }

    // Decrypt in place
    match cipher.decrypt_padded_mut::<block_padding::NoPadding>(&mut buffer) {
        Ok(decrypted) => {
            // Check if first 32 bytes match stream start bytes
            decrypted[..32] == kdbx.header.stream_start_bytes
        }
        Err(_) => false,
    }
}

/// Crack a KDBX file using a wordlist
///
/// Returns the password if found, None otherwise.
pub fn crack_kdbx(kdbx: &KdbxFile, wordlist: &[String]) -> Option<String> {
    use rayon::prelude::*;

    wordlist
        .par_iter()
        .find_any(|password| try_password(kdbx, password))
        .cloned()
}

/// Crack a KDBX file using a wordlist and key file
///
/// Returns the password if found, None otherwise.
pub fn crack_kdbx_with_keyfile(
    kdbx: &KdbxFile,
    wordlist: &[String],
    keyfile_data: &[u8],
) -> Option<String> {
    use rayon::prelude::*;

    wordlist
        .par_iter()
        .find_any(|password| try_password_with_keyfile(kdbx, password, keyfile_data))
        .cloned()
}

/// Crack a KDBX file using a wordlist, with progress callback
///
/// Returns the password if found, None otherwise.
pub fn crack_kdbx_with_progress<F>(
    kdbx: &KdbxFile,
    wordlist: &[String],
    progress_callback: F,
) -> Option<String>
where
    F: Fn(usize, &str) + Sync,
{
    for (i, password) in wordlist.iter().enumerate() {
        progress_callback(i, password);
        if try_password(kdbx, password) {
            return Some(password.clone());
        }
    }
    None
}

/// Parse hashed blocks format used by KDBX 3.x
fn parse_hashed_blocks(data: &[u8]) -> Result<Vec<u8>, KeePassError> {
    let mut result = Vec::new();
    let mut pos = 0;

    loop {
        if pos + 4 > data.len() {
            break;
        }

        // Block index (4 bytes)
        let _block_index =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        // Block hash (32 bytes)
        if pos + 32 > data.len() {
            break;
        }
        let _block_hash = &data[pos..pos + 32];
        pos += 32;

        // Block size (4 bytes)
        if pos + 4 > data.len() {
            break;
        }
        let block_size =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        // Check for end marker (size = 0)
        if block_size == 0 {
            break;
        }

        // Block data
        if pos + block_size > data.len() {
            return Err(KeePassError::InvalidHeader(
                "Block data truncated".to_string(),
            ));
        }
        result.extend_from_slice(&data[pos..pos + block_size]);
        pos += block_size;
    }

    Ok(result)
}

/// Decrypt the KDBX file and return the raw XML content
pub fn decrypt_kdbx(kdbx: &KdbxFile, password: &str) -> Result<Vec<u8>, KeePassError> {
    // Derive composite key
    let composite_key = derive_composite_key(password);

    // Transform the key
    let transformed_key = transform_key(
        &composite_key,
        &kdbx.header.transform_seed,
        kdbx.header.transform_rounds,
    );

    // Derive master key
    let master_key = derive_master_key(&transformed_key, &kdbx.header.master_seed);

    // Decrypt
    type Aes256CbcDec = Decryptor<Aes256>;

    let cipher = Aes256CbcDec::new_from_slices(&master_key, &kdbx.header.encryption_iv)
        .map_err(|_| KeePassError::DecryptionFailed)?;

    let mut buffer = kdbx.encrypted_data.clone();
    let decrypted = cipher
        .decrypt_padded_mut::<block_padding::NoPadding>(&mut buffer)
        .map_err(|_| KeePassError::DecryptionFailed)?;

    // Verify stream start bytes
    if decrypted[..32] != kdbx.header.stream_start_bytes {
        return Err(KeePassError::DecryptionFailed);
    }

    // Skip stream start bytes
    let content = &decrypted[32..];

    // Parse hashed blocks
    let block_data = parse_hashed_blocks(content)?;

    // Decompress if needed
    if kdbx.header.compression == 1 {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(block_data.as_slice());
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| KeePassError::IoError(e.to_string()))?;
        Ok(decompressed)
    } else {
        Ok(block_data)
    }
}

/// Inner stream cipher for decrypting protected values
pub struct InnerStreamCipher {
    state: Vec<u8>,
    position: usize,
}

impl InnerStreamCipher {
    /// Create a new Salsa20 inner stream cipher
    pub fn new_salsa20(protected_stream_key: &[u8]) -> Self {
        use salsa20::cipher::{KeyIvInit, StreamCipher};
        use sha2::{Digest, Sha256};

        // Hash the protected stream key
        let mut hasher = Sha256::new();
        hasher.update(protected_stream_key);
        let key_hash = hasher.finalize();

        // IV is 8 bytes of 0xe8, 0x30, 0x09, 0x4b, 0x97, 0x20, 0x5d, 0x2a
        let iv = [0xe8, 0x30, 0x09, 0x4b, 0x97, 0x20, 0x5d, 0x2a];

        // Generate keystream (pre-compute a large buffer)
        let mut cipher = salsa20::Salsa20::new(key_hash[..32].into(), iv[..].into());
        let mut keystream = vec![0u8; 65536]; // 64KB buffer
        cipher.apply_keystream(&mut keystream);

        Self {
            state: keystream,
            position: 0,
        }
    }

    /// Decrypt a protected value (Base64 encoded)
    pub fn decrypt_protected(&mut self, base64_value: &str) -> Option<String> {
        use base64::Engine;
        let encrypted = base64::engine::general_purpose::STANDARD
            .decode(base64_value)
            .ok()?;

        let mut decrypted = encrypted.clone();
        for byte in decrypted.iter_mut() {
            if self.position < self.state.len() {
                *byte ^= self.state[self.position];
                self.position += 1;
            }
        }

        String::from_utf8(decrypted).ok()
    }
}

/// Extract entries from decrypted KDBX XML
#[derive(Debug, Clone)]
pub struct KdbxEntry {
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
}

/// Parse entries from KDBX XML content
pub fn parse_entries(xml: &str, protected_stream_key: &[u8]) -> Vec<KdbxEntry> {
    let mut entries = Vec::new();
    let mut cipher = InnerStreamCipher::new_salsa20(protected_stream_key);

    // Simple XML parsing (not a full parser, just pattern matching)
    for entry_match in xml.split("<Entry>").skip(1) {
        if let Some(entry_end) = entry_match.find("</Entry>") {
            let entry_xml = &entry_match[..entry_end];

            // Skip history entries
            if entry_xml.contains("<History>") {
                continue;
            }

            let title = extract_string_value(entry_xml, "Title", &mut cipher);
            let username = extract_string_value(entry_xml, "UserName", &mut cipher);
            let password = extract_string_value(entry_xml, "Password", &mut cipher);
            let url = extract_string_value(entry_xml, "URL", &mut cipher);
            let notes = extract_string_value(entry_xml, "Notes", &mut cipher);

            // Only add entries with actual content
            if !title.is_empty() || !username.is_empty() || !password.is_empty() {
                entries.push(KdbxEntry {
                    title,
                    username,
                    password,
                    url,
                    notes,
                });
            }
        }
    }

    entries
}

fn extract_string_value(entry_xml: &str, key: &str, cipher: &mut InnerStreamCipher) -> String {
    let pattern = format!("<Key>{}</Key>", key);
    if let Some(key_pos) = entry_xml.find(&pattern) {
        let after_key = &entry_xml[key_pos + pattern.len()..];

        // Check if value is protected
        if let Some(value_start) = after_key.find("<Value") {
            let value_section = &after_key[value_start..];

            if value_section.contains("Protected=\"True\"") {
                // Protected value - need to decrypt
                if let Some(start) = value_section.find('>') {
                    let content_start = start + 1;
                    if let Some(end) = value_section[content_start..].find("</Value>") {
                        let encoded = &value_section[content_start..content_start + end];
                        return cipher.decrypt_protected(encoded).unwrap_or_default();
                    }
                }
            } else {
                // Plain value
                if let Some(start) = value_section.find('>') {
                    let content_start = start + 1;
                    if let Some(end) = value_section[content_start..].find("</Value>") {
                        return value_section[content_start..content_start + end].to_string();
                    }
                }
            }
        }
    }
    String::new()
}

/// Decrypt and extract entries from a KDBX file
pub fn extract_entries(kdbx: &KdbxFile, password: &str) -> Result<Vec<KdbxEntry>, KeePassError> {
    let xml_bytes = decrypt_kdbx(kdbx, password)?;
    let xml = String::from_utf8(xml_bytes).map_err(|e| KeePassError::IoError(e.to_string()))?;
    Ok(parse_entries(&xml, &kdbx.header.protected_stream_key))
}

/// Common passwords list for KeePass cracking
pub fn common_passwords() -> Vec<String> {
    vec![
        // Common passwords
        "password",
        "123456",
        "12345678",
        "qwerty",
        "abc123",
        "monkey",
        "1234567",
        "letmein",
        "trustno1",
        "dragon",
        "baseball",
        "iloveyou",
        "master",
        "sunshine",
        "ashley",
        "bailey",
        "shadow",
        "123123",
        "654321",
        "superman",
        "qazwsx",
        "michael",
        "football",
        "password1",
        "password123",
        "batman",
        "login",
        "admin",
        "admin123",
        "root",
        "toor",
        // Juice Shop themed
        "juice",
        "juiceshop",
        "owasp",
        "juice-shop",
        "JuiceShop",
        "support",
        "support123",
        "SupportTeam",
        "support_team",
        "incident",
        "incident123",
        "IncidentResponse",
        // Security themed
        "security",
        "secure",
        "s3cur1ty",
        "p@ssw0rd",
        "P@ssword1",
        // Simple patterns
        "test",
        "test123",
        "testing",
        "demo",
        "demo123",
        "secret",
        "secret123",
        "changeme",
        "welcome",
        "welcome1",
        // Numbers
        "111111",
        "000000",
        "121212",
        "123321",
        "666666",
        "696969",
        "7777777",
        "8675309",
        // Keyboard patterns
        "qwerty123",
        "qwertyuiop",
        "asdfgh",
        "zxcvbn",
        // Tech terms
        "hack",
        "hacker",
        "h4ck3r",
        "pwned",
        "r00t",
        "kali",
        "metasploit",
        "nmap",
        "burp",
        "wireshark",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Extended password list for more thorough cracking
pub fn extended_passwords() -> Vec<String> {
    let mut passwords = common_passwords();

    // Add variations
    let base_words = vec![
        "support", "incident", "juice", "admin", "user", "test", "password", "secret", "secure",
        "team", "help", "service",
    ];

    for word in &base_words {
        // With numbers
        for i in 0..=999 {
            passwords.push(format!("{}{}", word, i));
            passwords.push(format!("{}{:03}", word, i));
        }
        // With symbols
        passwords.push(format!("{}!", word));
        passwords.push(format!("{}@", word));
        passwords.push(format!("{}#", word));
        passwords.push(format!("{}$", word));
        passwords.push(format!("{}123!", word));
        // Capitalized
        let capitalized = format!(
            "{}{}",
            word.chars().next().unwrap().to_uppercase(),
            &word[1..]
        );
        passwords.push(capitalized.clone());
        passwords.push(format!("{}123", capitalized));
        passwords.push(format!("{}!", capitalized));
    }

    passwords
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_composite_key() {
        let key = derive_composite_key("test");
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_common_passwords() {
        let passwords = common_passwords();
        assert!(!passwords.is_empty());
        assert!(passwords.contains(&"password".to_string()));
        assert!(passwords.contains(&"juiceshop".to_string()));
    }

    #[test]
    fn test_extended_passwords() {
        let passwords = extended_passwords();
        assert!(passwords.len() > common_passwords().len());
        assert!(passwords.contains(&"support1".to_string()));
        assert!(passwords.contains(&"incident123".to_string()));
    }
}
