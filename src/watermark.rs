//! Image watermarking: composite an overlay image onto resized outputs.
//!
//! The overlay is composited with `OverCompositeOp` after resizing, before
//! the output write, so both plain and AVIF outputs receive it. Two modes
//! are supported: a single placement anchored to an edge (gravity style)
//! and tiling over the whole image with a gap expressed as a percentage of
//! the watermark's own width, which keeps the visual rhythm consistent
//! across output sizes.

use std::ffi::CString;
use std::path::{Path, PathBuf};

use clap::ValueEnum;

use crate::imaging::{
    AlphaChannelOption_SetAlphaChannel, ChannelType_AlphaChannel, ColorspaceType_GRAYColorspace,
    ColorspaceType_sRGBColorspace, CompositeOperator_OverCompositeOp, DestroyMagickWand,
    ImagingError, MagickBooleanType_MagickFalse, MagickBooleanType_MagickTrue,
    MagickCompositeImage, MagickEvaluateImage, MagickEvaluateOperator_MultiplyEvaluateOperator,
    MagickGetImageColorspace, MagickGetImageHeight, MagickGetImageWidth, MagickReadImage,
    MagickSetImageAlphaChannel, MagickSetImageChannelMask, MagickTransformImageColorspace,
    MagickWand, NewMagickWand, path_to_cstring, take_exception,
};

/// Where a single watermark is anchored inside the output image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Position {
    NorthWest,
    North,
    NorthEast,
    West,
    Center,
    East,
    SouthWest,
    South,
    SouthEast,
}

impl Position {
    /// Top-left offset of the overlay inside a base image of `base_w` x
    /// `base_h`, for an overlay of `overlay_w` x `overlay_h`. Offsets are
    /// clamped to the base so an oversized watermark still covers from an
    /// edge instead of overflowing into negative coordinates.
    fn offset(
        self,
        base_w: usize,
        base_h: usize,
        overlay_w: usize,
        overlay_h: usize,
    ) -> (isize, isize) {
        let x = match self {
            Position::NorthWest | Position::West | Position::SouthWest => 0,
            Position::North | Position::Center | Position::South => {
                base_w.saturating_sub(overlay_w) / 2
            }
            Position::NorthEast | Position::East | Position::SouthEast => {
                base_w.saturating_sub(overlay_w)
            }
        };
        let y = match self {
            Position::NorthWest | Position::North | Position::NorthEast => 0,
            Position::West | Position::Center | Position::East => {
                base_h.saturating_sub(overlay_h) / 2
            }
            Position::SouthWest | Position::South | Position::SouthEast => {
                base_h.saturating_sub(overlay_h)
            }
        };
        (x as isize, y as isize)
    }
}

/// How to stamp the watermark image onto every output.
#[derive(Debug, Clone)]
pub struct WatermarkOptions {
    /// Path of the overlay image (PNG with transparency works best).
    pub path: PathBuf,
    /// Anchor for single placement; ignored in repeat mode.
    pub position: Position,
    /// Tile the watermark over the whole image instead of placing it once.
    pub repeat: bool,
    /// Gap between tiled watermarks as a percentage of the watermark width,
    /// applied to both axes (vertically it is scaled by the height). Only
    /// used in repeat mode.
    pub gap: u32,
    /// Opacity of the overlay (0-100, 100 keeps the overlay unchanged).
    pub opacity: u8,
    /// Rotation of the overlay in degrees, counter-clockwise for positive
    /// values. The rotated corners become transparent. 0 keeps the overlay
    /// untouched.
    pub angle: f64,
}

/// Composite the watermark onto the given base wand in place.
///
/// # Safety
/// `wand` must be a valid `MagickWand` holding at least one image.
pub unsafe fn apply(wand: *mut MagickWand, options: &WatermarkOptions) -> Result<(), ImagingError> {
    let overlay_c = path_to_cstring(&options.path, |e| ImagingError::Watermark {
        path: options.path.clone(),
        message: e,
    })?;

    unsafe {
        let overlay = NewMagickWand();
        if overlay.is_null() {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: "failed to allocate MagickWand".into(),
            });
        }
        let result = run(wand, overlay, &overlay_c, options);
        DestroyMagickWand(overlay);
        result
    }
}

