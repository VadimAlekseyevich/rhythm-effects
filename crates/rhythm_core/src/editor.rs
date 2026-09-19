use crate::{
    animation::{AnimationInvariantError, Keyframe},
    domain::Vec2,
    ids::{KeyframeId, ObjectId},
    project::{Object, Project, ProjectValidationError},
    time::TempoMap,
};

pub const HISTORY_CAPACITY: usize = 500;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectRevision(u64);

impl ProjectRevision {
    pub const INITIAL: Self = Self(0);

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub label: String,
    pub payload: HistoryPayload,
    pub before_revision: ProjectRevision,
    pub after_revision: ProjectRevision,
}

#[derive(Debug, Clone, PartialEq)]
struct PendingHistoryEntry {
    label: String,
    payload: HistoryPayload,
}

impl PendingHistoryEntry {
    fn new(label: &str, payload: HistoryPayload) -> Self {
        Self {
            label: label.to_owned(),
            payload,
        }
    }
}

#[derive(Debug)]
struct History {
    entries: Vec<HistoryEntry>,
    cursor: usize,
    next_revision: u64,
    current_revision: ProjectRevision,
    saved_revision: Option<ProjectRevision>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            next_revision: 1,
            current_revision: ProjectRevision::INITIAL,
            saved_revision: Some(ProjectRevision::INITIAL),
        }
    }
}

impl History {
    fn push(&mut self, pending: PendingHistoryEntry) {
        self.entries.truncate(self.cursor);
        self.clear_unreachable_saved_revision();

        let before_revision = self.current_revision;
        let after_revision = ProjectRevision(self.next_revision);
        self.next_revision = self
            .next_revision
            .checked_add(1)
            .expect("session revision space exhausted");

        self.entries.push(HistoryEntry {
            label: pending.label,
            payload: pending.payload,
            before_revision,
            after_revision,
        });
        self.cursor = self.entries.len();
        self.current_revision = after_revision;

        if self.entries.len() > HISTORY_CAPACITY {
            let remove_count = self.entries.len() - HISTORY_CAPACITY;
            self.entries.drain(0..remove_count);
            self.cursor = self.entries.len();
            self.clear_unreachable_saved_revision();
        }
    }

