//! Explicit recovery Restore/Discard actions. Never publish to canonical .rhfx.

use crate::{
    file_dialogs::read_project_bytes,
    recovery_discovery::{RecoveryCandidate, discover_recovery_candidates},
    recovery_session::RecoverySession,
};
use rhythm_core::{editor::ProjectEditor, serialization::prepare_recovered_project_open};
use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryGeneration {
    Current,
    Previous,
}

#[derive(Debug)]
pub enum RecoveryRestoreError {
    ActiveProjectIsDirty,
    NoValidGeneration { current: String, previous: String },
}

impl fmt::Display for RecoveryRestoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ActiveProjectIsDirty => {
                write!(
                    formatter,
                    "save or close the current dirty project before Restore"
                )
            }
            Self::NoValidGeneration { current, previous } => write!(
                formatter,
                "both recovery generations failed (current: {current}; previous: {previous})"
            ),
        }
    }
}

impl Error for RecoveryRestoreError {}

/// Parse, migrate and validate current, then previous if current is invalid.
/// The caller's project/canonical path are untouched until a complete detached
/// replacement editor has been prepared. Restored work always remains dirty.
pub fn restore_recovery_candidate(
    active: &mut ProjectEditor,
    canonical_path: &mut Option<PathBuf>,
    candidate: &RecoveryCandidate,
) -> Result<RecoveryGeneration, RecoveryRestoreError> {
    if active.is_dirty() || active.has_active_transaction() {
        return Err(RecoveryRestoreError::ActiveProjectIsDirty);
    }
    let current = load_generation(&candidate.directory.join("current.rhfx"));
    let (replacement, generation) = match current {
        Ok(replacement) => (replacement, RecoveryGeneration::Current),
        Err(current) => match load_generation(&candidate.directory.join("previous.rhfx")) {
            Ok(replacement) => (replacement, RecoveryGeneration::Previous),
            Err(previous) => {
                return Err(RecoveryRestoreError::NoValidGeneration { current, previous });
            }
        },
    };

    *active = replacement;
    *canonical_path = candidate.metadata.canonical_project_path.clone();
    Ok(generation)
}

fn load_generation(path: &Path) -> Result<ProjectEditor, String> {
    let bytes = read_project_bytes(path).map_err(|error| error.to_string())?;
    prepare_recovered_project_open(&bytes).map_err(|error| error.to_string())
}

/// Explicitly discard only the selected direct child of the recovery root.
/// Revalidate its session identity and physical directory location at action
/// time; never resolve the optional canonical path as a deletion target.
pub fn discard_recovery_candidate(root: &Path, candidate: &RecoveryCandidate) -> io::Result<()> {
    let root_real = fs::canonicalize(root)?;
    let directory = &candidate.directory;
    if fs::symlink_metadata(directory)?.file_type().is_symlink()
        || fs::canonicalize(directory)?
            .parent()
            .is_none_or(|parent| parent != root_real)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "recovery candidate is not a direct session directory",
        ));
    }
    let metadata = RecoverySession::read_metadata(directory)?;
    if metadata.session_id != candidate.metadata.session_id {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "selected recovery session identity has changed",
        ));
    }
    fs::remove_dir_all(directory)
}

const STALE_RECOVERY_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// Conservative stale policy: never erase unsaved-only sessions or a recovery
/// newer than its canonical file, even if it is older than fourteen days.
/// Keep the active editing session regardless of timestamps.
#[must_use]
pub fn is_stale_cleanup_candidate(
    candidate: &RecoveryCandidate,
    now: SystemTime,
    active_session_id: Option<&str>,
) -> bool {
    if active_session_id == Some(candidate.metadata.session_id.as_str()) {
        return false;
    }
    let (Some(snapshot), Some(canonical)) = (
        candidate.latest_recovery_modified(),
        candidate.canonical_modified,
    ) else {
        return false;
    };
    canonical >= snapshot
        && now
            .duration_since(snapshot)
            .is_ok_and(|elapsed| elapsed >= STALE_RECOVERY_AGE)
}

