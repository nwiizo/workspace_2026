use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use chromiumoxide::browser::Browser;
use chromiumoxide::keys;
use chromiumoxide::page::{Page, ScreenshotParams};
use chromiumoxide_cdp::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType,
};
use chromiumoxide_cdp::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chrono::Local;
use clap::ValueEnum;
use futures::StreamExt;
use serde::Deserialize;
use tokio::time::{Instant, sleep};

use crate::metadata::{LocationMetadata, WebMetadata, WebPageMetadata, write_json_atomic};

const DEFAULT_CHROME: &str = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
pub const DEFAULT_DEBUG_PORT: u16 = 9445;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Single,
    Double,
    Default,
}

impl Layout {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Double => "double",
            Self::Default => "default",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum WaitStrategy {
    Hybrid,
    #[value(name = "location_change")]
    LocationChange,
    Fixed,
}

#[derive(Clone, Debug)]
pub struct WebCaptureOptions {
    pub asin: String,
    pub layout: Layout,
    pub output: PathBuf,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub headless: bool,
    pub max_pages: Option<usize>,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub chrome_profile: PathBuf,
    pub chrome_path: Option<PathBuf>,
    pub debug_port: u16,
    pub wait_strategy: WaitStrategy,
    pub wait_timeout: f64,
    pub browser_timeout_ms: u64,
    pub wait_for_login: bool,
    pub login_timeout_ms: u64,
    pub overwrite: bool,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct ReaderState {
    minimum: Option<i64>,
    maximum: Option<i64>,
    current: Option<i64>,
    top: Option<i64>,
    bottom: Option<i64>,
    has_next: bool,
    #[serde(default)]
    api_ready: bool,
}

impl ReaderState {
    fn progress(self) -> Option<i64> {
        self.bottom
            .filter(|value| *value > 0)
            .or(self.current.filter(|value| *value > 0))
    }
}

#[derive(Clone, Copy, Debug)]
struct NavigationKeys {
    forward: &'static str,
    backward: &'static str,
}

pub fn validate_asin(asin: &str) -> Result<String> {
    let asin = asin.trim().to_ascii_uppercase();
    ensure!(
        asin.len() == 10 && asin.bytes().all(|byte| byte.is_ascii_alphanumeric()),
        "ASIN must contain exactly 10 ASCII letters or digits"
    );
    Ok(asin)
}

pub fn reader_url(asin: &str) -> Result<String> {
    Ok(format!(
        "https://read.amazon.co.jp/?asin={}",
        validate_asin(asin)?
    ))
}

pub async fn capture_book(options: &WebCaptureOptions) -> Result<WebMetadata> {
    validate_options(options)?;
    if !options.overwrite {
        crate::capture_output::prepare(&options.output, false)?;
    }
    fs::create_dir_all(&options.chrome_profile).with_context(|| {
        format!(
            "failed to create dedicated Chrome profile: {}",
            options.chrome_profile.display()
        )
    })?;

    let chrome_path = options
        .chrome_path
        .as_deref()
        .unwrap_or_else(|| Path::new(DEFAULT_CHROME));
    ensure!(
        chrome_path.is_file(),
        "Chrome executable not found: {}; pass --chrome-path",
        chrome_path.display()
    );

    let url = reader_url(&options.asin)?;
    let (browser, mut handler) = connect_or_launch_chrome(chrome_path, &url, options).await?;
    let handler_task = tokio::spawn(async move {
        while let Some(message) = handler.next().await {
            if message.is_err() {
                break;
            }
        }
    });

    let result = capture_with_browser(&browser, options).await;
    handler_task.abort();

    result?;
    read_saved_metadata(options)
}

async fn connect_or_launch_chrome(
    chrome_path: &Path,
    url: &str,
    options: &WebCaptureOptions,
) -> Result<(Browser, chromiumoxide::handler::Handler)> {
    let endpoint = format!("http://127.0.0.1:{}", options.debug_port);
    if let Ok(connection) = Browser::connect(&endpoint).await {
        eprintln!(
            "connected to the existing Chrome session on port {}",
            options.debug_port
        );
        return Ok(connection);
    }

    let args = chrome_launch_args(
        &options.chrome_profile,
        options.debug_port,
        url,
        options.viewport_width,
        options.viewport_height,
        options.headless,
    );
    chrome_launch_command(chrome_path, &args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to start normal Chrome: {}", chrome_path.display()))?;

    let deadline = Instant::now() + Duration::from_millis(options.browser_timeout_ms);
    while Instant::now() < deadline {
        if let Ok(connection) = Browser::connect(&endpoint).await {
            eprintln!(
                "started a user-driven Chrome session on port {}; the window will remain open",
                options.debug_port
            );
            return Ok(connection);
        }
        sleep(Duration::from_millis(250)).await;
    }
    bail!(
        "Chrome did not expose debug port {}; close any process using {} and retry",
        options.debug_port,
        options.chrome_profile.display()
    )
}

fn chrome_launch_args(
    profile: &Path,
    debug_port: u16,
    url: &str,
    viewport_width: u32,
    viewport_height: u32,
    headless: bool,
) -> Vec<String> {
    let mut args = vec![
        format!("--remote-debugging-port={debug_port}"),
        format!("--user-data-dir={}", profile.display()),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        format!("--window-size={viewport_width},{viewport_height}"),
    ];
    if headless {
        args.push("--headless=new".into());
    }
    args.push(url.into());
    args
}

fn chrome_launch_command(chrome_path: &Path, args: &[String]) -> Command {
    #[cfg(target_os = "macos")]
    if let Some(app_bundle) = chrome_path
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
    {
        let mut command = Command::new("/usr/bin/open");
        command.arg("-na").arg(app_bundle).arg("--args").args(args);
        return command;
    }

    let mut command = Command::new(chrome_path);
    command.args(args);
    command
}

async fn capture_with_browser(browser: &Browser, options: &WebCaptureOptions) -> Result<()> {
    let url = reader_url(&options.asin)?;
    let page = browser
        .new_page(url)
        .await
        .context("failed to open a fresh Kindle Web Reader tab")?;
    let mut ready =
        wait_for_renderer(&page, Duration::from_millis(options.browser_timeout_ms)).await;
    if !ready && options.wait_for_login && !options.headless {
        eprintln!("Complete the Kindle sign-in in the newly opened tab; waiting for the reader...");
        ready = wait_for_renderer(&page, Duration::from_millis(options.login_timeout_ms)).await;
    }
    ensure!(
        ready,
        "Kindle Reader is unavailable; finish sign-in in the dedicated Chrome tab and retry"
    );

    dismiss_dialogs(&page).await;
    if options.layout != Layout::Default && !set_layout(&page, options.layout).await? {
        eprintln!(
            "warning: requested layout could not be selected; continuing with the current layout"
        );
    }
    let initial_state = reader_state(&page).await?;
    let minimum = initial_state
        .minimum
        .context("Kindle Reader did not report a minimum position")?;
    let maximum = initial_state
        .maximum
        .context("Kindle Reader did not report a maximum position")?;
    let use_private_api = initial_state.api_ready;
    let navigation_keys = if use_private_api {
        None
    } else {
        Some(detect_navigation_keys(&page, options).await?)
    };
    let start = options.start.unwrap_or(minimum);
    let end = options.end.unwrap_or(maximum);
    ensure!(
        start >= minimum && start <= maximum,
        "start position is outside the book range {minimum}..={maximum}"
    );
    ensure!(
        end >= start && end <= maximum,
        "end position must be within {start}..={maximum}"
    );
    if options.overwrite {
        crate::capture_output::prepare(&options.output, true)?;
    }

    let actual_start =
        navigate_to_position(&page, start, use_private_api, navigation_keys, options).await?;
    ensure!(
        actual_start <= end,
        "the nearest readable position {actual_start} is after the requested end {end}"
    );

    let mut pages = Vec::new();
    loop {
        wait_for_spinner(&page, Duration::from_secs_f64(options.wait_timeout)).await;
        let state = reader_state(&page).await?;
        if !pages.is_empty() && state.top.is_some_and(|top| top > end) {
            break;
        }
        let page_number = pages.len() + 1;
        let filename = format!("page_{page_number:04}.png");
        let destination = options.output.join(&filename);
        let temporary = options
            .output
            .join(format!(".{page_number:04}.pending.png"));
        let bytes = page
            .screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .full_page(false)
                    .build(),
            )
            .await
            .context("failed to capture the browser viewport")?;
        fs::write(&temporary, bytes)
            .with_context(|| format!("failed to write screenshot: {}", temporary.display()))?;
        fs::rename(&temporary, &destination)
            .with_context(|| format!("failed to finalize screenshot: {}", destination.display()))?;

        let current = state.progress().unwrap_or(0);
        let total = state.maximum.unwrap_or(maximum);
        let percent = if total > 0 {
            ((current as f64 / total as f64) * 100.0).floor() as i64
        } else {
            0
        };
        pages.push(WebPageMetadata {
            page: page_number,
            location: LocationMetadata {
                current,
                total,
                percent: u8::try_from(percent.clamp(0, 100))
                    .context("location percentage is outside the supported range")?,
            },
            timestamp: Local::now().to_rfc3339(),
        });
        eprintln!("captured page {page_number} (position {current}/{total})");

        if options
            .max_pages
            .is_some_and(|maximum| pages.len() >= maximum)
            || state.progress().is_some_and(|progress| progress >= end)
        {
            break;
        }
        if use_private_api && !state.has_next {
            break;
        }
        let previous = state.progress();
        let mut advanced = false;
        for _ in 0..3 {
            advance_reader(&page, use_private_api, navigation_keys).await?;
            if wait_after_navigation(&page, previous, options).await? {
                advanced = true;
                break;
            }
        }
        if !advanced {
            eprintln!("Reader position did not advance after 3 attempts; stopping capture");
            break;
        }
        if pages.len() % 50 == 0 && !wait_for_renderer(&page, Duration::from_secs(2)).await {
            bail!("Kindle session expired during capture");
        }
    }

    let metadata = WebMetadata {
        asin: validate_asin(&options.asin)?,
        layout: options.layout.as_str().into(),
        total_pages: pages.len(),
        position_range: [minimum, maximum],
        capture_range: [actual_start, end],
        captured_at: Local::now().to_rfc3339(),
        pages,
    };
    write_json_atomic(&options.output.join("metadata.json"), &metadata)
}

async fn advance_reader(
    page: &Page,
    use_private_api: bool,
    navigation_keys: Option<NavigationKeys>,
) -> Result<()> {
    if use_private_api {
        page.evaluate("KindleRenderer.nextScreen()")
            .await
            .context("failed to advance Kindle Web Reader")?;
    } else {
        press_page_key(
            page,
            navigation_keys
                .context("Reader navigation keys were not detected")?
                .forward,
        )
        .await
        .context("failed to send the next-page key to Kindle Web Reader")?;
    }
    Ok(())
}

fn read_saved_metadata(options: &WebCaptureOptions) -> Result<WebMetadata> {
    let path = options.output.join("metadata.json");
    let bytes = fs::read(&path)
        .with_context(|| format!("failed to read completed metadata: {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse completed metadata: {}", path.display()))
}

fn validate_options(options: &WebCaptureOptions) -> Result<()> {
    validate_asin(&options.asin)?;
    ensure!(
        options.viewport_width > 0 && options.viewport_height > 0,
        "viewport dimensions must be positive"
    );
    ensure!(
        options.wait_timeout.is_finite() && options.wait_timeout > 0.0,
        "wait timeout must be a positive finite number"
    );
    ensure!(
        options.browser_timeout_ms > 0,
        "browser timeout must be positive"
    );
    ensure!(
        options.login_timeout_ms > 0,
        "login timeout must be positive"
    );
    ensure!(
        options.debug_port > 0,
        "debug port must be greater than zero"
    );
    ensure!(
        options.max_pages.is_none_or(|maximum| maximum > 0),
        "max pages must be greater than zero"
    );
    ensure!(
        !looks_like_default_chrome_profile(&options.chrome_profile)?,
        "the normal Chrome profile cannot be automated safely; use a dedicated --chrome-profile"
    );
    Ok(())
}

fn looks_like_default_chrome_profile(path: &Path) -> Result<bool> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Ok(path.ends_with("Library/Application Support/Google/Chrome"));
    };
    let default = home.join("Library/Application Support/Google/Chrome");
    profile_is_default_or_nested(path, &default)
}

