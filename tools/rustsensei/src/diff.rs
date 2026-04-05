use crate::analyzer;
use crate::model::{DiffChange, DiffChangeType, DiffResult, OwnershipEvent};

/// Compare ownership flow between two versions of code.
pub fn compare(before_source: &str, after_source: &str) -> DiffResult {
    let before = analyzer::analyze(before_source);
    let after = analyzer::analyze(after_source);

    let changes = compute_changes(&before, &after);

    DiffResult {
        before,
        after,
        changes,
    }
}

fn compute_changes(
    before: &crate::model::AnalysisResult,
    after: &crate::model::AnalysisResult,
) -> Vec<DiffChange> {
    let mut changes = Vec::new();

    let before_moves = count_events(&before.steps, |e| matches!(e, OwnershipEvent::Move { .. }));
    let after_moves = count_events(&after.steps, |e| matches!(e, OwnershipEvent::Move { .. }));

    let before_borrows = count_events(&before.steps, |e| {
        matches!(e, OwnershipEvent::BorrowStart { .. })
    });
    let after_borrows = count_events(&after.steps, |e| {
        matches!(e, OwnershipEvent::BorrowStart { .. })
    });

    let before_clones = count_events(&before.steps, |e| matches!(e, OwnershipEvent::Clone { .. }));
    let after_clones = count_events(&after.steps, |e| matches!(e, OwnershipEvent::Clone { .. }));

    let before_errors = count_events(&before.steps, |e| {
        matches!(e, OwnershipEvent::CompileError { .. })
    });
    let after_errors = count_events(&after.steps, |e| {
        matches!(e, OwnershipEvent::CompileError { .. })
    });

    // Detect move changes
    if after_moves < before_moves {
        changes.push(DiffChange {
            description: format!(
                "Move が {} 箇所減りました ({} → {})",
                before_moves - after_moves,
                before_moves,
                after_moves
            ),
            change_type: DiffChangeType::MoveRemoved,
        });
    }

    // Detect borrow additions
    if after_borrows > before_borrows {
        changes.push(DiffChange {
            description: format!(
                "借用が {} 箇所増えました ({} → {})",
                after_borrows - before_borrows,
                before_borrows,
                after_borrows
            ),
            change_type: DiffChangeType::BorrowAdded,
        });
    }

    // Detect clone additions
    if after_clones > before_clones {
        changes.push(DiffChange {
            description: format!(
                "Clone が {} 箇所増えました ({} → {})",
                after_clones - before_clones,
                before_clones,
                after_clones
            ),
            change_type: DiffChangeType::CloneAdded,
        });
    }

    // Detect error fixes
    if before_errors > 0 && after_errors == 0 {
        changes.push(DiffChange {
            description: format!("{} 件の所有権エラーが修正されました", before_errors),
            change_type: DiffChangeType::ErrorFixed,
        });
    }

    // Detect new errors
    if after_errors > before_errors {
        changes.push(DiffChange {
            description: format!(
                "{} 件の新しい所有権エラーが発生しました",
                after_errors - before_errors
            ),
            change_type: DiffChangeType::ErrorIntroduced,
        });
    }

    changes
}

fn count_events(steps: &[crate::model::Step], pred: impl Fn(&OwnershipEvent) -> bool) -> usize {
    steps.iter().filter(|s| pred(&s.event)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_to_clone() {
        let before = r#"fn main() {
    let s = String::from("hello");
    let t = s;
}"#;
        let after = r#"fn main() {
    let s = String::from("hello");
    let t = s.clone();
}"#;
        let result = compare(before, after);
        assert!(
            result
                .changes
                .iter()
                .any(|c| c.change_type == DiffChangeType::MoveRemoved)
        );
        assert!(
            result
                .changes
                .iter()
                .any(|c| c.change_type == DiffChangeType::CloneAdded)
        );
    }

    #[test]
    fn test_move_to_borrow() {
        let before = r#"fn main() {
    let s = String::from("hello");
    let t = s;
}"#;
        let after = r#"fn main() {
    let s = String::from("hello");
    let t = &s;
}"#;
        let result = compare(before, after);
        assert!(
            result
                .changes
                .iter()
                .any(|c| c.change_type == DiffChangeType::MoveRemoved)
        );
        assert!(
            result
                .changes
                .iter()
                .any(|c| c.change_type == DiffChangeType::BorrowAdded)
        );
    }

    #[test]
    fn test_no_changes() {
        let code = r#"fn main() {
    let x = 42;
}"#;
        let result = compare(code, code);
        assert!(result.changes.is_empty());
    }
}
