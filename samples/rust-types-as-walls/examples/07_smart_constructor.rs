//! パターン3: Smart Constructor で制約を型に埋め込む。
//! コンストラクタを限定し、不正な値の `Email` や `CustomerName` は存在できないように。
//!
//! スライド「パターン3：Smart Constructorで制約を型に埋める」
//! 「複数の制約を組み合わせる」「Parse, don't validate」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use thiserror::Error;

// --- シンプル版: Email ---

#[derive(Debug, Clone)]
pub struct Email(String);

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("メールアドレスに @ がありません")]
    Invalid,
}

impl Email {
    pub fn new(s: &str) -> Result<Self, EmailError> {
        if !s.contains('@') {
            return Err(EmailError::Invalid);
        }
        Ok(Email(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// --- 複数制約版: CustomerName ---

#[derive(Debug, Clone)]
pub struct CustomerName {
    first: String,
    last: String,
}

#[derive(Debug, Error)]
pub enum NameError {
    #[error("名前が空です")]
    Empty,
    #[error("名前が長すぎます (50文字まで)")]
    TooLong,
}

impl CustomerName {
    pub fn new(first: &str, last: &str) -> Result<Self, NameError> {
        let trimmed_first = first.trim();
        let trimmed_last = last.trim();

        if trimmed_first.is_empty() || trimmed_last.is_empty() {
            return Err(NameError::Empty);
        }
        if trimmed_first.chars().count() > 50 || trimmed_last.chars().count() > 50 {
            return Err(NameError::TooLong);
        }
        Ok(CustomerName {
            first: trimmed_first.into(),
            last: trimmed_last.into(),
        })
    }

    pub fn full(&self) -> String {
        format!("{} {}", self.first, self.last)
    }
}

// --- Parse, don't validate ---
// validate は情報を型に残さない。parse は別の型に変換して情報を残す。

fn validate_email(s: &str) -> bool {
    s.contains('@')
}

fn parse_email(s: &str) -> Result<Email, EmailError> {
    Email::new(s)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Email
    let email = Email::new("user@example.com")?;
    println!("作成したメール: {}", email.as_str());

    match Email::new("invalid") {
        Err(e) => println!("不正な入力は弾かれる: {e}"),
        Ok(_) => unreachable!(),
    }

    // CustomerName
    let name = CustomerName::new("Yamada", "Taro")?;
    println!("顧客名: {}", name.full());

    assert!(matches!(
        CustomerName::new("", "Taro"),
        Err(NameError::Empty)
    ));
    assert!(matches!(
        CustomerName::new(&"a".repeat(51), "Taro"),
        Err(NameError::TooLong)
    ));

    // validate vs parse
    let input = "hello@example.com";
    let _is_valid: bool = validate_email(input);
    // `validate_email` の結果は bool。型の上では入力が有効なメールかどうかの情報が残らない。

    let parsed: Email = parse_email(input)?;
    // `parse_email` の結果は `Email` 型。型の上で「検証済み」が保証される。
    println!("parse後は型に情報が乗る: {}", parsed.as_str());

    Ok(())
}
