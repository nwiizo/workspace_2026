use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{Error as PasswordHashError, SaltString, rand_core::OsRng},
};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CredentialError {
    #[error("メールアドレスの形式が正しくありません")]
    InvalidEmail,
    #[error("パスワードは12文字以上128文字以下にしてください")]
    InvalidPasswordLength,
}

/// Normalizes an email address used as a local login identifier.
///
/// # Errors
///
/// Returns [`CredentialError::InvalidEmail`] when the address is empty,
/// ambiguous, contains whitespace, or exceeds common email length limits.
pub fn normalize_email(input: &str) -> Result<String, CredentialError> {
    let email = input.trim().to_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(CredentialError::InvalidEmail);
    };

    if email.chars().count() > 254
        || local.is_empty()
        || local.chars().count() > 64
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || email.chars().any(char::is_whitespace)
        || domain.contains('@')
    {
        return Err(CredentialError::InvalidEmail);
    }

    Ok(email)
}

/// Checks the local authentication password length policy.
///
/// # Errors
///
/// Returns [`CredentialError::InvalidPasswordLength`] unless the password is
/// between 12 and 128 Unicode scalar values.
pub fn validate_password(password: &str) -> Result<(), CredentialError> {
    if (12..=128).contains(&password.chars().count()) {
        Ok(())
    } else {
        Err(CredentialError::InvalidPasswordLength)
    }
}

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("パスワードを安全に処理できませんでした")]
    Hash,
    #[error("パスワード処理タスクが終了しました")]
    Worker,
}

/// Hashes a password with Argon2id outside the async executor thread.
///
/// # Errors
///
/// Returns [`PasswordError`] if salt generation, hashing, or the blocking task
/// fails. The password is never included in the error.
pub async fn hash_password(password: String) -> Result<String, PasswordError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| PasswordError::Hash)
    })
    .await
    .map_err(|_| PasswordError::Worker)?
}

/// Verifies a password against an encoded Argon2 hash.
///
/// # Errors
///
/// Returns [`PasswordError`] for a malformed stored hash or a worker failure.
/// An ordinary password mismatch returns `Ok(false)`.
pub async fn verify_password(
    password: String,
    encoded_hash: String,
) -> Result<bool, PasswordError> {
    tokio::task::spawn_blocking(move || {
        let hash = PasswordHash::new(&encoded_hash).map_err(|_| PasswordError::Hash)?;
        match Argon2::default().verify_password(password.as_bytes(), &hash) {
            Ok(()) => Ok(true),
            Err(PasswordHashError::Password) => Ok(false),
            Err(_) => Err(PasswordError::Hash),
        }
    })
    .await
    .map_err(|_| PasswordError::Worker)?
}
