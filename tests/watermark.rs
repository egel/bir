//! Integration tests for watermarking. Fixtures are generated
//! programmatically with MagickWand, so no binary assets are committed.

use bir::imaging::{self, OutputSpec};
use bir::watermark::{Position, WatermarkOptions};
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bir-watermark-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn spec(size: u32, path: PathBuf, watermark: Option<WatermarkOptions>) -> OutputSpec {
    OutputSpec {
        size,
        path,
        format: None,
        quality: None,
        watermark,
    }
}

fn options(path: PathBuf) -> WatermarkOptions {
    WatermarkOptions {
        path,
        position: Position::SouthEast,
        repeat: false,
        gap: 50,
        opacity: 100,
        angle: 0.0,
    }
}

/// Red overlay on a white base composited with OverCompositeOp yields pure
/// red where the watermark covers the image and white elsewhere.
fn is_red(color: [f64; 4]) -> bool {
    color[0] > 0.9 && color[1] < 0.1 && color[2] < 0.1
}

fn is_white(color: [f64; 4]) -> bool {
    color[0] > 0.9 && color[1] > 0.9 && color[2] > 0.9
}

#[test]
fn single_watermark_anchors_to_southeast() {
    imaging::ensure_genesis();
    let dir = tmp_dir("single");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 400, 300).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let output = dir.join("photo_x200.png");
    imaging::resize_file(
        &input,
        &spec(200, output.clone(), Some(options(overlay.clone()))),
    )
    .unwrap();

    // 400x300 resized to 200x150; the 50x40 watermark sits in the corner.
    assert_eq!(imaging::image_dimensions(&output).unwrap(), (200, 150));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 199, 149).unwrap()
    ));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 150, 110).unwrap()
    ));
    assert!(is_white(
        bir::watermark::pixel_color(&output, 0, 0).unwrap()
    ));
}

#[test]
fn tilted_watermark_reveals_corner_and_rotates_mark() {
    imaging::ensure_genesis();
    let dir = tmp_dir("tilt");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 400, 300).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let output = dir.join("photo_x200.png");
    let mut opts = options(overlay.clone());
    opts.angle = 45.0;
    imaging::resize_file(
        &input,
        &spec(200, output.clone(), Some(opts)),
    )
    .unwrap();

    assert_eq!(imaging::image_dimensions(&output).unwrap(), (200, 150));
    // Without the tilt the corner pixel is covered by the mark; rotated by
    // 45 degrees the corner is revealed and the red area moves inward.
    assert!(is_white(
        bir::watermark::pixel_color(&output, 199, 149).unwrap()
    ));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 165, 115).unwrap()
    ));
}

#[test]
fn single_watermark_position_center() {
    imaging::ensure_genesis();
    let dir = tmp_dir("center");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 300, 300).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 100, 100, "red").unwrap();

    let output = dir.join("photo_x300.png");
    let mut opts = options(overlay);
    opts.position = Position::Center;
    imaging::resize_file(&input, &spec(300, output.clone(), Some(opts))).unwrap();

    // No upscale: the output stays 300x300 and the watermark is centered.
    assert_eq!(imaging::image_dimensions(&output).unwrap(), (300, 300));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 150, 150).unwrap()
    ));
    assert!(is_white(
        bir::watermark::pixel_color(&output, 10, 10).unwrap()
    ));
    assert!(is_white(
        bir::watermark::pixel_color(&output, 290, 290).unwrap()
    ));
}

#[test]
fn repeat_tiles_the_whole_image_with_relative_gap() {
    imaging::ensure_genesis();
    let dir = tmp_dir("repeat");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 300, 200).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let output = dir.join("photo_x400.png");
    let mut opts = options(overlay);
    opts.repeat = true;
    // Default gap 50%: steps of 75px horizontally, 60px vertically.
    imaging::resize_file(&input, &spec(400, output.clone(), Some(opts))).unwrap();

    assert_eq!(imaging::image_dimensions(&output).unwrap(), (300, 200));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 10, 10).unwrap()
    ));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 80, 10).unwrap()
    ));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 10, 65).unwrap()
    ));
    // In the gap between tiles the base image stays untouched.
    assert!(is_white(
        bir::watermark::pixel_color(&output, 60, 10).unwrap()
    ));
    assert!(is_white(
        bir::watermark::pixel_color(&output, 10, 50).unwrap()
    ));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 230, 185).unwrap()
    ));
}

