use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use bir::discover;
use bir::imaging::{self, OutputSpec};

/// Bir - bulk image resizer.
///
/// Recursively finds images in a directory and creates longest-side resized
/// copies, optionally converting them to another format.
#[derive(Parser, Debug)]
#[command(name = "bir", version, about)]
struct Cli {
    /// Directory containing the images to process.
    input_dir: PathBuf,

    /// Comma-separated target sizes for the longest side, e.g. 200,400,800.
    #[arg(short, long, value_delimiter = ',', default_values_t = vec![200u32, 400, 800])]
    resolutions: Vec<u32>,

    /// Convert images to this format/extension (default: keep original).
    #[arg(short, long)]
    format: Option<String>,

    /// Output directory (default: alongside originals). Created if missing.
    /// The source's subfolder structure is mirrored inside it.
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Quality for lossy formats (0-100).
    #[arg(short, long, value_parser = clap::value_parser!(u8).range(0..=100))]
    quality: Option<u8>,

    /// Overwrite existing outputs (default: skip with a warning).
    #[arg(long)]
    force: bool,

    /// Process only the top directory, not subdirectories.
    #[arg(long = "no-recursive", default_value_t = true, action = clap::ArgAction::SetFalse)]
    recursive: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    imaging::ensure_genesis();

    let files = discover::find_images(&cli.input_dir, &cli.resolutions, cli.recursive)?;
    println!(
        "Found {} source image(s) in {}",
        files.len(),
        cli.input_dir.display()
    );

    let mut created = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for file in &files {
        for size in &cli.resolutions {
            let spec = build_output_spec(&cli.input_dir, file, *size, &cli);

            if spec.path.exists() && !cli.force {
                eprintln!("Skipping (exists): {}", spec.path.display());
                skipped += 1;
                continue;
            }

            if let Some(parent) = spec.path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("cannot create output directory {}", parent.display())
                })?;
            }

            match imaging::resize_file(file, &spec) {
                Ok(()) => {
                    println!("Resized: {} -> {}", file.display(), spec.path.display());
                    created += 1;
                }
                Err(err) => {
                    eprintln!("Failed:  {err}");
                    failed += 1;
                }
            }
        }
    }

    println!("Done: {created} created, {skipped} skipped, {failed} failed.");

    if failed > 0 {
        anyhow::bail!("{failed} output(s) failed");
    }
    Ok(())
}

/// Compute the destination path and conversion settings for one output.
///
/// Naming mirrors `bir.sh`: `<stem>_x<size>.<ext>` next to the original, or
/// inside `--output-dir` when given — where the source's subfolder structure
/// (relative to the scan root) is mirrored. Files outside the scan root fall
/// back to the flat output directory. The extension is `--format` when set,
/// otherwise the original file's extension.
fn build_output_spec(root: &Path, input: &Path, size: u32, cli: &Cli) -> OutputSpec {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_string());

    let ext = cli.format.clone().unwrap_or_else(|| {
        input
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_else(|| "png".to_string())
    });

    let dir = match &cli.output_dir {
        Some(out) => {
            let rel = input
                .strip_prefix(root)
                .unwrap_or(input)
                .parent()
                .unwrap_or_else(|| Path::new(""));
            out.join(rel)
        }
        None => input.parent().map(Path::to_path_buf).unwrap_or_default(),
    };

    OutputSpec {
        size,
        path: dir.join(format!("{stem}_x{size}.{ext}")),
        format: cli.format.clone(),
        quality: cli.quality,
    }
}
