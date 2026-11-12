# Vendored C libraries

Source copies committed directly to this repository (no git submodules).
Each directory is a trimmed upstream release: `.git`, `.github`, docs, tests
and examples removed. Build scripts and CMake files are kept intact.

| Directory | Project | Version | Upstream |
| --- | --- | --- | --- |
| `ImageMagick/` | ImageMagick 7 | 7.1.2-35 | https://github.com/ImageMagick/ImageMagick/archive/refs/tags/7.1.2-35.tar.gz |
| `delegates/zlib/` | zlib | 1.4 | https://github.com/madler/zlib/archive/refs/tags/v1.4.tar.gz |
| `delegates/libpng/` | libpng | 1.6.59 | https://github.com/pnggroup/libpng/archive/refs/tags/v1.6.59.tar.gz |
| `delegates/libjpeg-turbo/` | libjpeg-turbo | 3.2.1 | https://github.com/libjpeg-turbo/libjpeg-turbo/archive/refs/tags/3.2.1.tar.gz |
| `delegates/libwebp/` | libwebp | 1.6.0 | https://github.com/webmproject/libwebp/archive/refs/tags/v1.6.0.tar.gz |

ImageMagick needs its delegate libraries at build time; they are compiled
first (zlib → libpng / libjpeg-turbo / libwebp) by `build.rs`, then ImageMagick
is configured against the freshly built static delegates.

## Refresh procedure

1. Download the new release tarball from the upstream URL above.
2. Extract it and trim the same directories (`.git`, `.github`, `doc(s)`,
   `www`, `tests`, `test`, `examples`, `contrib`, `demos`, `PerlMagick`).
3. Replace the vendored directory, update the version in this table, commit.
