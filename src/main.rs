use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use log::LevelFilter;

use bir::discover;
use bir::imaging::{self, OutputSpec};
use bir::watermark::{Position, WatermarkOptions};
use rayon::prelude::*;

/// Bir - bulk image resizer.
///
/// Recursively finds images in a directory and creates longest-side resized
/// copies, optionally converting them to another format.
#[derive(Parser, Debug)]
#[command(name = "bir", version, about)]
struct Cli {
    /// Image file or directory containing the images to process.
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

    /// Overlay this image as a watermark on every output.
    #[arg(long)]
    watermark: Option<PathBuf>,

    /// Anchor for the watermark: northwest, north, northeast, west, center,
    /// east, southwest, south or southeast.
    #[arg(long, value_enum)]
    watermark_position: Option<Position>,

    /// Tile the watermark over the whole image instead of placing it once.
    #[arg(long, conflicts_with = "watermark_position")]
    watermark_repeat: bool,

    /// Gap between tiled watermarks as a percentage of the watermark width
    /// (repeat mode only).
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u32).range(0..=500))]
    watermark_gap: u32,

    /// Watermark opacity (0-100, default 100).
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u8).range(0..=100))]
    watermark_opacity: u8,

    /// Tilt the watermark by this many degrees (-90 to 90, positive is
    /// counter-clockwise).
    #[arg(long, default_value_t = 0.0)]
    watermark_angle: f64,

    /// Overwrite existing outputs (default: skip with a warning).
    #[arg(long)]
    force: bool,

    /// Print per-file results (Resized/Skipped lines).
    #[arg(short, long)]
    verbose: bool,

    /// Number of parallel worker threads (default: number of CPU cores).
    #[arg(short = 'j', long)]
    jobs: Option<usize>,

    /// Process only the top directory, not subdirectories.
    #[arg(long = "no-recursive", default_value_t = true, action = clap::ArgAction::SetFalse)]
    recursive: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let multi = MultiProgress::new();
    init_logging(multi.clone(), cli.verbose);
    imaging::ensure_genesis();

    if !(-90.0..=90.0).contains(&cli.watermark_angle) {
        anyhow::bail!(
            "watermark angle {} is out of range (-90 to 90)",
            cli.watermark_angle
        );
    }

    if let Some(jobs) = cli.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global()
            .context("failed to configure the thread pool")?;
    }

    let files = if cli.input_dir.is_dir() {
        discover::find_images(&cli.input_dir, &cli.resolutions, cli.recursive)?
    } else {
        discover::single_image(&cli.input_dir)?
    };
    println!(
        "Found {} source image(s) in {}",
        files.len(),
        cli.input_dir.display()
    );

    /// One pending output: source file plus its destination spec.
    struct Job {
        input: PathBuf,
        spec: OutputSpec,
    }

    let mut jobs = Vec::new();
    let mut skipped = 0u32;

    // For a single-file input there is no subfolder structure to mirror, so
    // the scan root is treated as the file's own directory.
    let root = if cli.input_dir.is_dir() {
        cli.input_dir.clone()
    } else {
        files[0]
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    };

    for file in &files {
        for size in &cli.resolutions {
            let spec = build_output_spec(&root, file, *size, &cli);

            if spec.path.exists() && !cli.force {
                log::info!("Skipping (exists): {}", spec.path.display());
                skipped += 1;
                continue;
            }

            if let Some(parent) = spec.path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("cannot create output directory {}", parent.display())
                })?;
            }

            jobs.push(Job {
                input: file.clone(),
                spec,
            });
        }
    }

    let bar = multi.add(ProgressBar::with_draw_target(
        Some(jobs.len() as u64),
        ProgressDrawTarget::stderr(),
    ));
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} files ({eta})",
        )
        .expect("valid progress template")
        .progress_chars("#>-"),
    );

    // Results are streamed: the bar updates as each file finishes, not after the batch.
    let created = AtomicU32::new(0);
    let failed = AtomicU32::new(0);

    jobs.par_iter().for_each(|job| {
        let result = imaging::resize_file(&job.input, &job.spec);
        match result {
            Ok(()) => {
                log::info!(
                    "Resized: {} -> {}",
                    job.input.display(),
                    job.spec.path.display()
                );
                created.fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                log::error!("Failed:  {} -- {err}", job.input.display());
                failed.fetch_add(1, Ordering::Relaxed);
            }
        }
        bar.inc(1);
    });

    bar.finish();

    let created = created.load(Ordering::Relaxed);
    let failed = failed.load(Ordering::Relaxed);

    println!("Done: {created} created, {skipped} skipped, {failed} failed.");

    if failed > 0 {
        anyhow::bail!("{failed} output(s) failed");
    }
    Ok(())
}

/// Initialize logging: per-file results with `--verbose`, errors and warnings only otherwise.
/// Log lines are routed through the given `MultiProgress` (add every bar to it)
/// so they don't smear over the progress bar.
fn init_logging(multi: MultiProgress, verbose: bool) {
    let level = if verbose {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };
    let logger = env_logger::Builder::new()
        .filter_level(level)
        .format_timestamp(None)
        .build();
    LogWrapper::new(multi, logger)
        .try_init()
        .expect("failed to init env_logger");
    log::set_max_level(level);
}

/// Compute the destination path and conversion settings for one output.
///
/// Naming mirrors `bir.sh`: `<stem>_x<size>.<ext>` next to the original, or
/// inside `--output-dir` when given, where the source's subfolder structure
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
        watermark: cli.watermark.clone().map(|path| WatermarkOptions {
            path,
            position: cli.watermark_position.unwrap_or(Position::SouthEast),
            repeat: cli.watermark_repeat,
            gap: cli.watermark_gap,
            opacity: cli.watermark_opacity,
            angle: cli.watermark_angle,
        }),
    }
}
