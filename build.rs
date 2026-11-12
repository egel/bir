use std::path::PathBuf;

fn main() {
    // Re-run when vendored sources change.
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=build.rs");

    if cfg!(feature = "vendored") {
        build_vendored();
    } else {
        build_system();
    }
}

const PREFIX_DIR: &str = "vendored";

fn prefix(out_dir: &str) -> PathBuf {
    PathBuf::from(out_dir).join(PREFIX_DIR)
}

/// Configure + build a vendored CMake project and return its install prefix.
fn cmake_project(
    name: &str,
    src: &str,
    out_dir: &str,
    defines: &[(&str, &str)],
) -> PathBuf {
    let prefix = prefix(out_dir).join(name);
    let mut cfg = cmake::Config::new(src);
    cfg.out_dir(PathBuf::from(out_dir).join("cmake-build").join(name))
        .define("CMAKE_INSTALL_PREFIX", &prefix)
        .define("BUILD_SHARED_LIBS", "OFF")
        .always_configure(true);

    for (k, v) in defines {
        cfg.define(k, v);
    }

    let install = cfg.build();
    println!("cargo:rustc-link-search=native={}", prefix.join("lib").display());
    // ld64 honors LC_LINKER_OPTION autolink hints embedded in the ImageMagick
    // objects and prefers .dylib over .a — make sure only static archives
    // remain so autolinks resolve to them.
    for entry in std::fs::read_dir(prefix.join("lib")).expect("lib dir missing") {
        let p = entry.expect("lib entry").path();
        if p.extension().and_then(|e| e.to_str()) == Some("dylib") {
            std::fs::remove_file(&p).expect("cannot remove dylib");
        }
    }
    prefix
}

