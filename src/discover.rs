//! Recursive discovery of source images inside a directory tree.
//!
//! Matches the reference `bir.sh` behavior: find JPG/JPEG/PNG/BMP/TIF/TIFF
//! files (case-insensitive), recursively, while skipping outputs produced by
//! previous runs (`<stem>_w<N>.<ext>` for every configured width).

use std::path::{Path, PathBuf};

use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

/// Extensions accepted as source images, lowercase without the dot.
pub const SOURCE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "tif", "tiff"];

#[derive(Debug, Error)]
pub enum DiscoverError {
    #[error("'{path}' is not a directory")]
    NotADirectory { path: PathBuf },
    #[error("cannot walk '{path}': {message}")]
    Walk { path: PathBuf, message: String },
    #[error("no source images found in '{path}'")]
    NoImages { path: PathBuf },
}

/// Find source images in `dir` that should be resized to the given widths.
///
/// A file is excluded when its stem ends in `_w<N>` for any of the
/// configured `widths` — those are outputs of previous runs, regardless of
/// their extension (so e.g. `photo_w200.webp` is skipped even when the source
/// extension filter is jpg/png only).
pub fn find_images(
    dir: &Path,
    widths: &[u32],
    recursive: bool,
) -> Result<Vec<PathBuf>, DiscoverError> {
    if !dir.is_dir() {
        return Err(DiscoverError::NotADirectory {
            path: dir.to_path_buf(),
        });
    }

    let mut walker = WalkDir::new(dir).follow_links(false);
    if !recursive {
        walker = walker.max_depth(1);
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for entry in walker {
        let entry = entry.map_err(|e| DiscoverError::Walk {
            path: dir.to_path_buf(),
            message: e.to_string(),
        })?;
        if is_source_image(&entry, widths) {
            files.push(entry.into_path());
        }
    }

    files.sort();
    if files.is_empty() {
        return Err(DiscoverError::NoImages {
            path: dir.to_path_buf(),
        });
    }
    Ok(files)
}

fn is_source_image(entry: &DirEntry, widths: &[u32]) -> bool {
    if !entry.file_type().is_file() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    let Some(dot) = name.rfind('.') else {
        return false;
    };
    let ext = name[dot + 1..].to_ascii_lowercase();
    if !SOURCE_EXTENSIONS.contains(&ext.as_str()) {
        return false;
    }
    let stem = &name[..dot];
    !widths.iter().any(|w| stem.ends_with(&format!("_w{w}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, b"x").unwrap();
        p
    }

    #[test]
    fn finds_and_sorts_images_case_insensitively() {
        let dir = std::env::temp_dir().join(format!("bir-disc-1-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        touch(&dir, "a.JPG");
        touch(&dir, "b.jpeg");
        touch(&dir, "c.PNG");
        touch(&dir, "d.bmp");
        touch(&dir, "e.tif");
        touch(&dir, "f.TIFF");
        touch(&dir, "notes.txt");
        touch(&dir, "noext");

        let found = find_images(&dir, &[200, 400, 800], true).unwrap();
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            vec!["a.JPG", "b.jpeg", "c.PNG", "d.bmp", "e.tif", "f.TIFF"]
        );
    }

    #[test]
    fn excludes_outputs_of_previous_runs() {
        let dir = std::env::temp_dir().join(format!("bir-disc-2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        touch(&dir, "photo.jpg");
        touch(&dir, "photo_w200.jpg");
        touch(&dir, "photo_w400.jpg");
        touch(&dir, "photo_w800.jpg");
        touch(&dir, "photo_w200.webp");
        touch(&dir, "photo_w999.jpg"); // not a configured width — kept

        let found = find_images(&dir, &[200, 400, 800], true).unwrap();
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["photo.jpg", "photo_w999.jpg"]);
    }

    #[test]
    fn respects_recursive_flag() {
        let dir = std::env::temp_dir().join(format!("bir-disc-3-{}", std::process::id()));
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        touch(&dir, "top.png");
        touch(&sub, "nested.png");

        assert_eq!(find_images(&dir, &[200], false).unwrap().len(), 1);
        assert_eq!(find_images(&dir, &[200], true).unwrap().len(), 2);
    }

    #[test]
    fn errors_on_missing_dir_and_empty_tree() {
        let err = find_images(Path::new("/definitely/not/here"), &[200], true).unwrap_err();
        assert!(matches!(err, DiscoverError::NotADirectory { .. }));

        let dir = std::env::temp_dir().join(format!("bir-disc-4-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        touch(&dir, "only.txt");
        let err = find_images(&dir, &[200], true).unwrap_err();
        assert!(matches!(err, DiscoverError::NoImages { .. }));
    }
}
