use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use chrono::Local;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use printpdf::{Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, RawImage, XObjectTransform};
use serde_json::{Value, json};

pub use crate::image_ops::DuplicateThresholds;
use crate::image_ops::{CropBox, compare_image_files, is_duplicate, list_page_images};
use crate::metadata::write_json_atomic;

#[derive(Debug, Eq, PartialEq)]
pub struct TrimSummary {
    pub processed: usize,
    pub output_dir: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DedupeSummary {
    pub duplicates: Vec<PathBuf>,
    pub applied: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PdfSummary {
    pub pages: usize,
    pub output: PathBuf,
}

pub fn trim_images(
    input: &Path,
    output: &Path,
    crop: CropBox,
    selected_pages: Option<&[u64]>,
) -> Result<TrimSummary> {
    ensure!(
        std::path::absolute(input)? != std::path::absolute(output)?,
        "trim output must be different from the input directory"
    );
    ensure_directory_is_new_or_empty(output)?;

    let selected = selected_pages.map(|pages| pages.iter().copied().collect::<HashSet<_>>());
    let pages = list_page_images(input)?
        .into_iter()
        .filter(|path| {
            selected.as_ref().is_none_or(|selected| {
                page_number(path).is_some_and(|number| selected.contains(&number))
            })
        })
        .collect::<Vec<_>>();
    ensure!(
        !pages.is_empty(),
        "no matching page_*.png images found in {}",
        input.display()
    );

    let mut original_size = None;
    for path in &pages {
        let (width, height) = image::image_dimensions(path)
            .with_context(|| format!("failed to inspect image: {}", path.display()))?;
        validate_crop(crop, width, height, path)?;
        original_size.get_or_insert((width, height));
    }

    fs::create_dir_all(output)
        .with_context(|| format!("failed to create trim output: {}", output.display()))?;
    for path in &pages {
        let image = image::open(path)
            .with_context(|| format!("failed to open image: {}", path.display()))?;
        let cropped = image.crop_imm(
            crop.left,
            crop.top,
            crop.right - crop.left,
            crop.bottom - crop.top,
        );
        let destination = output.join(
            path.file_name()
                .context("page image does not have a file name")?,
        );
        cropped
            .save(&destination)
            .with_context(|| format!("failed to save cropped image: {}", destination.display()))?;
    }

    let (original_width, original_height) = original_size.context("missing image dimensions")?;
    let metadata = json!({
        "source_dir": std::path::absolute(input)?.to_string_lossy(),
        "crop_box": {
            "left": crop.left,
            "top": crop.top,
            "right": crop.right,
            "bottom": crop.bottom
        },
        "original_size": {
            "width": original_width,
            "height": original_height
        },
        "trimmed_size": {
            "width": crop.right - crop.left,
            "height": crop.bottom - crop.top
        },
        "total_pages": pages.len(),
        "trimmed_at": Local::now().to_rfc3339(),
        "note": Value::Null,
        "history": []
    });
    write_json_atomic(&output.join("trim_metadata.json"), &metadata)?;

    Ok(TrimSummary {
        processed: pages.len(),
        output_dir: output.to_owned(),
    })
}

pub fn dedupe_tail(
    input: &Path,
    thresholds: DuplicateThresholds,
    min_pages: usize,
    apply: bool,
) -> Result<DedupeSummary> {
    ensure!(min_pages > 0, "min_pages must be greater than zero");
    ensure!(
        thresholds.mean_difference.is_finite() && thresholds.mean_difference >= 0.0,
        "mean difference must be a non-negative finite number"
    );
    ensure!(
        thresholds
            .size_kb
            .is_none_or(|value| value.is_finite() && value >= 0.0),
        "size threshold must be a non-negative finite number"
    );
    ensure!(
        thresholds
            .size_ratio
            .is_none_or(|value| value.is_finite() && value >= 0.0),
        "size ratio must be a non-negative finite number"
    );
    let pages = list_page_images(input)?;
    if pages.len() < 2 {
        return Ok(DedupeSummary {
            duplicates: Vec::new(),
            applied: apply,
        });
    }

    let mut duplicates = Vec::new();
    let mut index = pages.len() - 1;
    while index > 0 && pages.len() - duplicates.len() > min_pages {
        if images_are_duplicates(&pages[index - 1], &pages[index], thresholds)? {
            duplicates.push(pages[index].clone());
            index -= 1;
        } else {
            break;
        }
    }

    if apply && !duplicates.is_empty() {
        let trash = input.join(".dedupe-trash");
        for path in &duplicates {
            let destination = trash.join(
                path.file_name()
                    .context("duplicate image does not have a file name")?,
            );
            ensure!(
                !destination.exists(),
                "refusing to overwrite recoverable file: {}",
                destination.display()
            );
        }
        fs::create_dir_all(&trash)
            .with_context(|| format!("failed to create recovery directory: {}", trash.display()))?;
        for path in &duplicates {
            let destination = trash.join(
                path.file_name()
                    .context("duplicate image does not have a file name")?,
            );
            fs::rename(path, &destination).with_context(|| {
                format!(
                    "failed to move duplicate {} to {}",
                    path.display(),
                    destination.display()
                )
            })?;
        }
        update_capture_metadata(input, &duplicates)?;
    }

    duplicates.reverse();
    Ok(DedupeSummary {
        duplicates,
        applied: apply,
    })
}

pub fn create_pdf(
    input: &Path,
    output: &Path,
    resize: f32,
    quality: u8,
    overwrite: bool,
) -> Result<PdfSummary> {
    ensure!(
        (0.1..=1.0).contains(&resize),
        "resize must be between 0.1 and 1.0"
    );
    ensure!(
        (1..=100).contains(&quality),
        "quality must be between 1 and 100"
    );
    if output.exists() && !overwrite {
        bail!(
            "output already exists; pass --overwrite to replace it: {}",
            output.display()
        );
    }
    let pages = list_page_images(input)?;
    ensure!(
        !pages.is_empty(),
        "no page_*.png images found in {}",
        input.display()
    );

    let mut document = PdfDocument::new(
        input
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Kindle capture"),
    );
    let mut pdf_pages = Vec::with_capacity(pages.len());
    let mut warnings = Vec::new();
    const DPI: f32 = 96.0;

    for path in &pages {
        let bytes = if resize < 1.0 {
            let image = image::open(path)
                .with_context(|| format!("failed to open image: {}", path.display()))?;
            let width = ((image.width() as f32 * resize).round() as u32).max(1);
            let height = ((image.height() as f32 * resize).round() as u32).max(1);
            let resized = image
                .resize_exact(width, height, FilterType::Lanczos3)
                .to_rgb8();
            let mut bytes = Cursor::new(Vec::new());
            JpegEncoder::new_with_quality(&mut bytes, quality)
                .encode_image(&resized)
                .with_context(|| format!("failed to encode resized image: {}", path.display()))?;
            bytes.into_inner()
        } else {
            fs::read(path).with_context(|| format!("failed to read image: {}", path.display()))?
        };

        let raw = RawImage::decode_from_bytes(&bytes, &mut warnings)
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("failed to decode image for PDF: {}", path.display()))?;
        let width = raw.width;
        let height = raw.height;
        let image_id = document.add_image(&raw);
        let operation = Op::UseXobject {
            id: image_id,
            transform: XObjectTransform {
                dpi: Some(DPI),
                ..Default::default()
            },
        };
        pdf_pages.push(PdfPage::new(
            Mm(width as f32 / DPI * 25.4),
            Mm(height as f32 / DPI * 25.4),
            vec![operation],
        ));
    }

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create PDF output directory: {}",
                parent.display()
            )
        })?;
    }
    let bytes = document
        .with_pages(pdf_pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    let temporary = temporary_path(output);
    fs::write(&temporary, bytes)
        .with_context(|| format!("failed to write temporary PDF: {}", temporary.display()))?;
    fs::rename(&temporary, output)
        .with_context(|| format!("failed to finalize PDF: {}", output.display()))?;

    Ok(PdfSummary {
        pages: pages.len(),
        output: output.to_owned(),
    })
}

