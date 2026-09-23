//! 出張申請の提出。Souther の `behavior submit` に対応する。
use nutype::nutype;
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter};

/// 事前承認なしで提出できる申請額の上限 (円)。
pub const PRE_APPROVAL_LIMIT: i64 = 100_000;

/// 申請額。`data Amount = Int invariant value >= 0` に対応する。
#[nutype(
    validate(greater_or_equal = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct Amount(i64);

/// 社員番号。0 は表せない。
#[nutype(
    validate(greater = 0),
    derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)
)]
pub struct EmployeeId(u32);

/// 外界から受け取る時刻 (Unix 秒)。領域側では作らない。
#[nutype(derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize))]
pub struct DateTime(u64);

/// 提出前の申請。境界で JSON から復元するとき、`Amount` の不変条件も検査される。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Draft {
    pub applicant: EmployeeId,
    pub planned_cost: Amount,
}

/// 提出済みの申請。`constructs Submitted` に対応する。
///
/// フィールドが非公開なので、このモジュールの外では組み立てられない。
///
/// ```
/// use souther_style_rust::trip::{Amount, DateTime, EmployeeId};
/// let _ = (EmployeeId::try_new(1), Amount::try_new(1), DateTime::new(0));
/// ```
///
/// ```compile_fail,E0451
/// use souther_style_rust::trip::{Amount, DateTime, EmployeeId, Submitted};
/// let _ = Submitted {
///     applicant: EmployeeId::try_new(1).unwrap(),
///     planned_cost: Amount::try_new(1).unwrap(),
///     submitted_at: DateTime::new(0),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Submitted {
    applicant: EmployeeId,
    planned_cost: Amount,
    submitted_at: DateTime,
}

impl Submitted {
    pub fn applicant(&self) -> EmployeeId {
        self.applicant
    }

    pub fn planned_cost(&self) -> Amount {
        self.planned_cost
    }

    pub fn submitted_at(&self) -> DateTime {
        self.submitted_at
    }
}

/// 却下理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Reason {
    HighCost,
}

/// 却下された申請。これも `submit` だけが作る。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Rejected {
    reason: Reason,
}

impl Rejected {
    pub fn reason(&self) -> Reason {
        self.reason
    }
}

/// `Submitted | Rejected`。業務上の却下も `Result` ではなく和型の一ケースとして返す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumDiscriminants)]
#[strum_discriminants(name(SubmitKind), derive(EnumIter, Hash))]
pub enum SubmitOutcome {
    Submitted(Submitted),
    Rejected(Rejected),
}

/// `depends on currentTime`。実装は領域 crate の外が用意する。
pub trait Clock {
    fn now(&self) -> DateTime;
}

/// 申請を提出する。上限を超える額は却下する。
pub fn submit(draft: Draft, clock: &impl Clock) -> SubmitOutcome {
    if draft.planned_cost.into_inner() > PRE_APPROVAL_LIMIT {
        return SubmitOutcome::Rejected(Rejected {
            reason: Reason::HighCost,
        });
    }
    SubmitOutcome::Submitted(Submitted {
        applicant: draft.applicant,
        planned_cost: draft.planned_cost,
        submitted_at: clock.now(),
    })
}
