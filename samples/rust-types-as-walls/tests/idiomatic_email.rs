#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests keep assertions and fixture setup direct"
)]

use rust_types_as_walls::idiomatic_email::{Email, EmailError};

#[test]
fn try_from_and_from_str_accept_valid_email() -> Result<(), EmailError> {
    let by_try_from = Email::try_from("owner@example.com")?;
    let by_parse: Email = "support@example.com".parse()?;

    assert_eq!(by_try_from.as_str(), "owner@example.com");
    assert_eq!(by_parse.as_str(), "support@example.com");

    Ok(())
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(Email::try_from(""), Err(EmailError::Empty));
    assert_eq!(
        Email::try_from("no-at-sign"),
        Err(EmailError::MissingOrTooManyAtSigns)
    );
    assert_eq!(
        Email::try_from("user@example"),
        Err(EmailError::DomainMissingDot)
    );
}

#[test]
fn generated_ascii_addresses_parse() {
    let locals = ["a", "alice", "first.last", "team-01"];
    let domains = ["example", "mail", "sample-site"];
    let tlds = ["com", "jp", "dev"];

    for local in locals {
        for domain in domains {
            for tld in tlds {
                let input = format!("{local}@{domain}.{tld}");
                let parsed = input
                    .parse::<Email>()
                    .map(|email| email.as_str().to_owned());
                assert_eq!(parsed, Ok(input));
            }
        }
    }
}

#[test]
fn strings_without_at_are_rejected() {
    let invalid_inputs = [
        "",
        "plain",
        "alice.example.com",
        "team example.com",
        "customer-id",
    ];

    for candidate in invalid_inputs {
        assert!(
            Email::try_from(candidate).is_err(),
            "{candidate} should be invalid"
        );
    }
}
