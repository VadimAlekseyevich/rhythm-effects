//! Dirty-project recovery cadence and single-flight filesystem snapshots.
//! A committed Project clone crosses the thread boundary; editor state never does.

use rhythm_core::{
    editor::{ProjectEditor, ProjectRevision},
    project::Project,
    project_save::stage_project_file_save,
    serialization::{
        MAX_PROJECT_FILE_BYTES, migrate_project_file_to_current, parse_project_file_v1,
        validate_project_file_v1_candidate,
    },
};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
use tracing::warn;

const RECOVERY_INTERVAL: Duration = Duration::from_secs(30);
const TRANSACTION_SETTLE: Duration = Duration::from_secs(1);
const FAILED_WRITE_RETRY: Duration = Duration::from_secs(5);
const CURRENT_FILE: &str = "current.rhfx";
const PREVIOUS_FILE: &str = "previous.rhfx";

#[derive(Debug, Default)]
struct RecoverySchedule {
    last_success: Option<Instant>,
    last_attempt: Option<Instant>,
    was_editing: bool,
    edit_finished_at: Option<Instant>,
}

impl RecoverySchedule {
    fn observe(&mut self, now: Instant, active_transaction: bool) {
        if active_transaction {
            self.was_editing = true;
        } else if self.was_editing {
            self.was_editing = false;
            self.edit_finished_at = Some(now);
        }
    }

    fn settled(&self, now: Instant, active_transaction: bool) -> bool {
        !active_transaction
            && self
                .edit_finished_at
                .is_none_or(|ended| now.duration_since(ended) >= TRANSACTION_SETTLE)
    }

    fn due(&self, now: Instant, dirty: bool, active_transaction: bool) -> bool {
        dirty
            && self.settled(now, active_transaction)
            && self
                .last_success
                .is_none_or(|success| now.duration_since(success) >= RECOVERY_INTERVAL)
            && self
                .last_attempt
                .is_none_or(|attempt| now.duration_since(attempt) >= FAILED_WRITE_RETRY)
    }

    fn mark_dispatched(&mut self, now: Instant) {
        self.last_attempt = Some(now);
    }

    fn mark_success(&mut self, now: Instant) {
        self.last_success = Some(now);
    }
}

struct RecoveryResult {
    revision: ProjectRevision,
    outcome: io::Result<()>,
}

/// Poll on the app event loop. It only clones a committed project on a due
/// write/new revision and never synchronously performs recovery disk I/O.
pub struct RecoveryAutosave {
    directory: PathBuf,
    schedule: RecoverySchedule,
    finished_tx: Sender<RecoveryResult>,
    finished_rx: Receiver<RecoveryResult>,
    in_flight: Option<ProjectRevision>,
    queued: Option<(ProjectRevision, Project)>,
    pending_clean: bool,
}