fn profile_is_default_or_nested(path: &Path, default: &Path) -> Result<bool> {
    let candidate = fs::canonicalize(path).or_else(|_| std::path::absolute(path))?;
    let default = fs::canonicalize(default).or_else(|_| std::path::absolute(default))?;
    let candidate = candidate.to_string_lossy().to_lowercase();
    let default = default.to_string_lossy().to_lowercase();
    Ok(candidate == default || candidate.starts_with(&format!("{default}/")))
}

async fn wait_for_renderer(page: &Page, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut last_error = None;
    while Instant::now() < deadline {
        if renderer_present(page).await {
            dismiss_dialogs(page).await;
        }
        match reader_state(page).await {
            Ok(_) => return true,
            Err(error) => last_error = Some(error),
        }
        sleep(Duration::from_millis(500)).await;
    }
    if let Some(error) = last_error {
        eprintln!("Kindle Reader is not ready yet: {error:#}");
    }
    false
}

async fn renderer_present(page: &Page) -> bool {
    page.evaluate("typeof globalThis.KindleRenderer !== 'undefined'")
        .await
        .ok()
        .and_then(|result| result.into_value::<bool>().ok())
        .unwrap_or(false)
}

async fn reader_state(page: &Page) -> Result<ReaderState> {
    const SCRIPT: &str = r#"(() => {
try {
const renderer = globalThis.KindleRenderer;
const finite = value => Number.isFinite(Number(value)) ? Math.trunc(Number(value)) : null;
const range = renderer.getPagePositionRange?.() ?? {};
return {
  minimum: finite(renderer.getMinimumPosition?.()),
  maximum: finite(renderer.getMaximumPosition?.()),
  current: finite(renderer.getPosition?.()),
  top: finite(range.currentTopOfPage),
  bottom: finite(range.currentBottomOfPage),
  has_next: Boolean(renderer.hasNextScreen?.()),
  api_ready: true
};
} catch (_) {
  return {
    minimum: null,
    maximum: null,
    current: null,
    top: null,
    bottom: null,
    has_next: false,
    api_ready: false
  };
}
})()"#;
    let state = page
        .evaluate(SCRIPT)
        .await
        .context("failed to read Kindle position")?
        .into_value::<ReaderState>()
        .context("Kindle position response had an unexpected shape")?;
    if state.minimum.is_some() && state.maximum.is_some() {
        return Ok(state);
    }

    let text = page
        .evaluate("document.body?.textContent ?? ''")
        .await
        .context("failed to read the visible Kindle location")?
        .into_value::<String>()
        .context("Kindle page text had an unexpected shape")?;
    reader_state_from_text(&text).context(
        "Kindle position is unavailable; finish Reader prompts in Chrome and keep the book open",
    )
}

