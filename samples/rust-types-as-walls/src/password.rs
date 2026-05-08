//! const generic と Smart Constructor を組み合わせたパスワード型。

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Password<const MIN: usize>(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PasswordError {
    #[error("パスワードは {min} 文字以上必要です (actual={actual})")]
    TooShort { min: usize, actual: usize },
    #[error("パスワードは {max} 文字以下でなければなりません")]
    TooLong { max: usize },
    #[error("パスワードに空白は使えません")]
    ContainsWhitespace,
    #[error("パスワードには英字を 1 文字以上含めてください")]
    MissingLetter,
    #[error("パスワードには数字を 1 文字以上含めてください")]
    MissingDigit,
}

impl<const MIN: usize> Password<MIN> {
    pub const MAX_LEN: usize = 72;

    pub fn new(value: impl Into<String>) -> Result<Self, PasswordError> {
        let password = value.into();
        let len = password.chars().count();

        if len < MIN {
            return Err(PasswordError::TooShort {
                min: MIN,
                actual: len,
            });
        }
        if len > Self::MAX_LEN {
            return Err(PasswordError::TooLong { max: Self::MAX_LEN });
        }
        if password.chars().any(char::is_whitespace) {
            return Err(PasswordError::ContainsWhitespace);
        }
        if !password.chars().any(|ch| ch.is_ascii_alphabetic()) {
            return Err(PasswordError::MissingLetter);
        }
        if !password.chars().any(|ch| ch.is_ascii_digit()) {
            return Err(PasswordError::MissingDigit);
        }

        Ok(Self(password))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn has_letter(&self) -> bool {
        self.0.chars().any(|ch| ch.is_ascii_alphabetic())
    }

    pub fn has_digit(&self) -> bool {
        self.0.chars().any(|ch| ch.is_ascii_digit())
    }
}
