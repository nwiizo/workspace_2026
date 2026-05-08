//! `TryFrom<&str>` / `FromStr` を使った慣用的な Smart Constructor。

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EmailError {
    #[error("メールアドレスが空です")]
    Empty,
    #[error("メールアドレスに空白は使えません")]
    ContainsWhitespace,
    #[error("メールアドレスには @ が1つ必要です")]
    MissingOrTooManyAtSigns,
    #[error("ローカル部が空です")]
    EmptyLocalPart,
    #[error("ドメイン部が空です")]
    EmptyDomain,
    #[error("ドメインに . がありません")]
    DomainMissingDot,
}

impl Email {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for Email {
    type Error = EmailError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let candidate = value.trim();
        if candidate.is_empty() {
            return Err(EmailError::Empty);
        }
        if candidate.chars().any(char::is_whitespace) {
            return Err(EmailError::ContainsWhitespace);
        }

        let Some((local, domain)) = candidate.split_once('@') else {
            return Err(EmailError::MissingOrTooManyAtSigns);
        };
        if domain.contains('@') {
            return Err(EmailError::MissingOrTooManyAtSigns);
        }
        if local.is_empty() {
            return Err(EmailError::EmptyLocalPart);
        }
        if domain.is_empty() {
            return Err(EmailError::EmptyDomain);
        }
        if !domain.contains('.') {
            return Err(EmailError::DomainMissingDot);
        }

        Ok(Self(candidate.to_owned()))
    }
}

impl TryFrom<String> for Email {
    type Error = EmailError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl FromStr for Email {
    type Err = EmailError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}
