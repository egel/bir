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
| 🔄 **Format conversion** | PNG/JPEG/WebP/TIFF/BMP in → PNG/JPEG/WebP/TIFF out                                               |
| 🎚️ **Quality control**   | Configurable quality for lossy formats                                                           |
| 🔁 **Idempotent**        | Skips outputs of previous runs; re-running is always safe (`--force` to overwrite)               |
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

# Only the top directory, overwrite existing outputs
bir ./photos --no-recursive --force
```

```
bir [OPTIONS] <INPUT_DIR>

Options:
  -r, --resolutions <RESOLUTIONS>  Comma-separated target sizes for the longest side [default: 200,400,800]
  -f, --format <FORMAT>            Convert images to this format (default: keep original)
  -o, --output-dir <OUTPUT_DIR>    Output directory (default: alongside originals); source subfolders are mirrored inside it
  -q, --quality <QUALITY>          Quality for lossy formats (0-100)
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