fn reader_state_from_text(text: &str) -> Option<ReaderState> {
    let rest = ["位置", "Location"]
        .into_iter()
        .find_map(|marker| text.find(marker).map(|index| &text[index + marker.len()..]))?;
    let mut numbers = Vec::with_capacity(2);
    let mut digits = String::new();
    for character in rest.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
        } else if character == ',' && !digits.is_empty() {
            continue;
        } else if !digits.is_empty() {
            numbers.push(digits.parse::<i64>().ok()?);
            digits.clear();
            if numbers.len() == 2 {
                break;
            }
        }
    }
    if numbers.len() < 2 && !digits.is_empty() {
        numbers.push(digits.parse::<i64>().ok()?);
    }
    let [current, total] = *numbers.first_chunk::<2>()?;
    if current <= 0 || total <= 0 || current > total {
        return None;
    }
    Some(ReaderState {
        minimum: Some(1),
        maximum: Some(total),
        current: Some(current),
        top: Some(current),
        bottom: Some(current),
        has_next: current < total,
        api_ready: false,
    })
}

async fn detect_navigation_keys(
    page: &Page,
    options: &WebCaptureOptions,
) -> Result<NavigationKeys> {
    for (key, opposite) in [("ArrowRight", "ArrowLeft"), ("ArrowLeft", "ArrowRight")] {
        let before = reader_state(page)
            .await?
            .progress()
            .context("Kindle Reader did not report the current position")?;
        if let Some(after) = press_key_and_wait(page, key, before, options).await? {
            return if after > before {
                Ok(NavigationKeys {
                    forward: key,
                    backward: opposite,
                })
            } else {
                Ok(NavigationKeys {
                    forward: opposite,
                    backward: key,
                })
            };
        }
    }
    Ok(NavigationKeys {
        forward: "ArrowRight",
        backward: "ArrowLeft",
    })
}