#[test]
fn repeat_gap_percentage_changes_spacing() {
    imaging::ensure_genesis();
    let dir = tmp_dir("gap");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 300, 200).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let output = dir.join("photo_x400.png");
    let mut opts = options(overlay);
    opts.repeat = true;
    opts.gap = 0;
    // Gap 0: tiles touch, so the point between two tiles is also covered.
    imaging::resize_file(&input, &spec(400, output.clone(), Some(opts))).unwrap();

    assert!(is_red(
        bir::watermark::pixel_color(&output, 10, 10).unwrap()
    ));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 51, 10).unwrap()
    ));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 10, 41).unwrap()
    ));
}

#[test]
fn opacity_zero_leaves_the_base_untouched() {
    imaging::ensure_genesis();
    let dir = tmp_dir("opacity");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 400, 300).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let output = dir.join("photo_x200.png");
    let mut opts = options(overlay);
    opts.opacity = 0;
    imaging::resize_file(&input, &spec(200, output.clone(), Some(opts))).unwrap();

    assert!(is_white(
        bir::watermark::pixel_color(&output, 199, 149).unwrap()
    ));
}

#[test]
fn partial_opacity_blends_with_the_base() {
    imaging::ensure_genesis();
    let dir = tmp_dir("opacity-partial");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 400, 300).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let output = dir.join("photo_x200.png");
    let mut opts = options(overlay);
    opts.opacity = 50;
    imaging::resize_file(&input, &spec(200, output.clone(), Some(opts))).unwrap();

    let color = bir::watermark::pixel_color(&output, 199, 149).unwrap();
    // Red blended onto white: neither pure red nor pure white.
    assert!(!is_red(color) && !is_white(color), "color: {color:?}");
}

#[test]
fn missing_watermark_file_is_an_error() {
    imaging::ensure_genesis();
    let dir = tmp_dir("missing-overlay");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 400, 300).unwrap();

    let err = imaging::resize_file(
        &input,
        &spec(
            200,
            dir.join("photo_x200.png"),
            Some(options(dir.join("nope.png"))),
        ),
    )
    .unwrap_err();

    assert!(err.to_string().contains("nope.png"), "error: {err}");
}

#[test]
fn watermark_covering_oversized_overlay_is_clamped() {
    imaging::ensure_genesis();
    let dir = tmp_dir("oversized");
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 200, 100).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 300, 300, "red").unwrap();

    // Overlay larger than the (not upscaled) output: anchored placement
    // clamps to the top-left corner and the whole output is covered.
    let output = dir.join("photo_x400.png");
    imaging::resize_file(
        &input,
        &spec(400, output.clone(), Some(options(overlay.clone()))),
    )
    .unwrap();

    assert!(is_red(bir::watermark::pixel_color(&output, 0, 0).unwrap()));
    assert!(is_red(
        bir::watermark::pixel_color(&output, 199, 99).unwrap()
    ));
}

#[test]
fn cli_watermark_flag_produces_watermarked_outputs() {
    let dir = std::env::temp_dir().join(format!("bir-watermark-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("photo.png");
    imaging::write_solid_image(&input, 400, 300).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let exe = env!("CARGO_BIN_EXE_bir");
    let out = std::process::Command::new(exe)
        .args([
            dir.to_str().unwrap(),
            "-r",
            "200",
            "--watermark",
            overlay.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn bir");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let output = dir.join("photo_x200.png");
    assert!(output.exists());
    assert!(is_red(
        bir::watermark::pixel_color(&output, 199, 149).unwrap()
    ));
}

#[test]
fn cli_repeat_and_position_conflict_is_rejected() {
    let dir = std::env::temp_dir().join(format!("bir-watermark-cli2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    imaging::write_solid_image(&dir.join("photo.png"), 400, 300).unwrap();
    let overlay = dir.join("mark.png");
    imaging::write_solid_image_with_color(&overlay, 50, 40, "red").unwrap();

    let exe = env!("CARGO_BIN_EXE_bir");
    let out = std::process::Command::new(exe)
        .args([
            dir.to_str().unwrap(),
            "--watermark",
            overlay.to_str().unwrap(),
            "--watermark-repeat",
            "--watermark-position",
            "center",
        ])
        .output()
        .expect("failed to spawn bir");
    assert_ne!(Some(out.status.code().unwrap()), Some(0));
}