fn build_vendored() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let top = prefix(&out_dir);
    let zlib = cmake_project(
        "zlib",
        "vendor/delegates/zlib",
        &out_dir,
        &[("CMAKE_BUILD_TYPE", "Release"), ("CMAKE_POSITION_INDEPENDENT_CODE", "ON"), ("ZLIB_BUILD_TESTING", "OFF"), ("ZLIB_BUILD_EXAMPLES", "OFF"), ("ZLIB_BUILD_SHARED", "OFF")],
    );

    let png = cmake_project(
        "libpng",
        "vendor/delegates/libpng",
        &out_dir,
        &[
            ("CMAKE_BUILD_TYPE", "Release"),
            ("CMAKE_POSITION_INDEPENDENT_CODE", "ON"),
            ("ZLIB_LIBRARY", zlib.join("lib").join("libz.a").to_str().unwrap()),
            ("ZLIB_INCLUDE_DIR", zlib.join("include").to_str().unwrap()),
            ("PNG_SHARED", "OFF"),
            ("PNG_TESTS", "OFF"),
            ("PNG_TOOLS", "OFF"),
            ("PNG_EXECUTABLES", "OFF"),
            ("PNG_FRAMEWORK", "OFF"),
        ],
    );

    let jpeg = cmake_project(
        "libjpeg-turbo",
        "vendor/delegates/libjpeg-turbo",
        &out_dir,
        &[
            ("CMAKE_BUILD_TYPE", "Release"),
            ("CMAKE_POSITION_INDEPENDENT_CODE", "ON"),
            ("ENABLE_SHARED", "OFF"),
            ("ENABLE_STATIC", "ON"),
            ("WITH_SIMD", "OFF"), // avoids a NASM dependency
            ("WITH_TURBOJPEG", "OFF"),
        ],
    );

    let webp = cmake_project(
        "libwebp",
        "vendor/delegates/libwebp",
        &out_dir,
        &[
            ("CMAKE_BUILD_TYPE", "Release"),
            ("CMAKE_POSITION_INDEPENDENT_CODE", "ON"),
            ("WEBP_BUILD_ANIM_UTILS", "OFF"),
            ("WEBP_BUILD_C_WEBP", "OFF"),
            ("WEBP_BUILD_DWEBP", "OFF"),
            ("WEBP_BUILD_EXTRAS", "OFF"),
            ("WEBP_BUILD_GIF2WEBP", "OFF"),
            ("WEBP_BUILD_IMG2WEBP", "OFF"),
            ("WEBP_BUILD_VWEBP", "OFF"),
            ("WEBP_BUILD_WEBPINFO", "OFF"),
            ("WEBP_BUILD_WEBPQAQ", "OFF"),
            ("WEBP_BUILD_WEBPMUX", "OFF"),
            ("WEBP_BUILD_LIBWEBPMUX", "OFF"),
        ],
    );

    let tiff = cmake_project(
        "libtiff",
        "vendor/delegates/libtiff",
        &out_dir,
        &[
            ("CMAKE_BUILD_TYPE", "Release"),
            ("CMAKE_POSITION_INDEPENDENT_CODE", "ON"),
            ("tiff-tools", "OFF"),
            ("tiff-tests", "OFF"),
            ("tiff-docs", "OFF"),
            ("tiff-doc-pdf", "OFF"),
            ("tiff-contrib", "OFF"),
            ("jpeg", "ON"),
            ("jpeg-includes", jpeg.join("include").to_str().unwrap()),
            ("zlib", "ON"),
            ("zlib-includes", zlib.join("include").to_str().unwrap()),
            ("zlib-library", zlib.join("lib").join("libz.a").to_str().unwrap()),
            ("lzma", "OFF"),
            ("webp", "OFF"),
            ("zstd", "OFF"),
            ("jbig", "OFF"),
            ("libdeflate", "OFF"),
            ("cxx", "OFF"),
        ],
    );

    // ImageMagick 7.x has no CMake support — build via autotools against the
    // freshly compiled delegate libraries.
    let im_prefix = prefix(&out_dir).join("imagemagick");
    build_imagemagick_autotools(&im_prefix, &out_dir, &[(&zlib, "libz.a"), (&png, "libpng16.a"), (&jpeg, "libjpeg.a"), (&webp, "libwebp.a"), (&tiff, "libtiff.a")]);

    println!("cargo:rustc-link-search=native={}", im_prefix.join("lib").display());

    // Link every static archive the ImageMagick build produced (naming
    // includes the version/quantum depth, e.g. libMagickCore-7.Q16HDRI.a).
    let im_install = im_prefix.clone();
    link_vendored_magick(&im_prefix);
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=png16");
    println!("cargo:rustc-link-lib=static=jpeg");
    println!("cargo:rustc-link-lib=static=webp");
    println!("cargo:rustc-link-lib=static=webpdemux");
    println!("cargo:rustc-link-lib=static=tiff");

    if let Some(inc) = find_im_include(&im_install) {
        generate_bindings(&inc, &[&inc, &top.join("libjpeg-turbo").join("include").display().to_string()]);
    } else {
        panic!("ImageMagick install dir has no include/ directory");
    }
}

/// ImageMagick installs its headers under include/ImageMagick-7/ — locate it.
fn find_im_include(im_install: &std::path::Path) -> Option<String> {
    let inc_root = im_install.join("include");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&inc_root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs.into_iter()
        .find(|p| p.join("MagickWand").join("MagickWand.h").exists())
        .map(|p| p.display().to_string())
}