impl RecoveryAutosave {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        let (finished_tx, finished_rx) = mpsc::channel();
        Self {
            directory,
            schedule: RecoverySchedule::default(),
            finished_tx,
            finished_rx,
            in_flight: None,
            queued: None,
            pending_clean: false,
        }
    }

    /// Call only after explicit publication succeeded. If an old recovery
    /// worker is still writing, cleanup waits until that worker finishes.
    pub fn request_clean_after_save(&mut self, editor: &ProjectEditor) -> io::Result<()> {
        if editor.is_dirty() || editor.has_active_transaction() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot clean recovery while the project is dirty or editing",
            ));
        }
        self.queued = None;
        self.pending_clean = true;
        if self.in_flight.is_none() {
            self.remove_obsolete_generations()?;
        }
        Ok(())
    }

    /// Clean exit may wait for an already-running recovery worker to finish
    /// before removing obsolete generations. Dirty-project close does not
    /// invoke this method and keeps recovery intact.
    pub fn finish_clean_close(&mut self, editor: &ProjectEditor) -> io::Result<()> {
        if editor.is_dirty() || editor.has_active_transaction() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "dirty project recovery must remain on close",
            ));
        }
        self.queued = None;
        self.pending_clean = true;
        if self.in_flight.is_some() {
            let completion = self.finished_rx.recv().map_err(io::Error::other)?;
            self.in_flight = None;
            if let Err(error) = completion.outcome {
                warn!(%error, "in-flight recovery failed before clean close");
            }
        }
        self.remove_obsolete_generations()
    }

    /// Only invoke after the user has explicitly confirmed Don't Save.
    /// Wait for a pre-existing worker so a late write cannot recreate recovery
    /// after deletion; remove the entire active-session directory, not any
    /// other session or the optional canonical project file.
    pub fn discard_after_confirmation(&mut self) -> io::Result<()> {
        self.queued = None;
        if self.in_flight.is_some() {
            let completion = self.finished_rx.recv().map_err(io::Error::other)?;
            self.in_flight = None;
            if let Err(error) = completion.outcome {
                warn!(%error, "in-flight recovery failed before confirmed discard");
            }
        }
        fs::remove_dir_all(&self.directory)?;
        self.pending_clean = false;
        Ok(())
    }

    fn remove_obsolete_generations(&mut self) -> io::Result<()> {
        for filename in [CURRENT_FILE, PREVIOUS_FILE] {
            let path = self.directory.join(filename);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        self.pending_clean = false;
        self.schedule.last_success = None;
        self.schedule.last_attempt = None;
        Ok(())
    }

    pub fn tick(&mut self, now: Instant, editor: &ProjectEditor) {
        let active = editor.has_active_transaction();
        let dirty = editor.is_dirty();
        self.schedule.observe(now, active);

        while let Ok(completion) = self.finished_rx.try_recv() {
            self.in_flight = None;
            match completion.outcome {
                Ok(()) => self.schedule.mark_success(now),
                Err(error) => warn!(
                    %error,
                    revision = completion.revision.get(),
                    "recovery write failed; previous successful generation preserved"
                ),
            }
        }

        if !dirty {
            self.queued = None;
            if self.pending_clean
                && self.in_flight.is_none()
                && let Err(error) = self.remove_obsolete_generations()
            {
                warn!(%error, "cannot remove obsolete recovery after successful Save");
            }
            return;
        }
        // New creative edits supersede an earlier clean-save cleanup request.
        self.pending_clean = false;
        if active {
            return;
        }

        if let Some(revision) = self.in_flight {
            if editor.current_revision() != revision
                && self.queued.as_ref().is_none_or(|(queued_revision, _)| {
                    *queued_revision != editor.current_revision()
                })
            {
                // Overwrite pending work; only the newest committed snapshot
                // needs writing after the currently running worker finishes.
                self.queued = Some((editor.current_revision(), editor.project().clone()));
            }
            return;
        }

        if !self.schedule.settled(now, active) {
            return;
        }
        if let Some((revision, project)) = self.queued.take() {
            // Capture the actual newest revision if another edit arrived since
            // the prior worker finished and the queued snapshot was produced.
            if revision == editor.current_revision() {
                self.dispatch(now, revision, project);
            } else {
                self.dispatch(now, editor.current_revision(), editor.project().clone());
            }
        } else if self.schedule.due(now, dirty, active) {
            self.dispatch(now, editor.current_revision(), editor.project().clone());
        }
    }

    fn dispatch(&mut self, now: Instant, revision: ProjectRevision, project: Project) {
        let directory = self.directory.clone();
        let sender = self.finished_tx.clone();
        self.schedule.mark_dispatched(now);
        match thread::Builder::new()
            .name("rhythm-recovery-write".into())
            .spawn(move || {
                let outcome = write_recovery_generation(&directory, &project);
                let _ = sender.send(RecoveryResult { revision, outcome });
            }) {
            Ok(_) => self.in_flight = Some(revision),
            Err(error) => warn!(%error, "unable to start recovery write"),
        }
    }
}

/// Publish new current before replacing previous. A failed new stage/publish
/// never deletes a valid previous; if current was corrupt, do not rotate it
/// over a known-good previous generation.
pub fn write_recovery_generation(directory: &Path, project: &Project) -> io::Result<()> {
    let current = directory.join(CURRENT_FILE);
    let previous = directory.join(PREVIOUS_FILE);
    let new_current = stage_project_file_save(project, &current)
        .map_err(io::Error::other)?
        .flush_and_close()
        .map_err(io::Error::other)?;

    let previous_stage = if let Some(old) = valid_prior_project(&current)? {
        Some(
            stage_project_file_save(&old, &previous)
                .map_err(io::Error::other)?
                .flush_and_close()
                .map_err(io::Error::other)?,
        )
    } else {
        None
    };

    new_current.publish()?;
    if let Some(stage) = previous_stage {
        stage.publish()?;
    }
    Ok(())
}

