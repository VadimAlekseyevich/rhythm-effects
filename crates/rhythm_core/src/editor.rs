use crate::{
    animation::{AnimationInvariantError, Keyframe},
    domain::Vec2,
    ids::{KeyframeId, ObjectId},
    project::{Object, Project, ProjectValidationError},
    time::TempoMap,
};

#[derive(Debug, Clone, PartialEq)]
pub enum EditError {
    InvalidProject(ProjectValidationError),
    AnimationInvariant(AnimationInvariantError),
    ObjectNotFound(ObjectId),
    DuplicateObjectId(ObjectId),
    KeyframeNotFound(KeyframeId),
    InvalidValue(&'static str),
    HistoryInvariant(&'static str),
}

impl From<ProjectValidationError> for EditError {
    fn from(value: ProjectValidationError) -> Self {
        Self::InvalidProject(value)
    }
}

impl From<AnimationInvariantError> for EditError {
    fn from(value: AnimationInvariantError) -> Self {
        Self::AnimationInvariant(value)
    }
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

#[derive(Debug, Default)]
struct History {
    entries: Vec<HistoryEntry>,
    cursor: usize,
}

impl History {
    fn push(&mut self, entry: HistoryEntry) {
        self.entries.truncate(self.cursor);
        self.entries.push(entry);
        self.cursor = self.entries.len();
    }

    fn undo_entry(&self) -> Option<&HistoryEntry> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.entries.get(index))
    }

    fn redo_entry(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.cursor)
    }

    fn move_undo(&mut self) {
        self.cursor -= 1;
    }

    fn move_redo(&mut self) {
        self.cursor += 1;
    }
}

#[derive(Debug)]
pub struct ProjectEditor {
    project: Project,
    history: History,
}

impl ProjectEditor {
    pub fn new(project: Project) -> Result<Self, EditError> {
        project.validate().map_err(EditError::InvalidProject)?;
        Ok(Self {
            project,
            history: History::default(),
        })
    }

    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub fn history_len(&self) -> usize {
        self.history.entries.len()
    }

    #[must_use]
    pub const fn history_cursor(&self) -> usize {
        self.history.cursor
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.history.cursor > 0
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.history.cursor < self.history.entries.len()
    }

    pub fn execute(&mut self, command: EditCommand) -> Result<bool, EditError> {
        let Some(entry) = self.apply_command(command)? else {
            return Ok(false);
        };
        self.history.push(entry);
        Ok(true)
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        let Some(entry) = self.history.undo_entry().cloned() else {
            return Ok(false);
        };

        self.apply_history(&entry, HistoryDirection::Undo)?;
        self.history.move_undo();
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        let Some(entry) = self.history.redo_entry().cloned() else {
            return Ok(false);
        };

        self.apply_history(&entry, HistoryDirection::Redo)?;
        self.history.move_redo();
        Ok(true)
    }

    #[must_use]
    pub fn into_project(self) -> Project {
        self.project
    }

