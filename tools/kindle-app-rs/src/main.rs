use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use clap::{Args, Parser, Subcommand};
use kindle_capture::config::{Config, expand_tilde};
use kindle_capture::image_ops::{parse_crop_box, parse_region, sanitize_book_name};
use kindle_capture::macos::{AppCaptureOptions, NextKey, capture_book as capture_app};
use kindle_capture::operations::{DuplicateThresholds, create_pdf, dedupe_tail, trim_images};
use kindle_capture::web::{
    DEFAULT_DEBUG_PORT, Layout, WaitStrategy, WebCaptureOptions, capture_book as capture_web,
    validate_asin,
};

#[derive(Debug, Parser)]
#[command(
    name = "kindle-capture",
    version,
    about = "Capture Kindle pages on macOS and turn screenshots into PDFs"
)]
struct Cli {
    /// YAML settings. If omitted, ./config.yaml is read when present.
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Capture pages from Kindle Web Reader using a dedicated Chrome profile.
    Web(WebArgs),
    /// Capture pages from the foreground Kindle macOS app.
    App(AppArgs),
    /// Convert page_*.png screenshots to a PDF.
    Pdf(PdfArgs),
    /// Crop page_*.png screenshots into a new directory.
    Trim(TrimArgs),
    /// Find trailing duplicate pages; use --apply to move them to recovery storage.
    DedupeTail(DedupeArgs),
}

#[derive(Debug, Args)]
struct WebArgs {
    #[arg(long)]
    asin: String,
    /// Reader layout. Falls back to capture.default_layout.
    #[arg(long, value_enum)]
    layout: Option<Layout>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    start: Option<i64>,
    #[arg(long)]
    end: Option<i64>,
    /// Run Chrome without a visible window; login must already be complete.
    #[arg(long)]
    headless: bool,
    /// Stop after this many captured pages.
    #[arg(long)]
    max_pages: Option<usize>,
    /// Browser viewport width. Falls back to browser.viewport_width (3840 built in).
    #[arg(long)]
    viewport_width: Option<u32>,
    /// Browser viewport height. Falls back to browser.viewport_height (2160 built in).
    #[arg(long)]
    viewport_height: Option<u32>,
    /// Dedicated Chrome data directory. The normal Chrome profile is rejected.
    #[arg(long)]
    chrome_profile: Option<String>,
    #[arg(long)]
    chrome_path: Option<String>,
    /// DevTools port used by the dedicated normal Chrome session (9445 built in).
    #[arg(long, default_value_t = DEFAULT_DEBUG_PORT)]
    debug_port: u16,
    /// Page readiness method. Falls back to capture.wait_strategy (hybrid built in).
    #[arg(long, value_enum)]
    wait_strategy: Option<WaitStrategy>,
    #[arg(long)]
    wait_timeout: Option<f64>,
    #[arg(long)]
    browser_timeout_ms: Option<u64>,
    #[arg(long)]
    login_timeout_ms: Option<u64>,
    #[arg(long)]
    no_wait_for_login: bool,
    #[arg(long)]
    overwrite: bool,
}

#[derive(Debug, Args)]
struct AppArgs {
    #[arg(long)]
    book: String,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    app_name: Option<String>,
    #[arg(long)]
    process_name: Option<String>,
    #[arg(long)]
    window_title: Option<String>,
    #[arg(long)]
    region: Option<String>,
    /// Coordinate multiplier. Falls back to app_capture.scale (1.0 built in).
    #[arg(long)]
    scale: Option<f64>,
    /// Seconds to wait after a page turn (0.6 built in).
    #[arg(long = "wait")]
    wait_after_turn: Option<f64>,
    #[arg(long)]
    initial_wait: Option<f64>,
    /// Captured page limit (2000 built in as a disk-usage safeguard).
    #[arg(long)]
    max_pages: Option<usize>,
    /// Maximum dHash distance for duplicate detection (3 built in).
    #[arg(long)]
    dup_threshold: Option<u32>,
    /// Maximum mean pixel difference for duplicate detection (3.0 built in).
    #[arg(long)]
    dup_diff_mean: Option<f64>,
    #[arg(long)]
    dup_size_kb: Option<f64>,
    #[arg(long)]
    dup_size_ratio: Option<f64>,
    #[arg(long)]
    dup_limit: Option<usize>,
    #[arg(long)]
    min_pages: Option<usize>,
    #[arg(long, value_enum)]
    next_key: Option<NextKey>,
    #[arg(long)]
    overwrite: bool,
}

