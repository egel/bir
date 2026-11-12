//! End-to-end tests that run the compiled `bir` binary against real files.
//! Fixtures are generated with the engine, so no binary assets are committed.

use std::process::{Command, Output};

use bir::imaging;

fn run_bir(args: &[&str]) -> Output {
    let exe = env!("CARGO_BIN_EXE_bir");
    Command::new(exe)
        .args(args)
        .output()
        .expect("failed to spawn bir")
}

fn make_images(dir: &std::path::Path, files: &[(&str, usize, usize)]) {
    std::fs::create_dir_all(dir).unwrap();
    for (name, w, h) in files {
        imaging::write_solid_image(&dir.join(name), *w, *h).unwrap();
    }
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("bir was killed by a signal")
}

#[test]
fn end_to_end_resize_in_place() {
    let dir = std::env::temp_dir().join(format!("bir-cli-1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    make_images(&dir, &[("photo.png", 400, 300), ("wide.png", 100, 50)]);

    let out = run_bir(&[dir.to_str().unwrap()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    for size in [200, 400, 800] {
        let p = dir.join(format!("photo_x{size}.png"));
        assert!(p.exists(), "{} missing", p.display());
    }
    // aspect ratio preserved
    let (w, h) = imaging::image_dimensions(&dir.join("photo_x200.png")).unwrap();
    assert_eq!((w, h), (200, 150));
    // no upscale for the 100px-wide input
    let (w, h) = imaging::image_dimensions(&dir.join("wide_x400.png")).unwrap();
    assert_eq!((w, h), (100, 50));
}

#[test]
fn second_run_skips_all_outputs() {
    let dir = std::env::temp_dir().join(format!("bir-cli-2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    make_images(&dir, &[("photo.png", 400, 300)]);

    let first = run_bir(&[dir.to_str().unwrap()]);
    assert_eq!(code(&first), 0);

    let second = run_bir(&[dir.to_str().unwrap()]);
    assert_eq!(code(&second), 0);
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("3 skipped"), "stdout: {stdout}");
    assert!(stdout.contains("0 created"), "stdout: {stdout}");

    let forced = run_bir(&[dir.to_str().unwrap(), "--force"]);
    assert_eq!(code(&forced), 0);
    let stdout = String::from_utf8_lossy(&forced.stdout);
    assert!(stdout.contains("3 created"), "stdout: {stdout}");
}

#[test]
fn output_dir_and_format_conversion() {
    let dir = std::env::temp_dir().join(format!("bir-cli-3-{}", std::process::id()));
    let out = std::env::temp_dir().join(format!("bir-cli-3-out-{}", std::process::id()));
    let _ = (std::fs::remove_dir_all(&dir), std::fs::remove_dir_all(&out));
    make_images(&dir, &[("photo.png", 400, 300)]);

    let res = run_bir(&[
        dir.to_str().unwrap(),
        "-r",
        "100",
        "-f",
        "webp",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        code(&res),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&res.stderr)
    );

    let converted = out.join("photo_x100.webp");
    assert!(converted.exists());
    let (w, h) = imaging::image_dimensions(&converted).unwrap();
    assert_eq!((w, h), (100, 75));
    // originals directory untouched by the conversion
    assert!(!dir.join("photo_x100.webp").exists());
}

#[test]
fn missing_dir_and_empty_dir_exit_nonzero() {
    let out = run_bir(&["/definitely/not/here"]);
    assert_ne!(code(&out), 0);

    let dir = std::env::temp_dir().join(format!("bir-cli-4-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("readme.txt"), b"not an image").unwrap();
    let out = run_bir(&[dir.to_str().unwrap()]);
    assert_ne!(code(&out), 0);
}

#[test]
fn previous_run_outputs_are_not_sources() {
    let dir = std::env::temp_dir().join(format!("bir-cli-5-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    make_images(
        &dir,
        &[
            ("old_x200.png", 200, 120),
            ("legacy_w200.png", 200, 120), // legacy width-based output
            ("real.png", 400, 300),
        ],
    );

    let out = run_bir(&[dir.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("old_x200.png ->"),
        "old output was reprocessed: {stdout}"
    );
    assert!(
        !stdout.contains("legacy_w200.png ->"),
        "legacy output was reprocessed: {stdout}"
    );
    // configured-size outputs are still produced for the real source
    assert!(dir.join("real_x200.png").exists());
}