    fn apply_command(&mut self, command: EditCommand) -> Result<Option<HistoryEntry>, EditError> {
        match command {
            EditCommand::AddObject { object } => {
                if self
                    .project
                    .composition
                    .objects
                    .iter()
                    .any(|existing| existing.id == object.id)
                {
                    return Err(EditError::DuplicateObjectId(object.id));
                }

                let index = self.project.composition.objects.len();
                self.project.composition.objects.push(object.clone());
                if let Err(error) = self.project.validate() {
                    self.project.composition.objects.pop();
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(HistoryEntry {
                    label: "Add Object".to_owned(),
                    payload: HistoryPayload::ObjectInserted { index, object },
                }))
            }
            EditCommand::DeleteObject { object_id } => {
                let index = self.object_index(object_id)?;
                let object = self.project.composition.objects.remove(index);
                if let Err(error) = self.project.validate() {
                    self.project.composition.objects.insert(index, object);
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(HistoryEntry {
                    label: "Delete Object".to_owned(),
                    payload: HistoryPayload::ObjectDeleted { index, object },
                }))
            }
            EditCommand::RenameObject { object_id, name } => {
                let index = self.object_index(object_id)?;
                let before = self.project.composition.objects[index].name.clone();
                if before == name {
                    return Ok(None);
                }

                self.project.composition.objects[index].name = name.clone();
                Ok(Some(HistoryEntry {
                    label: "Rename Object".to_owned(),
                    payload: HistoryPayload::ObjectRenamed {
                        object_id,
                        before,
                        after: name,
                    },
                }))
            }
            EditCommand::SetPositionBase { object_id, value } => {
                let index = self.object_index(object_id)?;
                let before = *self.project.composition.objects[index]
                    .transform
                    .position
                    .base_value();
                if before == value {
                    return Ok(None);
                }

                *self.project.composition.objects[index]
                    .transform
                    .position
                    .base_value_mut() = value;
                Ok(Some(HistoryEntry {
                    label: "Edit Position".to_owned(),
                    payload: HistoryPayload::PositionBaseChanged {
                        object_id,
                        before,
                        after: value,
                    },
                }))
            }
            EditCommand::SetOpacityBase { object_id, value } => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(EditError::InvalidValue("opacity"));
                }

                let index = self.object_index(object_id)?;
                let before = *self.project.composition.objects[index]
                    .transform
                    .opacity
                    .base_value();
                if before == value {
                    return Ok(None);
                }

                *self.project.composition.objects[index]
                    .transform
                    .opacity
                    .base_value_mut() = value;
                Ok(Some(HistoryEntry {
                    label: "Edit Opacity".to_owned(),
                    payload: HistoryPayload::OpacityBaseChanged {
                        object_id,
                        before,
                        after: value,
                    },
                }))
            }
            EditCommand::AddOpacityKeyframe {
                object_id,
                keyframe,
            } => {
                if !keyframe.value.is_finite() || !(0.0..=1.0).contains(&keyframe.value) {
                    return Err(EditError::InvalidValue("opacity keyframe"));
                }

                let index = self.object_index(object_id)?;
                self.project.composition.objects[index]
                    .transform
                    .opacity
                    .insert_keyframe(keyframe.clone())?;

                if let Err(error) = self.project.validate() {
                    let _ = self.project.composition.objects[index]
                        .transform
                        .opacity
                        .remove_keyframe_by_id(keyframe.id);
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(HistoryEntry {
                    label: "Add Opacity Keyframe".to_owned(),
                    payload: HistoryPayload::OpacityKeyframeInserted {
                        object_id,
                        keyframe,
                    },
                }))
            }
            EditCommand::SetTempoMap { tempo_map } => {
                let before = self.project.tempo_map.clone();
                if before == tempo_map {
                    return Ok(None);
                }

                self.project.tempo_map = tempo_map.clone();
                if let Err(error) = self.project.validate() {
                    self.project.tempo_map = before;
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(HistoryEntry {
                    label: "Change Tempo".to_owned(),
                    payload: HistoryPayload::TempoMapChanged {
                        before,
                        after: tempo_map,
                    },
                }))
            }
        }
    }

    fn apply_history(
        &mut self,
        entry: &HistoryEntry,
        direction: HistoryDirection,
    ) -> Result<(), EditError> {
        match (&entry.payload, direction) {
            (HistoryPayload::ObjectInserted { index, object }, HistoryDirection::Undo)
            | (HistoryPayload::ObjectDeleted { index, object }, HistoryDirection::Redo) => {
                let removed = self
                    .project
                    .composition
                    .objects
                    .get(*index)
                    .ok_or(EditError::HistoryInvariant("object index missing"))?;
                if removed.id != object.id {
                    return Err(EditError::HistoryInvariant("object identity mismatch"));
                }
                self.project.composition.objects.remove(*index);
            }
            (HistoryPayload::ObjectInserted { index, object }, HistoryDirection::Redo)
            | (HistoryPayload::ObjectDeleted { index, object }, HistoryDirection::Undo) => {
                if *index > self.project.composition.objects.len() {
                    return Err(EditError::HistoryInvariant("object restore index invalid"));
                }
                self.project
                    .composition
                    .objects
                    .insert(*index, object.clone());
            }
            (
                HistoryPayload::ObjectRenamed {
                    object_id,
                    before,
                    after,
                },
                direction,
            ) => {
                let value = direction.pick(before, after);
                let index = self.object_index(*object_id)?;
                self.project.composition.objects[index].name = value.clone();
            }
            (
                HistoryPayload::PositionBaseChanged {
                    object_id,
                    before,
                    after,
                },
                direction,
            ) => {
                let value = *direction.pick(before, after);
                let index = self.object_index(*object_id)?;
                *self.project.composition.objects[index]
                    .transform
                    .position
                    .base_value_mut() = value;
            }
            (
                HistoryPayload::OpacityBaseChanged {
                    object_id,
                    before,
                    after,
                },
                direction,
            ) => {
                let value = *direction.pick(before, after);
                let index = self.object_index(*object_id)?;
                *self.project.composition.objects[index]
                    .transform
                    .opacity
                    .base_value_mut() = value;
            }
            (
                HistoryPayload::OpacityKeyframeInserted {
                    object_id,
                    keyframe,
                },
                HistoryDirection::Undo,
            ) => {
                let index = self.object_index(*object_id)?;
                self.project.composition.objects[index]
                    .transform
                    .opacity
                    .remove_keyframe_by_id(keyframe.id)
                    .ok_or(EditError::KeyframeNotFound(keyframe.id))?;
            }
            (
                HistoryPayload::OpacityKeyframeInserted {
                    object_id,
                    keyframe,
                },
                HistoryDirection::Redo,
            ) => {
                let index = self.object_index(*object_id)?;
                self.project.composition.objects[index]
                    .transform
                    .opacity
                    .insert_keyframe(keyframe.clone())?;
            }
            (HistoryPayload::TempoMapChanged { before, after }, direction) => {
                self.project.tempo_map = direction.pick(before, after).clone();
            }
        }

        self.project.validate().map_err(EditError::InvalidProject)
    }

    fn object_index(&self, object_id: ObjectId) -> Result<usize, EditError> {
        self.project
            .composition
            .objects
            .iter()
            .position(|object| object.id == object_id)
            .ok_or(EditError::ObjectNotFound(object_id))
    }
}

