use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IssueKey {
    pub issue_type: String,
    pub source: String,
    pub target: String,
}

impl IssueKey {
    pub fn new(
        issue_type: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            issue_type: issue_type.into(),
            source: source.into(),
            target: target.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueSetDiff<T> {
    pub new_issues: Vec<T>,
    pub resolved_issues: Vec<T>,
    pub unchanged: usize,
}

pub fn diff_issue_sets<T, F>(baseline: &[T], current: &[T], key: F) -> IssueSetDiff<T>
where
    T: Clone,
    F: Fn(&T) -> IssueKey + Copy,
{
    let baseline_keys: BTreeSet<IssueKey> = baseline.iter().map(key).collect();
    let current_keys: BTreeSet<IssueKey> = current.iter().map(key).collect();
    IssueSetDiff {
        new_issues: unique_by_key(current, |issue| key(issue), &baseline_keys),
        resolved_issues: unique_by_key(baseline, |issue| key(issue), &current_keys),
        unchanged: baseline_keys.intersection(&current_keys).count(),
    }
}

pub fn unique_by_key<T, F>(issues: &[T], key: F, known: &BTreeSet<IssueKey>) -> Vec<T>
where
    T: Clone,
    F: Fn(&T) -> IssueKey,
{
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for issue in issues {
        let issue_key = key(issue);
        if known.contains(&issue_key) || !seen.insert(issue_key) {
            continue;
        }
        out.push(issue.clone());
    }
    out
}

pub fn sort_dedup_by_key<T, F>(issues: &mut Vec<T>, key: F)
where
    F: Fn(&T) -> IssueKey,
{
    issues.sort_by_key(|issue| key(issue));
    issues.dedup_by(|left, right| key(left) == key(right));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_is_stable_and_deduplicated() {
        let old = vec![
            IssueKey::new("b", "src", "old"),
            IssueKey::new("a", "src", "same"),
        ];
        let new = vec![
            IssueKey::new("a", "src", "same"),
            IssueKey::new("c", "src", "new"),
            IssueKey::new("c", "src", "new"),
        ];
        let diff = diff_issue_sets(&old, &new, Clone::clone);
        assert_eq!(diff.unchanged, 1);
        assert_eq!(diff.new_issues, vec![IssueKey::new("c", "src", "new")]);
        assert_eq!(diff.resolved_issues, vec![IssueKey::new("b", "src", "old")]);
    }
}
