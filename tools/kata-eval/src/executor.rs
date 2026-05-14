//! Bias-suppressed executor. Shells out to `claude -p` so the executor
//! agent does not auto-merge CLAUDE.md or auto-discover skills.

use anyhow::{Context, Result, anyhow};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

pub const SELF_REPORT_TAIL: &str = r#"

---

When you finish the task, append a `## Self-report` block at the very end of
your response with this exact structure:

```
## Self-report

### Phase trace
- <phase name>: OK | stuck | skipped | missing — <reason if not OK>

### Unclear points
- Issue: <what was ambiguous>
  Cause: <which sentence / which absence in the skill caused it>
  General Fix Rule: <one-line addition that would have removed the ambiguity>

### Discretionary fill-ins
- <decision you had to make on your own>

### Retries
<integer — how many internal retries you needed>
```

If everything was clear, still emit the headings with empty bullet lists and
`Retries: 0`. Do not add commentary after this block.
"#;

#[derive(Debug, Clone)]
pub struct ExecutorOptions {
    pub model: Option<String>,
    pub timeout: Duration,
    pub require_self_report: bool,
    pub system_prompt: Option<String>,
}

impl Default for ExecutorOptions {
    fn default() -> Self {
        Self {
            model: None,
            timeout: Duration::from_secs(300),
            require_self_report: true,
            system_prompt: None,
        }
    }
}

/// Run `claude -p` with stdin = prompt and return stdout. Hard-fails on
/// non-zero exit. Suppresses skill auto-discovery via
/// `--disable-slash-commands`.
pub async fn run_claude(prompt: &str, opts: &ExecutorOptions) -> Result<String> {
    let claude = which_claude();
    let mut cmd = Command::new(claude);
    cmd.arg("-p")
        .arg("--disable-slash-commands")
        .arg("--no-session-persistence")
        .arg("--exclude-dynamic-system-prompt-sections")
        .arg("--output-format")
        .arg("text")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if let Some(m) = &opts.model {
        cmd.arg("--model").arg(m);
    }
    if let Some(sp) = &opts.system_prompt {
        cmd.arg("--system-prompt").arg(sp);
    }

    let final_prompt = if opts.require_self_report {
        let mut s = String::from(prompt);
        s.push_str(SELF_REPORT_TAIL);
        s
    } else {
        prompt.to_string()
    };

    let mut child = cmd.spawn().with_context(|| "spawning claude")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(final_prompt.as_bytes())
            .await
            .with_context(|| "writing prompt to claude stdin")?;
        stdin
            .shutdown()
            .await
            .with_context(|| "closing claude stdin")?;
    }
    // Keep `child` owned so we can kill it on timeout. `wait_with_output`
    // consumes child, so split the wait: poll stdout/stderr drainage via
    // `wait()` + reads on the piped handles.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let status = match timeout(opts.timeout, child.wait()).await {
        Ok(res) => res.with_context(|| "waiting on claude")?,
        Err(_) => {
            // SIGKILL the child explicitly to avoid leaking the process if
            // the OS reaper takes its time.
            child.kill().await.ok();
            return Err(anyhow!(
                "claude timed out after {}s",
                opts.timeout.as_secs()
            ));
        }
    };
    let stdout_bytes = read_to_end(stdout).await?;
    let stderr_bytes = read_to_end(stderr).await?;
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        return Err(anyhow!("claude exited with {status}: {}", stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&stdout_bytes).to_string())
}

async fn read_to_end<R: tokio::io::AsyncRead + Unpin>(reader: Option<R>) -> Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    if let Some(mut r) = reader {
        r.read_to_end(&mut buf)
            .await
            .with_context(|| "reading claude stdout/stderr")?;
    }
    Ok(buf)
}

fn which_claude() -> String {
    std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "claude".to_string())
}
