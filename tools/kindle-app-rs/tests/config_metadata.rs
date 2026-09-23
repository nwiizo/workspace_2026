use std::path::Path;

use kindle_capture::config::{Config, expand_tilde};
use kindle_capture::image_ops::Region;
use kindle_capture::macos::{NextKey, scale_region};
use kindle_capture::metadata::WebMetadata;
use kindle_capture::web::{Layout, WaitStrategy};

#[test]
fn partial_yaml_keeps_defaults_for_unspecified_settings() {
    let config = Config::from_yaml(
        r#"
browser:
  viewport_width: 1920
capture:
  default_layout: single
  wait_strategy: location_change
app_capture:
  next_key: pagedown
"#,
    )
    .unwrap();

    assert_eq!(config.browser.viewport_width, 1920);
    assert_eq!(config.browser.viewport_height, 2160);
    assert_eq!(config.capture.default_layout, Layout::Single);
    assert_eq!(config.capture.wait_strategy, WaitStrategy::LocationChange);
    assert_eq!(config.app_capture.next_key, NextKey::PageDown);
    assert_eq!(config.pdf.default_quality, 85);
}

#[test]
fn empty_yaml_uses_all_defaults() {
    let config = Config::from_yaml("  \n").unwrap();

    assert_eq!(config.capture.default_layout, Layout::Double);
    assert_eq!(config.app_capture.duplicate_limit, 5);
}

#[test]
fn missing_config_file_uses_safe_dedicated_profile_defaults() {
    let config = Config::load(Path::new("definitely-not-present.yaml")).unwrap();

    assert!(
        config
            .browser
            .chrome_profile
            .contains("kindle-app-rs/chrome-profile")
    );
    assert!(!config.browser.chrome_profile.contains("Google/Chrome"));
    assert_eq!(config.app_capture.process_name, "Kindle");
    assert_eq!(config.app_capture.max_pages, Some(2_000));
}

#[test]
fn tilde_expansion_only_changes_a_leading_home_component() {
    assert!(!expand_tilde("~/captures").starts_with("~"));
    assert_eq!(
        expand_tilde("captures/~archive"),
        Path::new("captures/~archive")
    );
}

#[test]
fn scaled_regions_are_checked() {
    assert_eq!(
        scale_region(
            Region {
                x: -10,
                y: 20,
                width: 100,
                height: 200,
            },
            2.0,
        )
        .unwrap(),
        Region {
            x: -20,
            y: 40,
            width: 200,
            height: 400,
        }
    );
    assert!(
        scale_region(
            Region {
                x: 0,
                y: 0,
                width: 1,
                height: 1
            },
            0.0
        )
        .is_err()
    );
}

#[test]
fn web_metadata_keeps_the_original_top_level_shape() {
    let metadata = WebMetadata {
        asin: "B0DSKPTJM5".into(),
        layout: "single".into(),
        total_pages: 0,
        position_range: [496, 141_946],
        capture_range: [496, 141_946],
        captured_at: "2026-08-31T15:30:00+09:00".into(),
        pages: Vec::new(),
    };
    let value = serde_json::to_value(metadata).unwrap();

    assert_eq!(value["asin"], "B0DSKPTJM5");
    assert_eq!(value["total_pages"], 0);
    assert!(value.get("source").is_none());
    assert!(value.get("schema_version").is_none());
}