/// Startup maintenance reuses validated discovery and the same identity/path
/// checks as explicit Discard; only safe-to-delete stale sessions qualify.
/// Never remove active, unsaved-only or canonical-older recovery records.
pub fn cleanup_stale_recovery(
    root: &Path,
    now: SystemTime,
    active_session_id: Option<&str>,
) -> io::Result<usize> {
    let mut removed = 0;
    for candidate in discover_recovery_candidates(root)? {
        if is_stale_cleanup_candidate(&candidate, now, active_session_id) {
            discard_recovery_candidate(root, &candidate)?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::{
        RecoveryGeneration, RecoveryRestoreError, cleanup_stale_recovery,
        discard_recovery_candidate, is_stale_cleanup_candidate, restore_recovery_candidate,
    };
    use crate::{
        recovery_autosave::write_recovery_generation,
        recovery_discovery::discover_recovery_candidates, recovery_session::RecoverySession,
    };
    use rhythm_core::{
        editor::{EditCommand, ProjectEditor},
        project::{AssetKind, AssetSource, Project, ProjectSettings},
        time::{GridOffsetNs, TempoMap},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, SystemTime},
    };

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);
    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "rhythm-recovery-actions-{}-{}",
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
    fn project(name: &str) -> Project {
        Project::new(
            name,
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        )
    }
    fn active_editor() -> ProjectEditor {
        ProjectEditor::new(project("Active")).expect("active")
    }

    #[test]
    fn restores_current_as_dirty_without_writing_canonical_file() {
        let root = TestDir::new();
        let canonical = root.0.join("saved.rhfx");
        fs::write(&canonical, b"known-good canonical bytes").expect("canonical");
        let session = RecoverySession::create(&root.0, "Saved", Some(&canonical)).expect("session");
        write_recovery_generation(session.directory(), &project("Recovered")).expect("write");
        let candidate = discover_recovery_candidates(&root.0)
            .expect("scan")
            .remove(0);
        let mut active = active_editor();
        let mut path = Some(root.0.join("active.rhfx"));

        assert_eq!(
            restore_recovery_candidate(&mut active, &mut path, &candidate).expect("restore"),
            RecoveryGeneration::Current
        );
        assert_eq!(active.project().metadata.name, "Recovered");
        assert!(active.is_dirty());
        assert_eq!(active.saved_revision(), None);
        assert_eq!(active.history_len(), 0);
        assert_eq!(path.as_deref(), Some(canonical.as_path()));
        assert_eq!(
            fs::read(&canonical).expect("untouched file"),
            b"known-good canonical bytes"
        );
    }

    #[test]
    fn corrupt_current_falls_back_to_previous_without_touching_canonical() {
        let root = TestDir::new();
        let session = RecoverySession::create(&root.0, "Unsaved", None).expect("session");
        write_recovery_generation(session.directory(), &project("First")).expect("first");
        write_recovery_generation(session.directory(), &project("Second")).expect("second");
        fs::write(session.directory().join("current.rhfx"), b"corrupt").expect("corrupt current");
        let candidate = discover_recovery_candidates(&root.0)
            .expect("scan")
            .remove(0);
        let mut active = active_editor();
        let mut path = None;
        assert_eq!(
            restore_recovery_candidate(&mut active, &mut path, &candidate).expect("fallback"),
            RecoveryGeneration::Previous
        );
        assert_eq!(active.project().metadata.name, "First");
        assert!(active.is_dirty());
        assert!(path.is_none());
    }

    #[test]
    fn invalid_both_generations_or_dirty_active_project_preserves_active_state() {
        let root = TestDir::new();
        let session = RecoverySession::create(&root.0, "Unsaved", None).expect("session");
        fs::write(session.directory().join("current.rhfx"), b"bad").expect("current");
        fs::write(session.directory().join("previous.rhfx"), b"also bad").expect("previous");
        let candidate = discover_recovery_candidates(&root.0)
            .expect("scan")
            .remove(0);
        let mut active = active_editor();
        let canonical = root.0.join("active.rhfx");
        let mut path = Some(canonical.clone());
        assert!(matches!(
            restore_recovery_candidate(&mut active, &mut path, &candidate),
            Err(RecoveryRestoreError::NoValidGeneration { .. })
        ));
        assert_eq!(active.project().metadata.name, "Active");
        assert!(!active.is_dirty());
        assert_eq!(path.as_deref(), Some(canonical.as_path()));

        active
            .execute(EditCommand::AddAsset {
                kind: AssetKind::Image,
                source: AssetSource::File {
                    path: "missing.png".into(),
                    relative_to_project: false,
                },
            })
            .expect("dirty edit");
        assert!(matches!(
            restore_recovery_candidate(&mut active, &mut path, &candidate),
            Err(RecoveryRestoreError::ActiveProjectIsDirty)
        ));
        assert!(active.is_dirty());
        assert_eq!(active.project().metadata.name, "Active");
        assert_eq!(path.as_deref(), Some(canonical.as_path()));
    }

    #[test]
    fn fourteen_day_stale_cleanup_requires_canonical_at_least_as_new_as_recovery() {
        let root = TestDir::new();
        let canonical = root.0.join("saved.rhfx");
        let session = RecoverySession::create(&root.0, "Safe to remove", Some(&canonical))
            .expect("session");
        write_recovery_generation(session.directory(), &project("Saved")).expect("recovery");
        fs::write(&canonical, b"newer canonical").expect("canonical");
        let candidate = discover_recovery_candidates(&root.0).expect("scan").remove(0);
        let timestamp = candidate.latest_recovery_modified().expect("snapshot time");
        let stale = timestamp + Duration::from_secs(15 * 24 * 60 * 60);
        let recent = timestamp + Duration::from_secs(13 * 24 * 60 * 60);
        assert!(!is_stale_cleanup_candidate(&candidate, recent, None));
        assert!(!is_stale_cleanup_candidate(
            &candidate,
            stale,
            Some(session.metadata().session_id.as_str())
        ));
        assert!(is_stale_cleanup_candidate(&candidate, stale, None));
        assert_eq!(cleanup_stale_recovery(&root.0, stale, None).expect("cleanup"), 1);
        assert!(!session.directory().exists());
        assert_eq!(fs::read(&canonical).expect("keep canonical"), b"newer canonical");
    }

    #[test]
    fn old_unsaved_only_and_canonical_older_recovery_are_never_cleaned() {
        let root = TestDir::new();
        let unsaved = RecoverySession::create(&root.0, "Unsaved", None).expect("unsaved");
        write_recovery_generation(unsaved.directory(), &project("Unsaved")).expect("unsaved data");
        let canonical = root.0.join("canonical.rhfx");
        fs::write(&canonical, b"old canonical").expect("canonical");
        let older = std::time::UNIX_EPOCH + Duration::from_secs(86400);
        fs::File::options()
            .write(true)
            .open(&canonical)
            .expect("open canonical")
            .set_times(fs::FileTimes::new().set_modified(older))
            .expect("set canonical timestamp");
        let saved = RecoverySession::create(&root.0, "Recoverable", Some(&canonical))
            .expect("saved");
        write_recovery_generation(saved.directory(), &project("Later edits"))
            .expect("recovery newer than canonical");
        let candidates = discover_recovery_candidates(&root.0).expect("scan");
        assert_eq!(candidates.len(), 2);
        let now = SystemTime::now() + Duration::from_secs(15 * 24 * 60 * 60);
        assert!(candidates
            .iter()
            .all(|candidate| !is_stale_cleanup_candidate(candidate, now, None)));
        assert_eq!(cleanup_stale_recovery(&root.0, now, None).expect("cleanup"), 0);
        assert!(unsaved.directory().exists());
        assert!(saved.directory().exists());
        assert_eq!(fs::read(&canonical).expect("canonical"), b"old canonical");
    }

    #[test]
    fn discard_deletes_only_selected_recovery_and_never_canonical() {
        let root = TestDir::new();
        let canonical = root.0.join("canonical.rhfx");
        fs::write(&canonical, b"canonical").expect("canonical");
        let first = RecoverySession::create(&root.0, "First", Some(&canonical)).expect("first");
        let second = RecoverySession::create(&root.0, "Second", None).expect("second");
        write_recovery_generation(first.directory(), &project("First")).expect("write first");
        write_recovery_generation(second.directory(), &project("Second")).expect("write second");
        let candidates = discover_recovery_candidates(&root.0).expect("scan");
        let selected = candidates
            .iter()
            .find(|candidate| candidate.metadata.session_id == first.metadata().session_id)
            .expect("selected");
        discard_recovery_candidate(&root.0, selected).expect("discard");
        assert!(!first.directory().exists());
        assert!(second.directory().exists());
        assert_eq!(
            fs::read(&canonical).expect("canonical untouched"),
            b"canonical"
        );
    }
}
