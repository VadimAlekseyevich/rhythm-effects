//! Per-editing-session metadata. No creative project bytes are stored here.

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(0);
const METADATA_FILE: &str = "session.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySessionMetadata {
    pub session_id: String,
    pub project_name: String,
    pub canonical_project_path: Option<PathBuf>,
    pub created_unix_ms: u64,
}

/// Owns one stable recovery identity across edits and Save/Save As; changing
/// the project path updates metadata, not the identity or recovery directory.
#[derive(Debug)]
pub struct RecoverySession {
    directory: PathBuf,
    metadata: RecoverySessionMetadata,
}

impl RecoverySession {
    pub fn create(root: &Path, project_name: &str, canonical_path: Option<&Path>) -> io::Result<Self> {
        for _ in 0..64 {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(io::Error::other)?;
            let session_id = format!(
                "session-{:032x}-{:08x}-{:016x}",
                now.as_nanos(),
                std::process::id(),
                NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)
            );
            let directory = root.join(&session_id);
            match fs::create_dir(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
            let metadata = RecoverySessionMetadata {
                session_id,
                project_name: project_name.to_owned(),
                canonical_project_path: canonical_path.map(Path::to_path_buf),
                created_unix_ms: now.as_millis().try_into().unwrap_or(u64::MAX),
            };
            let bytes = serde_json::to_vec_pretty(&metadata).map_err(io::Error::other);
            let result = bytes.and_then(|bytes| {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(directory.join(METADATA_FILE))?;
                file.write_all(&bytes)?;
                file.sync_all()
            });
            if let Err(error) = result {
                let _ = fs::remove_dir_all(&directory);
                return Err(error);
            }
            return Ok(Self {
                directory,
                metadata,
            });
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique recovery session",
        ))
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    #[must_use]
    pub fn metadata(&self) -> &RecoverySessionMetadata {
        &self.metadata
    }

    /// Publish changed display/canonical metadata through a synchronized
    /// sibling temp, never truncate the existing known-good session record.
    pub fn update_project_identity(
        &mut self,
        project_name: &str,
        canonical_path: Option<&Path>,
    ) -> io::Result<()> {
        let mut updated = self.metadata.clone();
        updated.project_name = project_name.to_owned();
        updated.canonical_project_path = canonical_path.map(Path::to_path_buf);
        let bytes = serde_json::to_vec_pretty(&updated).map_err(io::Error::other)?;
        let temp_path = self.directory.join(format!(
            ".session-{}.tmp",
            NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| -> io::Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp_path, self.directory.join(METADATA_FILE))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        result?;
        self.metadata = updated;
        Ok(())
    }

    pub fn read_metadata(directory: &Path) -> io::Result<RecoverySessionMetadata> {
        let bytes = fs::read(directory.join(METADATA_FILE))?;
        let metadata: RecoverySessionMetadata =
            serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if directory.file_name().and_then(|name| name.to_str())
            != Some(metadata.session_id.as_str())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "recovery session directory/metadata ID mismatch",
            ));
        }
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::RecoverySession;
    use std::{
        fs, io,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "rhythm-session-meta-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).expect("test root");
            Self(directory)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn session_id_is_unique_stable_and_persisted_for_unsaved_projects() {
        let root = TestDir::new();
        let mut first = RecoverySession::create(&root.0, "Unsaved", None).expect("first");
        let second = RecoverySession::create(&root.0, "Unsaved", None).expect("second");
        assert_ne!(first.metadata().session_id, second.metadata().session_id);
        let id = first.metadata().session_id.clone();
        assert!(first.directory().is_dir());
        assert_eq!(
            RecoverySession::read_metadata(first.directory()).expect("read initial"),
            *first.metadata()
        );
        assert!(first.metadata().canonical_project_path.is_none());

        let canonical = root.0.join("saved.rhfx");
        first
            .update_project_identity("Renamed", Some(&canonical))
            .expect("metadata update");
        assert_eq!(first.metadata().session_id, id);
        assert_eq!(first.metadata().canonical_project_path.as_deref(), Some(canonical.as_path()));
        assert_eq!(
            RecoverySession::read_metadata(first.directory()).expect("read updated"),
            *first.metadata()
        );
        assert_eq!(fs::read_dir(first.directory()).expect("list").count(), 1);
    }

    #[test]
    fn failed_metadata_replacement_does_not_mutate_in_memory_record() {
        let root = TestDir::new();
        let mut session = RecoverySession::create(&root.0, "Previous", None).expect("create");
        let before = session.metadata().clone();
        fs::remove_file(session.directory().join("session.json")).expect("remove");
        fs::create_dir(session.directory().join("session.json")).expect("block destination");

        assert!(session.update_project_identity("Updated", None).is_err());
        assert_eq!(session.metadata(), &before);
        assert_eq!(fs::read_dir(session.directory()).expect("list").count(), 1);
    }

    #[test]
    fn mismatch_and_malformed_metadata_are_rejected() {
        let root = TestDir::new();
        let session = RecoverySession::create(&root.0, "Original", None).expect("create");
        let wrong = root.0.join("wrong-id");
        fs::create_dir(&wrong).expect("wrong directory");
        fs::copy(session.directory().join("session.json"), wrong.join("session.json"))
            .expect("copy known metadata");
        assert_eq!(
            RecoverySession::read_metadata(&wrong).expect_err("mismatch").kind(),
            io::ErrorKind::InvalidData
        );
        fs::write(wrong.join("session.json"), b"invalid json").expect("corrupt");
        assert_eq!(
            RecoverySession::read_metadata(&wrong).expect_err("malformed").kind(),
            io::ErrorKind::Other
        );
    }
}
