use std::fs;

use image::{DynamicImage, GrayImage, Luma};
use kindle_capture::image_ops::{
    CropBox, Region, difference_hash, list_page_images, mean_image_difference, parse_crop_box,
    parse_region, sanitize_book_name,
};
use tempfile::tempdir;

#[test]
fn parses_and_validates_capture_region() {
    assert_eq!(
        parse_region("120,90,1800,2400").unwrap(),
        Region {
            x: 120,
            y: 90,
            width: 1800,
            height: 2400,
        }
    );
    assert!(parse_region("1,2,0,4").is_err());
    assert!(parse_region("1,2,3").is_err());
}

#[test]
fn parses_crop_box_and_rejects_inverted_edges() {
    assert_eq!(
        parse_crop_box("10,20,300,400").unwrap(),
        CropBox {
            left: 10,
            top: 20,
            right: 300,
            bottom: 400,
        }
    );
    assert!(parse_crop_box("300,20,10,400").is_err());
}

#[test]
fn sanitizes_book_name_for_a_single_output_directory() {
    assert_eq!(
        sanitize_book_name("  Rust/../Kindle:Guide  "),
        "Rust_.._Kindle_Guide"
    );
    assert_eq!(sanitize_book_name("/\\:*?\"<>|"), "kindle_book");
}

#[test]
fn page_images_are_sorted_numerically_and_unrelated_files_are_ignored() {
    let directory = tempdir().unwrap();
    for name in [
        "page_10.png",
        "page_2.png",
        "page_0001.png",
        "page_x.png",
        "cover.png",
    ] {
        fs::write(directory.path().join(name), b"fixture").unwrap();
    }

    let names = list_page_images(directory.path())
        .unwrap()
        .into_iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(names, ["page_0001.png", "page_2.png", "page_10.png"]);
}

#[test]
fn image_similarity_is_zero_for_equal_images_and_nonzero_for_changed_images() {
    let left = DynamicImage::ImageLuma8(GrayImage::from_fn(16, 16, |x, _| {
        Luma([u8::try_from(x * 8).unwrap()])
    }));
    let same = left.clone();
    let changed = DynamicImage::ImageLuma8(GrayImage::from_pixel(16, 16, Luma([255])));

    assert_eq!(difference_hash(&left), difference_hash(&same));
    assert_eq!(mean_image_difference(&left, &same), 0.0);
    assert!(mean_image_difference(&left, &changed) > 0.0);
}
