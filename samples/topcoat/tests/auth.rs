use tatami_log::auth::{CredentialError, normalize_email, validate_password};

#[test]
fn email_is_normalized_without_accepting_ambiguous_input() {
    assert_eq!(
        normalize_email("  Alice@Example.TEST "),
        Ok("alice@example.test".to_owned())
    );
    assert_eq!(
        normalize_email("alice example.test"),
        Err(CredentialError::InvalidEmail)
    );
    assert_eq!(
        normalize_email("alice@"),
        Err(CredentialError::InvalidEmail)
    );
}

#[test]
fn password_length_uses_characters_and_has_an_upper_bound() {
    assert_eq!(validate_password("十分に長いパスワードです"), Ok(()));
    assert_eq!(
        validate_password("short"),
        Err(CredentialError::InvalidPasswordLength)
    );
    assert_eq!(
        validate_password(&"a".repeat(129)),
        Err(CredentialError::InvalidPasswordLength)
    );
}
