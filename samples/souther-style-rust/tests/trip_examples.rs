//! Souther の `example` 行に対応する例表と、その網羅性の検査。
//!
//! 例表を 1 つの定数に置き、例の実行と網羅性の検査が同じ行を読む。
use std::collections::HashSet;

use souther_style_rust::trip::{
    Amount, Clock, DateTime, Draft, EmployeeId, PRE_APPROVAL_LIMIT, Reason, SubmitKind,
    SubmitOutcome, submit,
};
use strum::IntoEnumIterator;

/// `fake` 表に対応する時計。
struct FixedClock(DateTime);

impl Clock for FixedClock {
    fn now(&self) -> DateTime {
        self.0
    }
}

struct Row {
    name: &'static str,
    cost: i64,
    expected: SubmitKind,
}

const ROWS: &[Row] = &[
    Row {
        name: "zero",
        cost: 0,
        expected: SubmitKind::Submitted,
    },
    Row {
        name: "under",
        cost: PRE_APPROVAL_LIMIT - 1,
        expected: SubmitKind::Submitted,
    },
    // border の ON 点。この行が無いと `>` を `>=` に変えた実装を区別できない
    #[cfg(not(feature = "without-border"))]
    Row {
        name: "on_point",
        cost: PRE_APPROVAL_LIMIT,
        expected: SubmitKind::Submitted,
    },
    Row {
        name: "off_point",
        cost: PRE_APPROVAL_LIMIT + 1,
        expected: SubmitKind::Rejected,
    },
];

const SUBMITTED_AT: u64 = 1_785_142_800; // 2026-07-27T09:00:00Z

fn draft_with(cost: i64) -> Draft {
    Draft {
        applicant: EmployeeId::try_new(1).expect("1 is a valid employee id"),
        planned_cost: Amount::try_new(cost).expect("example costs are non-negative"),
    }
}

#[test]
fn submit_rows() {
    let clock = FixedClock(DateTime::new(SUBMITTED_AT));
    for row in ROWS {
        let draft = draft_with(row.cost);
        let out = submit(draft, &clock);
        assert_eq!(SubmitKind::from(&out), row.expected, "row {}", row.name);
        match out {
            SubmitOutcome::Submitted(s) => {
                assert_eq!(s.applicant(), draft.applicant, "row {}", row.name);
                assert_eq!(s.planned_cost(), draft.planned_cost, "row {}", row.name);
                assert_eq!(s.submitted_at(), clock.now(), "row {}", row.name);
            }
            SubmitOutcome::Rejected(r) => {
                assert_eq!(r.reason(), Reason::HighCost, "row {}", row.name);
            }
        }
    }
}

/// signature 網羅性: 出力の全ケースに少なくとも 1 行ある。
#[test]
fn every_outcome_has_a_row() {
    let observed: HashSet<SubmitKind> = ROWS.iter().map(|r| r.expected).collect();
    let specified: HashSet<SubmitKind> = SubmitKind::iter().collect();
    assert_eq!(observed, specified);
}

/// 境界の decoder: JSON から復元するときも不変条件が検査される。
#[test]
fn boundary_rejects_invariant_violations() {
    let ok: Draft =
        serde_json::from_str(r#"{"applicant":1,"planned_cost":500}"#).expect("valid draft decodes");
    assert_eq!(ok.planned_cost.into_inner(), 500);

    assert!(serde_json::from_str::<Draft>(r#"{"applicant":1,"planned_cost":-1}"#).is_err());
    assert!(serde_json::from_str::<Draft>(r#"{"applicant":0,"planned_cost":500}"#).is_err());
}
