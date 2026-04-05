use crate::error::AppError;
use crate::model::{Challenge, ChallengeMeta};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct ChallengeToml {
    id: String,
    title: String,
    level: u8,
    description: String,
    initial_code: String,
    solution_code: String,
    #[serde(default)]
    hints: Vec<String>,
}

/// Load all challenges from a directory of TOML files.
pub fn load_challenges(dir: &Path) -> Result<Vec<Challenge>, AppError> {
    let mut challenges = Vec::new();

    if !dir.exists() {
        return Ok(challenges);
    }

    let entries = std::fs::read_dir(dir)
        .map_err(|e| AppError::Internal(format!("Failed to read challenges dir: {e}")))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| AppError::Internal(format!("Failed to read dir entry: {e}")))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                AppError::Internal(format!("Failed to read {}: {e}", path.display()))
            })?;
            let parsed: ChallengeToml = toml::from_str(&content).map_err(|e| {
                AppError::Internal(format!("Failed to parse {}: {e}", path.display()))
            })?;
            challenges.push(Challenge {
                id: parsed.id,
                title: parsed.title,
                level: parsed.level,
                description: parsed.description,
                initial_code: parsed.initial_code,
                solution_code: parsed.solution_code,
                hints: parsed.hints,
            });
        }
    }

    challenges.sort_by(|a, b| a.level.cmp(&b.level).then(a.id.cmp(&b.id)));
    Ok(challenges)
}

/// Convert challenges to metadata (for listing).
pub fn to_meta(challenges: &[Challenge]) -> Vec<ChallengeMeta> {
    challenges
        .iter()
        .map(|c| ChallengeMeta {
            id: c.id.clone(),
            title: c.title.clone(),
            level: c.level,
            description: c.description.clone(),
        })
        .collect()
}
