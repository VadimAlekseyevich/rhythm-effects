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
    EncoderProbeSpawn(io::Error),
    EncoderProbeFailed(ExitStatus),
    MissingVersionBanner,
    UnexpectedVersion {
        expected_prefix: &'static str,
        actual: String,
    },
    MissingRequiredEncoder(&'static str),
}

impl FfmpegBundleError {
    /// Stable short copy suitable for an export dialog. Technical details
    /// (path, exit status, OS error) remain available in the structured error
    /// for logs and diagnostics.
    #[must_use]
    pub const fn user_message(&self) -> &'static str {
        match self {
            Self::InvalidApplicationExecutable => {
                "Rhythm Effects could not determine its portable package location."
            }
            Self::MissingBundledExecutable(_) => {
                "Bundled FFmpeg is missing. Re-extract the complete Rhythm Effects package."
            }
            Self::ProbeSpawn(_) | Self::EncoderProbeSpawn(_) => {
                "Bundled FFmpeg could not start. It may be blocked or quarantined by Windows security software."
            }
            Self::ProbeFailed(_) | Self::EncoderProbeFailed(_) => {
                "Bundled FFmpeg started but its self-check failed. Re-extract the release package."
            }
            Self::MissingVersionBanner | Self::UnexpectedVersion { .. } => {
                "The bundled FFmpeg version does not match this Rhythm Effects release."
            }
            Self::MissingRequiredEncoder(_) => {
                "Bundled FFmpeg is missing a required H.264 or AAC encoder."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegBundleVersion {
    pub executable: PathBuf,
    pub version_banner: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegBundleCapabilities {
    pub version: FfmpegBundleVersion,
    pub h264_encoder: &'static str,
    pub aac_encoder: &'static str,
}

pub const REQUIRED_H264_ENCODER: &str = "libx264";
pub const REQUIRED_AAC_ENCODER: &str = "aac";

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

fn encoder_list_contains(stdout: &[u8], encoder: &str) -> bool {
    String::from_utf8_lossy(stdout)
        .lines()
        .flat_map(str::split_whitespace)
        .any(|token| token == encoder)
}

/// Verify the release encoding contract after the pinned version probe.
/// The exact executable is queried directly; no codec fallback is accepted.
pub fn check_bundled_ffmpeg_capabilities(
    executable: &Path,
) -> Result<FfmpegBundleCapabilities, FfmpegBundleError> {
    let version = check_bundled_ffmpeg_version(executable)?;
    let output = Command::new(executable)
        .arg("-hide_banner")
        .arg("-encoders")
        .output()
        .map_err(FfmpegBundleError::EncoderProbeSpawn)?;
    if !output.status.success() {
        return Err(FfmpegBundleError::EncoderProbeFailed(output.status));
    }
    if !encoder_list_contains(&output.stdout, REQUIRED_H264_ENCODER) {
        return Err(FfmpegBundleError::MissingRequiredEncoder(
            REQUIRED_H264_ENCODER,
        ));
    }
    if !encoder_list_contains(&output.stdout, REQUIRED_AAC_ENCODER) {
        return Err(FfmpegBundleError::MissingRequiredEncoder(
            REQUIRED_AAC_ENCODER,
        ));
    }
    Ok(FfmpegBundleCapabilities {
        version,
        h264_encoder: REQUIRED_H264_ENCODER,
        aac_encoder: REQUIRED_AAC_ENCODER,
    })
}

pub fn resolve_and_check_bundled_ffmpeg() -> Result<FfmpegBundleCapabilities, FfmpegBundleError> {
    let executable = resolve_bundled_ffmpeg_path()?;
    check_bundled_ffmpeg_capabilities(&executable)
}

#[cfg(test)]
mod tests {
    use super::{
        FfmpegBundleError, PINNED_FFMPEG_VERSION_PREFIX, REQUIRED_AAC_ENCODER,
        REQUIRED_H264_ENCODER, bundled_ffmpeg_path_from_application, encoder_list_contains,
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
    fn encoder_list_requires_exact_h264_and_aac_names() {
        let output =
            b"Encoders:\n V....D libx264 H.264 / AVC\n A..... aac AAC (Advanced Audio Coding)\n";
        assert!(encoder_list_contains(output, REQUIRED_H264_ENCODER));
        assert!(encoder_list_contains(output, REQUIRED_AAC_ENCODER));
        assert!(!encoder_list_contains(output, "h264"));
        assert!(!encoder_list_contains(output, "libx265"));

        let deceptive = b"Encoders:\n V....D libx264rgb H.264 RGB\n A..... aac_latm LATM AAC\n";
        assert!(!encoder_list_contains(deceptive, REQUIRED_H264_ENCODER));
        assert!(!encoder_list_contains(deceptive, REQUIRED_AAC_ENCODER));
    }

    #[test]
    fn missing_bundle_has_actionable_user_message() {
        let missing = std::env::temp_dir().join("rhythm-missing-ffmpeg.exe");
        let error = super::check_bundled_ffmpeg_version(&missing).expect_err("missing");
        assert_eq!(
            error.user_message(),
            "Bundled FFmpeg is missing. Re-extract the complete Rhythm Effects package."
        );
    }

    #[cfg(windows)]
    #[test]
    fn invalid_or_quarantined_executable_has_actionable_user_message() {
        use std::{
            fs,
            path::PathBuf,
            sync::atomic::{AtomicU64, Ordering},
        };

        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root: PathBuf = std::env::temp_dir().join(format!(
            "rhythm-blocked-ffmpeg-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create isolated temp directory");
        let fake = root.join("ffmpeg.exe");
        fs::write(&fake, b"not a Windows executable").expect("write fake executable");

        let error = super::check_bundled_ffmpeg_version(&fake).expect_err("must not execute");
        assert!(matches!(error, FfmpegBundleError::ProbeSpawn(_)));
        assert_eq!(
            error.user_message(),
            "Bundled FFmpeg could not start. It may be blocked or quarantined by Windows security software."
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn wrong_version_and_missing_codec_have_distinct_user_messages() {
        let wrong = parse_version_banner(b"ffmpeg version 8.0-other").expect_err("wrong");
        assert_eq!(
            wrong.user_message(),
            "The bundled FFmpeg version does not match this Rhythm Effects release."
        );
        let missing = FfmpegBundleError::MissingRequiredEncoder(REQUIRED_H264_ENCODER);
        assert_eq!(
            missing.user_message(),
            "Bundled FFmpeg is missing a required H.264 or AAC encoder."
        );
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
        assert!(manifest.contains(&format!("\"{REQUIRED_H264_ENCODER}\"")));
        assert!(manifest.contains(&format!("\"{REQUIRED_AAC_ENCODER}\"")));
    }
}
