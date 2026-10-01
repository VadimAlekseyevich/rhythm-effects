//! Application-authored export prerequisite messaging.
//! Backend diagnostics stay structured; this module owns user-facing wording.

use std::io;

use rhythm_engine::ffmpeg_bundle::FfmpegBundleError;

pub const FFMPEG_UNAVAILABLE_MESSAGE: &str =
    "Export is unavailable because the bundled FFmpeg could not be found or launched. No export was started and no output file was created. Re-extract the complete Rhythm Effects ZIP. If antivirus software quarantined ffmpeg\\ffmpeg.exe, restore or allow that file and retry.";

pub const FFMPEG_MISMATCH_MESSAGE: &str =
    "Export is unavailable because the bundled FFmpeg does not match this Rhythm Effects package. No export was started and no output file was created. Re-extract the complete matching Rhythm Effects ZIP.";

#[must_use]
pub fn bundled_ffmpeg_user_message(error: &FfmpegBundleError) -> &'static str {
    match error {
        FfmpegBundleError::MissingBundledExecutable(_)
        | FfmpegBundleError::ProbeSpawn(error)
        | FfmpegBundleError::EncoderProbeSpawn(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            FFMPEG_UNAVAILABLE_MESSAGE
        }
        FfmpegBundleError::InvalidApplicationExecutable
        | FfmpegBundleError::ProbeFailed(_)
        | FfmpegBundleError::EncoderProbeFailed(_)
        | FfmpegBundleError::MissingVersionBanner
        | FfmpegBundleError::UnexpectedVersion { .. }
        | FfmpegBundleError::MissingRequiredEncoder(_)
        | FfmpegBundleError::ProbeSpawn(_)
        | FfmpegBundleError::EncoderProbeSpawn(_) => FFMPEG_MISMATCH_MESSAGE,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FFMPEG_MISMATCH_MESSAGE, FFMPEG_UNAVAILABLE_MESSAGE, bundled_ffmpeg_user_message,
    };
    use rhythm_engine::ffmpeg_bundle::FfmpegBundleError;
    use std::{io, path::PathBuf};

    #[test]
    fn missing_bundled_binary_has_actionable_non_destructive_message() {
        let error =
            FfmpegBundleError::MissingBundledExecutable(PathBuf::from(r"C:\portable\ffmpeg\ffmpeg.exe"));
        let message = bundled_ffmpeg_user_message(&error);
        assert_eq!(message, FFMPEG_UNAVAILABLE_MESSAGE);
        assert!(message.contains("No export was started"));
        assert!(message.contains("Re-extract"));
        assert!(message.contains("quarantined"));
        assert!(!message.contains("PATH"));
    }

    #[test]
    fn quarantine_like_permission_denial_uses_same_recovery_message() {
        let error = FfmpegBundleError::ProbeSpawn(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "simulated antivirus quarantine",
        ));
        assert_eq!(
            bundled_ffmpeg_user_message(&error),
            FFMPEG_UNAVAILABLE_MESSAGE
        );
    }

    #[test]
    fn wrong_version_or_capabilities_request_matching_package() {
        let version = FfmpegBundleError::UnexpectedVersion {
            expected_prefix: "expected",
            actual: "other".into(),
        };
        assert_eq!(
            bundled_ffmpeg_user_message(&version),
            FFMPEG_MISMATCH_MESSAGE
        );
        assert_eq!(
            bundled_ffmpeg_user_message(&FfmpegBundleError::MissingRequiredEncoder("libx264")),
            FFMPEG_MISMATCH_MESSAGE
        );
    }
}
