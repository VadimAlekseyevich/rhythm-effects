//! Application-owned canonical path transition for Save As.

use std::path::{Path, PathBuf};

use rhythm_core::{
    editor::ProjectEditor,
    project_save::{ProjectSaveError, save_project_file_transactional},
};

/// Publish the new .rhfx document before changing the session's canonical path.
///
/// A failed serialization, flush, sync, or publication keeps the previous path
/// and the editor's prior saved revision/dirty state. Rewriting asset paths for
/// a new project directory is the separate AI-241 step.
pub fn save_project_as(
    editor: &mut ProjectEditor,
    canonical_path: &mut Option<PathBuf>,
    destination: &Path,
) -> Result<(), ProjectSaveError> {
    let published_path = save_project_file_transactional(editor, destination)?;
    *canonical_path = Some(published_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::save_project_as;
    use rhythm_core::{
        editor::{EditCommand, ProjectEditor},
        project::{AssetKind, AssetSource, Project, ProjectSettings},
        project_save::{ProjectSaveError, ProjectSaveStageError},
        serialization::parse_project_file_v1,
        time::{GridOffsetNs, TempoMap},
    };
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rhythm-save-as-{}-{}",
                std::process::id(),
                NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create isolated test directory");
            Self(path)
        }

        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn dirty_editor() -> ProjectEditor {
        let project = Project::new(
            "Save As test",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let mut editor = ProjectEditor::new(project).expect("valid editor");
        editor
            .execute(EditCommand::AddAsset {
                kind: AssetKind::Image,
                source: AssetSource::File {
                    path: "sample.png".into(),
                    relative_to_project: false,
                },
            })
            .expect("make project dirty");
        assert!(editor.is_dirty());
        editor
    }

    #[test]
    fn first_save_as_sets_canonical_path_after_valid_publication() {
        let root = TestDir::new();
        let destination = root.join("first-ритм.rhfx");
        let mut editor = dirty_editor();
        let revision = editor.current_revision();
        let expected = editor.project().clone();
        let mut canonical_path = None;

        save_project_as(&mut editor, &mut canonical_path, &destination).expect("first Save As");

        assert_eq!(canonical_path.as_deref(), Some(destination.as_path()));
        assert_eq!(editor.current_revision(), revision);
        assert_eq!(editor.saved_revision(), Some(revision));
        assert!(!editor.is_dirty());
        assert_eq!(
            parse_project_file_v1(&fs::read(&destination).expect("published bytes"))
                .expect("valid project")
                .project,
            expected
        );
        assert_eq!(fs::read_dir(&root.0).expect("list directory").count(), 1);
    }

    #[test]
    fn successful_save_as_changes_path_but_preserves_previous_document() {
        let root = TestDir::new();
        let previous = root.join("previous.rhfx");
        let destination = root.join("new.rhfx");
        let old_bytes = b"old canonical file";
        fs::write(&previous, old_bytes).expect("previous document");
        let mut canonical_path = Some(previous.clone());
        let mut editor = dirty_editor();
        let revision = editor.current_revision();
        let history_len = editor.history_len();

        save_project_as(&mut editor, &mut canonical_path, &destination).expect("Save As");

        assert_eq!(canonical_path.as_deref(), Some(destination.as_path()));
        assert_eq!(fs::read(&previous).expect("previous stays"), old_bytes);
        assert!(parse_project_file_v1(&fs::read(&destination).expect("new file")).is_ok());
        assert_eq!(editor.current_revision(), revision);
        assert_eq!(editor.saved_revision(), Some(revision));
        assert_eq!(editor.history_len(), history_len);
        assert!(!editor.is_dirty());
        assert_eq!(fs::read_dir(&root.0).expect("list directory").count(), 2);
    }

    #[test]
    fn failed_save_as_staging_keeps_old_path_file_and_dirty_revision() {
        let root = TestDir::new();
        let previous = root.join("previous.rhfx");
        let blocker = root.join("not-a-directory");
        fs::write(&previous, b"known-good").expect("previous document");
        fs::write(&blocker, b"blocker").expect("blocking parent file");
        let destination = blocker.join("new.rhfx");
        let mut canonical_path = Some(previous.clone());
        let mut editor = dirty_editor();
        let revision = editor.current_revision();
        let saved = editor.saved_revision();
        let project = editor.project().clone();

        assert!(matches!(
            save_project_as(&mut editor, &mut canonical_path, &destination),
            Err(ProjectSaveError::Stage(ProjectSaveStageError::Io(_)))
        ));

        assert_eq!(canonical_path.as_deref(), Some(previous.as_path()));
        assert_eq!(editor.current_revision(), revision);
        assert_eq!(editor.saved_revision(), saved);
        assert_eq!(editor.project(), &project);
        assert!(editor.is_dirty());
        assert_eq!(fs::read(&previous).expect("old document"), b"known-good");
        assert_eq!(fs::read(&blocker).expect("parent blocker"), b"blocker");
        assert_eq!(fs::read_dir(&root.0).expect("list directory").count(), 2);
    }

    #[test]
    fn failed_save_as_publish_keeps_old_path_and_cleans_sibling_temp() {
        let root = TestDir::new();
        let previous = root.join("previous.rhfx");
        let destination = root.join("occupied.rhfx");
        fs::write(&previous, b"known-good").expect("previous document");
        fs::create_dir(&destination).expect("directory cannot be replaced with a file");
        let mut canonical_path = Some(previous.clone());
        let mut editor = dirty_editor();
        let revision = editor.current_revision();
        let saved = editor.saved_revision();

        assert!(matches!(
            save_project_as(&mut editor, &mut canonical_path, &destination),
            Err(ProjectSaveError::Publish(_))
        ));

        assert_eq!(canonical_path.as_deref(), Some(previous.as_path()));
        assert_eq!(editor.current_revision(), revision);
        assert_eq!(editor.saved_revision(), saved);
        assert!(editor.is_dirty());
        assert_eq!(fs::read(&previous).expect("old document"), b"known-good");
        assert!(destination.is_dir());
        assert_eq!(fs::read_dir(&root.0).expect("list directory").count(), 2);
    }

    #[test]
    fn failed_first_save_as_does_not_assign_canonical_path() {
        let root = TestDir::new();
        let blocker = root.join("not-a-directory");
        fs::write(&blocker, b"blocker").expect("blocking parent file");
        let mut canonical_path = None;
        let mut editor = dirty_editor();
        let saved = editor.saved_revision();

        assert!(
            save_project_as(&mut editor, &mut canonical_path, &blocker.join("new.rhfx")).is_err()
        );
        assert!(canonical_path.is_none());
        assert_eq!(editor.saved_revision(), saved);
        assert!(editor.is_dirty());
        assert_eq!(fs::read_dir(&root.0).expect("list directory").count(), 1);
    }
}
