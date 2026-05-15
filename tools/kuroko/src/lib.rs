//! kuroko — a lightweight AWS service emulator in Rust.
//!
//! See `README.md` for design notes and the supported-service matrix.

pub mod aws_error;
pub mod config;
pub mod persistence;
pub mod protocol;
pub mod registry;
pub mod server;
pub mod service;
pub mod services;
