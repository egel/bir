# bir

Bir — Bulk Image Resizer. Simple, fast, native.

`bir` recursively finds images in a directory and creates width-based resized
copies. It is a native Rust rewrite of the original `bir.sh` script, with
ImageMagick compiled into the binary — no external `magick` program is needed
at build or runtime.

## Features

- Width-based resizing that preserves aspect ratio and never upscales
  (mirrors `magick -resize <N>x`).
- Recursive directory traversal (opt out with `--no-recursive`).
- Format conversion: PNG, JPEG, WebP, TIFF, BMP inputs; PNG/JPEG/WebP/TIFF
  outputs.
- Configurable output directory and quality for lossy formats.
- Skips outputs of previous runs (`<stem>_w<N>.<ext>`) so the command is
  idempotent; `--force` overwrites.

## Usage

```
bir [OPTIONS] <INPUT_DIR>

Arguments:
  <INPUT_DIR>  Directory containing the images to process

Options:
  -r, --resolutions <RESOLUTIONS>  Comma-separated target widths [default: 200,400,800]
  -f, --format <FORMAT>            Convert images to this format (default: keep original)
  -o, --output-dir <OUTPUT_DIR>    Output directory (default: alongside originals)
  -q, --quality <QUALITY>          Quality for lossy formats (0-100)
      --force                      Overwrite existing outputs
      --no-recursive               Process only the top directory
  -h, --help                       Print help
  -V, --version                    Print version
```

Examples:

```sh
# Resize every image under ./photos to 200/400/800 px wide (in place)
bir ./photos

# Convert everything to WebP at 1200 px into ./out
bir ./photos -r 1200 -f webp -o ./out -q 80

# Only the top directory, overwrite existing outputs
bir ./photos --no-recursive --force
```

Outputs are named `<stem>_w<width>.<ext>`, e.g. `photo_w400.webp`. Files whose
name already ends in `_w<N>` for one of the configured widths are treated as
previous outputs and skipped during discovery.

## Building

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

## Development

```sh
cargo test          # unit, engine and end-to-end CLI tests
cargo clippy        # lints
cargo fmt           # formatting
```

## License

Licensed under the Apache-2.0 license. Vendored third-party libraries keep
their own licenses — see `VENDORED.md` and each library directory.