/// Build ImageMagick with its autotools build system, statically linking the
/// delegate libraries built by CMake in the previous steps.
fn build_imagemagick_autotools(
    im_prefix: &std::path::Path,
    out_dir: &str,
    delegates: &[(&PathBuf, &str)],
) {
    use std::process::Command;

    let src = std::fs::canonicalize("vendor/ImageMagick").expect("vendor/ImageMagick missing");
    let build_dir = PathBuf::from(out_dir).join("autotools-build").join("imagemagick");
    std::fs::create_dir_all(&build_dir).expect("cannot create autotools build dir");

    let mut include_dirs = Vec::new();
    let mut lib_dirs = Vec::new();
    for (prefix, _) in delegates {
        include_dirs.push(format!("-I{}", prefix.join("include").display()));
        lib_dirs.push(format!("-L{}", prefix.join("lib").display()));
    }

    let cflags = "-O2 -fPIC";
    let cppflags = include_dirs.join(" ");
    let ldflags = lib_dirs.join(" ");
    // libwebp pulls in sharpyuv symbols; adding it to LIBS keeps the
    // configure-time link checks happy.
    let libs = "-lsharpyuv";

    if !build_dir.join("Makefile").exists() {
        let mut cmd = Command::new(src.join("configure"));
        cmd.current_dir(&build_dir)
            .arg(format!("--prefix={}", im_prefix.display()))
            .arg("--disable-shared")
            .arg("--enable-static")
            .arg("--disable-dependency-tracking")
            .arg("--without-x")
            .arg("--without-magick-plus-plus")
            .arg("--without-perl")
            .arg("--without-modules")
            .arg("--disable-openmp")
            .arg("--without-freetype")
            .arg("--without-fontconfig")
            .arg("--without-xml")
            .arg("--without-lcms")
            .arg("--without-lzma")
            .arg("--without-bzlib")
            .arg("--without-zstd")
            .arg("--without-jbig")
            .arg("--without-openjp2")
            .arg("--without-heic")
            .arg("--without-raw")
            .arg("--without-djvu")
            .arg("--without-fftw")
            .arg("--without-pango")
            .arg("--without-raqm")
            .arg("--without-flif")
            .arg("--without-openexr")
            .arg("--disable-docs")
            .env("CC", std::env::var("CC").unwrap_or_else(|_| "cc".into()))
            .env("CFLAGS", cflags)
            .env("CPPFLAGS", cppflags)
            .env("LDFLAGS", ldflags)
            .env("LIBS", libs);
        run(cmd, "configure");
    }

    let jobs = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    run(
        {
            let mut c = Command::new("make");
            c.current_dir(&build_dir).arg(format!("-j{jobs}"));
            c
        },
        "make",
    );
    run(
        {
            let mut c = Command::new("make");
            c.current_dir(&build_dir).arg("install");
            c
        },
        "make install",
    );
}

fn run(mut cmd: std::process::Command, what: &str) {
    let status = cmd.status().unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    if !status.success() {
        panic!("{what} failed with status {status}");
    }
}

fn link_vendored_magick(im_prefix: &std::path::Path) {
    let lib_dir = im_prefix.join("lib");
    let mut archives: Vec<PathBuf> = std::fs::read_dir(&lib_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", lib_dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .map(|f| f.starts_with("libMagick") && f.ends_with(".a"))
                .unwrap_or(false)
        })
        .collect();
    archives.sort();
    if archives.is_empty() {
        panic!("no libMagick*.a archives found in {}", lib_dir.display());
    }
    for a in &archives {
        let name = a
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap()
            .trim_start_matches("lib")
            .trim_end_matches(".a")
            .to_string();
        println!("cargo:rustc-link-lib=static={name}");
    }
}

fn generate_bindings(header_dir: &str, include_dirs: &[&str]) {
    let header = PathBuf::from(header_dir)
        .join("MagickWand")
        .join("MagickWand.h");
    let mut b = bindgen::Builder::default()
        .header(header.to_str().expect("non-utf8 header path"))
        // magick-config.h requires these to be set by the consumer; they must
        // match the configured build (Q16HDRI by default).
        .clang_arg("-DMAGICKCORE_HDRI_ENABLE=1")
        .clang_arg("-DMAGICKCORE_QUANTUM_DEPTH=16")
        .allowlist_function("Magick.*")
        .allowlist_type("Magick.*|.*Wand.*|DrawInfo|ImageInfo|ExceptionInfo")
        .allowlist_var("Magick.*")
        .derive_default(true);

    for dir in include_dirs {
        b = b.clang_arg(format!("-I{dir}"));
    }

    let bindings = b
        .generate()
        .expect("bindgen failed to generate MagickWand bindings");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("magickwand_bindings.rs");
    bindings.write_to_file(&out).expect("cannot write bindings");
    println!("cargo:rustc-cfg=have_bindings");
}

fn build_system() {
    // Fallback: link against a system ImageMagick via pkg-config.
    let pc = pkg_config::probe_library("MagickWand")
        .expect("system MagickWand not found via pkg-config (install ImageMagick dev headers)");
    let include = pc.include_paths.last().cloned().expect("no include path");
    let inc_str = include.to_str().expect("non-utf8 include path");
    let header = include.join("MagickWand").join("MagickWand.h");
    if !header.exists() {
        panic!("MagickWand.h not found under {}", inc_str);
    }
    generate_bindings(inc_str, &pc.include_paths.iter().map(|p| p.to_str().unwrap()).collect::<Vec<_>>());
}
