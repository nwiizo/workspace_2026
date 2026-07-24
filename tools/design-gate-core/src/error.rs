use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to walk {path}: {source}")]
    Walk {
        path: PathBuf,
        source: walkdir::Error,
    },
    #[error("git command failed in {cwd}: {message}")]
    Git { cwd: PathBuf, message: String },
    #[error("not a git repository: {0}")]
    NotGitRepo(PathBuf),
    #[error("analysis path {path} is outside repository root {root}")]
    PathOutsideRepo { path: PathBuf, root: PathBuf },
    #[error("'{path}' does not exist at ref '{git_ref}' - new/untracked or renamed?")]
    BaselinePathMissing { path: PathBuf, git_ref: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
