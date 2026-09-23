//! Souther の保証を Rust の型と crate 構成で作る最小例。
//!
//! 領域 crate は `no_std` なので、時刻、ファイル、環境変数へは届かない。
//! 外界は [`trip::Clock`] のような trait として引数で受け取る。
#![no_std]

pub mod trip;

#[cfg(kani)]
mod proofs;
