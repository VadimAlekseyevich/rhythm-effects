//! Immutable semantic export-job capture, independent of later editor edits.
//! Runtime assets and GPU state are prepared in subsequent export steps.

use rhythm_core::{
    editor::{ProjectEditor, ProjectRevision},
    project::{Project, ProjectValidationError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportSnapshotError {
    ActiveEditorTransaction,
    InvalidProject(ProjectValidationError),
}

/// Export owns an independent, validated Project clone. No mutable Project
/// accessor is exposed, and no editor/history lock crosses the job boundary.
#[derive(Debug, Clone)]
pub struct ExportJobSnapshot {
    project: Project,
    captured_revision: ProjectRevision,
}

impl ExportJobSnapshot {
    /// A drag/keyframe edit must be committed before capture; otherwise an
    /// export could contain an intermediate state without a history revision.
    pub fn capture(editor: &ProjectEditor) -> Result<Self, ExportSnapshotError> {
        if editor.has_active_transaction() {
            return Err(ExportSnapshotError::ActiveEditorTransaction);
        }
        let project = editor.project().clone();
        project
            .validate()
            .map_err(ExportSnapshotError::InvalidProject)?;
        Ok(Self {
            project,
            captured_revision: editor.current_revision(),
        })
    }

    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub const fn captured_revision(&self) -> ProjectRevision {
        self.captured_revision
    }

    /// Scene evaluation reads this captured creative state, never the live
    /// ProjectEditor or later editor revisions.
    pub fn evaluate_at(
        &self,
        time: rhythm_core::time::ProjectTimeNs,
    ) -> Result<crate::scene_eval::EvaluatedScene, crate::scene_eval::SceneEvaluationError> {
        crate::scene_eval::evaluate_scene(&self.project, time)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportJobSnapshot, ExportSnapshotError};
    use crate::reference_scenes::five_effects_reference_scene;
    use rhythm_core::{
        editor::{EditCommand, ProjectEditor},
        ids::ObjectId,
        project::{AssetKind, AssetSource},
        time::ProjectTimeNs,
    };

    #[test]
    fn capture_owns_snapshot_even_when_live_editor_changes_and_undoes() {
        let fixture = five_effects_reference_scene();
        let mut editor = ProjectEditor::new(fixture.project).expect("valid reference project");
        let initial_revision = editor.current_revision();
        let initial_dirty = editor.is_dirty();
        let original = editor.project().clone();

        let snapshot = ExportJobSnapshot::capture(&editor).expect("capture");
        let snapshot_scene = snapshot
            .evaluate_at(ProjectTimeNs::new(0))
            .expect("evaluate snapshot");
        assert_eq!(snapshot.project(), &original);
        assert_eq!(snapshot.captured_revision(), initial_revision);
        assert_eq!(editor.is_dirty(), initial_dirty);
        assert_eq!(editor.current_revision(), initial_revision);

        editor
            .execute(EditCommand::AddAsset {
                kind: AssetKind::Image,
                source: AssetSource::File {
                    path: "new-resource.png".into(),
                    relative_to_project: false,
                },
            })
            .expect("later creative edit");
        assert_ne!(editor.current_revision(), initial_revision);
        assert_eq!(editor.project().assets.len(), 1);
        assert!(editor.is_dirty());

        assert_eq!(snapshot.project(), &original);
        assert!(snapshot.project().assets.is_empty());
        assert_eq!(
            snapshot.evaluate_at(ProjectTimeNs::new(0)).expect("evaluate again"),
            snapshot_scene
        );
        assert_eq!(snapshot.captured_revision(), initial_revision);

        editor.undo().expect("undo later edit");
        assert!(editor.project().assets.is_empty());
        assert_eq!(snapshot.project(), &original);
    }

    #[test]
    fn active_uncommitted_drag_is_not_a_valid_export_boundary() {
        let fixture = five_effects_reference_scene();
        let mut editor = ProjectEditor::new(fixture.project).expect("editor");
        editor
            .begin_position_transaction(ObjectId::new(1).expect("valid ID"))
            .expect("begin drag");
        assert_eq!(
            ExportJobSnapshot::capture(&editor).expect_err("no intermediate snapshot"),
            ExportSnapshotError::ActiveEditorTransaction
        );
        assert!(editor.has_active_transaction());
        editor.cancel_transaction().expect("cancel drag");
        assert!(ExportJobSnapshot::capture(&editor).is_ok());
    }

    #[test]
    fn snapshot_clone_does_not_share_mutable_editor_state() {
        let fixture = five_effects_reference_scene();
        let mut editor = ProjectEditor::new(fixture.project).expect("editor");
        let first = ExportJobSnapshot::capture(&editor).expect("first capture");
        let independent = first.clone();
        editor
            .execute(EditCommand::AddAsset {
                kind: AssetKind::Image,
                source: AssetSource::File {
                    path: "later.png".into(),
                    relative_to_project: false,
                },
            })
            .expect("make an edit");
        let second = ExportJobSnapshot::capture(&editor).expect("later capture");
        assert_eq!(first.project(), independent.project());
        assert_ne!(first.captured_revision(), second.captured_revision());
        assert_ne!(first.project(), second.project());
    }
}
