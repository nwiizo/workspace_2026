//! Executable, dependency-free teaching implementations across all twelve chapters
//! of Grokking AI Algorithms, Second Edition. Stochastic routines accept seeds.
//!
//! These are teaching implementations, not production machine-learning systems.

pub mod autodiff;
pub mod boosting;
pub mod clustering;
pub mod diffusion;
pub mod evolution;
pub mod language;
pub mod learning;
pub mod neural;
pub mod numeric;
pub mod optimization;
pub mod random;
pub mod reinforcement;
pub mod search;
pub mod supervised;
pub mod swarm;
pub mod training;
pub mod tuning;
pub mod vision;

pub type Result<T> = std::result::Result<T, &'static str>;