async fn navigate_to_position(
    page: &Page,
    target: i64,
    use_private_api: bool,
    keys: Option<NavigationKeys>,
    options: &WebCaptureOptions,
) -> Result<i64> {
    if use_private_api {
        let previous = reader_state(page).await?.progress();
        page.evaluate(format!("KindleRenderer.gotoPosition({target})"))
            .await
            .context("failed to move to the requested start position")?;
        wait_after_navigation(page, previous, options).await?;
        return reader_state(page)
            .await?
            .progress()
            .context("Kindle Reader did not report the selected start position");
    }

    let keys = keys.context("Reader navigation keys were not detected")?;
    for attempt in 0..10_000 {
        let current = reader_state(page)
            .await?
            .progress()
            .context("Kindle Reader did not report the current position")?;
        if current == target {
            return Ok(current);
        }
        let moving_forward = target > current;
        let key = if moving_forward {
            keys.forward
        } else {
            keys.backward
        };
        let Some(next) = press_key_and_wait(page, key, current, options).await? else {
            return Ok(current);
        };
        ensure!(
            (moving_forward && next > current) || (!moving_forward && next < current),
            "Kindle Reader moved in the unexpected direction ({current} -> {next})"
        );
        if (moving_forward && next >= target) || (!moving_forward && next <= target) {
            return Ok(next);
        }
        if attempt > 0 && attempt % 25 == 0 {
            eprintln!("moving to start position: {next} toward {target}");
        }
    }
    bail!("could not reach Kindle position {target} within 10000 page turns")
}

