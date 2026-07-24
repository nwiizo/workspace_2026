use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{CoreError, Result};

pub struct BaselineWorktree {
    guard: WorktreeGuard,
    baseline_path: PathBuf,
}

impl BaselineWorktree {
    pub fn baseline_path(&self) -> &Path {
        &self.baseline_path
    }

    pub fn worktree_path(&self) -> &Path {
        self.guard.path()
    }
}

pub struct WorktreeGuard {
    repo_root: PathBuf,
    worktree: Option<PathBuf>,
}

impl WorktreeGuard {
    pub fn new(repo_root: PathBuf, worktree: PathBuf) -> Self {
        Self {
            repo_root,
            worktree: Some(worktree),
        }
    }

    pub fn path(&self) -> &Path {
        self.worktree.as_deref().expect("worktree path exists")
    }

    pub fn remove(&mut self) -> Result<()> {
        if let Some(worktree) = self.worktree.take() {
            remove_worktree(&self.repo_root, &worktree)
        } else {
            Ok(())
        }
    }
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

pub fn prepare_baseline_worktree(
    current_path: &Path,
    git_ref: &str,
    tool_name: &str,
) -> Result<BaselineWorktree> {
    let root = repo_root(current_path)?;
    let subpath = relative_subpath(&root, current_path)?;
    let worktree = add_worktree(&root, git_ref, tool_name)?;
    let guard = WorktreeGuard::new(root, worktree);
    let baseline_path = guard.path().join(subpath);
    if !baseline_path.exists() {
        return Err(CoreError::BaselinePathMissing {
            path: current_path.to_path_buf(),
            git_ref: git_ref.to_string(),
        });
    }
    Ok(BaselineWorktree {
        guard,
        baseline_path,
    })
}

pub fn repo_root(path: &Path) -> Result<PathBuf> {
    let output = run_git(path, ["rev-parse", "--show-toplevel"])?;
    if !output.status.success() {
        return Err(CoreError::NotGitRepo(path.to_path_buf()));
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err(CoreError::NotGitRepo(path.to_path_buf()));
    }
    let root = PathBuf::from(text);
    Ok(root.canonicalize().unwrap_or(root))
}

pub fn relative_subpath(root: &Path, path: &Path) -> Result<PathBuf> {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    absolute
        .strip_prefix(&canonical_root)
        .map(Path::to_path_buf)
        .map_err(|_| CoreError::PathOutsideRepo {
            path: absolute,
            root: canonical_root,
        })
}

pub fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<Output> {
    run_git_with(cwd, |command| {
        command.args(args);
    })
}

pub fn run_git_with(cwd: &Path, setup: impl FnOnce(&mut Command)) -> Result<Output> {
    let mut command = Command::new("git");
    command.current_dir(cwd);
    setup(&mut command);
    command.output().map_err(CoreError::Io)
}

fn add_worktree(repo_root: &Path, git_ref: &str, tool_name: &str) -> Result<PathBuf> {
    let worktree = temp_worktree_path(git_ref, tool_name);
    let output = run_git_with(repo_root, |command| {
        command
            .args(["worktree", "add", "--detach"])
            .arg(&worktree)
            .arg(git_ref);
    })?;
    if output.status.success() {
        Ok(worktree)
    } else {
        Err(CoreError::Git {
            cwd: repo_root.to_path_buf(),
            message: format!(
                "git worktree add failed for ref {git_ref}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        })
    }
}

fn remove_worktree(repo_root: &Path, worktree: &Path) -> Result<()> {
    let output = run_git_with(repo_root, |command| {
        command
            .args(["worktree", "remove", "--force"])
            .arg(worktree);
    })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CoreError::Git {
            cwd: repo_root.to_path_buf(),
            message: format!(
                "git worktree remove failed for {}: {}",
                worktree.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        })
    }
}

fn temp_worktree_path(git_ref: &str, tool_name: &str) -> PathBuf {
    let sanitized_ref = sanitize(git_ref);
    let sanitized_tool = sanitize(tool_name);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("{sanitized_tool}-{sanitized_ref}-{nanos}"))
}

fn sanitize(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}
