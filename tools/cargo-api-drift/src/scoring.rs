use crate::issue::{ChangeKind, Classification, Issue, Severity};

pub use design_gate_core::Grade;

pub fn severity(classification: Classification, change_kind: ChangeKind) -> Severity {
    match classification {
        Classification::Breaking => match change_kind {
            ChangeKind::Removed
            | ChangeKind::FieldRemoved
            | ChangeKind::VariantRemoved
            | ChangeKind::TraitMethodAdded
            | ChangeKind::TraitMethodDefaultRemoved => Severity::Critical,
            _ => Severity::High,
        },
        Classification::Risky => match change_kind {
            ChangeKind::ReExportRemoved
            | ChangeKind::ReprChanged
            | ChangeKind::OrderChanged
            | ChangeKind::CfgChanged => Severity::High,
            _ => Severity::Medium,
        },
        Classification::Safe => Severity::Low,
    }
}

pub fn grade(issues: &[Issue]) -> Grade {
    design_gate_core::grade_for_severities(issues.iter().map(|issue| issue.severity))
}
