//! Builder と Smart Constructor の役割分担。
//! Builder は「未完成でまだ不正かもしれない入力」を集める場所、
//! Smart Constructor は「完成したら不変条件を満たす値だけを返す場所」。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rust_types_as_walls::idiomatic_email::{Email, EmailError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DisplayName(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
enum DisplayNameError {
    #[error("表示名は空にできません")]
    Empty,
    #[error("表示名は 40 文字以内です")]
    TooLong,
}

impl DisplayName {
    fn new(value: impl Into<String>) -> Result<Self, DisplayNameError> {
        let raw_value = value.into();
        let trimmed = raw_value.trim();

        if trimmed.is_empty() {
            return Err(DisplayNameError::Empty);
        }
        if trimmed.chars().count() > 40 {
            return Err(DisplayNameError::TooLong);
        }

        Ok(Self(trimmed.to_owned()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
struct RegisteredUser {
    email: Email,
    display_name: DisplayName,
    tags: Vec<String>,
}

#[derive(Debug, Default)]
struct UserRegistrationBuilder {
    email: Option<String>,
    display_name: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Error)]
enum BuildError {
    #[error("email が未設定です")]
    MissingEmail,
    #[error("display_name が未設定です")]
    MissingDisplayName,
    #[error(transparent)]
    InvalidEmail(#[from] EmailError),
    #[error(transparent)]
    InvalidDisplayName(#[from] DisplayNameError),
}

impl UserRegistrationBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn email(&mut self, value: impl Into<String>) -> &mut Self {
        self.email = Some(value.into());
        self
    }

    fn display_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.display_name = Some(value.into());
        self
    }

    fn push_tag(&mut self, value: impl Into<String>) -> &mut Self {
        self.tags.push(value.into());
        self
    }

    fn build(self) -> Result<RegisteredUser, BuildError> {
        let email = self.email.ok_or(BuildError::MissingEmail)?;
        let display_name = self.display_name.ok_or(BuildError::MissingDisplayName)?;

        Ok(RegisteredUser {
            email: Email::try_from(email)?,
            display_name: DisplayName::new(display_name)?,
            tags: self.tags,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = UserRegistrationBuilder::new();
    builder
        .email("owner@example.com")
        .display_name("型で守る太郎")
        .push_tag("speaker")
        .push_tag("rust");

    let user = builder.build()?;
    println!(
        "registered: email={} display_name={} tags={:?}",
        user.email,
        user.display_name.as_str(),
        user.tags
    );

    let mut bad_builder = UserRegistrationBuilder::new();
    bad_builder.email("not-an-email").display_name("  ");
    assert!(bad_builder.build().is_err());

    // Builder の途中状態は不正でもよいが、
    // `RegisteredUser` は build() を通らないと作れない。

    Ok(())
}
