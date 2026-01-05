// Allow dead_code for handlers that are defined for future use
#![allow(dead_code)]

mod auth;
mod callback;
mod consent;
mod dashboard;
mod health;
mod login;
mod logout;
mod pages;
pub mod platform;
pub mod tenant;

pub use auth::*;
pub use callback::*;
pub use consent::*;
pub use dashboard::*;
pub use health::*;
pub use login::*;
pub use logout::*;
pub use pages::*;
