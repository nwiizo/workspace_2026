//! SKILL.md loader (frontmatter + body).

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub path: PathBuf,
}

/// Look up the skill body under `<skills_root>/<name>/SKILL.md`. Skill names
/// containing `/` are treated as relative paths so callers can point at
/// nested layouts.
pub fn load_skill(skills_root: &Path, name: &str) -> Result<Skill> {
    let candidate = skills_root.join(name).join("SKILL.md");
    let text = std::fs::read_to_string(&candidate)
        .with_context(|| format!("reading skill at {}", candidate.display()))?;
    let (front, body) = split_frontmatter(&text)?;
    let mut frontmatter_name = name.to_string();
    let mut description = String::new();
    if let Some(front) = front {
        let value: serde_yaml::Value = serde_yaml::from_str(front)
            .with_context(|| format!("parsing SKILL.md frontmatter at {}", candidate.display()))?;
        if let Some(s) = value.get("name").and_then(|v| v.as_str()) {
            frontmatter_name = s.to_string();
        }
        if let Some(s) = value.get("description").and_then(|v| v.as_str()) {
            description = s.to_string();
        }
    }
    Ok(Skill {
        name: frontmatter_name,
        description,
        body: body.to_string(),
        path: candidate,
    })
}

fn split_frontmatter(text: &str) -> Result<(Option<&str>, &str)> {
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let front = &rest[..end];
            let body = &rest[end + "\n---\n".len()..];
            return Ok((Some(front), body));
        }
        if let Some(end) = rest.find("\n---") {
            // tolerate missing trailing newline
            let front = &rest[..end];
            let body = rest[end + "\n---".len()..].trim_start_matches('\n');
            return Ok((Some(front), body));
        }
        return Err(anyhow!(
            "SKILL.md frontmatter opener `---` has no closing `---`"
        ));
    }
    Ok((None, text))
}