#[derive(Debug, Clone, Copy)]
enum HistoryDirection {
    Undo,
    Redo,
}

impl HistoryDirection {
    fn pick<'a, T>(self, before: &'a T, after: &'a T) -> &'a T {
        match self {
            Self::Undo => before,
            Self::Redo => after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EditCommand, ProjectEditor};
    use crate::{
        animation::Animated,
        domain::{LinearRgba, Vec2},
        ids::ObjectId,
        project::{
            Object, ObjectContent, Project, ProjectSettings, RectangleObject, TransformAnimation,
        },
        time::{GridOffsetNs, TempoMap},
    };

    fn object(id: u64, name: &str) -> Object {
        Object {
            id: ObjectId::new(id).expect("object id"),
            name: name.to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(0.0, 0.0).expect("finite position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("finite scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("finite anchor")),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 50.0).expect("finite size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        }
    }

    fn editor_with_object() -> ProjectEditor {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(object(1, "A"));
        project.next_entity_id = 2;
        ProjectEditor::new(project).expect("valid project")
    }

    #[test]
    fn undo_redo_and_new_edit_truncate_redo_branch() {
        let mut editor = editor_with_object();

        assert_eq!(
            editor.execute(EditCommand::RenameObject {
                object_id: ObjectId::new(1).expect("object id"),
                name: "B".to_owned(),
            }),
            Ok(true)
        );
        assert_eq!(
            editor.execute(EditCommand::SetOpacityBase {
                object_id: ObjectId::new(1).expect("object id"),
                value: 0.5,
            }),
            Ok(true)
        );
        assert_eq!(editor.history_len(), 2);
        assert_eq!(editor.history_cursor(), 2);

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(editor.history_cursor(), 1);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .opacity
                .base_value(),
            1.0
        );

        assert_eq!(editor.redo(), Ok(true));
        assert_eq!(editor.history_cursor(), 2);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .opacity
                .base_value(),
            0.5
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            editor.execute(EditCommand::RenameObject {
                object_id: ObjectId::new(1).expect("object id"),
                name: "C".to_owned(),
            }),
            Ok(true)
        );
        assert!(!editor.can_redo());
        assert_eq!(editor.history_len(), 2);
        assert_eq!(editor.project().composition.objects[0].name, "C");
    }

    #[test]
    fn history_payloads_are_semantic_not_project_snapshots() {
        let payload = super::HistoryPayload::ObjectRenamed {
            object_id: ObjectId::new(1).expect("object id"),
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
        let command = EditCommand::RenameObject {
            object_id: ObjectId::new(1).expect("object id"),
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
