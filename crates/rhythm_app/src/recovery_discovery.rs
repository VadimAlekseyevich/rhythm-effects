//! Read-only startup discovery of potentially restorable project snapshots.

use crate::recovery_session::{RecoverySession, RecoverySessionMetadata};
use std::{
    fs,
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RecoveryCandidate {
    pub directory: PathBuf,
    pub metadata: RecoverySessionMetadata,
    pub current_modified: Option<SystemTime>,
    pub previous_modified: Option<SystemTime>,
    pub canonical_modified: Option<SystemTime>,
}

impl RecoveryCandidate {
    #[must_use]
    pub fn latest_recovery_modified(&self) -> Option<SystemTime> {
        self.current_modified.max(self.previous_modified)
    }
}

/// Discover only non-symlink session directories with valid matching metadata
/// and at least one regular generation file. Discovery is observational: it
/// never deletes, restores or overwrites any canonical project document.
pub fn discover_recovery_candidates(root: &Path) -> io::Result<Vec<RecoveryCandidate>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let directory = entry.path();
        let metadata = match RecoverySession::read_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) => {
                warn!(path = %directory.display(), %error, "skipping invalid recovery metadata");
                continue;
            }
        };
        let current_modified = regular_file_modified(&directory.join("current.rhfx"))?;
        let previous_modified = regular_file_modified(&directory.join("previous.rhfx"))?;
        if current_modified.is_none() && previous_modified.is_none() {
            continue;
        }
        let canonical_modified = metadata
            .canonical_project_path
            .as_deref()
            .filter(|path| path.is_absolute())
            .map(regular_file_modified)
            .transpose()?
            .flatten();

        candidates.push(RecoveryCandidate {
            directory,
            metadata,
            current_modified,
            previous_modified,
            canonical_modified,
        });
    }
    candidates.sort_by(|left, right| {
        right
            .latest_recovery_modified()
            .cmp(&left.latest_recovery_modified())
            .then_with(|| left.metadata.session_id.cmp(&right.metadata.session_id))
    });
    Ok(candidates)
}

fn regular_file_modified(path: &Path) -> io::Result<Option<SystemTime>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if metadata.is_file() {
        metadata.modified().map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::discover_recovery_candidates;
    use crate::{
        recovery_autosave::write_recovery_generation,
        recovery_session::RecoverySession,
    };
    use rhythm_core::{
        project::{Project, ProjectSettings},
        time::{GridOffsetNs, TempoMap},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "rhythm-recovery-discovery-{}-{}",
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

    fn project() -> Project {
        Project::new(
            "Recovered",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        )
    }

    #[test]
    fn absent_root_and_session_without_generation_are_not_candidates() {
        let root = TestDir::new();
        assert!(discover_recovery_candidates(&root.0.join("absent")).expect("missing").is_empty());
        RecoverySession::create(&root.0, "Unsaved", None).expect("create metadata");
        assert!(discover_recovery_candidates(&root.0).expect("discover").is_empty());
    }

    #[test]
    fn startup_finds_saved_and_unsaved_sessions_without_modifying_documents() {
        let root = TestDir::new();
        let canonical = root.0.join("canonical.rhfx");
        fs::write(&canonical, b"existing canonical bytes").expect("canonical");
        let saved = RecoverySession::create(&root.0, "Saved", Some(&canonical))
            .expect("saved session");
        let unsaved = RecoverySession::create(&root.0, "Unsaved", None)
            .expect("unsaved session");
        write_recovery_generation(saved.directory(), &project()).expect("saved recovery");
        write_recovery_generation(unsaved.directory(), &project()).expect("unsaved recovery");
        let before = fs::read(&canonical).expect("canonical bytes");
        let candidates = discover_recovery_candidates(&root.0).expect("scan");
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|candidate| {
            candidate.metadata.session_id == saved.metadata().session_id
                && candidate.canonical_modified.is_some()
                && candidate.current_modified.is_some()
        }));
        assert!(candidates.iter().any(|candidate| {
            candidate.metadata.session_id == unsaved.metadata().session_id
                && candidate.metadata.canonical_project_path.is_none()
        }));
        assert_eq!(fs::read(&canonical).expect("canonical"), before);
    }

    #[test]
    fn invalid_metadata_and_previous_only_generation_are_handled_individually() {
        let root = TestDir::new();
        let valid = RecoverySession::create(&root.0, "Valid", None).expect("valid session");
        write_recovery_generation(valid.directory(), &project()).expect("current");
        fs::rename(
            valid.directory().join("current.rhfx"),
            valid.directory().join("previous.rhfx"),
        )
        .expect("previous only");
        let invalid = root.0.join("unknown-id");
        fs::create_dir(&invalid).expect("invalid directory");
        fs::write(invalid.join("session.json"), b"invalid").expect("invalid metadata");
        fs::write(invalid.join("current.rhfx"), b"invalid").expect("invalid snapshot");
        let found = discover_recovery_candidates(&root.0).expect("scan");
        assert_eq!(found.len(), 1);
        assert!(found[0].current_modified.is_none());
        assert!(found[0].previous_modified.is_some());
        assert_eq!(found[0].metadata.session_id, valid.metadata().session_id);
    }
}