#[derive(Debug, Args)]
struct PdfArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
    /// JPEG quality used only when resizing (85 built in).
    #[arg(long)]
    quality: Option<u8>,
    /// Resize ratio from 0.1 through 1.0 (1.0 built in).
    #[arg(long)]
    resize: Option<f32>,
    #[arg(long)]
    overwrite: bool,
}

#[derive(Debug, Args)]
struct TrimArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    crop: String,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, value_delimiter = ',')]
    pages: Vec<u64>,
}

#[derive(Debug, Args)]
struct DedupeArgs {
    #[arg(long)]
    input: PathBuf,
    /// Maximum dHash distance for duplicate detection (3 built in).
    #[arg(long)]
    dup_threshold: Option<u32>,
    /// Maximum mean pixel difference for duplicate detection (3.0 built in).
    #[arg(long)]
    dup_diff_mean: Option<f64>,
    #[arg(long)]
    dup_size_kb: Option<f64>,
    #[arg(long)]
    dup_size_ratio: Option<f64>,
    #[arg(long)]
    min_pages: Option<usize>,
    #[arg(long)]
    apply: bool,
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("kindle-capture: {error:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let Cli { config, command } = cli;
    let config = if let Some(path) = config {
        ensure!(
            path.is_file(),
            "config file does not exist: {}",
            path.display()
        );
        Config::load(&path)?
    } else {
        Config::load(Path::new("config.yaml"))?
    };
    match command {
        Command::Web(args) => tokio::runtime::Runtime::new()
            .context("failed to create async runtime")?
            .block_on(run_web(args, &config)),
        Command::App(args) => run_app(args, &config),
        Command::Pdf(args) => run_pdf(args, &config),
        Command::Trim(args) => run_trim(args, &config),
        Command::DedupeTail(args) => run_dedupe(args, &config),
    }
}

async fn run_web(args: WebArgs, config: &Config) -> Result<()> {
    let asin = validate_asin(&args.asin)?;
    let output = args
        .output
        .unwrap_or_else(|| expand_tilde(&config.capture.output_dir).join(&asin));
    let options = WebCaptureOptions {
        asin,
        layout: args.layout.unwrap_or(config.capture.default_layout),
        output,
        start: args.start,
        end: args.end,
        headless: args.headless || config.browser.headless,
        max_pages: args.max_pages.or(config.capture.max_pages),
        viewport_width: args.viewport_width.unwrap_or(config.browser.viewport_width),
        viewport_height: args
            .viewport_height
            .unwrap_or(config.browser.viewport_height),
        chrome_profile: expand_tilde(
            args.chrome_profile
                .as_deref()
                .unwrap_or(&config.browser.chrome_profile),
        ),
        chrome_path: args.chrome_path.as_deref().map(expand_tilde),
        debug_port: args.debug_port,
        wait_strategy: args.wait_strategy.unwrap_or(config.capture.wait_strategy),
        wait_timeout: args.wait_timeout.unwrap_or(config.capture.wait_timeout),
        browser_timeout_ms: args.browser_timeout_ms.unwrap_or(config.browser.timeout),
        wait_for_login: config.browser.wait_for_login && !args.no_wait_for_login,
        login_timeout_ms: args
            .login_timeout_ms
            .unwrap_or(config.browser.login_timeout),
        overwrite: args.overwrite,
    };
    let metadata = capture_web(&options).await?;
    println!(
        "Captured {} pages to {}",
        metadata.total_pages,
        options.output.display()
    );
    Ok(())
}

fn run_app(args: AppArgs, config: &Config) -> Result<()> {
    let app = &config.app_capture;
    let output = args
        .output
        .unwrap_or_else(|| expand_tilde(&app.output_dir).join(sanitize_book_name(&args.book)));
    let region = args
        .region
        .as_deref()
        .or(app.capture_region.as_deref())
        .map(parse_region)
        .transpose()?;
    let options = AppCaptureOptions {
        book: args.book,
        output,
        app_name: args.app_name.unwrap_or_else(|| app.app_name.clone()),
        process_name: args
            .process_name
            .unwrap_or_else(|| app.process_name.clone()),
        window_title: args.window_title.or_else(|| app.window_title.clone()),
        region,
        scale: args.scale.unwrap_or(app.scale),
        wait_after_turn: args.wait_after_turn.unwrap_or(app.wait_after_turn),
        initial_wait: args.initial_wait.unwrap_or(app.initial_wait),
        max_pages: args.max_pages.or(app.max_pages),
        duplicate_threshold: args.dup_threshold.unwrap_or(app.duplicate_threshold),
        duplicate_diff_mean: args.dup_diff_mean.unwrap_or(app.duplicate_diff_mean),
        duplicate_size_kb: args.dup_size_kb.or(app.duplicate_size_kb),
        duplicate_size_ratio: args.dup_size_ratio.or(app.duplicate_size_ratio),
        duplicate_limit: args.dup_limit.unwrap_or(app.duplicate_limit),
        min_pages: args.min_pages.unwrap_or(app.min_pages),
        next_key: args.next_key.unwrap_or(app.next_key),
        overwrite: args.overwrite,
    };
    let metadata = capture_app(&options)?;
    println!(
        "Captured {} pages to {}",
        metadata.total_pages,
        options.output.display()
    );
    Ok(())
}

fn run_pdf(args: PdfArgs, config: &Config) -> Result<()> {
    let output = args.output.unwrap_or_else(|| default_pdf_path(&args.input));
    let summary = create_pdf(
        &args.input,
        &output,
        args.resize.unwrap_or(config.pdf.default_resize),
        args.quality.unwrap_or(config.pdf.default_quality),
        args.overwrite,
    )?;
    println!(
        "Created {} with {} pages",
        summary.output.display(),
        summary.pages
    );
    Ok(())
}

fn run_trim(args: TrimArgs, config: &Config) -> Result<()> {
    let output = args
        .output
        .unwrap_or_else(|| args.input.join(&config.trim.default_output_subdir));
    let selected = (!args.pages.is_empty()).then_some(args.pages.as_slice());
    let summary = trim_images(&args.input, &output, parse_crop_box(&args.crop)?, selected)?;
    println!(
        "Trimmed {} pages to {}",
        summary.processed,
        summary.output_dir.display()
    );
    Ok(())
}

fn run_dedupe(args: DedupeArgs, config: &Config) -> Result<()> {
    let app = &config.app_capture;
    let summary = dedupe_tail(
        &args.input,
        DuplicateThresholds {
            hash_distance: args.dup_threshold.unwrap_or(app.duplicate_threshold),
            mean_difference: args.dup_diff_mean.unwrap_or(app.duplicate_diff_mean),
            size_kb: args.dup_size_kb.or(app.duplicate_size_kb),
            size_ratio: args.dup_size_ratio.or(app.duplicate_size_ratio),
        },
        args.min_pages.unwrap_or(app.min_pages),
        args.apply,
    )?;
    if summary.duplicates.is_empty() {
        println!("No trailing duplicates found");
    } else if summary.applied {
        println!(
            "Moved {} duplicate pages to {}/.dedupe-trash",
            summary.duplicates.len(),
            args.input.display()
        );
    } else {
        println!(
            "Found {} trailing duplicate pages; rerun with --apply to move them",
            summary.duplicates.len()
        );
        for path in summary.duplicates {
            println!("{}", path.display());
        }
    }
    Ok(())
}

fn default_pdf_path(input: &Path) -> PathBuf {
    let name = input
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("kindle-capture");
    PathBuf::from(format!("{name}.pdf"))
}
