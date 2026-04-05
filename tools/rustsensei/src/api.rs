use crate::analyzer;
use crate::challenge;
use crate::compiler;
use crate::diff;
use crate::error::AppError;
use crate::model::{
    AnalysisResult, Challenge, ChallengeMeta, CompileResult, DiffResult, Lang, QuizQuestion,
    QuizResult, SuggestionResult,
};
use crate::quiz;
use crate::suggestion;
use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use std::sync::Arc;

/// Max source code length (32KB).
const MAX_SOURCE_LEN: usize = 32 * 1024;

/// Shared application state.
pub struct AppState {
    pub challenges: Vec<Challenge>,
}

#[derive(Deserialize)]
pub struct SourceInput {
    pub source: String,
    #[serde(default)]
    pub lang: Lang,
}

fn validate_source(source: &str) -> Result<(), AppError> {
    if source.len() > MAX_SOURCE_LEN {
        return Err(AppError::ParseError(format!(
            "Source code too large ({} bytes, max {})",
            source.len(),
            MAX_SOURCE_LEN
        )));
    }
    Ok(())
}

/// POST /api/analyze
pub async fn analyze_handler(
    Json(input): Json<SourceInput>,
) -> Result<Json<AnalysisResult>, AppError> {
    validate_source(&input.source)?;
    let result = analyzer::analyze_with_lang(&input.source, input.lang);
    Ok(Json(result))
}

/// POST /api/compile
pub async fn compile_handler(
    Json(input): Json<SourceInput>,
) -> Result<Json<CompileResult>, AppError> {
    validate_source(&input.source)?;
    let result = compiler::check_compile(&input.source).await?;
    Ok(Json(result))
}

/// POST /api/suggest
pub async fn suggest_handler(
    Json(input): Json<SourceInput>,
) -> Result<Json<SuggestionResult>, AppError> {
    validate_source(&input.source)?;
    Ok(Json(suggestion::suggest_fixes(&input.source)))
}

/// GET /api/challenges
pub async fn list_challenges_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ChallengeMeta>> {
    Json(challenge::to_meta(&state.challenges))
}

/// GET /api/challenges/:id
pub async fn get_challenge_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Challenge>, AppError> {
    state
        .challenges
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Challenge '{id}' not found")))
}

/// GET /api/quiz/questions
pub async fn quiz_questions_handler() -> Json<Vec<QuizQuestion>> {
    Json(quiz::builtin_questions())
}

#[derive(Deserialize)]
pub struct QuizAnswer {
    pub question_id: String,
    pub prediction: bool,
}

/// POST /api/quiz/check
pub async fn quiz_check_handler(
    Json(input): Json<QuizAnswer>,
) -> Result<Json<QuizResult>, AppError> {
    quiz::check_prediction(&input.question_id, input.prediction)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("Question '{}' not found", input.question_id)))
}

#[derive(Deserialize)]
pub struct DiffInput {
    pub before: String,
    pub after: String,
}

/// POST /api/diff
pub async fn diff_handler(Json(input): Json<DiffInput>) -> Result<Json<DiffResult>, AppError> {
    validate_source(&input.before)?;
    validate_source(&input.after)?;
    Ok(Json(diff::compare(&input.before, &input.after)))
}
