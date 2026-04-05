#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

pub mod analysis;
pub mod config;
pub mod cost;
pub mod driver;
pub mod error;
pub mod report;
