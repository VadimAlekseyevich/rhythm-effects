//! Windows per-user application-data layout.
//! Never fall back to the current directory when LOCALAPPDATA is unavailable.

use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

pub const APPLICATION_DIRECTORY: &str = "RhythmEffects";
pub const SETTINGS_DIRECTORY: &str = "settings";
pub const LOGS_DIRECTORY: &str = "logs";
pub const RECOVERY_DIRECTORY: &str = "recovery";
pub const CACHE_DIRECTORY: &str = "cache";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
    settings: PathBuf,
    logs: PathBuf,
    recovery: PathBuf,
    cache: PathBuf,
}

impl AppPaths {
    fn from_local_app_data(base: Option<&OsStr>) -> io::Result<Self> {
        let base = base
            .filter(|value| !value.is_empty())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is not set"))?;
        let base = Path::new(base);
        if !base.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "LOCALAPPDATA must be an absolute directory",
            ));
        }
        if base
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "LOCALAPPDATA must not contain parent traversal",
            ));
        }

        let root = base.join(APPLICATION_DIRECTORY);
        Ok(Self {
            settings: root.join(SETTINGS_DIRECTORY),
            logs: root.join(LOGS_DIRECTORY),
            recovery: root.join(RECOVERY_DIRECTORY),
            cache: root.join(CACHE_DIRECTORY),
            root,
        })
    }

    pub fn ensure() -> io::Result<Self> {
        Self::ensure_at(env::var_os("LOCALAPPDATA").as_deref())
    }

    fn ensure_at(base: Option<&OsStr>) -> io::Result<Self> {
        let paths = Self::from_local_app_data(base)?;
        // Create the app root first, then every owned subdirectory. Existing
        // files at any expected directory cause an error; nothing is removed.
        fs::create_dir_all(&paths.root)?;
        for directory in [&paths.settings, &paths.logs, &paths.recovery, &paths.cache] {
            fs::create_dir_all(directory)?;
        }
        Ok(paths)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn settings(&self) -> &Path {
        &self.settings
    }

    #[must_use]
    pub fn logs(&self) -> &Path {
        &self.logs
    }

    #[must_use]
    pub fn recovery(&self) -> &Path {
        &self.recovery
    }

    #[must_use]
    pub fn cache(&self) -> &Path {
        &self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::{
        APPLICATION_DIRECTORY, AppPaths, CACHE_DIRECTORY, LOGS_DIRECTORY, RECOVERY_DIRECTORY,
        SETTINGS_DIRECTORY,
    };
    use std::{
        ffi::OsStr,
        fs, io,
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn test_base(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rhythm-app-paths-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn resolves_all_owned_directories_under_one_local_app_data_root() {
        let base = test_base("layout");
        let paths = AppPaths::from_local_app_data(Some(base.as_os_str())).expect("resolve");
        let root = base.join(APPLICATION_DIRECTORY);
        assert_eq!(paths.root(), root);
        assert_eq!(paths.settings(), root.join(SETTINGS_DIRECTORY));
        assert_eq!(paths.logs(), root.join(LOGS_DIRECTORY));
        assert_eq!(paths.recovery(), root.join(RECOVERY_DIRECTORY));
        assert_eq!(paths.cache(), root.join(CACHE_DIRECTORY));
        assert!(!root.exists());
    }

    #[test]
    fn ensure_creates_complete_layout_idempotently() {
        let base = test_base("create");
        let first = AppPaths::ensure_at(Some(base.as_os_str())).expect("first create");
        let second = AppPaths::ensure_at(Some(base.as_os_str())).expect("second create");
        assert_eq!(first, second);
        for path in [
            first.root(),
            first.settings(),
            first.logs(),
            first.recovery(),
            first.cache(),
        ] {
            assert!(path.is_dir(), "{} should be a directory", path.display());
        }
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn missing_empty_relative_and_parent_traversal_roots_are_rejected() {
        assert_eq!(
            AppPaths::from_local_app_data(None)
                .expect_err("missing")
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            AppPaths::from_local_app_data(Some(OsStr::new("")))
                .expect_err("empty")
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            AppPaths::from_local_app_data(Some(OsStr::new("relative")))
                .expect_err("relative")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!Path::new("relative").join(APPLICATION_DIRECTORY).exists());
    }

    #[test]
    fn blocker_file_is_never_deleted_or_replaced() {
        let base = test_base("blocked");
        fs::create_dir_all(&base).expect("base");
        let root = base.join(APPLICATION_DIRECTORY);
        fs::create_dir_all(&root).expect("root");
        let blocker = root.join(LOGS_DIRECTORY);
        fs::write(&blocker, b"keep me").expect("blocker");
        assert!(AppPaths::ensure_at(Some(base.as_os_str())).is_err());
        assert_eq!(fs::read(&blocker).expect("unchanged"), b"keep me");
        fs::remove_dir_all(base).expect("cleanup");
    }
}
