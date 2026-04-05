use crate::simulate::impact::{ChangeKind, RequiredChange};
use crate::simulate::transform::Transform;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Safety score for a proposed refactoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyScore {
    /// Overall score (0–100).
    pub total: u32,
    /// Structural safety — can the compiler verify correctness? (0–100)
    pub structural: u32,
    /// Semantic safety — does the ownership semantics change? (0–100)
    pub semantics: u32,
    /// Performance impact — does this add clones or allocations? (0–100)
    pub performance: u32,
    /// Warnings about specific changes.
    pub warnings: Vec<String>,
}

impl SafetyScore {
    pub fn compute(changes: &[RequiredChange], transform: &Transform) -> Self {
        let structural = Self::compute_structural(changes);
        let semantics = Self::compute_semantics(changes, transform);
        let performance = Self::compute_performance(changes);

        let total = (structural * 4 + semantics * 4 + performance * 2) / 10;

        let warnings = Self::collect_warnings(changes);

        SafetyScore {
            total,
            structural,
            semantics,
            performance,
            warnings,
        }
    }

    fn compute_structural(changes: &[RequiredChange]) -> u32 {
        // All changes are type-system-level and compiler-verifiable.
        // Lower score if there are many changes (more room for human error in review).
        if changes.is_empty() {
            return 100;
        }
        let penalty = (changes.len() as u32).min(20);
        100u32.saturating_sub(penalty)
    }

    fn compute_semantics(changes: &[RequiredChange], transform: &Transform) -> u32 {
        let mut score = 100u32;

        // Transforms that change ownership semantics are riskier.
        match transform {
            Transform::RefToOwned | Transform::OwnedToRef => score = score.saturating_sub(10),
            Transform::RefToMutRef => score = score.saturating_sub(15),
            _ => {}
        }

        // Moves are semantically significant.
        let move_count = changes
            .iter()
            .filter(|c| c.kind == ChangeKind::ConvertToMove)
            .count() as u32;
        score = score.saturating_sub(move_count * 3);

        // Mut borrow conflicts are serious.
        let conflict_count = changes
            .iter()
            .filter(|c| c.kind == ChangeKind::MutBorrowConflict)
            .count() as u32;
        score = score.saturating_sub(conflict_count * 10);

        score
    }

    fn compute_performance(changes: &[RequiredChange]) -> u32 {
        let mut score = 100u32;

        let clone_count = changes
            .iter()
            .filter(|c| c.kind == ChangeKind::AddClone)
            .count() as u32;
        score = score.saturating_sub(clone_count * 5);

        let to_owned_count = changes
            .iter()
            .filter(|c| c.kind == ChangeKind::AddToOwned)
            .count() as u32;
        score = score.saturating_sub(to_owned_count * 5);

        score
    }

    fn collect_warnings(changes: &[RequiredChange]) -> Vec<String> {
        let mut warnings = Vec::new();

        for change in changes {
            match change.kind {
                ChangeKind::ConvertToMove => {
                    warnings.push(format!(
                        "⚠ {} - moveに変換されるため、以降の使用は不可",
                        change.span,
                    ));
                }
                ChangeKind::MutBorrowConflict => {
                    warnings.push(format!(
                        "⚠ {} - 可変借用の排他制約に違反する可能性",
                        change.span,
                    ));
                }
                _ => {}
            }
        }

        warnings
    }
}

impl fmt::Display for SafetyScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "リファクタリング安全性スコア: {}/100", self.total)?;
        writeln!(f)?;
        writeln!(f, "  構造的安全性:     {}/100", self.structural)?;
        writeln!(f, "  セマンティクス:    {}/100", self.semantics)?;
        writeln!(f, "  パフォーマンス影響: {}/100", self.performance)?;
        if !self.warnings.is_empty() {
            writeln!(f)?;
            writeln!(f, "  注意箇所:")?;
            for w in &self.warnings {
                writeln!(f, "  {w}")?;
            }
        }
        Ok(())
    }
}
