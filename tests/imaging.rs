//! Integration tests for the imaging engine. Fixtures are generated
//! programmatically with MagickWand, so no binary assets are committed.

use bir::imaging::{self, OutputSpec};
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bir-tests-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn resize_png_to_width() {
    imaging::ensure_genesis();
    let dir = tmp_dir("resize-png");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 400, 300).unwrap();

    let output = dir.join("photo_w200.png");
    imaging::resize_file(
        &input,
        &OutputSpec {
            width: 200,
            path: output.clone(),
            format: None,
            quality: None,
        },
    )
    .unwrap();

    assert!(output.exists());
    let (w, h) = imaging::image_dimensions(&output).unwrap();
    assert_eq!((w, h), (200, 150)); // aspect ratio preserved
}

#[test]
fn no_upscale_for_narrow_images() {
    imaging::ensure_genesis();
    let dir = tmp_dir("no-upscale");
    let input = dir.join("tiny.png");
    imaging::write_solid_image(&input, 100, 80).unwrap();

    let output = dir.join("tiny_w400.png");
    imaging::resize_file(
        &input,
        &OutputSpec {
            width: 400,
            path: output.clone(),
            format: None,
            quality: None,
        },
    )
    .unwrap();

    let (w, h) = imaging::image_dimensions(&output).unwrap();
    assert_eq!((w, h), (100, 80)); // untouched, not upscaled
}

#[test]
fn convert_to_webp() {
    imaging::ensure_genesis();
    let dir = tmp_dir("convert-webp");
    let input = dir.join("snapshot.png");
    imaging::write_solid_image(&input, 200, 200).unwrap();

    let output = dir.join("snapshot_w100.webp");
    imaging::resize_file(
        &input,
        &OutputSpec {
            width: 100,
            path: output.clone(),
            format: Some("webp".into()),
            quality: Some(80),
        },
    )
    .unwrap();

    let (w, h) = imaging::image_dimensions(&output).unwrap();
    assert_eq!((w, h), (100, 100));
}

#[test]
fn convert_jpeg_with_quality() {
    imaging::ensure_genesis();
    let dir = tmp_dir("convert-jpeg");
    let input = dir.join("picture.png");
    imaging::write_solid_image(&input, 300, 200).unwrap();

    let output = dir.join("picture_w50.jpg");
    imaging::resize_file(
        &input,
        &OutputSpec {
            width: 50,
            path: output.clone(),
            format: Some("jpeg".into()),
            quality: Some(75),
        },
    )
    .unwrap();

    let (w, h) = imaging::image_dimensions(&output).unwrap();
    assert_eq!((w, h), (50, 33)); // 50 * (200/300) rounds to 33
}

#[test]
fn missing_input_is_an_error() {
    imaging::ensure_genesis();
    let dir = tmp_dir("missing-input");
    let missing = dir.join("does-not-exist.png");

    let err = imaging::resize_file(
        &missing,
        &OutputSpec {
            width: 200,
            path: dir.join("x_w200.png"),
            format: None,
            quality: None,
        },
    )
    .unwrap_err();

    assert!(err.to_string().contains("does-not-exist.png"));
}

#[test]
fn tiff_roundtrip() {
    imaging::ensure_genesis();
    let dir = tmp_dir("tiff");
    let input = dir.join("scan.png");
    imaging::write_solid_image(&input, 160, 90).unwrap();

    let output = dir.join("scan_w80.tif");
    imaging::resize_file(
        &input,
        &OutputSpec {
            width: 80,
            path: output.clone(),
            format: None,
            quality: None,
        },
    )
    .unwrap();

    let (w, h) = imaging::image_dimensions(&output).unwrap();
    assert_eq!((w, h), (80, 45));
}
