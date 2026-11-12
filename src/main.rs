use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Bir - bulk image resizer.
///
/// Recursively finds images in a directory and creates width-based resized
/// copies, optionally converting them to another format.
#[derive(Parser, Debug)]
#[command(name = "bir", version, about)]
struct Cli {
    /// Directory containing the images to process.
    input_dir: PathBuf,

    /// Comma-separated target widths, e.g. 200,400,800.
    #[arg(short, long, value_delimiter = ',', default_values_t = vec![200u32, 400, 800])]
    resolutions: Vec<u32>,

    /// Convert images to this format/extension (default: keep original).
    #[arg(short, long)]
    format: Option<String>,

    /// Output directory (default: alongside originals). Created if missing.
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Quality for lossy formats (0-100).
    #[arg(short, long)]
    quality: Option<u8>,

    /// Overwrite existing outputs (default: skip with a warning).
    #[arg(long)]
    force: bool,

    /// Process subdirectories as well (default: recursive, matching bir.sh).
    #[arg(long = "no-recursive", default_value_t = true, action = clap::ArgAction::SetFalse, hide = true)]
    recursive: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    eprintln!(
        "bir: input={:?} resolutions={:?} format={:?} output_dir={:?} quality={:?} force={} recursive={}",
        cli.input_dir,
        cli.resolutions,
        cli.format,
        cli.output_dir,
        cli.quality,
        cli.force,
        cli.recursive,
    );
    eprintln!("not implemented yet: engine lands in Step 3");

    Ok(())
}
