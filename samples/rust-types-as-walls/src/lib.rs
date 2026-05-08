//! 関数型まつり2026 公募セッション「型は壁、Rustでもバグを直すな、表現できなくせよ」
//! スライド中のサンプルコードを実コンパイル可能な形で検証するためのライブラリ。
//!
//! 各サンプルは `examples/` 配下に自己完結した形で配置されています。
//! このライブラリ本体は共通の型エイリアスのみを提供します。

pub mod api_evolution;
pub mod customer_id;
pub mod idiomatic_email;
pub mod order_service;
pub mod password;
pub mod sealed_payment;

/// 金額は最小通貨単位（例: 円、セント）を符号なし整数で表す。
pub type Money = u64;
