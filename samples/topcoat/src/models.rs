#[derive(Debug, Clone, toasty::Model)]
pub(crate) struct User {
    #[key]
    #[auto]
    pub(crate) id: u64,

    #[unique]
    pub(crate) email: String,

    pub(crate) password_hash: String,
}

#[derive(Debug, Clone, toasty::Model)]
pub(crate) struct AuthSession {
    #[key]
    pub(crate) token_hash: String,

    #[index]
    pub(crate) user_id: u64,

    pub(crate) expires_at: i64,
}

#[derive(Debug, Clone, toasty::Model)]
pub(crate) struct TrainingSession {
    #[key]
    #[auto]
    pub(crate) id: u64,

    #[index]
    pub(crate) user_id: u64,

    pub(crate) trained_on: String,
    pub(crate) uniform: String,
    pub(crate) rounds: u64,
    pub(crate) reflection: String,
}

#[derive(Debug, Clone, toasty::Model)]
pub(crate) struct TechniqueCard {
    #[key]
    #[auto]
    pub(crate) id: u64,

    #[index]
    pub(crate) user_id: u64,

    #[index]
    pub(crate) training_session_id: u64,

    pub(crate) name: String,
    pub(crate) position: String,
    pub(crate) cue: String,
    pub(crate) answer: String,
    pub(crate) next_try: String,
}

#[derive(Debug, Clone, toasty::Model)]
pub(crate) struct Review {
    #[key]
    #[auto]
    pub(crate) id: u64,

    #[index]
    pub(crate) user_id: u64,

    #[index]
    pub(crate) card_id: u64,

    pub(crate) reviewed_on: String,
    pub(crate) recall_rating: String,
    pub(crate) application_result: String,
    pub(crate) next_review_on: String,
}
