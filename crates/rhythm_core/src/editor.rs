use crate::{
    animation::Keyframe,
    domain::Vec2,
    ids::ObjectId,
    project::{Object, Project, ProjectValidationError},
    time::TempoMap,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidProject(ProjectValidationError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    AddObject {
        object: Object,
    },
    DeleteObject {
        object_id: ObjectId,
    },
    RenameObject {
        object_id: ObjectId,
        name: String,
    },
    SetPositionBase {
        object_id: ObjectId,
        value: Vec2,
    },
    SetOpacityBase {
        object_id: ObjectId,
        value: f32,
    },
    AddOpacityKeyframe {
        object_id: ObjectId,
        keyframe: Keyframe<f32>,
    },
    SetTempoMap {
        tempo_map: TempoMap,
    },
}

impl EditCommand {
    #[must_use]
    pub const fn semantic_name(&self) -> &'static str {
        match self {
            Self::AddObject { .. } => "AddObject",
            Self::DeleteObject { .. } => "DeleteObject",
            Self::RenameObject { .. } => "RenameObject",
            Self::SetPositionBase { .. } => "SetPositionBase",
            Self::SetOpacityBase { .. } => "SetOpacityBase",
            Self::AddOpacityKeyframe { .. } => "AddOpacityKeyframe",
            Self::SetTempoMap { .. } => "SetTempoMap",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryPayload {
    ObjectInserted {
        index: usize,
        object: Object,
    },
    ObjectDeleted {
        index: usize,
        object: Object,
    },
    ObjectRenamed {
        object_id: ObjectId,
        before: String,
        after: String,
    },
    PositionBaseChanged {
        object_id: ObjectId,
        before: Vec2,
        after: Vec2,
    },
    OpacityBaseChanged {
        object_id: ObjectId,
        before: f32,
        after: f32,
    },
    OpacityKeyframeInserted {
        object_id: ObjectId,
        keyframe: Keyframe<f32>,
    },
    TempoMapChanged {
        before: TempoMap,
        after: TempoMap,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub label: String,
    pub payload: HistoryPayload,
}

#[derive(Debug)]
pub struct ProjectEditor {
    project: Project,
}

impl ProjectEditor {
    pub fn new(project: Project) -> Result<Self, EditError> {
        project.validate().map_err(EditError::InvalidProject)?;
        Ok(Self { project })
    }

    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub fn into_project(self) -> Project {
        self.project
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectEditor;
    use crate::{
        project::{Project, ProjectSettings},
        time::{GridOffsetNs, TempoMap},
    };

    #[test]
    fn history_payloads_are_semantic_not_project_snapshots() {
        let payload = super::HistoryPayload::ObjectRenamed {
            object_id: crate::ids::ObjectId::new(1).expect("object id"),
            before: "A".to_owned(),
            after: "B".to_owned(),
        };
        let entry = super::HistoryEntry {
            label: "Rename Object".to_owned(),
            payload,
        };

        assert_eq!(entry.label, "Rename Object");
    }

    #[test]
    fn edit_commands_are_explicit_and_loggable() {
        let command = super::EditCommand::RenameObject {
            object_id: crate::ids::ObjectId::new(1).expect("object id"),
            name: "Beat".to_owned(),
        };

        assert_eq!(command.semantic_name(), "RenameObject");
    }

    #[test]
    fn project_editor_owns_a_valid_project_and_exposes_read_only_access() {
        let project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );

        let editor = ProjectEditor::new(project).expect("valid project");
        assert_eq!(editor.project().metadata.name, "Untitled");
        assert!(editor.project().composition.objects.is_empty());
    }
}
