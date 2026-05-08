//! sealed trait は「この trait を実装できる型集合」をライブラリ側で閉じる。
//! enum はもともと closed world だが、trait は放っておくと外部 crate から増やせる。
//! そこで private trait を supertrait にして、拡張点を意図的に制限する。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rust_types_as_walls::sealed_payment::{Authorized, Payment, PaymentMethod, audit_line};

fn main() {
    let authorized = Payment::<Authorized>::authorize("pay_001", PaymentMethod::Card);
    println!("認可直後: {}", audit_line(&authorized));

    let captured = authorized.capture();
    println!("売上計上後: {}", audit_line(&captured));

    let bank_transfer = Payment::<Authorized>::authorize("pay_002", PaymentMethod::BankTransfer);
    println!("別メソッド: {}", audit_line(&bank_transfer));

    // `PaymentMethod` は enum なので、外部 crate からバリアントを追加できない。
    // `PaymentState` は trait だが sealed にしてあるので、やはり外部 crate から実装できない。
    // どこまで拡張を許すかを API 側で決められる。
}
