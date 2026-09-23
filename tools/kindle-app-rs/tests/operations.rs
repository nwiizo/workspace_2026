use std::fs;
use std::path::Path;

use image::{ImageFormat, Rgb, RgbImage};
use kindle_capture::image_ops::CropBox;
use kindle_capture::operations::{DuplicateThresholds, create_pdf, dedupe_tail, trim_images};
use tempfile::tempdir;

fn save_fixture(path: &Path, width: u32, height: u32, value: u8) {
    let image = RgbImage::from_pixel(width, height, Rgb([value, value, value]));
    image.save_with_format(path, ImageFormat::Png).unwrap();
}

#[test]
fn trim_creates_selected_crops_without_touching_sources() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input");
    let output = directory.path().join("output");
    fs::create_dir(&input).unwrap();
    save_fixture(&input.join("page_0001.png"), 100, 80, 10);
    save_fixture(&input.join("page_0002.png"), 100, 80, 20);

    let summary = trim_images(
        &input,
        &output,
        CropBox {
            left: 10,
            top: 5,
            right: 90,
            bottom: 65,
        },
        Some(&[2]),
    )
    .unwrap();

    assert_eq!(summary.processed, 1);
    assert!(!output.join("page_0001.png").exists());
    assert_eq!(
        image::open(output.join("page_0002.png")).unwrap().width(),
        80
    );
    assert_eq!(
        image::open(input.join("page_0002.png")).unwrap().width(),
        100
    );
    assert!(output.join("trim_metadata.json").exists());
}

#[test]
fn dedupe_is_a_dry_run_by_default_and_apply_moves_recoverable_files() {
    let directory = tempdir().unwrap();
    let input = directory.path();
    save_fixture(&input.join("page_0001.png"), 32, 32, 0);
    save_fixture(&input.join("page_0002.png"), 32, 32, 255);
    save_fixture(&input.join("page_0003.png"), 32, 32, 255);

    let thresholds = DuplicateThresholds {
        hash_distance: 0,
        mean_difference: 0.0,
        size_kb: None,
        size_ratio: None,
    };
    let preview = dedupe_tail(input, thresholds, 1, false).unwrap();
    assert_eq!(preview.duplicates.len(), 1);
    assert!(input.join("page_0003.png").exists());

    let applied = dedupe_tail(input, thresholds, 1, true).unwrap();
    assert_eq!(applied.duplicates.len(), 1);
    assert!(!input.join("page_0003.png").exists());
    assert!(input.join(".dedupe-trash/page_0003.png").exists());
}

#[test]
fn dedupe_updates_web_pages_by_page_number_without_adding_top_level_fields() {
    let directory = tempdir().unwrap();
    let input = directory.path();
    save_fixture(&input.join("page_0001.png"), 32, 32, 0);
    save_fixture(&input.join("page_0002.png"), 32, 32, 255);
    save_fixture(&input.join("page_0003.png"), 32, 32, 255);
    fs::write(
        input.join("metadata.json"),
        r#"{
          "asin":"B0H6W8JQ6B",
          "layout":"double",
          "total_pages":3,
          "position_range":[1,30],
          "capture_range":[1,30],
          "captured_at":"2026-08-31T17:00:00+09:00",
          "pages":[
            {"page":1,"location":{"current":1,"total":30,"percent":3},"timestamp":"a"},
            {"page":2,"location":{"current":15,"total":30,"percent":50},"timestamp":"b"},
            {"page":3,"location":{"current":15,"total":30,"percent":50},"timestamp":"c"}
          ]
        }"#,
    )
    .unwrap();

    dedupe_tail(
        input,
        DuplicateThresholds {
            hash_distance: 0,
            mean_difference: 0.0,
            size_kb: None,
            size_ratio: None,
        },
        1,
        true,
    )
    .unwrap();

    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(input.join("metadata.json")).unwrap()).unwrap();
    let keys = metadata.as_object().unwrap().keys().collect::<Vec<_>>();
    assert_eq!(metadata["total_pages"], 2);
    assert_eq!(metadata["pages"].as_array().unwrap().len(), 2);
    assert_eq!(metadata["pages"][1]["page"], 2);
    assert_eq!(
        keys,
        [
            "asin",
            "capture_range",
            "captured_at",
            "layout",
            "pages",
            "position_range",
            "total_pages"
        ]
    );
}

#[test]
fn pdf_contains_one_page_per_input_image() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input");
    let output = directory.path().join("book.pdf");
    fs::create_dir(&input).unwrap();
    save_fixture(&input.join("page_0001.png"), 120, 80, 20);
    save_fixture(&input.join("page_0002.png"), 80, 120, 220);

    let summary = create_pdf(&input, &output, 1.0, 85, false).unwrap();
    let document = lopdf::Document::load(&output).unwrap();

    assert_eq!(summary.pages, 2);
    assert_eq!(document.get_pages().len(), 2);
    let original = fs::read(&output).unwrap();
    assert!(create_pdf(&input, &output, 1.0, 85, false).is_err());
    assert_eq!(fs::read(&output).unwrap(), original);
}
