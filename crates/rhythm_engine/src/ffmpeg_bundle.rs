//! Resolve and validate the pinned portable FFmpeg executable.
//! Normal release behavior never falls back to PATH.

use std::{
    io,
    path::{Component, Path, PathBuf},
    process::{Command, ExitStatus},
};

pub const PINNED_FFMPEG_VERSION_PREFIX: &str = "ffmpeg version 9.0.2-essentials_build-www.gyan.dev";

#[derive(Debug)]
pub enum FfmpegBundleError {
    InvalidApplicationExecutable,
    MissingBundledExecutable(PathBuf),
    ProbeSpawn(io::Error),
    ProbeFailed(ExitStatus),
    MissingVersionBanner,
    UnexpectedVersion {
        expected_prefix: &'static str,
        actual: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegBundleVersion {
    pub executable: PathBuf,
    pub version_banner: String,
}

fn normalized_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

/// Resolve the portable ZIP contract:
/// <package>/RhythmEffects.exe
/// <package>/ffmpeg/ffmpeg.exe
pub fn bundled_ffmpeg_path_from_application(
    application_executable: &Path,
) -> Result<PathBuf, FfmpegBundleError> {
    if !normalized_absolute(application_executable) || application_executable.file_name().is_none()
    {
        return Err(FfmpegBundleError::InvalidApplicationExecutable);
    }
    let parent = application_executable
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or(FfmpegBundleError::InvalidApplicationExecutable)?;
    Ok(parent.join("ffmpeg").join("ffmpeg.exe"))
}

pub fn resolve_bundled_ffmpeg_path() -> Result<PathBuf, FfmpegBundleError> {
    let application = std::env::current_exe().map_err(FfmpegBundleError::ProbeSpawn)?;
    let path = bundled_ffmpeg_path_from_application(&application)?;
    if !path.is_file() {
        return Err(FfmpegBundleError::MissingBundledExecutable(path));
    }
    Ok(path)
}

fn parse_version_banner(stdout: &[u8]) -> Result<String, FfmpegBundleError> {
    let text = String::from_utf8_lossy(stdout);
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or(FfmpegBundleError::MissingVersionBanner)?;
    if !line.starts_with(PINNED_FFMPEG_VERSION_PREFIX) {
        return Err(FfmpegBundleError::UnexpectedVersion {
            expected_prefix: PINNED_FFMPEG_VERSION_PREFIX,
            actual: line.to_owned(),
        });
    }
    Ok(line.to_owned())
}

/// Query the exact executable path; never execute "ffmpeg" by name.
pub fn check_bundled_ffmpeg_version(
    executable: &Path,
) -> Result<FfmpegBundleVersion, FfmpegBundleError> {
    if !normalized_absolute(executable) || !executable.is_file() {
        return Err(FfmpegBundleError::MissingBundledExecutable(
            executable.to_path_buf(),
        ));
    }
    let output = Command::new(executable)
        .arg("-hide_banner")
        .arg("-version")
        .output()
        .map_err(FfmpegBundleError::ProbeSpawn)?;
    if !output.status.success() {
        return Err(FfmpegBundleError::ProbeFailed(output.status));
    }
    let version_banner = parse_version_banner(&output.stdout)?;
    Ok(FfmpegBundleVersion {
        executable: executable.to_path_buf(),
        version_banner,
    })
}

pub fn resolve_and_check_bundled_ffmpeg() -> Result<FfmpegBundleVersion, FfmpegBundleError> {
    let executable = resolve_bundled_ffmpeg_path()?;
    check_bundled_ffmpeg_version(&executable)
}

#[cfg(test)]
mod tests {
    use super::{
        FfmpegBundleError, PINNED_FFMPEG_VERSION_PREFIX, bundled_ffmpeg_path_from_application,
        parse_version_banner,
    };
    use std::path::Path;

    #[test]
    fn package_path_is_resolved_only_beside_application() {
        let application = std::env::temp_dir()
            .join("rhythm-portable-package")
            .join("RhythmEffects.exe");
        let expected = application
            .parent()
            .expect("package root")
            .join("ffmpeg")
            .join("ffmpeg.exe");
        assert_eq!(
            bundled_ffmpeg_path_from_application(&application).expect("resolve"),
            expected
        );
    }

    #[test]
    fn relative_or_lexically_unsafe_application_path_is_rejected() {
        assert!(matches!(
            bundled_ffmpeg_path_from_application(Path::new("RhythmEffects.exe")),
            Err(FfmpegBundleError::InvalidApplicationExecutable)
        ));
        let unsafe_path = std::env::temp_dir()
            .join("package")
            .join("..")
            .join("RhythmEffects.exe");
        assert!(matches!(
            bundled_ffmpeg_path_from_application(&unsafe_path),
            Err(FfmpegBundleError::InvalidApplicationExecutable)
        ));
    }

    #[test]
    fn exact_pinned_version_prefix_is_required() {
        let expected = format!("{PINNED_FFMPEG_VERSION_PREFIX} Copyright...");
        assert_eq!(
            parse_version_banner(expected.as_bytes()).expect("known version"),
            expected
        );
        assert!(matches!(
            parse_version_banner(b"ffmpeg version 9.1-something"),
            Err(FfmpegBundleError::UnexpectedVersion { .. })
        ));
        assert!(matches!(
            parse_version_banner(b"\r\n"),
            Err(FfmpegBundleError::MissingVersionBanner)
        ));
    }

    #[test]
    fn arbitrary_path_does_not_gain_path_fallback_semantics() {
        let missing = std::env::temp_dir().join("definitely-missing-ffmpeg.exe");
        let error = super::check_bundled_ffmpeg_version(&missing).expect_err("missing");
        assert!(matches!(
            error,
            FfmpegBundleError::MissingBundledExecutable(path) if path == missing
        ));
    }

    #[test]
    fn pinned_manifest_and_runtime_version_contract_do_not_drift() {
        let manifest = include_str!("../../../packaging/ffmpeg/manifest.json");
        assert!(manifest.contains("\"ffmpeg_version\": \"9.0.2\""));
        assert!(manifest.contains(PINNED_FFMPEG_VERSION_PREFIX));
        assert!(manifest.contains("\"bundled_executable_relative_path\": \"ffmpeg/ffmpeg.exe\""));
    }
}
