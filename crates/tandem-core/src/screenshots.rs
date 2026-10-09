//! In-game screenshots of an instance (`screenshots/`), with small JPEG thumbnails
//! cached in `cache/thumbnails/<instance>/<file>.jpg`.

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::error::{Error, Result};
use crate::paths::DataDir;

/// Width of the cached thumbnails: sharp in a grid cell on a 2× display.
pub const THUMBNAIL_WIDTH: u32 = 480;
const THUMBNAIL_QUALITY: u8 = 82;
const EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    /// File name in `screenshots/`, which identifies the screenshot.
    pub file_name: String,
    /// Absolute path, to reveal it in the file manager.
    pub path: PathBuf,
    /// Unix milliseconds (file modification time).
    pub taken_at: i64,
    pub size_bytes: u64,
}

fn screenshots(game_dir: &Path) -> PathBuf {
    game_dir.join("screenshots")
}

fn thumbnails_of(data: &DataDir, instance_id: &str) -> PathBuf {
    data.cache().join("thumbnails").join(instance_id)
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// A screenshot name is a single plain path component with an image extension.
fn check_name(file_name: &str) -> Result<()> {
    let mut components = Path::new(file_name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) if is_image(Path::new(file_name)) => Ok(()),
        _ => Err(Error::InvalidInput(format!(
            "invalid screenshot name: {file_name}"
        ))),
    }
}

/// Screenshots of an instance, newest first.
pub fn list(game_dir: &Path) -> Vec<Screenshot> {
    let Ok(entries) = std::fs::read_dir(screenshots(game_dir)) else {
        return Vec::new();
    };
    let mut shots: Vec<Screenshot> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()) && is_image(&e.path()))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            Some(Screenshot {
                file_name: e.file_name().to_string_lossy().into_owned(),
                path: e.path(),
                taken_at: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_millis() as i64),
                size_bytes: meta.len(),
            })
        })
        .collect();
    // Same-second names (`2026-10-09_15.40.06_2.png`) break ties in order.
    shots.sort_by(|a, b| {
        b.taken_at
            .cmp(&a.taken_at)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    shots
}

/// Path of an existing screenshot.
pub fn path(game_dir: &Path, file_name: &str) -> Result<PathBuf> {
    check_name(file_name)?;
    let path = screenshots(game_dir).join(file_name);
    if !path.is_file() {
        return Err(Error::InvalidInput(format!(
            "screenshot not found: {file_name}"
        )));
    }
    Ok(path)
}

/// Path of the screenshot's thumbnail, made (or remade, if the screenshot is newer)
/// on demand.
pub fn thumbnail(
    data: &DataDir,
    instance_id: &str,
    game_dir: &Path,
    file_name: &str,
) -> Result<PathBuf> {
    let source = path(game_dir, file_name)?;
    let dir = thumbnails_of(data, instance_id);
    let thumb = dir.join(format!("{file_name}.jpg"));
    let modified = |p: &Path| p.metadata().and_then(|m| m.modified()).ok();
    if let (Some(t), Some(s)) = (modified(&thumb), modified(&source)) {
        if t >= s {
            return Ok(thumb);
        }
    }

    let image = image::open(&source)?;
    let small = if image.width() > THUMBNAIL_WIDTH {
        let height = (u64::from(image.height()) * u64::from(THUMBNAIL_WIDTH)
            / u64::from(image.width()))
        .max(1) as u32;
        image.thumbnail(THUMBNAIL_WIDTH, height)
    } else {
        image
    };
    std::fs::create_dir_all(&dir)?;
    let tmp = dir.join(format!(".{file_name}.jpg.tmp"));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
    // JPEG has no alpha channel.
    small
        .to_rgb8()
        .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut out,
            THUMBNAIL_QUALITY,
        ))?;
    drop(out);
    std::fs::rename(&tmp, &thumb)?;
    Ok(thumb)
}

/// Deletes a screenshot and its thumbnail.
pub fn delete(data: &DataDir, instance_id: &str, game_dir: &Path, file_name: &str) -> Result<()> {
    std::fs::remove_file(path(game_dir, file_name)?)?;
    let _ = std::fs::remove_file(thumbnails_of(data, instance_id).join(format!("{file_name}.jpg")));
    tracing::info!(instance = instance_id, file_name, "screenshot deleted");
    Ok(())
}

/// Drops every cached thumbnail of an instance.
pub fn clear_thumbnails(data: &DataDir, instance_id: &str) {
    let _ = std::fs::remove_dir_all(thumbnails_of(data, instance_id));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_shot(game_dir: &Path, name: &str, width: u32, height: u32) {
        let dir = screenshots(game_dir);
        std::fs::create_dir_all(&dir).unwrap();
        image::RgbaImage::from_pixel(width, height, image::Rgba([90, 180, 60, 255]))
            .save(dir.join(name))
            .unwrap();
    }

    #[test]
    fn lists_thumbnails_and_deletes() {
        let tmp = tempfile::tempdir().unwrap();
        let data = DataDir::new(tmp.path().join("data"));
        let game_dir = tmp.path().join("instance");
        make_shot(&game_dir, "2026-10-09_15.40.06.png", 1920, 1080);
        make_shot(&game_dir, "2026-10-09_15.40.06_2.png", 200, 100);
        std::fs::write(screenshots(&game_dir).join("notes.txt"), b"x").unwrap();

        let shots = list(&game_dir);
        let names: Vec<_> = shots.iter().map(|s| s.file_name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"2026-10-09_15.40.06.png"));

        let thumb = thumbnail(&data, "i", &game_dir, "2026-10-09_15.40.06.png").unwrap();
        let (w, h) = image::image_dimensions(&thumb).unwrap();
        assert_eq!((w, h), (THUMBNAIL_WIDTH, 270));
        // Cached: the same file comes back without being rewritten.
        let before = thumb.metadata().unwrap().modified().unwrap();
        thumbnail(&data, "i", &game_dir, "2026-10-09_15.40.06.png").unwrap();
        assert_eq!(thumb.metadata().unwrap().modified().unwrap(), before);
        // Small images are not upscaled.
        let small = thumbnail(&data, "i", &game_dir, "2026-10-09_15.40.06_2.png").unwrap();
        assert_eq!(image::image_dimensions(small).unwrap(), (200, 100));

        delete(&data, "i", &game_dir, "2026-10-09_15.40.06.png").unwrap();
        assert!(!thumb.exists());
        assert_eq!(list(&game_dir).len(), 1);
    }

    #[test]
    fn rejects_paths_outside_screenshots() {
        let tmp = tempfile::tempdir().unwrap();
        let data = DataDir::new(tmp.path().join("data"));
        assert!(path(tmp.path(), "../options.png").is_err());
        assert!(path(tmp.path(), "notes.txt").is_err());
        assert!(thumbnail(&data, "i", tmp.path(), "missing.png").is_err());
        assert!(delete(&data, "i", tmp.path(), "../../x.png").is_err());
    }
}
