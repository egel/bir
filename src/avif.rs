//! AVIF output support, implemented on top of the pure-Rust `ravif` crate.
//!
//! ImageMagick cannot write AVIF here, because no HEIF delegate library is
//! vendored. Instead, `imaging` exports the resized pixels as raw RGBA and
//! hands them to [`encode_rgba_to_file`], which encodes them with `rav1e`
//! and writes the file. This keeps all AVIF-specific knowledge in one
//! module and leaves the vendored ImageMagick build untouched.

use std::fs;
use std::path::Path;

use ravif::{Encoder, Img, RGBA8};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AvifError {
    #[error("{message}")]
    Encode { message: String },
    #[error("cannot write {path}: {source}")]
    Write {
        path: Box<Path>,
        #[source]
        source: std::io::Error,
    },
}

/// True when the requested output format/extension means AVIF.
/// Matching is case-insensitive, so `-f AVIF` works too.
pub fn is_requested_format(format: &str) -> bool {
    format.eq_ignore_ascii_case("avif")
}

/// Encode a raw RGBA8 buffer (`row-major`, fully opaque or alpha) to an
/// AVIF file at `path`.
///
/// `quality` is the 0-100 scale used across bir; higher means better
/// quality. When `None`, the `ravif` default quality (75) is used.
///
/// Speed is fixed at 6 (1 slowest/best, 10 fastest) as a bulk-friendly
/// compromise between file size and throughput.
pub fn encode_rgba_to_file(
    rgba: Vec<RGBA8>,
    width: usize,
    height: usize,
    quality: Option<u8>,
    path: &Path,
) -> Result<(), AvifError> {
    let mut encoder = Encoder::new().with_speed(6);
    if let Some(quality) = quality {
        encoder = encoder.with_quality(quality as f32);
    }

    let encoded = encoder
        .encode_rgba(Img::new(&rgba, width, height))
        .map_err(|e| AvifError::Encode {
            message: e.to_string(),
        })?;

    fs::write(path, encoded.avif_file).map_err(|source| AvifError::Write {
        path: path.into(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_format_case_insensitively() {
        for format in ["avif", "AVIF", "AvIf"] {
            assert!(is_requested_format(format));
        }
        for format in ["png", "webp", "jpeg", "avifs"] {
            assert!(!is_requested_format(format));
        }
    }

    #[test]
    fn encodes_and_writes_avif() {
        let dir = std::env::temp_dir().join(format!("bir-avif-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("solid.avif");

        // 8x8 solid red, fully opaque.
        let rgba = vec![RGBA8 { r: 255, g: 0, b: 0, a: 255 }; 64];
        encode_rgba_to_file(rgba, 8, 8, Some(80), &path).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        // AVIF files start with a `ftyp` box carrying the `avif` brand.
        assert_eq!(&bytes[4..12], b"ftypavif");
    }
}
