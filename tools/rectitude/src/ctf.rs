//! CTF Challenge Verification Module
//!
//! Provides traits for verifying challenge completion in CTF platforms.
//! Implement `ChallengeVerifier` for your specific CTF platform.
//!
//! # Example
//!
//! ```ignore
//! use rectitude::ctf::{ChallengeVerifier, ChallengeProgress};
//! use async_trait::async_trait;
//!
//! struct MyCtfVerifier { /* ... */ }
//!
//! #[async_trait]
//! impl ChallengeVerifier for MyCtfVerifier {
//!     async fn is_solved(&self, challenge_key: &str) -> rectitude::Result<bool> {
//!         // Check your CTF platform's API
//!         Ok(true)
//!     }
//!
//!     async fn get_progress(&self) -> rectitude::Result<ChallengeProgress> {
//!         // Fetch progress from your CTF platform
//!         Ok(ChallengeProgress::new(5, 10, Default::default()))
//!     }
//! }
//! ```

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Progress information for a CTF platform
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChallengeProgress {
    /// Total number of challenges
    pub total: usize,
    /// Number of solved challenges
    pub solved: usize,
    /// Percentage of challenges solved
    pub percentage: f64,
    /// Individual challenge states (key -> solved)
    pub challenges: HashMap<String, bool>,
}

impl ChallengeProgress {
    /// Create new progress from totals
    pub fn new(solved: usize, total: usize, challenges: HashMap<String, bool>) -> Self {
        let percentage = if total > 0 {
            (solved as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        Self {
            total,
            solved,
            percentage,
            challenges,
        }
    }

    /// Get list of unsolved challenge keys
    pub fn unsolved(&self) -> Vec<&String> {
        self.challenges
            .iter()
            .filter(|(_, solved)| !**solved)
            .map(|(key, _)| key)
            .collect()
    }

    /// Get list of solved challenge keys
    pub fn solved_list(&self) -> Vec<&String> {
        self.challenges
            .iter()
            .filter(|(_, solved)| **solved)
            .map(|(key, _)| key)
            .collect()
    }
}

/// Trait for CTF platform challenge verification
///
/// Implement this trait for your specific CTF platform to enable
/// automatic challenge verification in scenario tests.
#[async_trait]
pub trait ChallengeVerifier: Send + Sync {
    /// Check if a specific challenge is solved
    async fn is_solved(&self, challenge_key: &str) -> Result<bool>;

    /// Get overall progress
    async fn get_progress(&self) -> Result<ChallengeProgress>;

    /// Get count of solved challenges
    async fn solved_count(&self) -> Result<usize> {
        Ok(self.get_progress().await?.solved)
    }

    /// Compare progress before and after an operation
    ///
    /// Returns a list of challenge keys that were newly solved.
    async fn compare_progress(&self, before: &ChallengeProgress) -> Result<Vec<String>> {
        let after = self.get_progress().await?;
        let newly_solved: Vec<String> = after
            .challenges
            .iter()
            .filter(|(key, solved)| {
                **solved && !before.challenges.get(*key).copied().unwrap_or(false)
            })
            .map(|(key, _)| key.clone())
            .collect();
        Ok(newly_solved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_progress() {
        let mut challenges = HashMap::new();
        challenges.insert("challenge1".to_string(), true);
        challenges.insert("challenge2".to_string(), false);
        challenges.insert("challenge3".to_string(), true);

        let progress = ChallengeProgress::new(2, 3, challenges);

        assert_eq!(progress.solved, 2);
        assert_eq!(progress.total, 3);
        assert!((progress.percentage - 66.67).abs() < 1.0);
        assert_eq!(progress.unsolved().len(), 1);
        assert_eq!(progress.solved_list().len(), 2);
    }

    #[test]
    fn test_default_progress() {
        let progress = ChallengeProgress::default();
        assert_eq!(progress.total, 0);
        assert_eq!(progress.solved, 0);
    }
}
