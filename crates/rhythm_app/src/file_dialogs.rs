use std::path::{Path, PathBuf};

use rfd::FileDialog;

const PROJECT_EXTENSION: &str = "rhfx";
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];
const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "ogg", "m4a", "aac"];

#[must_use]
pub fn pick_project_to_open() -> Option<PathBuf> {
    FileDialog::new()
        .set_title("Open Rhythm Effects Project")
        .add_filter("Rhythm Effects Project", &[PROJECT_EXTENSION])
        .pick_file()
}

#[must_use]
pub fn pick_project_save_path(suggested_name: &str) -> Option<PathBuf> {
    let suggested_name = project_file_name(suggested_name);
    FileDialog::new()
        .set_title("Save Rhythm Effects Project")
        .add_filter("Rhythm Effects Project", &[PROJECT_EXTENSION])
        .set_file_name(&suggested_name)
        .save_file()
        .map(|path| ensure_extension(path, PROJECT_EXTENSION))
}

#[must_use]
pub fn pick_media_to_import() -> Option<PathBuf> {
    FileDialog::new()
        .set_title("Import Media")
        .add_filter("Images", IMAGE_EXTENSIONS)
        .add_filter("Audio", AUDIO_EXTENSIONS)
        .pick_file()
}

#[must_use]
pub fn pick_image_to_relink() -> Option<PathBuf> {
    FileDialog::new()
        .set_title("Relink Image")
        .add_filter("Images", IMAGE_EXTENSIONS)
        .pick_file()
}

#[must_use]
pub fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

fn project_file_name(project_name: &str) -> String {
    let trimmed = project_name.trim();
    let stem = if trimmed.is_empty() {
        "Untitled"
    } else {
        trimmed
    };
    if Path::new(stem)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PROJECT_EXTENSION))
    {
        stem.to_owned()
    } else {
        format!("{stem}.{PROJECT_EXTENSION}")
    }
}

fn ensure_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    if path
        .extension()
        .and_then(|observed| observed.to_str())
        .is_none_or(|observed| !observed.eq_ignore_ascii_case(extension))
    {
        path.set_extension(extension);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::{ensure_extension, is_supported_image_path, project_file_name};
    use std::path::{Path, PathBuf};

    #[test]
    fn supported_image_path_is_extension_case_insensitive() {
        assert!(is_supported_image_path(Path::new("image.png")));
        assert!(is_supported_image_path(Path::new("photo.JPEG")));
        assert!(is_supported_image_path(Path::new("art.WebP")));
        assert!(!is_supported_image_path(Path::new("audio.wav")));
        assert!(!is_supported_image_path(Path::new("README")));
    }

    #[test]
    fn project_file_name_uses_rhfx_once() {
        assert_eq!(project_file_name("Untitled"), "Untitled.rhfx");
        assert_eq!(project_file_name("Demo.rhfx"), "Demo.rhfx");
        assert_eq!(project_file_name("Demo.RHFX"), "Demo.RHFX");
        assert_eq!(project_file_name("   "), "Untitled.rhfx");
    }

    #[test]
    fn save_path_normalization_replaces_non_project_extension() {
        assert_eq!(
            ensure_extension(PathBuf::from("demo"), "rhfx"),
            PathBuf::from("demo.rhfx")
        );
        assert_eq!(
            ensure_extension(PathBuf::from("demo.json"), "rhfx"),
            PathBuf::from("demo.rhfx")
        );
        assert_eq!(
            ensure_extension(PathBuf::from("demo.RHFX"), "rhfx"),
            PathBuf::from("demo.RHFX")
        );
    }
}