fn validate_crop(crop: CropBox, width: u32, height: u32, path: &Path) -> Result<()> {
    ensure!(
        crop.right <= width && crop.bottom <= height,
        "crop box exceeds image dimensions for {} ({}x{})",
        path.display(),
        width,
        height
    );
    Ok(())
}

fn ensure_directory_is_new_or_empty(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    ensure!(
        path.is_dir(),
        "output is not a directory: {}",
        path.display()
    );
    ensure!(
        fs::read_dir(path)
            .with_context(|| format!("failed to inspect output directory: {}", path.display()))?
            .next()
            .is_none(),
        "output directory is not empty: {}; remove its contents or pass a different --output",
        path.display()
    );
    Ok(())
}

fn page_number(path: &Path) -> Option<u64> {
    path.file_stem()?
        .to_str()?
        .strip_prefix("page_")?
        .parse()
        .ok()
}

fn images_are_duplicates(
    left_path: &Path,
    right_path: &Path,
    thresholds: DuplicateThresholds,
) -> Result<bool> {
    let similarity = compare_image_files(left_path, right_path)?;
    Ok(is_duplicate(similarity, thresholds))
}

fn update_capture_metadata(input: &Path, removed: &[PathBuf]) -> Result<()> {
    let path = input.join("metadata.json");
    if !path.exists() {
        return Ok(());
    }
    let mut metadata: Value = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("failed to read metadata: {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse metadata: {}", path.display()))?;
    let removed_names = removed
        .iter()
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .collect::<HashSet<_>>();
    let removed_numbers = removed
        .iter()
        .filter_map(|path| page_number(path))
        .collect::<HashSet<_>>();
    if let Some(pages) = metadata.get_mut("pages").and_then(Value::as_array_mut) {
        pages.retain(|page| {
            let keeps_file = page
                .get("file")
                .and_then(Value::as_str)
                .is_none_or(|name| !removed_names.contains(name));
            let keeps_page_number = page
                .get("page")
                .and_then(Value::as_u64)
                .is_none_or(|number| !removed_numbers.contains(&number));
            keeps_file && keeps_page_number
        });
    }
    let remaining_pages = list_page_images(input)?.len();
    metadata["total_pages"] = json!(remaining_pages);
    write_json_atomic(&path, &metadata)
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    path.with_file_name(format!(".{file_name}.partial"))
}