async fn press_key_and_wait(
    page: &Page,
    key: &str,
    previous: i64,
    options: &WebCaptureOptions,
) -> Result<Option<i64>> {
    press_page_key(page, key)
        .await
        .with_context(|| format!("failed to send {key} to Kindle Web Reader"))?;
    let deadline = Instant::now() + Duration::from_secs_f64(options.wait_timeout);
    while Instant::now() < deadline {
        if let Ok(state) = reader_state(page).await
            && let Some(current) = state.progress()
            && current != previous
        {
            sleep(Duration::from_millis(300)).await;
            return Ok(Some(current));
        }
        sleep(Duration::from_millis(100)).await;
    }
    Ok(None)
}

async fn press_page_key(page: &Page, key: &str) -> Result<()> {
    let definition =
        keys::get_key_definition(key).with_context(|| format!("unsupported browser key: {key}"))?;
    let event = |event_type| {
        DispatchKeyEventParams::builder()
            .r#type(event_type)
            .key(definition.key)
            .code(definition.code)
            .windows_virtual_key_code(definition.key_code)
            .native_virtual_key_code(definition.key_code)
            .build()
            .map_err(anyhow::Error::msg)
    };
    page.execute(event(DispatchKeyEventType::RawKeyDown)?)
        .await
        .context("failed to send browser key-down event")?;
    page.execute(event(DispatchKeyEventType::KeyUp)?)
        .await
        .context("failed to send browser key-up event")?;
    Ok(())
}

async fn wait_after_navigation(
    page: &Page,
    previous: Option<i64>,
    options: &WebCaptureOptions,
) -> Result<bool> {
    let timeout = Duration::from_secs_f64(options.wait_timeout);
    if options.wait_strategy == WaitStrategy::Fixed {
        sleep(timeout).await;
        return Ok(true);
    }
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(state) = reader_state(page).await
            && (previous.is_none() || state.progress() != previous)
        {
            sleep(Duration::from_millis(300)).await;
            return Ok(true);
        }
        sleep(Duration::from_millis(100)).await;
    }
    if options.wait_strategy == WaitStrategy::Hybrid {
        sleep(Duration::from_millis(500)).await;
    }
    Ok(false)
}

async fn wait_for_spinner(page: &Page, timeout: Duration) {
    const SCRIPT: &str = r#"(() => {
const nodes = document.querySelectorAll('[role="progressbar"], [class*="loading"], [class*="spinner"]');
return [...nodes].every(node => {
  const style = getComputedStyle(node);
  return style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0';
});
})()"#;
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let hidden = page
            .evaluate(SCRIPT)
            .await
            .ok()
            .and_then(|result| result.into_value::<bool>().ok())
            .unwrap_or(false);
        if hidden {
            return;
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn dismiss_dialogs(page: &Page) {
    const SCRIPT: &str = r#"(() => {
const alert = document.querySelector('ion-alert[is-open="true"]');
if (alert) {
  const no = alert.buttons?.find(button => ['No', 'いいえ'].includes(button.text)) ?? alert.buttons?.[0];
  try { no?.handler?.(); } catch (_) {}
  alert.remove();
}
const wanted = ['No', 'No Thanks', 'Not Now', 'Close', 'Cancel', 'いいえ', 'キャンセル', '閉じる', '今はしない'];
for (const button of document.querySelectorAll('button, [role="button"]')) {
  const text = (button.textContent || '').trim();
  if (wanted.some(label => text === label || text.includes(label))) button.click();
}
})()"#;
    let _ = page.evaluate(SCRIPT).await;
}

