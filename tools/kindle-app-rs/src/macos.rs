use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use chrono::Local;
use clap::ValueEnum;
use serde::Deserialize;

use crate::image_ops::{
    DuplicateThresholds, ImageSimilarity, Region, compare_image_files, difference_hash,
    is_duplicate,
};
use crate::metadata::{AppMetadata, AppPageMetadata, CaptureRegionMetadata, write_json_atomic};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum NextKey {
    Right,
    Left,
    Space,
    #[value(name = "pagedown")]
    #[serde(rename = "pagedown")]
    PageDown,
}

impl NextKey {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Right => "right",
            Self::Left => "left",
            Self::Space => "space",
            Self::PageDown => "pagedown",
        }
    }

    const fn key_code(self) -> &'static str {
        match self {
            Self::Right => "124",
            Self::Left => "123",
            Self::Space => "49",
            Self::PageDown => "121",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppCaptureOptions {
    pub book: String,
    pub output: PathBuf,
    pub app_name: String,
    pub process_name: String,
    pub window_title: Option<String>,
    pub region: Option<Region>,
    pub scale: f64,
    pub wait_after_turn: f64,
    pub initial_wait: f64,
    pub max_pages: Option<usize>,
    pub duplicate_threshold: u32,
    pub duplicate_diff_mean: f64,
    pub duplicate_size_kb: Option<f64>,
    pub duplicate_size_ratio: Option<f64>,
    pub duplicate_limit: usize,
    pub min_pages: usize,
    pub next_key: NextKey,
    pub overwrite: bool,
}

pub fn scale_region(region: Region, scale: f64) -> Result<Region> {
    ensure!(
        scale.is_finite() && scale > 0.0,
        "scale must be a positive finite number"
    );
    let scale_signed = |value: i32, label: &str| -> Result<i32> {
        let scaled = (f64::from(value) * scale).round();
        ensure!(
            scaled >= f64::from(i32::MIN) && scaled <= f64::from(i32::MAX),
            "scaled {label} is outside the supported range"
        );
        Ok(scaled as i32)
    };
    let scale_unsigned = |value: u32, label: &str| -> Result<u32> {
        let scaled = (f64::from(value) * scale).round();
        ensure!(
            scaled >= 1.0 && scaled <= f64::from(u32::MAX),
            "scaled {label} is outside the supported range"
        );
        u32::try_from(scaled as u64)
            .with_context(|| format!("scaled {label} is outside the supported range"))
    };

    Ok(Region {
        x: scale_signed(region.x, "x")?,
        y: scale_signed(region.y, "y")?,
        width: scale_unsigned(region.width, "width")?,
        height: scale_unsigned(region.height, "height")?,
    })
}

pub fn capture_book(options: &AppCaptureOptions) -> Result<AppMetadata> {
    ensure!(
        cfg!(target_os = "macos"),
        "app capture is supported only on macOS"
    );
    ensure!(!options.book.trim().is_empty(), "book name cannot be empty");
    ensure!(
        options.wait_after_turn.is_finite() && options.wait_after_turn >= 0.0,
        "wait must be a non-negative finite number"
    );
    ensure!(
        options.initial_wait.is_finite() && options.initial_wait >= 0.0,
        "initial wait must be a non-negative finite number"
    );
    ensure!(
        options.duplicate_limit > 0,
        "duplicate limit must be greater than zero"
    );
    ensure!(
        options.min_pages > 0,
        "minimum pages must be greater than zero"
    );
    ensure!(
        options.max_pages.is_none_or(|maximum| maximum > 0),
        "max pages must be greater than zero"
    );
    ensure!(
        options.duplicate_diff_mean.is_finite() && options.duplicate_diff_mean >= 0.0,
        "duplicate mean difference must be a non-negative finite number"
    );
    ensure!(
        options
            .duplicate_size_kb
            .is_none_or(|value| value.is_finite() && value >= 0.0),
        "duplicate size threshold must be a non-negative finite number"
    );
    ensure!(
        options
            .duplicate_size_ratio
            .is_none_or(|value| value.is_finite() && value >= 0.0),
        "duplicate size ratio must be a non-negative finite number"
    );
    if !options.overwrite {
        crate::capture_output::prepare(&options.output, false)?;
    }

    if !is_app_running(&options.process_name)? {
        bail!("{} is not running", options.process_name);
    }
    activate_app(&options.process_name)?;
    let raw_region = match options.region {
        Some(region) => region,
        None => window_region(&options.process_name, options.window_title.as_deref())?,
    };
    let region = scale_region(raw_region, options.scale)?;
    if options.overwrite {
        crate::capture_output::prepare(&options.output, true)?;
    }
    if options.initial_wait > 0.0 {
        thread::sleep(Duration::from_secs_f64(options.initial_wait));
    }

    let mut pages = Vec::new();
    let mut previous_path: Option<PathBuf> = None;
    let mut duplicate_streak = 0_usize;
    let mut attempts = 0_usize;
    let page_limit = options.max_pages.unwrap_or(2_000);

    loop {
        let page_number = pages.len() + 1;
        let filename = format!("page_{page_number:04}.png");
        let final_path = options.output.join(&filename);
        let pending_path = options
            .output
            .join(format!(".{page_number:04}.pending.png"));
        capture_region(region, &pending_path)?;
        attempts += 1;

        let similarity = previous_path
            .as_deref()
            .map(|previous| compare_image_files(previous, &pending_path))
            .transpose()?;
        if similarity.is_some_and(|metrics| is_duplicate(metrics, duplicate_thresholds(options))) {
            duplicate_streak += 1;
            fs::remove_file(&pending_path).with_context(|| {
                format!(
                    "failed to remove duplicate temporary image: {}",
                    pending_path.display()
                )
            })?;
            eprintln!(
                "duplicate page detected ({duplicate_streak}/{})",
                options.duplicate_limit
            );
            if attempts >= options.min_pages && duplicate_streak >= options.duplicate_limit {
                break;
            }
            send_next_page(&options.process_name, options.next_key)?;
            sleep_after_turn(options.wait_after_turn);
            continue;
        }

        duplicate_streak = 0;
        fs::rename(&pending_path, &final_path)
            .with_context(|| format!("failed to finalize screenshot: {}", final_path.display()))?;
        let image = image::open(&final_path)
            .with_context(|| format!("failed to open screenshot: {}", final_path.display()))?;
        let hash = difference_hash(&image);
        let size_kb = fs::metadata(&final_path)?.len() as f64 / 1024.0;
        pages.push(app_page_metadata(
            page_number,
            filename,
            hash,
            size_kb,
            similarity,
        ));
        previous_path = Some(final_path);
        eprintln!("captured page {page_number}");

        if pages.len() >= page_limit {
            eprintln!("reached page limit: {page_limit}");
            break;
        }
        send_next_page(&options.process_name, options.next_key)?;
        sleep_after_turn(options.wait_after_turn);
    }

    let metadata = AppMetadata {
        source: "app".into(),
        book: options.book.clone(),
        app_name: options.app_name.clone(),
        window_title: options.window_title.clone(),
        capture_region: CaptureRegionMetadata {
            x: region.x,
            y: region.y,
            width: region.width,
            height: region.height,
            scale: options.scale,
        },
        total_pages: pages.len(),
        wait_after_turn: options.wait_after_turn,
        duplicate_threshold: options.duplicate_threshold,
        duplicate_diff_mean: options.duplicate_diff_mean,
        duplicate_size_kb: options.duplicate_size_kb,
        duplicate_size_ratio: options.duplicate_size_ratio,
        duplicate_limit: options.duplicate_limit,
        min_pages: options.min_pages,
        next_key: options.next_key.as_str().into(),
        captured_at: Local::now().to_rfc3339(),
        pages,
    };
    write_json_atomic(&options.output.join("metadata.json"), &metadata)?;
    Ok(metadata)
}

fn duplicate_thresholds(options: &AppCaptureOptions) -> DuplicateThresholds {
    DuplicateThresholds {
        hash_distance: options.duplicate_threshold,
        mean_difference: options.duplicate_diff_mean,
        size_kb: options.duplicate_size_kb,
        size_ratio: options.duplicate_size_ratio,
    }
}

fn app_page_metadata(
    page: usize,
    file: String,
    hash: u64,
    size_kb: f64,
    similarity: Option<ImageSimilarity>,
) -> AppPageMetadata {
    AppPageMetadata {
        page,
        file,
        timestamp: Local::now().to_rfc3339(),
        hash: format!("{hash:016x}"),
        hash_distance: similarity.map(|metrics| metrics.hash_distance),
        mean_diff: similarity.map(|metrics| round(metrics.mean_difference, 4)),
        size_kb: round(size_kb, 2),
        size_delta_kb: similarity.map(|metrics| round(metrics.size_delta_kb, 2)),
        size_delta_ratio: similarity.map(|metrics| round(metrics.size_ratio, 6)),
    }
}

fn round(value: f64, places: i32) -> f64 {
    let factor = 10_f64.powi(places);
    (value * factor).round() / factor
}

fn run_osascript(script: &str, arguments: &[&str]) -> Result<String> {
    let output = Command::new("osascript")
        .args(["-e", script])
        .args(arguments)
        .output()
        .context("failed to start osascript")?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!("osascript failed: {message}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn is_app_running(process_name: &str) -> Result<bool> {
    let script = r#"on run argv
tell application "System Events" to return exists process (item 1 of argv)
end run"#;
    Ok(run_osascript(script, &[process_name])?.eq_ignore_ascii_case("true"))
}

fn activate_app(process_name: &str) -> Result<()> {
    let script = r#"on run argv
tell application "System Events"
  set targetProcess to process (item 1 of argv)
  set frontmost of targetProcess to true
end tell
end run"#;
    run_osascript(script, &[process_name])?;
    thread::sleep(Duration::from_millis(300));
    Ok(())
}

fn window_region(process_name: &str, window_title: Option<&str>) -> Result<Region> {
    let script = r#"on run argv
set processName to item 1 of argv
set wantedTitle to item 2 of argv
tell application "System Events"
  tell process processName
    if (count of windows) is 0 then error "Window not found"
    set targetWindow to item 1 of windows
    if wantedTitle is not "" then
      set targetWindow to missing value
      repeat with candidate in windows
        if name of candidate contains wantedTitle then
          set targetWindow to candidate
          exit repeat
        end if
      end repeat
      if targetWindow is missing value then error "Window not found"
    end if
    set windowPosition to position of targetWindow
    set windowSize to size of targetWindow
    set AppleScript's text item delimiters to ","
    return {item 1 of windowPosition, item 2 of windowPosition, item 1 of windowSize, item 2 of windowSize} as text
  end tell
end tell
end run"#;
    let output = run_osascript(script, &[process_name, window_title.unwrap_or("")])?;
    crate::image_ops::parse_region(&output).context("unexpected Kindle window bounds")
}

fn capture_region(region: Region, output: &Path) -> Result<()> {
    let region = format!(
        "{},{},{},{}",
        region.x, region.y, region.width, region.height
    );
    let status = Command::new("screencapture")
        .args(["-x", "-t", "png", "-R", &region])
        .arg(output)
        .status()
        .context("failed to start screencapture")?;
    ensure!(status.success(), "screencapture failed with {status}");
    Ok(())
}

fn send_next_page(process_name: &str, key: NextKey) -> Result<()> {
    let script = r#"on run argv
tell application "System Events"
  set frontmost of process (item 1 of argv) to true
  key code (item 2 of argv as integer)
end tell
end run"#;
    run_osascript(script, &[process_name, key.key_code()])?;
    Ok(())
}

fn sleep_after_turn(seconds: f64) {
    if seconds > 0.0 {
        thread::sleep(Duration::from_secs_f64(seconds));
    }
}
