//! const generic + Smart Constructor。
//! 最小長のような「環境や用途で変わる制約」を型引数に逃がしつつ、
//! 実際の検証は Smart Constructor に閉じ込める。
//!
//! 性質検証は `tests/password_props.rs` の proptest で行う。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rust_types_as_walls::password::Password;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_password = Password::<12>::new("RustFest2026")?;
    let admin_password = Password::<20>::new("AdminPassword2026Lock")?;

    println!(
        "user password: len={} has_digit={}",
        user_password.len(),
        user_password.has_digit()
    );
    println!(
        "admin password: len={} has_letter={}",
        admin_password.len(),
        admin_password.has_letter()
    );

    assert!(Password::<12>::new("short7").is_err());
    assert!(Password::<12>::new("onlylettersonly").is_err());
    assert!(Password::<12>::new("123456789012").is_err());

    // 次の行のコメントを外すと、型が違うので代入できない。
    // let escalated: Password<20> = user_password;

    Ok(())
}
