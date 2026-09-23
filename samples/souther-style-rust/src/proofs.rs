//! Kani の証明。`cargo kani` のときだけコンパイルされる。
use crate::trip::{
    Amount, Clock, DateTime, Draft, EmployeeId, PRE_APPROVAL_LIMIT, Reason, SubmitOutcome, submit,
};

struct FixedClock(DateTime);

impl Clock for FixedClock {
    fn now(&self) -> DateTime {
        self.0
    }
}

/// `Amount` は負の値をすべて拒否し、0 以上をすべて受理する。
#[kani::proof]
fn amount_accepts_exactly_non_negative() {
    let raw: i64 = kani::any();
    assert_eq!(Amount::try_new(raw).is_ok(), raw >= 0);
}

/// どの申請額でも、上限以下なら提出され、上限超なら却下される。
#[kani::proof]
fn submit_splits_at_the_limit() {
    let raw: i64 = kani::any();
    kani::assume(raw >= 0);
    let applicant: u32 = kani::any();
    kani::assume(applicant > 0);
    let at = DateTime::new(kani::any());

    let draft = Draft {
        applicant: EmployeeId::try_new(applicant).unwrap(),
        planned_cost: Amount::try_new(raw).unwrap(),
    };
    match submit(draft, &FixedClock(at)) {
        SubmitOutcome::Submitted(s) => {
            assert!(raw <= PRE_APPROVAL_LIMIT);
            assert_eq!(s.planned_cost(), draft.planned_cost);
            assert_eq!(s.applicant(), draft.applicant);
            assert_eq!(s.submitted_at(), at);
        }
        SubmitOutcome::Rejected(r) => {
            assert!(raw > PRE_APPROVAL_LIMIT);
            assert_eq!(r.reason(), Reason::HighCost);
        }
    }
}
