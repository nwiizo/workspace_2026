use serde::{Deserialize, Serialize};

/// Language for UI text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Lang {
    #[default]
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "en")]
    En,
}

/// What happened to ownership at this step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OwnershipEvent {
    /// A new variable is created and takes ownership of a value.
    Bind {
        variable: String,
        type_name: String,
        value_hint: String,
    },
    /// Ownership moves from one variable to another.
    Move { from: String, to: String },
    /// A borrow (shared or mutable) is created.
    BorrowStart {
        from: String,
        to: String,
        mutable: bool,
    },
    /// A borrow ends (reference goes out of scope or is last used).
    BorrowEnd { variable: String, owner: String },
    /// A value is cloned, creating a new independent copy.
    Clone { from: String, to: String },
    /// A value is dropped (goes out of scope).
    Drop { variable: String },
    /// A compile error was detected at this point.
    CompileError { message: String },
}

/// Current status of a variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableStatus {
    Owned,
    Moved,
    BorrowedShared,
    BorrowedMut,
    Dropped,
    /// This variable is a live shared reference.
    LiveRef,
    /// This variable is a live mutable reference.
    LiveMutRef,
}

/// Where a value lives in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLocation {
    Stack,
    Heap,
}

/// State of a single variable at a given step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableState {
    pub name: String,
    pub type_name: String,
    pub status: VariableStatus,
    pub memory: MemoryLocation,
    pub value_hint: String,
    /// Which variables are borrowing from this one.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub borrowed_by: Vec<String>,
    /// If this is a reference, which variable does it borrow from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borrows_from: Option<String>,
}

/// A region of heap memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRegion {
    pub address: String,
    pub content: String,
    pub owner: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
}

/// A single step in the ownership execution trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub index: usize,
    pub source_line: usize,
    pub description: String,
    pub event: OwnershipEvent,
    pub variables: Vec<VariableState>,
    pub memory: Vec<MemoryRegion>,
}

/// The full result of analyzing a piece of Rust source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub source: String,
    pub steps: Vec<Step>,
    pub has_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Result of compilation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
}

/// A structured compiler diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

/// Metadata for a challenge (used in listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeMeta {
    pub id: String,
    pub title: String,
    pub level: u8,
    pub description: String,
}

/// A full challenge with code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub title: String,
    pub level: u8,
    pub description: String,
    pub initial_code: String,
    pub solution_code: String,
    pub hints: Vec<String>,
}

// --- Fix Suggestion Engine ---

/// A suggested fix for an ownership error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixSuggestion {
    pub strategy: FixStrategy,
    pub title: String,
    pub description: String,
    pub fixed_code: String,
    pub trade_off: String,
}

/// Which fix strategy is being proposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixStrategy {
    Clone,
    Borrow,
    ScopeChange,
    Rc,
}

/// Result of analyzing code for fix suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResult {
    pub source: String,
    pub error_pattern: Option<String>,
    pub suggestions: Vec<FixSuggestion>,
}

// --- Error Prediction Quiz ---

/// A quiz question for error prediction mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizQuestion {
    pub id: String,
    pub code: String,
    pub compiles: bool,
    pub explanation: String,
    pub related_concept: String,
}

/// Result of checking a quiz answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizResult {
    pub correct: bool,
    pub expected: bool,
    pub explanation: String,
    pub analysis: AnalysisResult,
}

// --- Diff View ---

/// Comparison of ownership flow between two code versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub before: AnalysisResult,
    pub after: AnalysisResult,
    pub changes: Vec<DiffChange>,
}

/// A single difference between before/after ownership flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffChange {
    pub description: String,
    pub change_type: DiffChangeType,
}

/// Type of change between before and after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffChangeType {
    MoveRemoved,
    BorrowAdded,
    CloneAdded,
    ErrorFixed,
    ErrorIntroduced,
}
