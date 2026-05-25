use crate::state::{CleanupStatus, SessionState};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct CleanupReport {
    pub session_id: String,
    pub steps: Vec<CleanupStep>,
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct CleanupStep {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Best-effort cleanup for a session.
///
/// - Normal mode: the state file must load. If the PID is alive it's signalled,
///   the state is marked `Stopped` and the file removed.
/// - `force` mode: even if the state file is missing or unparseable, attempt to
///   remove any stale state JSON and the session's artifact directory so a
///   subsequent `up` won't trip over leftovers.
pub fn run(
    state_dir: &Path,
    artifact_dir: &Path,
    session_id: &str,
    force: bool,
) -> anyhow::Result<CleanupReport> {
    let mut steps = Vec::new();

    let state = match SessionState::load(state_dir, session_id) {
        Ok(s) => {
            steps.push(CleanupStep {
                name: "load state".into(),
                ok: true,
                detail: "loaded".into(),
            });
            Some(s)
        }
        // In force mode, a missing or corrupt state file is expected — that is
        // exactly the case force is meant to recover. We log it as ok=true with
        // the underlying error in `detail` so `report.ok` still reflects whether
        // cleanup actions succeeded.
        Err(e) if force => {
            steps.push(CleanupStep {
                name: "load state".into(),
                ok: true,
                detail: format!("skipped (force): {e}"),
            });
            None
        }
        Err(e) => return Err(e),
    };

    if let Some(mut state) = state {
        // Stop the running process (if any).
        let pid_killed = stop_pid(state.pid);
        steps.push(CleanupStep {
            name: format!("stop pid {}", state.pid),
            ok: true,
            detail: if pid_killed {
                "signalled".into()
            } else {
                "already gone".into()
            },
        });

        // Persist Stopped status so doctor stops flagging it. Best-effort; not fatal.
        state.cleanup_status = CleanupStatus::Stopped;
        if let Err(e) = state.save(state_dir) {
            steps.push(CleanupStep {
                name: "persist Stopped".into(),
                ok: false,
                detail: e.to_string(),
            });
        }
    } else {
        steps.push(CleanupStep {
            name: "stop pid".into(),
            ok: true,
            detail: "skipped (no state; pid unknown)".into(),
        });
    }

    // Always try to remove the state file. With force this is the main way to
    // get rid of a corrupt file.
    match SessionState::delete(state_dir, session_id) {
        Ok(_) => steps.push(CleanupStep {
            name: "delete state file".into(),
            ok: true,
            detail: "removed".into(),
        }),
        Err(e) => steps.push(CleanupStep {
            name: "delete state file".into(),
            ok: false,
            detail: e.to_string(),
        }),
    }

    // With --force, also try to remove the session's artifact directory.
    // Without force we leave artifacts alone (they're the whole point of the
    // session and might still be of interest).
    if force {
        let session_artifact = artifact_dir.join(session_id);
        if session_artifact.exists() {
            match std::fs::remove_dir_all(&session_artifact) {
                Ok(()) => steps.push(CleanupStep {
                    name: "remove artifact dir".into(),
                    ok: true,
                    detail: format!("removed {}", session_artifact.display()),
                }),
                Err(e) => steps.push(CleanupStep {
                    name: "remove artifact dir".into(),
                    ok: false,
                    detail: format!("{}: {e}", session_artifact.display()),
                }),
            }
        } else {
            steps.push(CleanupStep {
                name: "remove artifact dir".into(),
                ok: true,
                detail: "no artifact dir to remove".into(),
            });
        }
    }

    let ok = steps.iter().all(|s| s.ok);
    Ok(CleanupReport {
        session_id: session_id.to_string(),
        steps,
        ok,
    })
}

#[cfg(unix)]
fn stop_pid(pid: u32) -> bool {
    use std::process::Command;
    if pid == 0 {
        return false;
    }
    let alive = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !alive {
        return false;
    }
    let _ = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();
    true
}

#[cfg(not(unix))]
fn stop_pid(_pid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn force_cleanup_removes_corrupt_state_file_and_artifact_dir() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().join("state");
        let artifact_dir = tmp.path().join("artifacts");
        std::fs::create_dir_all(&state_dir).unwrap();
        let session_artifact = artifact_dir.join("sess-123");
        std::fs::create_dir_all(&session_artifact).unwrap();
        std::fs::write(session_artifact.join("report.md"), b"stale").unwrap();
        // Write an unparseable state file.
        std::fs::write(state_dir.join("sess-123.json"), b"not valid json").unwrap();

        let report = run(&state_dir, &artifact_dir, "sess-123", true).unwrap();

        assert!(report.ok, "force cleanup must succeed: {report:?}");
        assert!(!state_dir.join("sess-123.json").exists());
        assert!(!session_artifact.exists());
        // The load-state step is logged but force makes it non-blocking.
        let load_step = report
            .steps
            .iter()
            .find(|s| s.name == "load state")
            .unwrap();
        assert!(load_step.ok);
        assert!(load_step.detail.contains("skipped (force)"));
        assert!(
            report
                .steps
                .iter()
                .any(|s| s.name == "remove artifact dir" && s.ok)
        );
    }

    #[test]
    fn cleanup_without_force_errors_on_missing_state() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().join("state");
        let artifact_dir = tmp.path().join("artifacts");
        std::fs::create_dir_all(&state_dir).unwrap();
        let err = run(&state_dir, &artifact_dir, "missing", false).unwrap_err();
        assert!(err.to_string().contains("missing") || err.to_string().contains("No such"));
    }

    #[test]
    fn cleanup_force_with_no_artifact_dir_is_still_ok() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().join("state");
        let artifact_dir = tmp.path().join("artifacts");
        std::fs::create_dir_all(&state_dir).unwrap();
        let report = run(&state_dir, &artifact_dir, "never-existed", true).unwrap();
        assert!(report.ok);
        let artifact_step = report
            .steps
            .iter()
            .find(|s| s.name == "remove artifact dir")
            .unwrap();
        assert!(artifact_step.ok);
        assert!(artifact_step.detail.contains("no artifact dir"));
    }
}
