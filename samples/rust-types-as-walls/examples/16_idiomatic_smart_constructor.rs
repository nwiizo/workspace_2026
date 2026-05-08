//! `Email::new` ではなく `TryFrom<&str>` / `FromStr` を実装すると、
//! `parse()` や `collect::<Result<Vec<_>, _>>()` と自然に繋がる。
//!
//! 既存の Smart Constructor 例を、より現実的な Rust の書き味に寄せたもの。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rust_types_as_walls::idiomatic_email::{Email, EmailError};

#[derive(Debug)]
struct NewsletterRequest {
    owner_email: String,
    recipients: Vec<String>,
}

#[derive(Debug)]
struct Newsletter {
    owner_email: Email,
    recipients: Vec<Email>,
}

fn parse_request(req: NewsletterRequest) -> Result<Newsletter, EmailError> {
    let owner_email = Email::try_from(req.owner_email)?;
    let recipients = req
        .recipients
        .into_iter()
        .map(Email::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Newsletter {
        owner_email,
        recipients,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let owner = Email::try_from("owner@example.com")?;
    let support: Email = "support@example.com".parse()?;
    println!("TryFrom で作成: {owner}");
    println!("FromStr で作成: {support}");

    let request = NewsletterRequest {
        owner_email: "editor@example.com".into(),
        recipients: vec!["alice@example.com".into(), "bob@example.com".into()],
    };
    let newsletter = parse_request(request)?;
    let recipients = newsletter
        .recipients
        .iter()
        .map(Email::as_str)
        .collect::<Vec<_>>();
    println!(
        "配信リスト作成: owner={} recipients={:?}",
        newsletter.owner_email, recipients
    );

    let bad_request = NewsletterRequest {
        owner_email: "editor@example.com".into(),
        recipients: vec!["not-an-email".into()],
    };
    match parse_request(bad_request) {
        Err(error) => println!("境界で弾かれた: {error}"),
        Ok(_) => unreachable!(),
    }

    Ok(())
}