unsafe fn run(
    wand: *mut MagickWand,
    overlay: *mut MagickWand,
    overlay_c: &CString,
    options: &WatermarkOptions,
) -> Result<(), ImagingError> {
    unsafe {
        if MagickReadImage(overlay, overlay_c.as_ptr()) == MagickBooleanType_MagickFalse {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: take_exception(overlay),
            });
        }

        let overlay_w = MagickGetImageWidth(overlay);
        let overlay_h = MagickGetImageHeight(overlay);
        if overlay_w == 0 || overlay_h == 0 {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: "watermark image has zero dimensions".into(),
            });
        }

        let base_w = MagickGetImageWidth(wand);
        let base_h = MagickGetImageHeight(wand);

        // ImageMagick 7.1.2 silently no-ops a color composite onto a
        // grayscale image, so convert a grayscale base to sRGB first. The
        // change is visually neutral: gray values become equal RGB values.
        if MagickGetImageColorspace(wand) == ColorspaceType_GRAYColorspace
            && MagickTransformImageColorspace(wand, ColorspaceType_sRGBColorspace)
                == MagickBooleanType_MagickFalse
        {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: take_exception(wand),
            });
        }

        apply_opacity(overlay, options)?;
        rotate(overlay, options)?;

        let overlay_w = MagickGetImageWidth(overlay);
        let overlay_h = MagickGetImageHeight(overlay);

        if options.repeat {
            tile(wand, overlay, options, base_w, base_h, overlay_w, overlay_h)
        } else {
            let (x, y) = options
                .position
                .offset(base_w, base_h, overlay_w, overlay_h);
            composite(wand, overlay, x, y, options)
        }
    }
}

/// Ensure the overlay has an alpha channel, then multiply that channel by
/// the requested opacity. Channel state is restored afterwards so later
/// operations on the overlay see all channels.
unsafe fn apply_opacity(
    overlay: *mut MagickWand,
    options: &WatermarkOptions,
) -> Result<(), ImagingError> {
    unsafe {
        if options.opacity >= 100 {
            // Fully opaque: keep the overlay exactly as read.
            return Ok(());
        }

        if MagickSetImageAlphaChannel(overlay, AlphaChannelOption_SetAlphaChannel)
            == MagickBooleanType_MagickFalse
        {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: take_exception(overlay),
            });
        }

        let previous = MagickSetImageChannelMask(overlay, ChannelType_AlphaChannel);
        let failure = MagickEvaluateImage(
            overlay,
            MagickEvaluateOperator_MultiplyEvaluateOperator,
            options.opacity as f64 / 100.0,
        ) == MagickBooleanType_MagickFalse;
        MagickSetImageChannelMask(overlay, previous);
        if failure {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: take_exception(overlay),
            });
        }
        Ok(())
    }
}

/// Rotate the overlay counter-clockwise by `options.angle` degrees, filling
/// the enlarged canvas with transparency so the revealed corners keep the
/// base image visible. Rotation is skipped for 0 degrees to avoid needless
/// resampling.
unsafe fn rotate(
    overlay: *mut MagickWand,
    options: &WatermarkOptions,
) -> Result<(), ImagingError> {
    unsafe {
        if options.angle == 0.0 {
            return Ok(());
        }

        use crate::imaging::{
            AlphaChannelOption_SetAlphaChannel, DestroyPixelWand, MagickRotateImage,
            MagickSetImageAlphaChannel, NewPixelWand, PixelSetColor,
        };

        // The overlay may lack an alpha channel entirely; without one the
        // rotation fills the enlarged canvas with an opaque background.
        if MagickSetImageAlphaChannel(overlay, AlphaChannelOption_SetAlphaChannel)
            == MagickBooleanType_MagickFalse
        {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: take_exception(overlay),
            });
        }

        let background = NewPixelWand();
        if background.is_null() {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: "failed to allocate PixelWand".into(),
            });
        }
        PixelSetColor(background, c"none".as_ptr());
        let ok = MagickRotateImage(overlay, background, options.angle)
            != MagickBooleanType_MagickFalse;
        DestroyPixelWand(background);
        if !ok {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: take_exception(overlay),
            });
        }
        Ok(())
    }
}

