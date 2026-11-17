# bir

**Bir: Bulk Image Resizer. Simple, fast, native.**

[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey)](#building)

`bir` recursively finds images in a directory and creates resized copies
scaled by their longest side. It is a single self-contained native binary:
ImageMagick is compiled into it, so there is nothing to install and no
external `magick` program is needed at build or runtime.

## ✨ Features

|                          |                                                                                                  |
| ------------------------ | ------------------------------------------------------------------------------------------------ |
| ⚡ **Fast & parallel**   | Uses all CPU cores by default (`-j` to tune), ~11 files/s on an M1 ([benchmark](#-performance)) |
| 📁 **Recursive**         | Walks entire directory trees; mirrors subfolder structure in the output                          |
| 📊 **Progress bar**      | Live progress indicator during long batches                                                      |
| 🖼️ **Smart resizing**    | Longest-side scaling, aspect ratio preserved, never upscales                                     |
| 🔄 **Format conversion** | PNG/JPEG/WebP/TIFF/BMP in → PNG/JPEG/WebP/TIFF/AVIF out                                |
| 🎚️ **Quality control**   | Configurable quality for lossy formats                                                           |
| 🔁 **Idempotent**        | Skips outputs of previous runs; re-running is always safe (`--force` to overwrite)               |
| 💧 **Watermarking**      | Logo overlay with position, tiling, opacity and tilt                                             |
| 📦 **Self-contained**    | Single ~5 MB binary with ImageMagick baked in, zero runtime dependencies                         |

## 🚀 Install

```sh
cargo install --git https://github.com/egel/bir   # straight from the repository
```

Or from a local checkout:

```sh
cargo install --path .
```

The first install builds the vendored C libraries and takes a few minutes;
later reinstalls are fast. The installed copy is independent of `target/`,
so `cargo clean` afterwards is safe.

## 💡 Usage

Resize every image under `./photos` so the longest side is 200/400/1000 px:

```sh
bir ./photos
```

That's the whole idea. More realistic examples:

```sh
# Convert everything to WebP, longest side 1200 px, into ./out
bir ./photos -r 1200 -f webp -o ./out -q 80

# Convert everything to AVIF instead (encoded in pure Rust, no extra tools)
bir ./photos -r 1200 -f avif -o ./out -q 80

# Only the top directory, overwrite existing outputs
bir ./photos --no-recursive --force

# Stamp a logo watermark in the bottom-right corner of every output
bir ./photos --watermark logo.png

# Tile the watermark over the whole image, 25% gap, 80% opacity
bir ./photos --watermark logo.png --watermark-repeat --watermark-gap 25 --watermark-opacity 80

# Tilt the watermark by 12 degrees (counter-clockwise)
bir ./photos --watermark logo.png --watermark-angle 12

# A single image file also works: outputs land next to it (or in ./out with -o)
bir photo.jpg -r 1200
bir photo.jpg -r 1200 -f webp -o ./out
```

```
bir [OPTIONS] <INPUT_DIR|FILE>

Options:
  -r, --resolutions <RESOLUTIONS>  Comma-separated target sizes for the longest side [default: 200,400,800]
  -f, --format <FORMAT>            Convert images to this format (default: keep original)
  -o, --output-dir <OUTPUT_DIR>    Output directory (default: alongside originals); source subfolders are mirrored inside it
  -q, --quality <QUALITY>          Quality for lossy formats (0-100)
      --watermark <WATERMARK>      Overlay this image as a watermark on every output
      --watermark-position <POSITION>  Anchor for the watermark (northwest...southeast, default: southeast)
      --watermark-repeat           Tile the watermark over the whole image
      --watermark-gap <GAP>        Gap between tiled watermarks as a percentage of the watermark width [default: 50]
      --watermark-opacity <OPACITY>  Watermark opacity (0-100) [default: 100]
      --watermark-angle <ANGLE>    Tilt the watermark in degrees, -90 to 90, positive is counter-clockwise [default: 0]
      --force                      Overwrite existing outputs
  -j, --jobs <JOBS>                Number of parallel workers (default: all CPU cores)
      --no-recursive               Process only the top directory
  -h, --help                       Print help
  -V, --version                    Print version
```

### Output naming and layout

Outputs are named `<stem>_x<size>.<ext>`, e.g. `photo_x400.webp`. The size
always refers to the longest side, whether the image is landscape or portrait.
With `--output-dir`, the subfolder layout under the input directory is
recreated inside the output directory:

```sh
bir ./photos -r 400 -o ./out
# photos/cats/mia.png  -> out/cats/mia_x400.png
# photos/dog.jpg       -> out/dog_x400.jpg
```

Files whose name already ends in `_x<N>` (or the legacy width-based `_w<N>`)
for one of the configured sizes are treated as previous outputs and skipped
during discovery, so re-running the command is always safe.

## 📖 Recipes

Common setups, from quick previews to production-ready web assets.

### Web gallery with previews and full-size versions

```sh
bir ./photos -r 400,1200 -f webp -o ./out -q 80
# out/photo_x400.webp   thumbnail / card image
# out/photo_x1200.webp  full-size view
```

WebP with quality 80 is a good default for the web: much smaller than JPEG
at visually the same quality, supported by every modern browser.

### Fastest load times: AVIF

```sh
bir ./photos -r 400,1200 -f avif -o ./out -q 60
```

AVIF compresses better than WebP but encodes slower (each file is re-encoded
in pure Rust, no extra tools needed). Use it when load speed matters more
than processing time, and lower the quality a bit, AVIF tolerates it well.

### Keep originals safe, convert to JPEG

```sh
bir ./raw-scan -r 2000 -f jpg -q 85 -o ./share
```

Useful for scans or camera dumps: one large JPEG per source, written to a
separate folder, originals untouched. Start at quality 85; go up to 90-95
for prints, down to 70-75 for small previews.

### Preview strips from a mixed folder

```sh
bir ./screenshots -r 800 -f png -o ./previews
```

PNG keeps sharp edges and text crisp, so it is the right choice for
screenshots, diagrams and logos. For photographs, prefer WebP or JPEG.

### Watermarked social media copies

```sh
bir ./shots -r 1080 -f jpg -q 90 \
    --watermark logo.png --watermark-position southeast \
    --watermark-opacity 80 -o ./ready
```

See the watermarking guide below for how to prepare the logo.

### Tips

- Run the command once on a small folder or a single file first
  (`bir photo.jpg -r 400 -f webp`) and check the output before
  processing the whole library.
- Re-running is safe: outputs from previous runs are detected and skipped.
  Use `--force` only when you changed settings and want fresh files.
- Keep sources and outputs apart with `-o` when you convert formats,
  otherwise every new output sits next to its source.
- `-j 4` limits workers on shared machines; the default uses all cores.
- `--no-recursive` speeds up a flat folder that you know has no
  subfolders.

## 💧 Watermarking guide

The watermark is your logo image, stamped onto every output. It is applied
after resizing, so it stays proportionate at every generated size.

### Prepare a good watermark file

- **Use PNG with a transparent background.** The logo's own alpha channel is
  respected, so transparency in the file means transparency on the photo.
  A JPEG logo will show as an opaque rectangle.
- **Keep it modest in size.** The overlay is stamped at its natural size
  regardless of the output resolution, so a single placement logo of roughly
  100-300 px wide works well across 400-1200 px outputs.
- **Design for legibility at small sizes.** A simple mark with strong
  contrast survives the smallest outputs; thin strokes and fine text do not.
- **Bake translucency into the file if you like, or use
  `--watermark-opacity`.** A solid logo plus `--watermark-opacity 70` is
  usually the easiest route to a subtle, unobtrusive mark.
- **White logos need a dark backdrop to judge.** Test on a photo with both
  bright and dark regions before batch processing.

If you do not have a transparent PNG yet, ImageMagick can make one from a
plain logo:

```sh
# Make white pixels transparent (works for simple logos on white)
magick logo.jpg -fuzz 15% -transparent white logo.png
```

### Placement modes

Single placement (default, anchored to an edge):

```sh
bir ./photos --watermark logo.png --watermark-position southeast
```

Positions: `northwest`, `north`, `northeast`, `west`, `center`, `east`,
`southwest`, `south`, `southeast`. The default is `southeast`.

Tiled over the whole image (stronger protection, no corner to crop away):

```sh
bir ./photos --watermark logo.png --watermark-repeat --watermark-gap 50 --watermark-opacity 40
```

The gap is a percentage of the logo width (scaled by its height vertically),
so the rhythm stays even at every output size. At low opacity, tiling reads
as a texture rather than a stamp.

### Tilt

```sh
bir ./photos --watermark logo.png --watermark-angle -15
```

Angles range from -90 to 90 degrees, positive tilts counter-clockwise.
Small tilts between 5 and 20 degrees look intentional; anything steeper
rarely does. Tilt combines with both placement modes, and with opacity:

```sh
bir ./photos --watermark logo.png --watermark-repeat --watermark-angle 12 --watermark-opacity 30
```

### Suggested starting points

| Goal                        | Flags                                                                              |
| --------------------------- | ---------------------------------------------------------------------------------- |
| Subtle credit in the corner | `--watermark-position southeast --watermark-opacity 70`                             |
| Strong anti-crop protection | `--watermark-repeat --watermark-gap 60 --watermark-opacity 30`                      |
| Diagonal preview stamp      | `--watermark-repeat --watermark-angle -12 --watermark-opacity 40 --watermark-gap 80` |


## 📊 Performance

Real-world run on an Apple M1 (32 GB), resizing 249 source images to three
sizes (200/400/1000 px, JPEG output) with all cores in parallel:

```text
$ bir on-website-images -o on-website-images-fixed -r 200,400,1000 -f jpg
Found 249 source image(s) in on-website-images
  [00:01:05] [##############################] 747/747 files (0s)
Done: 747 created, 0 skipped, 0 failed.
```

747 output files in ~65 seconds (~11 files/s), with the progress bar shown live
during the run.

## 🔧 Building

Prerequisites: a Rust toolchain (edition 2024) and CMake with a C compiler.
The vendored C libraries (see `VENDORED.md`) are built automatically during
`cargo build`; the first build takes a few minutes, subsequent builds are fast.

```sh
cargo build --release
```

To build against a system-installed ImageMagick instead (faster iteration,
requires ImageMagick dev headers and `pkg-config`):

```sh
cargo build --no-default-features
```

## 🛠️ Development

```sh
cargo test          # unit, engine and end-to-end CLI tests
cargo clippy        # lints
cargo fmt           # formatting
```

## 🗑️ Uninstalling

```sh
cargo uninstall bir
# and, if you placed a system-wide copy:
sudo rm /usr/local/bin/bir
```

Nothing else is left behind: no config files, no dynamic libraries, no caches.

## License

Licensed under the Apache-2.0 license. Vendored third-party libraries keep
their own licenses, see `VENDORED.md` and each library directory.