    fn undo_entry(&self) -> Option<&HistoryEntry> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.entries.get(index))
    }

    fn redo_entry(&self) -> Option<&HistoryEntry> {
        self.entries.get(self.cursor)
    }

    fn move_undo(&mut self, entry: &HistoryEntry) {
        self.cursor -= 1;
        self.current_revision = entry.before_revision;
    }

    fn move_redo(&mut self, entry: &HistoryEntry) {
        self.cursor += 1;
        self.current_revision = entry.after_revision;
    }

    fn mark_saved(&mut self) {
        self.saved_revision = Some(self.current_revision);
    }

    fn is_dirty(&self) -> bool {
        self.saved_revision != Some(self.current_revision)
    }

    fn clear_unreachable_saved_revision(&mut self) {
        if let Some(saved_revision) = self.saved_revision
            && !self.is_revision_reachable(saved_revision)
        {
            self.saved_revision = None;
        }
    }

    fn is_revision_reachable(&self, revision: ProjectRevision) -> bool {
        if self.entries.is_empty() {
            return revision == self.current_revision;
        }

        revision == self.entries[0].before_revision
            || self
                .entries
                .iter()
                .any(|entry| entry.after_revision == revision)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ActiveTransaction {
    ObjectPosition {
        object_id: ObjectId,
        before: Vec2,
    },
}

#[derive(Debug)]
pub struct ProjectEditor {
    project: Project,
    history: History,
    transaction: Option<ActiveTransaction>,
}

impl ProjectEditor {
    pub fn new(project: Project) -> Result<Self, EditError> {
        project.validate().map_err(EditError::InvalidProject)?;
        Ok(Self {
            project,
            history: History::default(),
            transaction: None,
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

    #[must_use]
    pub const fn current_revision(&self) -> ProjectRevision {
        self.history.current_revision
    }

    #[must_use]
    pub const fn saved_revision(&self) -> Option<ProjectRevision> {
        self.history.saved_revision
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty()
    }

    pub fn mark_saved(&mut self) {
        self.history.mark_saved();
    }

    pub fn execute(&mut self, command: EditCommand) -> Result<bool, EditError> {
        if self.transaction.is_some() {
            return Err(EditError::HistoryInvariant(
                "cannot execute command while transaction is active",
            ));
        }

        let Some(entry) = self.apply_command(command)? else {
            return Ok(false);
        };
        self.history.push(entry);
        Ok(true)
    }

    pub fn begin_position_transaction(&mut self, object_id: ObjectId) -> Result<(), EditError> {
        if self.transaction.is_some() {
            return Err(EditError::HistoryInvariant("transaction already active"));
        }

        let index = self.object_index(object_id)?;
        let before = *self.project.composition.objects[index]
            .transform
            .position
            .base_value();

        self.transaction = Some(ActiveTransaction::ObjectPosition { object_id, before });
        Ok(())
    }

    pub fn update_position_transaction(&mut self, value: Vec2) -> Result<(), EditError> {
        let object_id = match self.transaction {
            Some(ActiveTransaction::ObjectPosition { object_id, .. }) => object_id,
            None => return Err(EditError::HistoryInvariant("no active position transaction")),
        };

        let index = self.object_index(object_id)?;
        *self.project.composition.objects[index]
            .transform
            .position
            .base_value_mut() = value;
        Ok(())
    }

    pub fn commit_transaction(&mut self) -> Result<bool, EditError> {
        let Some(transaction) = self.transaction.take() else {
            return Ok(false);
        };

        match transaction {
            ActiveTransaction::ObjectPosition { object_id, before } => {
                let index = self.object_index(object_id)?;
                let after = *self.project.composition.objects[index]
                    .transform
                    .position
                    .base_value();

                if before == after {
                    return Ok(false);
                }

                self.history.push(PendingHistoryEntry::new(
                    "Move Object",
                    HistoryPayload::PositionBaseChanged {
                        object_id,
                        before,
                        after,
                    },
                ));
                Ok(true)
            }
        }
    }

    pub fn cancel_transaction(&mut self) -> Result<bool, EditError> {
        let Some(transaction) = self.transaction.take() else {
            return Ok(false);
        };

        match transaction {
            ActiveTransaction::ObjectPosition { object_id, before } => {
                let index = self.object_index(object_id)?;
                *self.project.composition.objects[index]
                    .transform
                    .position
                    .base_value_mut() = before;
            }
        }

        Ok(true)
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        if self.transaction.is_some() {
            self.cancel_transaction()?;
        }

        let Some(entry) = self.history.undo_entry().cloned() else {
            return Ok(false);
        };

        self.apply_history(&entry, HistoryDirection::Undo)?;
        self.history.move_undo(&entry);
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        if self.transaction.is_some() {
            self.cancel_transaction()?;
        }

        let Some(entry) = self.history.redo_entry().cloned() else {
            return Ok(false);
        };

        self.apply_history(&entry, HistoryDirection::Redo)?;
        self.history.move_redo(&entry);
        Ok(true)
    }

    #[must_use]
    pub fn into_project(self) -> Project {
        self.project
    }

    fn apply_command(
        &mut self,
        command: EditCommand,
    ) -> Result<Option<PendingHistoryEntry>, EditError> {
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

                Ok(Some(PendingHistoryEntry::new(
                    "Add Object",
                    HistoryPayload::ObjectInserted { index, object },
                )))
            }
            EditCommand::DeleteObject { object_id } => {
                let index = self.object_index(object_id)?;
                let object = self.project.composition.objects.remove(index);
                if let Err(error) = self.project.validate() {
                    self.project.composition.objects.insert(index, object);
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Delete Object",
                    HistoryPayload::ObjectDeleted { index, object },
                )))
            }
            EditCommand::RenameObject { object_id, name } => {
                let index = self.object_index(object_id)?;
                let before = self.project.composition.objects[index].name.clone();
                if before == name {
                    return Ok(None);
                }

                self.project.composition.objects[index].name = name.clone();
                Ok(Some(PendingHistoryEntry::new(
                    "Rename Object",
                    HistoryPayload::ObjectRenamed {
                        object_id,
                        before,
                        after: name,
                    },
                )))
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
                Ok(Some(PendingHistoryEntry::new(
                    "Edit Position",
                    HistoryPayload::PositionBaseChanged {
                        object_id,
                        before,
                        after: value,
                    },
                )))
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
                Ok(Some(PendingHistoryEntry::new(
                    "Edit Opacity",
                    HistoryPayload::OpacityBaseChanged {
                        object_id,
                        before,
                        after: value,
                    },
                )))
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

                Ok(Some(PendingHistoryEntry::new(
                    "Add Opacity Keyframe",
                    HistoryPayload::OpacityKeyframeInserted {
                        object_id,
                        keyframe,
                    },
                )))
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

                Ok(Some(PendingHistoryEntry::new(
                    "Change Tempo",
                    HistoryPayload::TempoMapChanged {
                        before,
                        after: tempo_map,
                    },
                )))
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
    use super::{EditCommand, HISTORY_CAPACITY, HistoryEntry, HistoryPayload, ProjectEditor};
    use crate::{
        animation::{Animated, Interpolation, Keyframe},
        domain::{LinearRgba, Vec2},
        ids::{KeyframeId, ObjectId},
        project::{
            Object, ObjectContent, Project, ProjectSettings, RectangleObject, TransformAnimation,
        },
        time::{
            BpmMicros, GridOffsetNs, MusicalTick, TempoMap, TimeSignature,
        },
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
    fn dirty_state_tracks_saved_revision_across_undo_redo() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        assert!(!editor.is_dirty());
        assert_eq!(editor.current_revision(), super::ProjectRevision::INITIAL);

        editor
            .execute(EditCommand::RenameObject {
                object_id,
                name: "B".to_owned(),
            })
            .expect("rename");
        assert!(editor.is_dirty());
        assert_eq!(editor.current_revision().get(), 1);

        editor.mark_saved();
        assert!(!editor.is_dirty());

        editor
            .execute(EditCommand::SetOpacityBase {
                object_id,
                value: 0.5,
            })
            .expect("opacity");
        assert!(editor.is_dirty());
        assert_eq!(editor.current_revision().get(), 2);

        editor.undo().expect("undo");
        assert!(!editor.is_dirty());
        assert_eq!(editor.current_revision().get(), 1);

        editor.redo().expect("redo");
        assert!(editor.is_dirty());
        assert_eq!(editor.current_revision().get(), 2);
    }

    #[test]
    fn saved_revision_in_truncated_redo_branch_becomes_unreachable() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        editor
            .execute(EditCommand::RenameObject {
                object_id,
                name: "B".to_owned(),
            })
            .expect("rename");
        editor
            .execute(EditCommand::SetOpacityBase {
                object_id,
                value: 0.5,
            })
            .expect("opacity");
        editor.mark_saved();

        editor.undo().expect("undo to revision 1");
        editor
            .execute(EditCommand::RenameObject {
                object_id,
                name: "C".to_owned(),
            })
            .expect("branch edit");

        assert_eq!(editor.saved_revision(), None);
        assert!(editor.is_dirty());

        editor.undo().expect("undo branch edit");
        assert_eq!(editor.current_revision().get(), 1);
        assert!(editor.is_dirty());
    }

    #[test]
    fn history_capacity_keeps_only_latest_500_entries() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        for index in 0..(HISTORY_CAPACITY + 25) {
            editor
                .execute(EditCommand::RenameObject {
                    object_id,
                    name: format!("Object {index}"),
                })
                .expect("rename");
        }

        assert_eq!(editor.history_len(), HISTORY_CAPACITY);
        assert_eq!(editor.history_cursor(), HISTORY_CAPACITY);
        assert_eq!(editor.current_revision().get(), 525);

        for _ in 0..HISTORY_CAPACITY {
            assert_eq!(editor.undo(), Ok(true));
        }
        assert_eq!(editor.undo(), Ok(false));
        assert_eq!(editor.current_revision().get(), 25);
    }

    #[test]
    fn one_drag_transaction_creates_one_history_entry() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        editor
            .begin_position_transaction(object_id)
            .expect("begin transaction");
        for x in [10.0, 20.0, 30.0, 40.0] {
            editor
                .update_position_transaction(Vec2::new(x, x * 0.5).expect("finite position"))
                .expect("preview");
        }

        assert_eq!(editor.history_len(), 0);
        assert_eq!(editor.commit_transaction(), Ok(true));
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.current_revision().get(), 1);
        assert_eq!(
            editor.project().composition.objects[0]
                .transform
                .position
                .base_value()
                .x(),
            40.0
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            editor.project().composition.objects[0]
                .transform
                .position
                .base_value()
                .x(),
            0.0
        );
    }

    #[test]
    fn bpm_and_grid_offset_edits_preserve_musical_ticks() {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::with_initial_tempo(
                GridOffsetNs::new(0),
                BpmMicros::new(120_000_000).expect("valid BPM"),
                TimeSignature::default(),
            ),
        );
        let mut animated_object = object(1, "Animated");
        animated_object.transform.opacity = Animated::with_keyframes(
            1.0,
            vec![
                Keyframe::new(
                    KeyframeId::new(2).expect("keyframe id"),
                    MusicalTick::new(240),
                    0.25,
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(3).expect("keyframe id"),
                    MusicalTick::new(960),
                    0.75,
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("unique keyframes");
        project.composition.objects.push(animated_object);
        project.next_entity_id = 4;

        let mut editor = ProjectEditor::new(project).expect("valid project");
        let original_ticks: Vec<_> = editor.project().composition.objects[0]
            .transform
            .opacity
            .keyframes()
            .iter()
            .map(|keyframe| keyframe.tick)
            .collect();

        editor
            .execute(EditCommand::SetTempoMap {
                tempo_map: TempoMap::with_initial_tempo(
                    GridOffsetNs::new(0),
                    BpmMicros::new(150_000_000).expect("valid BPM"),
                    TimeSignature::default(),
                ),
            })
            .expect("BPM edit");

        editor
            .execute(EditCommand::SetTempoMap {
                tempo_map: TempoMap::with_initial_tempo(
                    GridOffsetNs::new(350_000_000),
                    BpmMicros::new(150_000_000).expect("valid BPM"),
                    TimeSignature::default(),
                ),
            })
            .expect("grid offset edit");

        let after_ticks: Vec<_> = editor.project().composition.objects[0]
            .transform
            .opacity
            .keyframes()
            .iter()
            .map(|keyframe| keyframe.tick)
            .collect();

        assert_eq!(after_ticks, original_ticks);

        editor.undo().expect("undo offset");
        editor.undo().expect("undo BPM");
        let restored_ticks: Vec<_> = editor.project().composition.objects[0]
            .transform
            .opacity
            .keyframes()
            .iter()
            .map(|keyframe| keyframe.tick)
            .collect();
        assert_eq!(restored_ticks, original_ticks);
    }

    #[test]
    fn cancelled_position_transaction_restores_before_state() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        editor
            .begin_position_transaction(object_id)
            .expect("begin transaction");
        editor
            .update_position_transaction(Vec2::new(50.0, 25.0).expect("finite position"))
            .expect("preview transaction");

        assert_eq!(
            editor.project().composition.objects[0]
                .transform
                .position
                .base_value()
                .x(),
            50.0
        );
        assert_eq!(editor.cancel_transaction(), Ok(true));
        assert_eq!(
            editor.project().composition.objects[0]
                .transform
                .position
                .base_value()
                .x(),
            0.0
        );
        assert_eq!(editor.history_len(), 0);
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
        let payload = HistoryPayload::ObjectRenamed {
            object_id: ObjectId::new(1).expect("object id"),
            before: "A".to_owned(),
            after: "B".to_owned(),
        };
        let entry = HistoryEntry {
            label: "Rename Object".to_owned(),
            payload,
            before_revision: super::ProjectRevision::INITIAL,
            after_revision: super::ProjectRevision(1),
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
