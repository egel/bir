# bir

Bir — Bulk Image Resizer. Simple, fast, native.

`bir` recursively finds images in a directory and creates resized copies
scaled by their longest side. It is a native Rust rewrite of the original
`bir.sh` script, with ImageMagick compiled into the binary — no external
`magick` program is needed at build or runtime.

## Features

- Longest-side resizing that preserves aspect ratio and never upscales
  (mirrors `magick -resize <N>x<N>>`): landscape images are scaled to width
  `<N>`, portrait images to height `<N>`. Images with both sides smaller
  than `<N>` are written unchanged.
- Recursive directory traversal (opt out with `--no-recursive`).
- Format conversion: PNG, JPEG, WebP, TIFF, BMP inputs; PNG/JPEG/WebP/TIFF
  outputs.
- Configurable output directory and quality for lossy formats.
- Skips outputs of previous runs (`<stem>_x<N>.<ext>`, plus the legacy
  width-based `<stem>_w<N>.<ext>`) so the command is idempotent; `--force`
  overwrites.

## Usage

```
bir [OPTIONS] <INPUT_DIR>

Arguments:
  <INPUT_DIR>  Directory containing the images to process

Options:
  -r, --resolutions <RESOLUTIONS>  Comma-separated target sizes for the longest side [default: 200,400,800]
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
# Resize every image under ./photos so the longest side is 200/400/800 px (in place)
bir ./photos

# Convert everything to WebP with the longest side at 1200 px into ./out
bir ./photos -r 1200 -f webp -o ./out -q 80

# Only the top directory, overwrite existing outputs
bir ./photos --no-recursive --force
```

Outputs are named `<stem>_x<size>.<ext>`, e.g. `photo_x400.webp` — the size
always refers to the longest side, whether the image is landscape or portrait.
Files whose name already ends in `_x<N>` (or the legacy width-based `_w<N>`)
for one of the configured sizes are treated as previous outputs and skipped
during discovery.

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

## Installing and uninstalling

Install the binary to `~/.cargo/bin` (already on `PATH` if you use Rust):

```sh
cargo install --path .            # from a local checkout
cargo install --git https://github.com/egel/bir   # straight from the repository
```

The first install builds the vendored C libraries and takes a few minutes;
later reinstalls are fast (use `cargo install --path . --force` to upgrade
after code changes). The installed copy is independent of `target/`, so
`cargo clean` afterwards is safe. It is a single self-contained binary (~5 MB)
with no runtime dependencies beyond macOS `libSystem` / Linux libc — to make
it system-wide instead, move it: `sudo cp ~/.cargo/bin/bir /usr/local/bin/`.

Uninstall:

```sh
cargo uninstall bir
# and, if you placed a system-wide copy:
sudo rm /usr/local/bin/bir
```

Nothing else is left behind — no config files, no dynamic libraries, no caches.

## License

Licensed under the Apache-2.0 license. Vendored third-party libraries keep
their own licenses — see `VENDORED.md` and each library directory.
