#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests keep assertions and fixture setup direct"
)]

use proptest::prelude::*;
use proptest::string::string_regex;
use rust_types_as_walls::password::Password;

fn candidate_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..96)
        .prop_map(|chars| chars.into_iter().collect::<String>())
}

fn valid_password_strategy() -> impl Strategy<Value = String> {
    (
        string_regex("[a-z]{6,24}").expect("regex should compile"),
        string_regex("[0-9]{1,8}").expect("regex should compile"),
        string_regex("[A-Za-z0-9!-]{5,24}").expect("regex should compile"),
    )
        .prop_map(|(letters, digits, suffix)| format!("{letters}{digits}{suffix}"))
}

proptest! {
    #[test]
    fn accepted_passwords_always_satisfy_invariants(candidate in candidate_strategy()) {
        if let Ok(password) = Password::<12>::new(candidate) {
            prop_assert!(password.len() >= 12);
            prop_assert!(password.len() <= Password::<12>::MAX_LEN);
            prop_assert!(!password.as_str().chars().any(char::is_whitespace));
            prop_assert!(password.has_letter());
            prop_assert!(password.has_digit());
        }
    }

    #[test]
    fn generated_valid_passwords_are_accepted(candidate in valid_password_strategy()) {
        let parsed = Password::<12>::new(candidate.clone());
        prop_assert_eq!(parsed.as_ref().map(Password::as_str), Ok(candidate.as_str()));
    }
}
