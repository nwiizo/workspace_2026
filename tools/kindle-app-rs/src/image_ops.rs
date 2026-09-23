use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use image::{DynamicImage, imageops::FilterType};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CropBox {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageSimilarity {
    pub hash_distance: u32,
    pub mean_difference: f64,
    pub size_delta_kb: f64,
    pub size_ratio: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct DuplicateThresholds {
    pub hash_distance: u32,
    pub mean_difference: f64,
    pub size_kb: Option<f64>,
    pub size_ratio: Option<f64>,
}

pub fn is_duplicate(similarity: ImageSimilarity, thresholds: DuplicateThresholds) -> bool {
    similarity.hash_distance <= thresholds.hash_distance
        && similarity.mean_difference <= thresholds.mean_difference
        && thresholds
            .size_kb
            .is_none_or(|maximum| similarity.size_delta_kb <= maximum)
        && thresholds
            .size_ratio
            .is_none_or(|maximum| similarity.size_ratio <= maximum)
}

pub fn parse_region(value: &str) -> Result<Region> {
    let values = parse_four_integers(value, "region")?;
    ensure!(
        values[2] > 0 && values[3] > 0,
        "region width and height must be positive"
    );

    Ok(Region {
        x: i32::try_from(values[0]).context("region x is outside the supported range")?,
        y: i32::try_from(values[1]).context("region y is outside the supported range")?,
        width: u32::try_from(values[2]).context("region width is outside the supported range")?,
        height: u32::try_from(values[3]).context("region height is outside the supported range")?,
    })
}

pub fn parse_crop_box(value: &str) -> Result<CropBox> {
    let values = parse_four_integers(value, "crop box")?;
    ensure!(
        values.iter().all(|value| *value >= 0),
        "crop coordinates cannot be negative"
    );
    ensure!(
        values[0] < values[2],
        "crop left must be smaller than right"
    );
    ensure!(
        values[1] < values[3],
        "crop top must be smaller than bottom"
    );

    Ok(CropBox {
        left: u32::try_from(values[0]).context("crop left is outside the supported range")?,
        top: u32::try_from(values[1]).context("crop top is outside the supported range")?,
        right: u32::try_from(values[2]).context("crop right is outside the supported range")?,
        bottom: u32::try_from(values[3]).context("crop bottom is outside the supported range")?,
    })
}

fn parse_four_integers(value: &str, label: &str) -> Result<[i64; 4]> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    ensure!(
        parts.len() == 4,
        "{label} must contain exactly four comma-separated integers"
    );

    let mut values = [0_i64; 4];
    for (index, part) in parts.into_iter().enumerate() {
        values[index] = part
            .parse::<i64>()
            .with_context(|| format!("invalid integer in {label}: {part}"))?;
    }
    Ok(values)
}

pub fn sanitize_book_name(name: &str) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;

    for character in name.trim().chars().take(120) {
        let invalid = character.is_control()
            || matches!(
                character,
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
            );
        if invalid {
            if !last_was_separator && !output.is_empty() {
                output.push('_');
            }
            last_was_separator = true;
        } else {
            output.push(character);
            last_was_separator = false;
        }
    }

    let output = output.trim_matches([' ', '_']).to_owned();
    if output.is_empty() || output == "." || output == ".." {
        "kindle_book".to_owned()
    } else {
        output
    }
}

pub fn list_page_images(directory: &Path) -> Result<Vec<PathBuf>> {
    ensure!(
        directory.is_dir(),
        "input directory does not exist: {}",
        directory.display()
    );

    let mut pages = Vec::new();
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("failed to read input directory: {}", directory.display()))?
    {
        let entry = entry.context("failed to read directory entry")?;
        if !entry
            .file_type()
            .context("failed to inspect directory entry")?
            .is_file()
        {
            continue;
        }
        let path = entry.path();
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Some(number) = stem.strip_prefix("page_") else {
            continue;
        };
        if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let number = number
            .parse::<u64>()
            .with_context(|| format!("page number is too large: {stem}"))?;
        pages.push((number, path));
    }

    pages.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.file_name().cmp(&right.1.file_name()))
    });
    Ok(pages.into_iter().map(|(_, path)| path).collect())
}

pub fn difference_hash(image: &DynamicImage) -> u64 {
    let grayscale = image.resize_exact(9, 8, FilterType::Lanczos3).to_luma8();
    let mut hash = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            hash <<= 1;
            if grayscale.get_pixel(x, y)[0] > grayscale.get_pixel(x + 1, y)[0] {
                hash |= 1;
            }
        }
    }
    hash
}

pub fn mean_image_difference(left: &DynamicImage, right: &DynamicImage) -> f64 {
    let left = left.resize_exact(64, 64, FilterType::Lanczos3).to_luma8();
    let right = right.resize_exact(64, 64, FilterType::Lanczos3).to_luma8();
    let total = left
        .pixels()
        .zip(right.pixels())
        .map(|(left, right)| u64::from(left[0].abs_diff(right[0])))
        .sum::<u64>();
    total as f64 / f64::from(64 * 64)
}

pub fn compare_image_files(left_path: &Path, right_path: &Path) -> Result<ImageSimilarity> {
    let left = image::open(left_path)
        .with_context(|| format!("failed to open image: {}", left_path.display()))?;
    let right = image::open(right_path)
        .with_context(|| format!("failed to open image: {}", right_path.display()))?;
    let hash_distance = (difference_hash(&left) ^ difference_hash(&right)).count_ones();
    let mean_difference = mean_image_difference(&left, &right);
    let left_size = std::fs::metadata(left_path)?.len() as f64 / 1024.0;
    let right_size = std::fs::metadata(right_path)?.len() as f64 / 1024.0;
    let size_delta_kb = (left_size - right_size).abs();
    let size_ratio = if left_size > 0.0 {
        size_delta_kb / left_size
    } else {
        0.0
    };

    Ok(ImageSimilarity {
        hash_distance,
        mean_difference,
        size_delta_kb,
        size_ratio,
    })
}
