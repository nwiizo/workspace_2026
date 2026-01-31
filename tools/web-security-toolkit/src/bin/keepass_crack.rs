//! KeePass KDBX Password Cracker CLI
//!
//! Usage:
//!   keepass-crack <kdbx-file> [options]
//!
//! Example:
//!   keepass-crack database.kdbx
//!   keepass-crack database.kdbx --wordlist passwords.txt
//!   keepass-crack database.kdbx --extended
//!   keepass-crack database.kdbx --keyfile image.jpg --password "test"

use clap::{Parser, Subcommand};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use web_security_toolkit::keepass::{
    common_passwords, decrypt_kdbx, extended_passwords, extract_entries, try_password,
    try_password_with_keyfile, KdbxFile,
};

#[derive(Parser)]
#[command(name = "keepass-crack")]
#[command(about = "KeePass KDBX Password Cracker for CTF")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show information about a KDBX file
    Info {
        /// Path to the KDBX file
        file: PathBuf,
    },
    /// Crack a KDBX file
    Crack {
        /// Path to the KDBX file
        file: PathBuf,

        /// Path to key file (optional, used together with password)
        #[arg(short, long)]
        keyfile: Option<PathBuf>,

        /// Path to wordlist file (one password per line)
        #[arg(short = 'w', long)]
        wordlist: Option<PathBuf>,

        /// Use extended password list (slower but more thorough)
        #[arg(short, long)]
        extended: bool,

        /// Try a single password
        #[arg(short, long)]
        password: Option<String>,

        /// Show progress every N attempts
        #[arg(long, default_value = "100")]
        progress: usize,
    },
    /// Generate a sample wordlist
    Wordlist {
        /// Use extended list
        #[arg(short, long)]
        extended: bool,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Decrypt a KDBX file and show contents
    Decrypt {
        /// Path to the KDBX file
        file: PathBuf,

        /// Master password
        #[arg(short, long)]
        password: String,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Extract entries (credentials) from a KDBX file
    Extract {
        /// Path to the KDBX file
        file: PathBuf,

        /// Master password
        #[arg(short, long)]
        password: String,

        /// Output format (table, json, csv)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => {
            let data = match fs::read(&file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            match KdbxFile::parse(&data) {
                Ok(kdbx) => {
                    println!("File: {}", file.display());
                    println!("{}", kdbx.info());
                    println!("\nHeader details:");
                    println!("  Master seed: {} bytes", kdbx.header.master_seed.len());
                    println!(
                        "  Transform seed: {} bytes",
                        kdbx.header.transform_seed.len()
                    );
                    println!("  Encryption IV: {} bytes", kdbx.header.encryption_iv.len());
                    println!(
                        "  Stream start bytes: {} bytes",
                        kdbx.header.stream_start_bytes.len()
                    );

                    if kdbx.header.transform_rounds <= 10 {
                        println!(
                            "\n⚠️  WARNING: Very low transform rounds ({})!",
                            kdbx.header.transform_rounds
                        );
                        println!("   This file should be easy to crack.");
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing KDBX: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Crack {
            file,
            keyfile,
            wordlist,
            extended,
            password,
            progress,
        } => {
            let data = match fs::read(&file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            let kdbx = match KdbxFile::parse(&data) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error parsing KDBX: {}", e);
                    std::process::exit(1);
                }
            };

            // Load key file if provided
            let keyfile_data: Option<Vec<u8>> =
                keyfile.as_ref().map(|kf_path| match fs::read(kf_path) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error reading keyfile: {}", e);
                        std::process::exit(1);
                    }
                });

            println!("KDBX Info:");
            println!("{}", kdbx.info());
            if let Some(kf) = &keyfile {
                println!("Key file: {}", kf.display());
            }
            println!();

            // Build password list
            let passwords: Vec<String> = if let Some(pw) = password {
                vec![pw]
            } else if let Some(ref wordlist_path) = wordlist {
                let file = match fs::File::open(wordlist_path) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Error opening wordlist: {}", e);
                        std::process::exit(1);
                    }
                };
                BufReader::new(file).lines().map_while(Result::ok).collect()
            } else if extended {
                extended_passwords()
            } else {
                common_passwords()
            };

            println!("Attempting {} passwords...\n", passwords.len());

            // Crack with or without keyfile
            let result: Option<String> = if let Some(ref kf_data) = keyfile_data {
                crack_with_keyfile(&kdbx, &passwords, kf_data, progress)
            } else {
                crack_without_keyfile(&kdbx, &passwords, progress)
            };

            match result {
                Some(password) => {
                    println!("\n✅ PASSWORD FOUND: {}", password);
                    if keyfile.is_some() {
                        println!("\nOpen with KeePass using password + keyfile.");
                    } else {
                        println!(
                            "\nYou can now open the KDBX file with KeePass using this password."
                        );
                    }
                }
                None => {
                    println!("\n❌ Password not found in wordlist.");
                    if !extended && wordlist.is_none() {
                        println!("   Try using --extended for a larger wordlist.");
                    }
                    if keyfile.is_none() {
                        println!("   This file may require a key file (--keyfile).");
                    }
                }
            }
        }

        Commands::Wordlist { extended, output } => {
            let passwords = if extended {
                extended_passwords()
            } else {
                common_passwords()
            };

            let content = passwords.join("\n");

            if let Some(path) = output {
                match fs::write(&path, content) {
                    Ok(_) => println!("Wrote {} passwords to {}", passwords.len(), path.display()),
                    Err(e) => {
                        eprintln!("Error writing file: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                println!("{}", content);
            }
        }

        Commands::Decrypt {
            file,
            password,
            output,
        } => {
            let data = match fs::read(&file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            let kdbx = match KdbxFile::parse(&data) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error parsing KDBX: {}", e);
                    std::process::exit(1);
                }
            };

            println!("Decrypting KDBX...");

            match decrypt_kdbx(&kdbx, &password) {
                Ok(content) => {
                    if let Some(path) = output {
                        match fs::write(&path, &content) {
                            Ok(_) => println!("✅ Decrypted content saved to {}", path.display()),
                            Err(e) => {
                                eprintln!("Error writing file: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        // Try to display as UTF-8, otherwise show hex
                        match String::from_utf8(content.clone()) {
                            Ok(s) => println!("{}", s),
                            Err(_) => {
                                println!("Binary content ({} bytes):", content.len());
                                println!(
                                    "{}",
                                    hex::encode(&content[..std::cmp::min(500, content.len())])
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Decryption failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Extract {
            file,
            password,
            format,
        } => {
            let data = match fs::read(&file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            let kdbx = match KdbxFile::parse(&data) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error parsing KDBX: {}", e);
                    std::process::exit(1);
                }
            };

            match extract_entries(&kdbx, &password) {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("No entries found in database.");
                        return;
                    }

                    match format.as_str() {
                        "json" => {
                            let json_entries: Vec<serde_json::Value> = entries
                                .iter()
                                .map(|e| {
                                    serde_json::json!({
                                        "title": e.title,
                                        "username": e.username,
                                        "password": e.password,
                                        "url": e.url,
                                        "notes": e.notes
                                    })
                                })
                                .collect();
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json_entries).unwrap_or_default()
                            );
                        }
                        "csv" => {
                            println!("title,username,password,url,notes");
                            for e in &entries {
                                println!(
                                    "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                                    e.title.replace('"', "\"\""),
                                    e.username.replace('"', "\"\""),
                                    e.password.replace('"', "\"\""),
                                    e.url.replace('"', "\"\""),
                                    e.notes.replace('"', "\"\"").replace('\n', "\\n")
                                );
                            }
                        }
                        _ => {
                            // Table format (default)
                            println!("Found {} entries:\n", entries.len());
                            println!("{:<20} {:<25} {:<30} URL", "Title", "Username", "Password");
                            println!("{}", "-".repeat(100));
                            for e in &entries {
                                println!(
                                    "{:<20} {:<25} {:<30} {}",
                                    truncate(&e.title, 20),
                                    truncate(&e.username, 25),
                                    truncate(&e.password, 30),
                                    truncate(&e.url, 40)
                                );
                                if !e.notes.is_empty() {
                                    println!("  Notes: {}", truncate(&e.notes, 80));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Extraction failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn crack_with_keyfile(
    kdbx: &KdbxFile,
    passwords: &[String],
    keyfile_data: &[u8],
    progress_interval: usize,
) -> Option<String> {
    for (i, pw) in passwords.iter().enumerate() {
        if i % progress_interval == 0 {
            println!("[{}/{}] Trying: {}", i + 1, passwords.len(), pw);
        }
        if try_password_with_keyfile(kdbx, pw, keyfile_data) {
            return Some(pw.clone());
        }
    }
    None
}

fn crack_without_keyfile(
    kdbx: &KdbxFile,
    passwords: &[String],
    progress_interval: usize,
) -> Option<String> {
    for (i, pw) in passwords.iter().enumerate() {
        if i % progress_interval == 0 {
            println!("[{}/{}] Trying: {}", i + 1, passwords.len(), pw);
        }
        if try_password(kdbx, pw) {
            return Some(pw.clone());
        }
    }
    None
}