fn valid_prior_project(current: &Path) -> io::Result<Option<Project>> {
    let file = match File::open(current) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PROJECT_FILE_BYTES {
        return Ok(None);
    }
    let Some(project) = parse_project_file_v1(&bytes)
        .ok()
        .and_then(|file| migrate_project_file_to_current(file).ok())
        .and_then(|file| validate_project_file_v1_candidate(file).ok())
    else {
        return Ok(None);
    };
    Ok(Some(project.project))
}

#[cfg(test)]
mod tests {
    use super::{
        RECOVERY_INTERVAL, RecoveryAutosave, RecoverySchedule, TRANSACTION_SETTLE,
        write_recovery_generation,
    };
    use crate::{
        recovery_actions::{RecoveryGeneration, restore_recovery_candidate},
        recovery_discovery::discover_recovery_candidates,
        recovery_session::RecoverySession,
    };
    use rhythm_core::{
        editor::{EditCommand, ProjectEditor},
        project::{AssetKind, AssetSource, Project, ProjectSettings},
        serialization::parse_project_file_v1,
        time::{GridOffsetNs, TempoMap},
    };
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };
    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "rhythm-recovery-autosave-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).expect("test directory");
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

    #[test]
    fn dirty_interval_is_30_seconds_from_success_and_retries_are_bounded() {
        let start = Instant::now();
        let mut schedule = RecoverySchedule::default();
        schedule.observe(start, false);
        assert!(!schedule.due(start, false, false));
        assert!(schedule.due(start, true, false));
        schedule.mark_dispatched(start);
        assert!(!schedule.due(start + Duration::from_secs(4), true, false));
        assert!(schedule.due(start + Duration::from_secs(5), true, false));
        schedule.mark_success(start);
        assert!(!schedule.due(
            start + RECOVERY_INTERVAL - Duration::from_millis(1),
            true,
            false
        ));
        assert!(schedule.due(start + RECOVERY_INTERVAL, true, false));
    }

    #[test]
    fn active_transaction_defers_snapshot_until_one_second_after_end() {
        let start = Instant::now();
        let mut schedule = RecoverySchedule::default();
        schedule.observe(start, true);
        assert!(!schedule.due(start + Duration::from_secs(40), true, true));
        let finished = start + Duration::from_secs(41);
        schedule.observe(finished, false);
        assert!(!schedule.due(finished, true, false));
        assert!(!schedule.due(
            finished + TRANSACTION_SETTLE - Duration::from_millis(1),
            true,
            false
        ));
        assert!(schedule.due(finished + TRANSACTION_SETTLE, true, false));
    }

    #[test]
    fn newer_pending_committed_revision_replaces_older_while_write_is_in_flight() {
        let root = TestDir::new();
        let mut editor = ProjectEditor::new(project("Initial")).expect("valid editor");
        let mut coordinator = RecoveryAutosave::new(root.0.clone());
        coordinator.in_flight = Some(editor.current_revision());
        for path in ["a.png", "b.png", "c.png"] {
            editor
                .execute(EditCommand::AddAsset {
                    kind: AssetKind::Image,
                    source: AssetSource::File {
                        path: path.into(),
                        relative_to_project: false,
                    },
                })
                .expect("committed edit");
            coordinator.tick(Instant::now(), &editor);
        }
        assert_eq!(
            coordinator.queued.as_ref().map(|(revision, _)| *revision),
            Some(editor.current_revision())
        );
        assert_eq!(
            coordinator
                .queued
                .as_ref()
                .map(|(_, project)| project.assets.len()),
            Some(3)
        );
        assert!(coordinator.in_flight.is_some());
        assert_eq!(fs::read_dir(&root.0).expect("no second worker").count(), 0);
    }

    #[test]
    fn two_successful_generations_keep_latest_current_and_older_previous() {
        let root = TestDir::new();
        write_recovery_generation(&root.0, &project("First")).expect("first");
        assert!(!root.0.join("previous.rhfx").exists());
        write_recovery_generation(&root.0, &project("Second")).expect("second");
        let current =
            parse_project_file_v1(&fs::read(root.0.join("current.rhfx")).expect("current"))
                .expect("parse current");
        let previous =
            parse_project_file_v1(&fs::read(root.0.join("previous.rhfx")).expect("previous"))
                .expect("parse previous");
        assert_eq!(current.project.metadata.name, "Second");
        assert_eq!(previous.project.metadata.name, "First");
        assert_eq!(fs::read_dir(&root.0).expect("only two files").count(), 2);
    }

    #[test]
    fn corrupted_current_never_overwrites_valid_previous() {
        let root = TestDir::new();
        write_recovery_generation(&root.0, &project("First")).expect("first");
        write_recovery_generation(&root.0, &project("Second")).expect("second");
        let previous_bytes = fs::read(root.0.join("previous.rhfx")).expect("previous");
        fs::write(root.0.join("current.rhfx"), b"corrupted").expect("corrupt current");
        write_recovery_generation(&root.0, &project("Third")).expect("third");
        assert_eq!(
            fs::read(root.0.join("previous.rhfx")).expect("previous intact"),
            previous_bytes
        );
        assert_eq!(
            parse_project_file_v1(&fs::read(root.0.join("current.rhfx")).expect("current"))
                .expect("parse current")
                .project
                .metadata
                .name,
            "Third"
        );
    }

    #[test]
    fn successful_save_cleans_only_recovery_generations_not_session_metadata() {
        let root = TestDir::new();
        let mut editor = ProjectEditor::new(project("Saved")).expect("editor");
        write_recovery_generation(&root.0, editor.project()).expect("first generation");
        write_recovery_generation(&root.0, editor.project()).expect("second generation");
        fs::write(root.0.join("session.json"), b"keep stable session").expect("metadata");
        let mut manager = RecoveryAutosave::new(root.0.clone());

        manager
            .request_clean_after_save(&editor)
            .expect("clean Save");
        assert!(root.0.join("session.json").exists());
        assert!(!root.0.join("current.rhfx").exists());
        assert!(!root.0.join("previous.rhfx").exists());
        editor
            .execute(EditCommand::AddAsset {
                kind: AssetKind::Image,
                source: AssetSource::File {
                    path: "later.png".into(),
                    relative_to_project: false,
                },
            })
            .expect("new edit");
        assert!(
            manager
                .schedule
                .due(Instant::now(), editor.is_dirty(), false)
        );
    }

    #[test]
    fn in_flight_generation_defers_clean_save_cleanup_until_completion() {
        let root = TestDir::new();
        let editor = ProjectEditor::new(project("Saved")).expect("editor");
        write_recovery_generation(&root.0, editor.project()).expect("generation");
        let mut manager = RecoveryAutosave::new(root.0.clone());
        manager.in_flight = Some(editor.current_revision());
        manager.request_clean_after_save(&editor).expect("request");
        assert!(root.0.join("current.rhfx").exists());
        manager
            .finished_tx
            .send(super::RecoveryResult {
                revision: editor.current_revision(),
                outcome: Ok(()),
            })
            .expect("finish");
        manager.tick(Instant::now(), &editor);
        assert!(!root.0.join("current.rhfx").exists());
        assert!(!manager.pending_clean);
    }

    #[test]
    fn dirty_project_close_rejects_cleanup_and_preserves_recovery() {
        let root = TestDir::new();
        let mut editor = ProjectEditor::new(project("Unsaved")).expect("editor");
        editor
            .execute(EditCommand::AddAsset {
                kind: AssetKind::Image,
                source: AssetSource::File {
                    path: "unsaved.png".into(),
                    relative_to_project: false,
                },
            })
            .expect("edit");
        write_recovery_generation(&root.0, editor.project()).expect("generation");
        let mut manager = RecoveryAutosave::new(root.0.clone());
        assert!(manager.finish_clean_close(&editor).is_err());
        assert!(root.0.join("current.rhfx").exists());
        assert!(manager.request_clean_after_save(&editor).is_err());
        assert!(root.0.join("current.rhfx").exists());
    }

    #[test]
    fn confirmed_dont_save_deletes_only_active_session_after_worker_finishes() {
        let root = TestDir::new();
        let active = root.0.join("active");
        let other = root.0.join("other");
        fs::create_dir(&active).expect("active dir");
        fs::create_dir(&other).expect("other dir");
        let canonical = root.0.join("canonical.rhfx");
        fs::write(&canonical, b"known-good").expect("canonical");
        write_recovery_generation(&active, &project("Dirty")).expect("active generation");
        write_recovery_generation(&other, &project("Different")).expect("other generation");
        let editor = ProjectEditor::new(project("Dirty")).expect("editor");
        let mut manager = RecoveryAutosave::new(active.clone());
        manager.in_flight = Some(editor.current_revision());
        manager
            .finished_tx
            .send(super::RecoveryResult {
                revision: editor.current_revision(),
                outcome: Ok(()),
            })
            .expect("completed writer");
        manager
            .discard_after_confirmation()
            .expect("explicit discard");
        assert!(!active.exists());
        assert!(other.join("current.rhfx").exists());
        assert_eq!(
            fs::read(&canonical).expect("canonical unchanged"),
            b"known-good"
        );
    }

    /// Runs only when explicitly launched as a separate process by the
    /// integration test. The parent kills this process without unwinding, so
    /// neither clean-close nor Drop-based cleanup runs.
    #[test]
    fn forced_termination_child() {
        if std::env::var("RHYTHM_RECOVERY_CRASH_TEST_CHILD")
            .ok()
            .as_deref()
            != Some("run") {
            return;
        }
        let directory = PathBuf::from(
            std::env::var_os("RHYTHM_RECOVERY_CRASH_TEST_DIR")
                .expect("parent supplies isolated recovery directory"),
        );
        write_recovery_generation(&directory, &project("Before crash")).expect("first write");
        write_recovery_generation(&directory, &project("Latest before crash"))
            .expect("second write");
        fs::write(directory.join(".ready"), b"ready").expect("signal fully published generations");
        loop {
            std::thread::sleep(Duration::from_secs(60));
        }
    }

    #[test]
    fn forced_process_termination_preserves_recovery_and_canonical_document() {
        let root = TestDir::new();
        let canonical = root.0.join("original.rhfx");
        fs::write(&canonical, b"known-good canonical").expect("existing document");
        let session =
            RecoverySession::create(&root.0, "Recoverable", Some(&canonical)).expect("session");
        let ready = session.directory().join(".ready");
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args([
                "--exact",
                "recovery_autosave::tests::forced_termination_child",
                "--nocapture",
            ])
            .env("RHYTHM_RECOVERY_CRASH_TEST_CHILD", "run")
            .env("RHYTHM_RECOVERY_CRASH_TEST_DIR", session.directory())
            .spawn()
            .expect("launch recovery child");
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() && Instant::now() < deadline {
            if let Some(status) = child.try_wait().expect("poll child") {
                panic!("crash child unexpectedly exited before writing: {status}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if !ready.exists() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("crash child did not publish recovery generations");
        }
        child.kill().expect("force-kill process without clean close");
        let status = child.wait().expect("reap child");
        assert!(!status.success());

        let candidate = discover_recovery_candidates(&root.0)
            .expect("recovery discovery after forced termination")
            .remove(0);
        let mut editor = ProjectEditor::new(project("Active")).expect("editor");
        let mut path = Some(canonical.clone());
        assert_eq!(
            restore_recovery_candidate(&mut editor, &mut path, &candidate).expect("restore current"),
            RecoveryGeneration::Current
        );
        assert_eq!(editor.project().metadata.name, "Latest before crash");
        assert!(editor.is_dirty());
        assert_eq!(path.as_deref(), Some(canonical.as_path()));

        fs::write(session.directory().join("current.rhfx"), b"corrupt")
            .expect("simulate corrupt latest after crash");
        let mut fallback_editor = ProjectEditor::new(project("Active")).expect("editor");
        assert_eq!(
            restore_recovery_candidate(&mut fallback_editor, &mut path, &candidate)
                .expect("restore previous"),
            RecoveryGeneration::Previous
        );
        assert_eq!(fallback_editor.project().metadata.name, "Before crash");
        assert!(fallback_editor.is_dirty());
        assert_eq!(
            fs::read(&canonical).expect("canonical still known-good"),
            b"known-good canonical"
        );
    }

    #[test]
    fn failed_current_publication_preserves_previous_successful_generation() {
        let root = TestDir::new();
        write_recovery_generation(&root.0, &project("First")).expect("first");
        write_recovery_generation(&root.0, &project("Second")).expect("second");
        let previous_bytes = fs::read(root.0.join("previous.rhfx")).expect("previous");
        fs::remove_file(root.0.join("current.rhfx")).expect("remove");
        fs::create_dir(root.0.join("current.rhfx")).expect("block publish");
        assert!(write_recovery_generation(&root.0, &project("Third")).is_err());
        assert_eq!(
            fs::read(root.0.join("previous.rhfx")).expect("previous intact"),
            previous_bytes
        );
        assert!(root.0.join("current.rhfx").is_dir());
        assert_eq!(
            fs::read_dir(&root.0).expect("no temporary files").count(),
            2
        );
    }
}