/// Stamp the overlay over the whole image on a grid. The gap is a
/// percentage of the overlay width horizontally and height vertically, so
/// tiles stay evenly spaced regardless of the logo's aspect ratio.
/// `MagickCompositeImage` clips tiles that overhang the edges.
unsafe fn tile(
    wand: *mut MagickWand,
    overlay: *mut MagickWand,
    options: &WatermarkOptions,
    base_w: usize,
    base_h: usize,
    overlay_w: usize,
    overlay_h: usize,
) -> Result<(), ImagingError> {
    unsafe {
        let gap_x = (overlay_w as f64 * options.gap as f64 / 100.0).round() as isize;
        let gap_y = (overlay_h as f64 * options.gap as f64 / 100.0).round() as isize;
        let step_x = overlay_w as isize + gap_x;
        let step_y = overlay_h as isize + gap_y;

        let mut y = 0isize;
        while y < base_h as isize {
            let mut x = 0isize;
            while x < base_w as isize {
                composite(wand, overlay, x, y, options)?;
                x += step_x;
            }
            y += step_y;
        }
        Ok(())
    }
}

unsafe fn composite(
    wand: *mut MagickWand,
    overlay: *mut MagickWand,
    x: isize,
    y: isize,
    options: &WatermarkOptions,
) -> Result<(), ImagingError> {
    unsafe {
        let op = CompositeOperator_OverCompositeOp;
        let ok = MagickCompositeImage(wand, overlay, op, MagickBooleanType_MagickTrue, x, y)
            != MagickBooleanType_MagickFalse;
        if !ok {
            return Err(ImagingError::Watermark {
                path: options.path.clone(),
                message: take_exception(wand),
            });
        }
        Ok(())
    }
}

/// Read the RGBA color (0.0-1.0 components) of one pixel of `path`.
/// Used by tests to assert on composited output.
pub fn pixel_color(path: &Path, x: isize, y: isize) -> Result<[f64; 4], ImagingError> {
    crate::imaging::ensure_genesis();
    let input_c = path_to_cstring(path, |e| ImagingError::Read {
        path: path.to_path_buf(),
        message: e,
    })?;

    unsafe {
        use crate::imaging::{
            DestroyPixelWand, MagickBooleanType_MagickFalse as MagickFalse,
            MagickGetImagePixelColor, NewMagickWand, NewPixelWand, PixelGetAlpha, PixelGetBlue,
            PixelGetGreen, PixelGetRed,
        };
        let wand = NewMagickWand();
        if wand.is_null() {
            return Err(ImagingError::Read {
                path: path.to_path_buf(),
                message: "failed to allocate MagickWand".into(),
            });
        }
        if MagickReadImage(wand, input_c.as_ptr()) == MagickFalse {
            let message = take_exception(wand);
            DestroyMagickWand(wand);
            return Err(ImagingError::Read {
                path: path.to_path_buf(),
                message,
            });
        }
        let pixel = NewPixelWand();
        let ok = MagickGetImagePixelColor(wand, x, y, pixel) != MagickFalse;
        if !ok {
            DestroyPixelWand(pixel);
            DestroyMagickWand(wand);
            return Err(ImagingError::Read {
                path: path.to_path_buf(),
                message: take_exception(wand),
            });
        }
        let color = [
            PixelGetRed(pixel),
            PixelGetGreen(pixel),
            PixelGetBlue(pixel),
            PixelGetAlpha(pixel),
        ];
        DestroyPixelWand(pixel);
        DestroyMagickWand(wand);
        Ok(color)
    }
}
