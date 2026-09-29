//! Publish a finalized MP4 only after the owned FFmpeg child exits
//! successfully. A sibling partial never replaces a previous good output on
//! failure, and abandoned/cancelled stages remove their own partial files.

use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    export_ffmpeg::{FfmpegProcessError, FfmpegRawVideoProcess},
    export_progress::ExportCancellationToken,
};

static NEXT_EXPORT_STAGE: AtomicU64 = AtomicU64::new(0);
const MAX_STAGE_ATTEMPTS: usize = 64;

#[derive(Debug)]
pub enum ExportPublicationError {
    InvalidDestination,
    TemporaryNameExhausted,
    Io(io::Error),
    EmptyOutput,
    Encoder(FfmpegProcessError),
    Cancelled,
}

/// A unique staging name is held by a separate create_new marker. FFmpeg
/// receives the initially nonexistent sibling .partial.mp4 destination and
/// -n (do not replace); marker ownership persists until publication or drop.
#[derive(Debug)]
pub struct PartialExportOutput {
    destination: PathBuf,
    partial: PathBuf,
    marker: PathBuf,
}

impl PartialExportOutput {
    pub fn reserve(destination: &Path) -> Result<Self, ExportPublicationError> {
        let filename = destination
            .file_name()
            .ok_or(ExportPublicationError::InvalidDestination)?;
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(ExportPublicationError::InvalidDestination)?;
        if !destination.is_absolute() {
            return Err(ExportPublicationError::InvalidDestination);
        }

        for _ in 0..MAX_STAGE_ATTEMPTS {
            let mut stem = OsString::from(".");
            stem.push(filename);
            stem.push(format!(
                ".{}.{}",
                std::process::id(),
                NEXT_EXPORT_STAGE.fetch_add(1, Ordering::Relaxed)
            ));
            let mut marker_name = stem.clone();
            marker_name.push(".export-lock");
            let marker = parent.join(marker_name);
            match OpenOptions::new().write(true).create_new(true).open(&marker) {
                Ok(file) => drop(file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(ExportPublicationError::Io(error)),
            }
            stem.push(".partial.mp4");
            let partial = parent.join(stem);
            if partial.exists() {
                let _ = fs::remove_file(marker);
                continue;
            }
            return Ok(Self {
                destination: destination.to_path_buf(),
                partial,
                marker,
            });
        }
        Err(ExportPublicationError::TemporaryNameExhausted)
    }

    #[must_use]
    pub fn partial_path(&self) -> &Path {
        &self.partial
    }

    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// Close the rawvideo pipe, verify FFmpeg's exit status and only then
    /// validate/synchronize the nonempty MP4 and rename over the target.
    /// All failure paths keep the previous destination unchanged.
    pub fn finish_and_publish(
        self,
        process: FfmpegRawVideoProcess,
    ) -> Result<PathBuf, ExportPublicationError> {
        process.finish().map_err(ExportPublicationError::Encoder)?;
        self.publish_completed()
    }

    /// Cancellation is tested both before finalizing FFmpeg and after it
    /// exits but before final output replacement. The partial is removed
    /// automatically by Drop on either canceled path.
    pub fn finish_and_publish_checked(
        self,
        process: FfmpegRawVideoProcess,
        cancellation: &ExportCancellationToken,
    ) -> Result<PathBuf, ExportPublicationError> {
        process
            .finish_checked(cancellation)
            .map_err(ExportPublicationError::Encoder)?;
        if cancellation.is_cancelled() {
            return Err(ExportPublicationError::Cancelled);
        }
        self.publish_completed()
    }

    fn publish_completed(mut self) -> Result<PathBuf, ExportPublicationError> {
        let staged = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.partial)
            .map_err(ExportPublicationError::Io)?;
        let metadata = staged.metadata().map_err(ExportPublicationError::Io)?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(ExportPublicationError::EmptyOutput);
        }
        staged.sync_all().map_err(ExportPublicationError::Io)?;
        drop(staged); // close before Windows rename/replace

        fs::rename(&self.partial, &self.destination).map_err(ExportPublicationError::Io)?;
        self.partial = PathBuf::new();
        Ok(std::mem::take(&mut self.destination))
    }
}

impl Drop for PartialExportOutput {
    fn drop(&mut self) {
        if !self.partial.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.partial);
        }
        if !self.marker.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.marker);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportPublicationError, PartialExportOutput};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rhythm-export-publication-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).expect("test directory");
            Self(root)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn successful_finalized_partial_replaces_old_output_after_sync_and_close() {
        let root = TestDir::new();
        let final_path = root.0.join("final output.mp4");
        fs::write(&final_path, b"old known good").expect("old output");
        let stage = PartialExportOutput::reserve(&final_path).expect("reserve");
        assert!(!stage.partial_path().exists());
        assert_eq!(fs::read(&final_path).expect("old still intact"), b"old known good");
        let partial = stage.partial_path().to_path_buf();
        fs::write(&partial, b"finalized MP4 bytes").expect("simulate finalized encoder");
        assert_eq!(stage.publish_completed().expect("publish"), final_path);
        assert_eq!(fs::read(&final_path).expect("new output"), b"finalized MP4 bytes");
        assert!(!partial.exists());
        assert_eq!(fs::read_dir(&root.0).expect("list").count(), 1);
    }

    #[test]
    fn abort_removes_only_own_partial_and_preserves_old_output() {
        let root = TestDir::new();
        let final_path = root.0.join("final.mp4");
        fs::write(&final_path, b"old known good").expect("old");
        let first = PartialExportOutput::reserve(&final_path).expect("first");
        let second = PartialExportOutput::reserve(&final_path).expect("second");
        assert_ne!(first.partial_path(), second.partial_path());
        let unfinished = first.partial_path().to_path_buf();
        let other = second.partial_path().to_path_buf();
        fs::write(&unfinished, b"unfinished bytes").expect("partial");
        fs::write(&other, b"other partial").expect("other");
        drop(first);
        assert!(!unfinished.exists());
        assert_eq!(fs::read(&other).expect("second untouched"), b"other partial");
        assert_eq!(fs::read(&final_path).expect("old untouched"), b"old known good");
        drop(second);
        assert!(!other.exists());
        assert_eq!(fs::read_dir(&root.0).expect("list").count(), 1);
    }

    #[test]
    fn empty_or_failed_publish_cannot_replace_known_good_destination() {
        let root = TestDir::new();
        let final_path = root.0.join("final.mp4");
        fs::write(&final_path, b"old known good").expect("old");
        let stage = PartialExportOutput::reserve(&final_path).expect("reserve");
        fs::write(stage.partial_path(), b"").expect("empty");
        assert!(matches!(
            stage.publish_completed(),
            Err(ExportPublicationError::EmptyOutput)
        ));
        assert_eq!(fs::read(&final_path).expect("old untouched"), b"old known good");

        let blocked = root.0.join("destination-is-directory.mp4");
        fs::create_dir(&blocked).expect("directory");
        let stage = PartialExportOutput::reserve(&blocked).expect("reserve");
        let temp = stage.partial_path().to_path_buf();
        fs::write(&temp, b"valid staged bytes").expect("staged");
        assert!(matches!(
            stage.publish_completed(),
            Err(ExportPublicationError::Io(_))
        ));
        assert!(blocked.is_dir());
        assert!(!temp.exists());
    }

    #[test]
    fn invalid_relative_destination_does_not_create_working_directory_artifacts() {
        assert!(matches!(
            PartialExportOutput::reserve(std::path::Path::new("relative.mp4")),
            Err(ExportPublicationError::InvalidDestination)
        ));
    }
}