async fn set_layout(page: &Page, layout: Layout) -> Result<bool> {
    const OPEN_SETTINGS: &str = r#"(() => {
const aria = ['Reader settings', 'リーダー設定', '表示設定'];
const controls = [...document.querySelectorAll('button, [role="button"]')];
const target = controls.find(node => aria.includes(node.getAttribute('aria-label')) || (node.textContent || '').trim() === 'Aa');
if (!target) return false;
target.click();
return true;
})()"#;
    let opened = page
        .evaluate(OPEN_SETTINGS)
        .await
        .context("failed to open reader settings")?
        .into_value::<bool>()
        .context("reader settings response had an unexpected shape")?;
    if !opened {
        return Ok(false);
    }
    sleep(Duration::from_millis(500)).await;
    let labels = if layout == Layout::Single {
        ["Single Column", "単一列", "1列", "1 カラム", "1カラム"]
    } else {
        ["Two Columns", "見開き", "2列", "2 カラム", "2カラム"]
    };
    let labels = serde_json::to_string(&labels)?;
    let script = format!(
        r#"(() => {{
const labels = {labels};
const target = [...document.querySelectorAll('button, [role="button"], label')]
  .find(node => labels.some(label => (node.textContent || '').trim().includes(label)));
if (!target) return false;
target.click();
return true;
}})()"#
    );
    let selected = page
        .evaluate(script)
        .await
        .context("failed to select reader layout")?
        .into_value::<bool>()
        .context("reader layout response had an unexpected shape")?;
    sleep(Duration::from_millis(300)).await;
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{
        chrome_launch_args, chrome_launch_command, profile_is_default_or_nested,
        reader_state_from_text,
    };

    #[test]
    fn normal_chrome_launch_args_do_not_advertise_automation() {
        let args = chrome_launch_args(
            Path::new("/tmp/kindle-profile"),
            9445,
            "https://read.amazon.co.jp/?asin=B0H6W8JQ6B",
            1920,
            1080,
            false,
        );

        assert!(args.iter().any(|arg| arg == "--remote-debugging-port=9445"));
        assert!(
            args.iter()
                .any(|arg| arg == "--user-data-dir=/tmp/kindle-profile")
        );
        assert!(
            args.iter()
                .any(|arg| arg == "https://read.amazon.co.jp/?asin=B0H6W8JQ6B")
        );
        assert!(!args.iter().any(|arg| arg.contains("enable-automation")));
        assert!(!args.iter().any(|arg| arg.contains("use-mock-keychain")));
        assert!(!args.iter().any(|arg| arg.contains("disable-sync")));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn normal_chrome_launch_uses_a_new_macos_app_instance() {
        let args = vec!["--remote-debugging-port=9445".to_owned()];
        let command = chrome_launch_command(
            Path::new("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            &args,
        );
        let actual_args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(command.get_program(), "/usr/bin/open");
        assert_eq!(
            actual_args,
            [
                "-na",
                "/Applications/Google Chrome.app",
                "--args",
                "--remote-debugging-port=9445"
            ]
        );
    }

    #[test]
    fn visible_location_text_is_a_fallback_when_the_private_api_is_unavailable() {
        let japanese = reader_state_from_text(
            "ヒトとＡＩ (岩波新書)\n位置1039/2352  ●  41%\n読書速度を学習中...",
        )
        .unwrap();
        let english = reader_state_from_text("Location 1,028 of 2,352 41%").unwrap();

        assert_eq!(japanese.current, Some(1039));
        assert_eq!(japanese.maximum, Some(2352));
        assert!(japanese.has_next);
        assert!(!japanese.api_ready);
        assert_eq!(english.current, Some(1028));
        assert_eq!(english.maximum, Some(2352));
    }

    #[test]
    fn default_chrome_profile_aliases_and_children_are_rejected() {
        let directory = tempdir().unwrap();
        let default = directory.path().join("Google/Chrome");
        let nested = default.join("Default");
        fs::create_dir_all(&nested).unwrap();

        assert!(profile_is_default_or_nested(&default, &default).unwrap());
        assert!(profile_is_default_or_nested(&nested, &default).unwrap());
        assert!(profile_is_default_or_nested(&default.join("../Chrome"), &default).unwrap());
        assert!(
            !profile_is_default_or_nested(&directory.path().join("kindle-profile"), &default)
                .unwrap()
        );
    }
}
