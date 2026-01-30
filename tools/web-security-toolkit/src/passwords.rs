//! Password and credential utilities
//!
//! Provides common passwords, hash identification, and credential generation.

use sha2::{Digest, Sha256};

/// Password entry with metadata
#[derive(Debug, Clone)]
pub struct PasswordEntry {
    pub password: String,
    pub category: PasswordCategory,
    pub strength: PasswordStrength,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordCategory {
    Default,
    Common,
    Keyboard,
    Numeric,
    Name,
    Year,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PasswordStrength {
    Weak,
    Medium,
    Strong,
}

impl PasswordEntry {
    pub fn new(password: impl Into<String>, category: PasswordCategory, strength: PasswordStrength) -> Self {
        Self {
            password: password.into(),
            category,
            strength,
        }
    }
}

/// Top 100 most common passwords
pub fn top_passwords() -> Vec<PasswordEntry> {
    vec![
        // Default/Admin passwords
        PasswordEntry::new("admin", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("admin123", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("administrator", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("root", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("password", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("password1", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("password123", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("pass123", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("test", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("test123", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("guest", PasswordCategory::Default, PasswordStrength::Weak),
        PasswordEntry::new("default", PasswordCategory::Default, PasswordStrength::Weak),

        // Common words
        PasswordEntry::new("123456", PasswordCategory::Numeric, PasswordStrength::Weak),
        PasswordEntry::new("12345678", PasswordCategory::Numeric, PasswordStrength::Weak),
        PasswordEntry::new("123456789", PasswordCategory::Numeric, PasswordStrength::Weak),
        PasswordEntry::new("1234567890", PasswordCategory::Numeric, PasswordStrength::Weak),
        PasswordEntry::new("000000", PasswordCategory::Numeric, PasswordStrength::Weak),
        PasswordEntry::new("111111", PasswordCategory::Numeric, PasswordStrength::Weak),

        // Keyboard patterns
        PasswordEntry::new("qwerty", PasswordCategory::Keyboard, PasswordStrength::Weak),
        PasswordEntry::new("qwerty123", PasswordCategory::Keyboard, PasswordStrength::Weak),
        PasswordEntry::new("qwertyuiop", PasswordCategory::Keyboard, PasswordStrength::Weak),
        PasswordEntry::new("asdfgh", PasswordCategory::Keyboard, PasswordStrength::Weak),
        PasswordEntry::new("zxcvbn", PasswordCategory::Keyboard, PasswordStrength::Weak),
        PasswordEntry::new("1qaz2wsx", PasswordCategory::Keyboard, PasswordStrength::Weak),

        // Common words
        PasswordEntry::new("letmein", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("welcome", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("monkey", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("dragon", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("master", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("login", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("football", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("baseball", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("iloveyou", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("trustno1", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("sunshine", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("princess", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("shadow", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("superman", PasswordCategory::Common, PasswordStrength::Weak),
        PasswordEntry::new("batman", PasswordCategory::Common, PasswordStrength::Weak),
    ]
}

/// Common admin usernames
pub fn common_usernames() -> Vec<&'static str> {
    vec![
        "admin",
        "administrator",
        "root",
        "user",
        "test",
        "guest",
        "demo",
        "info",
        "support",
        "manager",
        "operator",
        "sysadmin",
        "webmaster",
        "postgres",
        "mysql",
        "oracle",
        "sa",
        "dba",
    ]
}

/// Hash type identification
#[derive(Debug, Clone, PartialEq)]
pub enum HashType {
    Md5,
    Sha1,
    Sha256,
    Sha512,
    Bcrypt,
    Argon2,
    NtHash,
    Unknown,
}

/// Identify hash type by length and format
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::passwords::{identify_hash, HashType};
///
/// assert_eq!(identify_hash("5d41402abc4b2a76b9719d911017c592"), HashType::Md5);
/// assert_eq!(identify_hash("$2a$10$abcdefghij"), HashType::Bcrypt);
/// ```
pub fn identify_hash(hash: &str) -> HashType {
    let hash = hash.trim();

    // Check for Bcrypt prefix
    if hash.starts_with("$2a$") || hash.starts_with("$2b$") || hash.starts_with("$2y$") {
        return HashType::Bcrypt;
    }

    // Check for Argon2 prefix
    if hash.starts_with("$argon2") {
        return HashType::Argon2;
    }

    // Check by length (hex-encoded)
    match hash.len() {
        32 if hash.chars().all(|c| c.is_ascii_hexdigit()) => HashType::Md5,
        40 if hash.chars().all(|c| c.is_ascii_hexdigit()) => HashType::Sha1,
        64 if hash.chars().all(|c| c.is_ascii_hexdigit()) => HashType::Sha256,
        128 if hash.chars().all(|c| c.is_ascii_hexdigit()) => HashType::Sha512,
        _ => HashType::Unknown,
    }
}

/// Generate MD5 hash (for testing, not for production)
pub fn md5_hash(input: &str) -> String {
    let digest = md5::compute(input.as_bytes());
    format!("{:x}", digest)
}

/// Generate SHA256 hash
pub fn sha256_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Juice Shop known credentials
pub fn juice_shop_credentials() -> Vec<Credential> {
    vec![
        Credential::new("admin@juice-sh.op", "admin123", "Admin user"),
        Credential::new("jim@juice-sh.op", "ncc-1701", "Jim - Star Trek reference"),
        Credential::new("bender@juice-sh.op", "OhG0dPlease1LubYou", "Bender"),
        Credential::new("mc.safesearch@juice-sh.op", "Mr. N00dles", "MC SafeSearch - song lyric"),
        Credential::new("testing@juice-sh.op", "IamUsedForTesting", "Testing account"),
        Credential::new("amy@juice-sh.op", "K1f.....................", "Amy - 21 char password"),
        Credential::new("bjoern.kimminich@gmail.com", "bW9jLmxpYW1nQGhjaW5pbW1pay5ucmVvamI=", "Bjoern - reversed Base64"),
        Credential::new("wurstbrot@juice-sh.op", "EinBansen", "Wurstbrot"),
    ]
}

/// Security question answers for Juice Shop
pub fn juice_shop_security_answers() -> Vec<SecurityAnswer> {
    vec![
        SecurityAnswer::new("bjoern@owasp.org", "ペットの名前", "Zaya"),
        SecurityAnswer::new("jim@juice-sh.op", "兄弟の名前", "Samuel"),
        SecurityAnswer::new("bender@juice-sh.op", "勤務先", "Stop'n'Drop"),
        SecurityAnswer::new("emma@juice-sh.op", "勤務先", "ITsec"),
        SecurityAnswer::new("john@juice-sh.op", "場所", "Daniel Boone National Forest"),
        SecurityAnswer::new("morty@juice-sh.op", "不明", "5N0wb41L"),
    ]
}

/// Credential with metadata
#[derive(Debug, Clone)]
pub struct Credential {
    pub email: String,
    pub password: String,
    pub description: String,
}

impl Credential {
    pub fn new(email: impl Into<String>, password: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
            description: description.into(),
        }
    }
}

/// Security question answer
#[derive(Debug, Clone)]
pub struct SecurityAnswer {
    pub email: String,
    pub question: String,
    pub answer: String,
}

impl SecurityAnswer {
    pub fn new(email: impl Into<String>, question: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            question: question.into(),
            answer: answer.into(),
        }
    }
}

/// Generate password variations from a base word
pub fn generate_variations(base: &str) -> Vec<String> {
    let mut variations = Vec::new();
    variations.push(base.to_string());
    variations.push(base.to_lowercase());
    variations.push(base.to_uppercase());

    // With numbers
    for suffix in &["1", "12", "123", "1234", "!", "!!", "@", "#", "1!", "123!"] {
        variations.push(format!("{}{}", base, suffix));
        variations.push(format!("{}{}", base.to_lowercase(), suffix));
    }

    // Capitalized
    if let Some(first) = base.chars().next() {
        let cap = format!("{}{}", first.to_uppercase(), &base[1..].to_lowercase());
        variations.push(cap.clone());
        variations.push(format!("{}1", cap));
        variations.push(format!("{}123", cap));
    }

    // Leet speak
    let leet = base
        .replace('a', "4")
        .replace('e', "3")
        .replace('i', "1")
        .replace('o', "0")
        .replace('s', "5")
        .replace('t', "7");
    variations.push(leet);

    variations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_passwords() {
        let passwords = top_passwords();
        assert!(!passwords.is_empty());
        assert!(passwords.iter().any(|p| p.password == "admin123"));
    }

    #[test]
    fn test_identify_hash() {
        assert_eq!(identify_hash("5d41402abc4b2a76b9719d911017c592"), HashType::Md5);
        assert_eq!(identify_hash("da39a3ee5e6b4b0d3255bfef95601890afd80709"), HashType::Sha1);
        assert_eq!(identify_hash("$2a$10$abcdefghij"), HashType::Bcrypt);
    }

    #[test]
    fn test_md5_hash() {
        let hash = md5_hash("admin123");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_sha256_hash() {
        let hash = sha256_hash("admin123");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_juice_shop_credentials() {
        let creds = juice_shop_credentials();
        assert!(creds.iter().any(|c| c.email == "admin@juice-sh.op"));
    }

    #[test]
    fn test_generate_variations() {
        let variations = generate_variations("admin");
        assert!(variations.contains(&"admin".to_string()));
        assert!(variations.contains(&"admin123".to_string()));
        assert!(variations.contains(&"ADMIN".to_string()));
    }
}
