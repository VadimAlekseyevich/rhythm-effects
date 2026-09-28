//! Windows MVP recovery root: %LOCALAPPDATA%\\RhythmEffects\\recovery.
//! Never fall back to a relative/working-directory path on a missing variable.

use std::{
    env,
    ffi::OsStr,
    fs,
    io,
    path::{Path, PathBuf},
};

const APPLICATION_DIRECTORY: &str = "RhythmEffects";
const RECOVERY_DIRECTORY: &str = "recovery";

fn recovery_root_from_local_app_data(base: Option<&OsStr>) -> io::Result<PathBuf> {
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
    Ok(base.join(APPLICATION_DIRECTORY).join(RECOVERY_DIRECTORY))
}

/// Resolve the per-user Windows recovery directory, creating it only after
/// validating the configured absolute application-data root.
pub fn ensure_recovery_root() -> io::Result<PathBuf> {
    ensure_recovery_root_at(env::var_os("LOCALAPPDATA").as_deref())
}

fn ensure_recovery_root_at(base: Option<&OsStr>) -> io::Result<PathBuf> {
    let directory = recovery_root_from_local_app_data(base)?;
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

#[cfg(test)]
mod tests {
    use super::{ensure_recovery_root_at, recovery_root_from_local_app_data};
    use std::{
        ffi::OsStr,
        fs,
        io,
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn directory_is_scoped_to_local_app_data_and_created_on_demand() {
        let base = std::env::temp_dir().join(format!(
            "rhythm-recovery-base-{}-{}",
            std::process::id(),
            NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        let expected = base.join("RhythmEffects").join("recovery");
        assert_eq!(
            recovery_root_from_local_app_data(Some(base.as_os_str())).expect("resolve"),
            expected
        );
        assert!(!expected.exists());
        assert_eq!(
            ensure_recovery_root_at(Some(base.as_os_str())).expect("create"),
            expected
        );
        assert!(expected.is_dir());
        assert_eq!(
            ensure_recovery_root_at(Some(base.as_os_str())).expect("idempotent"),
            expected
        );
        fs::remove_dir_all(base).expect("cleanup");
    }

    #[test]
    fn missing_empty_and_relative_local_app_data_never_write_to_cwd() {
        assert_eq!(
            recovery_root_from_local_app_data(None)
                .expect_err("missing variable")
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            ensure_recovery_root_at(Some(OsStr::new("")))
                .expect_err("empty variable")
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            ensure_recovery_root_at(Some(OsStr::new("relative-app-data")))
                .expect_err("relative path")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!Path::new("relative-app-data").join("RhythmEffects").exists());
    }

    #[test]
    fn blocked_destination_returns_error_without_removing_existing_file() {
        let base = std::env::temp_dir().join(format!(
            "rhythm-recovery-blocker-{}-{}",
            std::process::id(),
            NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&base).expect("create base");
        let blocker = base.join("RhythmEffects");
        fs::write(&blocker, b"known-good").expect("blocker");
        assert!(ensure_recovery_root_at(Some(base.as_os_str())).is_err());
        assert_eq!(fs::read(&blocker).expect("unchanged file"), b"known-good");
        fs::remove_dir_all(base).expect("cleanup");
    }
}
