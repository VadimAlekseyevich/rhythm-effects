use std::collections::HashSet;

use crate::{
    animation::{AnimationInvariantError, Interpolation, Keyframe},
    domain::Vec2,
    ids::{EntityIdAllocator, IdAllocationError, KeyframeId, ObjectId},
    project::{Object, Project, ProjectValidationError},
    property::{
        AnimatableProperty, PropertyAccessError, PropertyKeyframe, PropertyValue,
        insert_property_keyframe, locate_property_keyframe, property_base_value,
        property_keyframe_by_id, property_keyframe_count, remove_property_keyframe_by_id,
        set_property_base_value, set_property_keyframe_interpolation, set_property_keyframe_value,
    },
    time::{MusicalTick, TempoMap},
};

pub const HISTORY_CAPACITY: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub enum EditError {
    InvalidProject(ProjectValidationError),
    AnimationInvariant(AnimationInvariantError),
    PropertyAccess(PropertyAccessError),
    IdAllocation(IdAllocationError),
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

impl From<PropertyAccessError> for EditError {
    fn from(value: PropertyAccessError) -> Self {
        Self::PropertyAccess(value)
    }
}

impl From<IdAllocationError> for EditError {
    fn from(value: IdAllocationError) -> Self {
        Self::IdAllocation(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyKeyframeDraft {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub tick: MusicalTick,
    pub value: PropertyValue,
    pub interpolation: crate::animation::Interpolation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyKeyframeInsertRecord {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub keyframe: PropertyKeyframe,
    pub replaced: Option<PropertyKeyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyKeyframeMove {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub keyframe_id: KeyframeId,
    pub target_tick: MusicalTick,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyKeyframeMoveRecord {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub before: PropertyKeyframe,
    pub after: PropertyKeyframe,
    pub replaced: Option<PropertyKeyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyKeyframeDeleteRecord {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub keyframe: PropertyKeyframe,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyBaseChange {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub before: PropertyValue,
    pub after: PropertyValue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyKeyframeInterpolationRecord {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub keyframe_id: KeyframeId,
    pub before: Interpolation,
    pub after: Interpolation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyKeyframeValueRecord {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub before: Option<PropertyKeyframe>,
    pub after: PropertyKeyframe,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    AddObject {
        object: Box<Object>,
    },
    DeleteObject {
        object_id: ObjectId,
    },
    RenameObject {
        object_id: ObjectId,
        name: String,
    },
    SetObjectVisible {
        object_id: ObjectId,
        visible: bool,
    },
    SetObjectLocked {
        object_id: ObjectId,
        locked: bool,
    },
    SetPropertyBase {
        object_id: ObjectId,
        property: AnimatableProperty,
        value: PropertyValue,
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
    InsertPropertyKeyframe {
        object_id: ObjectId,
        property: AnimatableProperty,
        keyframe: PropertyKeyframe,
    },
    RemovePropertyKeyframe {
        object_id: ObjectId,
        property: AnimatableProperty,
        keyframe: PropertyKeyframe,
    },
    InsertPropertyKeyframes {
        entries: Vec<(ObjectId, AnimatableProperty, PropertyKeyframe)>,
    },
    DeletePropertyKeyframes {
        keyframe_ids: Vec<KeyframeId>,
    },
    MovePropertyKeyframes {
        moves: Vec<PropertyKeyframeMove>,
    },
    SetPropertyKeyframeInterpolations {
        keyframe_ids: Vec<KeyframeId>,
        interpolation: Interpolation,
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
            Self::SetObjectVisible { .. } => "SetObjectVisible",
            Self::SetObjectLocked { .. } => "SetObjectLocked",
            Self::SetPropertyBase { .. } => "SetPropertyBase",
            Self::SetPositionBase { .. } => "SetPositionBase",
            Self::SetOpacityBase { .. } => "SetOpacityBase",
            Self::AddOpacityKeyframe { .. } => "AddOpacityKeyframe",
            Self::InsertPropertyKeyframe { .. } => "InsertPropertyKeyframe",
            Self::RemovePropertyKeyframe { .. } => "RemovePropertyKeyframe",
            Self::InsertPropertyKeyframes { .. } => "InsertPropertyKeyframes",
            Self::DeletePropertyKeyframes { .. } => "DeletePropertyKeyframes",
            Self::MovePropertyKeyframes { .. } => "MovePropertyKeyframes",
            Self::SetPropertyKeyframeInterpolations { .. } => "SetPropertyKeyframeInterpolations",
            Self::SetTempoMap { .. } => "SetTempoMap",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryPayload {
    ObjectInserted {
        index: usize,
        object: Box<Object>,
    },
    ObjectDeleted {
        index: usize,
        object: Box<Object>,
    },
    ObjectRenamed {
        object_id: ObjectId,
        before: String,
        after: String,
    },
    ObjectVisibilityChanged {
        object_id: ObjectId,
        before: bool,
        after: bool,
    },
    ObjectLockedChanged {
        object_id: ObjectId,
        before: bool,
        after: bool,
    },
    PropertyBaseChanged {
        change: PropertyBaseChange,
    },
    PositionBaseChanged {
        object_id: ObjectId,
        before: Vec2,
        after: Vec2,
    },
    PositionsBaseChanged {
        changes: Vec<(ObjectId, Vec2, Vec2)>,
    },
    DirectPositionsChanged {
        base_changes: Vec<(ObjectId, Vec2, Vec2)>,
        keyframe_changes: Vec<PropertyKeyframeValueRecord>,
    },
    ScaleBaseChanged {
        object_id: ObjectId,
        before: Vec2,
        after: Vec2,
    },
    RotationBaseChanged {
        object_id: ObjectId,
        before: f32,
        after: f32,
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
    PropertyKeyframeInserted {
        object_id: ObjectId,
        property: AnimatableProperty,
        keyframe: PropertyKeyframe,
    },
    PropertyKeyframeRemoved {
        object_id: ObjectId,
        property: AnimatableProperty,
        keyframe: PropertyKeyframe,
        base_before: Option<PropertyValue>,
        base_after: Option<PropertyValue>,
    },
    PropertyKeyframesInserted {
        records: Vec<PropertyKeyframeInsertRecord>,
    },
    PropertyKeyframesDeleted {
        records: Vec<PropertyKeyframeDeleteRecord>,
        base_changes: Vec<PropertyBaseChange>,
    },
    PropertyKeyframesMoved {
        records: Vec<PropertyKeyframeMoveRecord>,
    },
    PropertyKeyframeInterpolationsChanged {
        records: Vec<PropertyKeyframeInterpolationRecord>,
    },
    PropertyKeyframeValueChanged {
        object_id: ObjectId,
        property: AnimatableProperty,
        before: Option<PropertyKeyframe>,
        after: PropertyKeyframe,
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
enum DirectPositionTarget {
    Base {
        object_id: ObjectId,
        before: Vec2,
    },
    Keyframe {
        object_id: ObjectId,
        keyframe_id: KeyframeId,
        before: Option<PropertyKeyframe>,
        initial: Vec2,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum ActiveTransaction {
    Position {
        object_id: ObjectId,
        before: Vec2,
    },
    Positions {
        before: Vec<(ObjectId, Vec2)>,
    },
    DirectPositions {
        targets: Vec<DirectPositionTarget>,
        next_entity_id_before: u64,
    },
    Scale {
        object_id: ObjectId,
        before: Vec2,
    },
    Rotation {
        object_id: ObjectId,
        before: f32,
    },
    PropertyKeyframeValue {
        object_id: ObjectId,
        property: AnimatableProperty,
        keyframe_id: KeyframeId,
        before: Option<PropertyKeyframe>,
        initial_value: PropertyValue,
        inserted_next_entity_id_before: Option<u64>,
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

    pub fn next_keyframe_id(&self) -> Result<KeyframeId, EditError> {
        let mut allocator = EntityIdAllocator::new(self.project.next_entity_id)?;
        Ok(allocator.allocate_keyframe()?)
    }

    pub fn create_first_property_keyframe(
        &mut self,
        object_id: ObjectId,
        property: AnimatableProperty,
        tick: MusicalTick,
    ) -> Result<Option<KeyframeId>, EditError> {
        if property_keyframe_count(&self.project, object_id, property)? != 0 {
            return Ok(None);
        }

        let value = property_base_value(&self.project, object_id, property)?;
        self.create_property_keyframe(object_id, property, tick, value)
    }

    pub fn create_property_keyframe(
        &mut self,
        object_id: ObjectId,
        property: AnimatableProperty,
        tick: MusicalTick,
        value: PropertyValue,
    ) -> Result<Option<KeyframeId>, EditError> {
        let keyframe_id = self.next_keyframe_id()?;
        let keyframe = PropertyKeyframe {
            id: keyframe_id,
            tick,
            value,
            interpolation: crate::animation::Interpolation::Linear,
        };

        let changed = self.execute(EditCommand::InsertPropertyKeyframe {
            object_id,
            property,
            keyframe,
        })?;

        Ok(changed.then_some(keyframe_id))
    }

    pub fn remove_property_keyframe(
        &mut self,
        object_id: ObjectId,
        property: AnimatableProperty,
        keyframe: PropertyKeyframe,
    ) -> Result<bool, EditError> {
        self.execute(EditCommand::RemovePropertyKeyframe {
            object_id,
            property,
            keyframe,
        })
    }

    pub fn insert_property_keyframes(
        &mut self,
        drafts: Vec<PropertyKeyframeDraft>,
    ) -> Result<Vec<KeyframeId>, EditError> {
        if drafts.is_empty() {
            return Ok(Vec::new());
        }

        let mut allocator = EntityIdAllocator::new(self.project.next_entity_id)?;
        let mut entries = Vec::with_capacity(drafts.len());
        let mut ids = Vec::with_capacity(drafts.len());

        for draft in drafts {
            let id = allocator.allocate_keyframe()?;
            ids.push(id);
            entries.push((
                draft.object_id,
                draft.property,
                PropertyKeyframe {
                    id,
                    tick: draft.tick,
                    value: draft.value,
                    interpolation: draft.interpolation,
                },
            ));
        }

        let changed = self.execute(EditCommand::InsertPropertyKeyframes { entries })?;
        Ok(if changed { ids } else { Vec::new() })
    }

    pub fn delete_property_keyframes(
        &mut self,
        keyframe_ids: Vec<KeyframeId>,
    ) -> Result<bool, EditError> {
        self.execute(EditCommand::DeletePropertyKeyframes { keyframe_ids })
    }

    pub fn move_property_keyframe(
        &mut self,
        object_id: ObjectId,
        property: AnimatableProperty,
        keyframe_id: KeyframeId,
        target_tick: MusicalTick,
    ) -> Result<bool, EditError> {
        self.move_property_keyframes(vec![PropertyKeyframeMove {
            object_id,
            property,
            keyframe_id,
            target_tick,
        }])
    }

    pub fn move_property_keyframes(
        &mut self,
        moves: Vec<PropertyKeyframeMove>,
    ) -> Result<bool, EditError> {
        self.execute(EditCommand::MovePropertyKeyframes { moves })
    }

    pub fn set_property_keyframe_interpolations(
        &mut self,
        keyframe_ids: Vec<KeyframeId>,
        interpolation: Interpolation,
    ) -> Result<bool, EditError> {
        self.execute(EditCommand::SetPropertyKeyframeInterpolations {
            keyframe_ids,
            interpolation,
        })
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

    pub fn begin_property_keyframe_value_transaction(
        &mut self,
        object_id: ObjectId,
        property: AnimatableProperty,
        tick: MusicalTick,
        initial_value: PropertyValue,
    ) -> Result<KeyframeId, EditError> {
        if self.transaction.is_some() {
            return Err(EditError::HistoryInvariant("transaction already active"));
        }

        let existing =
            crate::property::property_keyframe_at_tick(&self.project, object_id, property, tick)?;
        let (keyframe_id, inserted_next_entity_id_before) = if let Some(existing) = existing {
            (existing.id, None)
        } else {
            let keyframe_id = self.next_keyframe_id()?;
            let keyframe = PropertyKeyframe {
                id: keyframe_id,
                tick,
                value: initial_value,
                interpolation: Interpolation::Linear,
            };
            let previous_next_entity_id = self.project.next_entity_id;
            let next_entity_id = previous_next_entity_id
                .checked_add(1)
                .ok_or(EditError::IdAllocation(IdAllocationError::Exhausted))?;

            insert_property_keyframe(&mut self.project, object_id, property, keyframe)?;
            self.project.next_entity_id = next_entity_id;
            if let Err(error) = self.project.validate() {
                let _ = remove_property_keyframe_by_id(
                    &mut self.project,
                    object_id,
                    property,
                    keyframe_id,
                );
                self.project.next_entity_id = previous_next_entity_id;
                return Err(EditError::InvalidProject(error));
            }
            (keyframe_id, Some(previous_next_entity_id))
        };

        self.transaction = Some(ActiveTransaction::PropertyKeyframeValue {
            object_id,
            property,
            keyframe_id,
            before: existing,
            initial_value,
            inserted_next_entity_id_before,
        });
        Ok(keyframe_id)
    }

    pub fn update_property_keyframe_value_transaction(
        &mut self,
        value: PropertyValue,
    ) -> Result<(), EditError> {
        let (object_id, property, keyframe_id) = match self.transaction {
            Some(ActiveTransaction::PropertyKeyframeValue {
                object_id,
                property,
                keyframe_id,
                ..
            }) => (object_id, property, keyframe_id),
            _ => {
                return Err(EditError::HistoryInvariant(
                    "no active property keyframe value transaction",
                ));
            }
        };

        set_property_keyframe_value(&mut self.project, object_id, property, keyframe_id, value)?
            .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
        Ok(())
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

        self.transaction = Some(ActiveTransaction::Position { object_id, before });
        Ok(())
    }

    pub fn update_position_transaction(&mut self, value: Vec2) -> Result<(), EditError> {
        let object_id = match self.transaction {
            Some(ActiveTransaction::Position { object_id, .. }) => object_id,
            _ => {
                return Err(EditError::HistoryInvariant(
                    "no active position transaction",
                ));
            }
        };

        let index = self.object_index(object_id)?;
        *self.project.composition.objects[index]
            .transform
            .position
            .base_value_mut() = value;
        Ok(())
    }

    pub fn begin_multi_position_transaction(
        &mut self,
        object_ids: &[ObjectId],
    ) -> Result<(), EditError> {
        if self.transaction.is_some() {
            return Err(EditError::HistoryInvariant("transaction already active"));
        }
        if object_ids.is_empty() {
            return Err(EditError::HistoryInvariant(
                "multi-position transaction requires objects",
            ));
        }

        let mut object_ids = object_ids.to_vec();
        object_ids.sort_by_key(|object_id| object_id.get());
        object_ids.dedup();

        let mut before = Vec::with_capacity(object_ids.len());
        for object_id in object_ids {
            let index = self.object_index(object_id)?;
            before.push((
                object_id,
                *self.project.composition.objects[index]
                    .transform
                    .position
                    .base_value(),
            ));
        }

        self.transaction = Some(ActiveTransaction::Positions { before });
        Ok(())
    }

    pub fn update_multi_position_transaction(&mut self, delta: Vec2) -> Result<(), EditError> {
        let before = match &self.transaction {
            Some(ActiveTransaction::Positions { before }) => before.clone(),
            _ => {
                return Err(EditError::HistoryInvariant(
                    "no active multi-position transaction",
                ));
            }
        };

        for (object_id, position) in before {
            let index = self.object_index(object_id)?;
            *self.project.composition.objects[index]
                .transform
                .position
                .base_value_mut() = Vec2::new(position.x() + delta.x(), position.y() + delta.y())
                .map_err(|_| EditError::InvalidValue("position"))?;
        }
        Ok(())
    }

    pub fn begin_direct_multi_position_transaction(
        &mut self,
        object_ids: &[ObjectId],
        tick: MusicalTick,
        evaluated_positions: &[(ObjectId, Vec2)],
    ) -> Result<(), EditError> {
        if self.transaction.is_some() {
            return Err(EditError::HistoryInvariant("transaction already active"));
        }
        if object_ids.len() < 2 {
            return Err(EditError::HistoryInvariant(
                "direct multi-position transaction requires multiple objects",
            ));
        }

        let next_entity_id_before = self.project.next_entity_id;
        let mut targets = Vec::with_capacity(object_ids.len());
        let mut ids = object_ids.to_vec();
        ids.sort_by_key(|object_id| object_id.get());
        ids.dedup();

        for object_id in ids {
            if property_keyframe_count(&self.project, object_id, AnimatableProperty::Position)? == 0
            {
                let index = self.object_index(object_id)?;
                targets.push(DirectPositionTarget::Base {
                    object_id,
                    before: *self.project.composition.objects[index]
                        .transform
                        .position
                        .base_value(),
                });
                continue;
            }

            let initial = evaluated_positions
                .iter()
                .find_map(|(evaluated_id, position)| {
                    (*evaluated_id == object_id).then_some(*position)
                })
                .ok_or(EditError::HistoryInvariant(
                    "missing evaluated position for animated multi-move member",
                ))?;
            let before = crate::property::property_keyframe_at_tick(
                &self.project,
                object_id,
                AnimatableProperty::Position,
                tick,
            )?;
            let keyframe_id = if let Some(before) = before {
                before.id
            } else {
                let keyframe_id = self.next_keyframe_id()?;
                let keyframe = PropertyKeyframe {
                    id: keyframe_id,
                    tick,
                    value: PropertyValue::Vec2(initial),
                    interpolation: Interpolation::Linear,
                };
                insert_property_keyframe(
                    &mut self.project,
                    object_id,
                    AnimatableProperty::Position,
                    keyframe,
                )?;
                self.project.next_entity_id = self
                    .project
                    .next_entity_id
                    .checked_add(1)
                    .ok_or(EditError::IdAllocation(IdAllocationError::Exhausted))?;
                keyframe_id
            };
            targets.push(DirectPositionTarget::Keyframe {
                object_id,
                keyframe_id,
                before,
                initial,
            });
        }

        self.transaction = Some(ActiveTransaction::DirectPositions {
            targets,
            next_entity_id_before,
        });
        Ok(())
    }

    pub fn update_direct_multi_position_transaction(
        &mut self,
        delta: Vec2,
    ) -> Result<(), EditError> {
        let targets = match &self.transaction {
            Some(ActiveTransaction::DirectPositions { targets, .. }) => targets.clone(),
            _ => {
                return Err(EditError::HistoryInvariant(
                    "no active direct multi-position transaction",
                ));
            }
        };

        for target in targets {
            match target {
                DirectPositionTarget::Base { object_id, before } => {
                    let index = self.object_index(object_id)?;
                    *self.project.composition.objects[index]
                        .transform
                        .position
                        .base_value_mut() =
                        Vec2::new(before.x() + delta.x(), before.y() + delta.y())
                            .map_err(|_| EditError::InvalidValue("position"))?;
                }
                DirectPositionTarget::Keyframe {
                    object_id,
                    keyframe_id,
                    before,
                    initial,
                } => {
                    let start = before
                        .and_then(|keyframe| match keyframe.value {
                            PropertyValue::Vec2(value) => Some(value),
                            _ => None,
                        })
                        .unwrap_or(initial);
                    let value = Vec2::new(start.x() + delta.x(), start.y() + delta.y())
                        .map_err(|_| EditError::InvalidValue("position"))?;
                    set_property_keyframe_value(
                        &mut self.project,
                        object_id,
                        AnimatableProperty::Position,
                        keyframe_id,
                        PropertyValue::Vec2(value),
                    )?
                    .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                }
            }
        }
        Ok(())
    }

    pub fn begin_scale_transaction(&mut self, object_id: ObjectId) -> Result<(), EditError> {
        if self.transaction.is_some() {
            return Err(EditError::HistoryInvariant("transaction already active"));
        }

        let index = self.object_index(object_id)?;
        let before = *self.project.composition.objects[index]
            .transform
            .scale
            .base_value();

        self.transaction = Some(ActiveTransaction::Scale { object_id, before });
        Ok(())
    }

    pub fn update_scale_transaction(&mut self, value: Vec2) -> Result<(), EditError> {
        let object_id = match self.transaction {
            Some(ActiveTransaction::Scale { object_id, .. }) => object_id,
            _ => {
                return Err(EditError::HistoryInvariant("no active scale transaction"));
            }
        };

        let index = self.object_index(object_id)?;
        *self.project.composition.objects[index]
            .transform
            .scale
            .base_value_mut() = value;
        Ok(())
    }

    pub fn begin_rotation_transaction(&mut self, object_id: ObjectId) -> Result<(), EditError> {
        if self.transaction.is_some() {
            return Err(EditError::HistoryInvariant("transaction already active"));
        }

        let index = self.object_index(object_id)?;
        let before = *self.project.composition.objects[index]
            .transform
            .rotation_degrees
            .base_value();

        self.transaction = Some(ActiveTransaction::Rotation { object_id, before });
        Ok(())
    }

    pub fn update_rotation_transaction(&mut self, value: f32) -> Result<(), EditError> {
        if !value.is_finite() {
            return Err(EditError::InvalidValue("rotation"));
        }

        let object_id = match self.transaction {
            Some(ActiveTransaction::Rotation { object_id, .. }) => object_id,
            _ => {
                return Err(EditError::HistoryInvariant(
                    "no active rotation transaction",
                ));
            }
        };

        let index = self.object_index(object_id)?;
        *self.project.composition.objects[index]
            .transform
            .rotation_degrees
            .base_value_mut() = value;
        Ok(())
    }

    pub fn commit_transaction(&mut self) -> Result<bool, EditError> {
        let Some(transaction) = self.transaction.take() else {
            return Ok(false);
        };

        match transaction {
            ActiveTransaction::Position { object_id, before } => {
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
            ActiveTransaction::Positions { before } => {
                let mut changes = Vec::with_capacity(before.len());
                for (object_id, before_position) in before {
                    let index = self.object_index(object_id)?;
                    let after = *self.project.composition.objects[index]
                        .transform
                        .position
                        .base_value();
                    if before_position != after {
                        changes.push((object_id, before_position, after));
                    }
                }

                if changes.is_empty() {
                    return Ok(false);
                }

                let count = changes.len();
                self.history.push(PendingHistoryEntry::new(
                    &format!("Move {count} Objects"),
                    HistoryPayload::PositionsBaseChanged { changes },
                ));
                Ok(true)
            }
            ActiveTransaction::DirectPositions {
                targets,
                next_entity_id_before,
            } => {
                let mut base_changes = Vec::new();
                let mut keyframe_changes = Vec::new();

                for target in targets {
                    match target {
                        DirectPositionTarget::Base { object_id, before } => {
                            let index = self.object_index(object_id)?;
                            let after = *self.project.composition.objects[index]
                                .transform
                                .position
                                .base_value();
                            if before != after {
                                base_changes.push((object_id, before, after));
                            }
                        }
                        DirectPositionTarget::Keyframe {
                            object_id,
                            keyframe_id,
                            before,
                            initial,
                        } => {
                            let after = property_keyframe_by_id(
                                &self.project,
                                object_id,
                                AnimatableProperty::Position,
                                keyframe_id,
                            )?
                            .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                            let changed = before
                                .as_ref()
                                .map_or(after.value != PropertyValue::Vec2(initial), |before| {
                                    before.value != after.value
                                });
                            if changed {
                                keyframe_changes.push(PropertyKeyframeValueRecord {
                                    object_id,
                                    property: AnimatableProperty::Position,
                                    before,
                                    after,
                                });
                            } else if before.is_none() {
                                remove_property_keyframe_by_id(
                                    &mut self.project,
                                    object_id,
                                    AnimatableProperty::Position,
                                    keyframe_id,
                                )?
                                .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                            }
                        }
                    }
                }

                if base_changes.is_empty() && keyframe_changes.is_empty() {
                    self.project.next_entity_id = next_entity_id_before;
                    return Ok(false);
                }

                let count = base_changes.len() + keyframe_changes.len();
                self.history.push(PendingHistoryEntry::new(
                    &format!("Move {count} Objects"),
                    HistoryPayload::DirectPositionsChanged {
                        base_changes,
                        keyframe_changes,
                    },
                ));
                Ok(true)
            }
            ActiveTransaction::Scale { object_id, before } => {
                let index = self.object_index(object_id)?;
                let after = *self.project.composition.objects[index]
                    .transform
                    .scale
                    .base_value();

                if before == after {
                    return Ok(false);
                }

                self.history.push(PendingHistoryEntry::new(
                    "Scale Object",
                    HistoryPayload::ScaleBaseChanged {
                        object_id,
                        before,
                        after,
                    },
                ));
                Ok(true)
            }
            ActiveTransaction::Rotation { object_id, before } => {
                let index = self.object_index(object_id)?;
                let after = *self.project.composition.objects[index]
                    .transform
                    .rotation_degrees
                    .base_value();

                if before == after {
                    return Ok(false);
                }

                self.history.push(PendingHistoryEntry::new(
                    "Rotate Object",
                    HistoryPayload::RotationBaseChanged {
                        object_id,
                        before,
                        after,
                    },
                ));
                Ok(true)
            }
            ActiveTransaction::PropertyKeyframeValue {
                object_id,
                property,
                keyframe_id,
                before,
                initial_value,
                inserted_next_entity_id_before,
            } => {
                let after =
                    property_keyframe_by_id(&self.project, object_id, property, keyframe_id)?
                        .ok_or(EditError::KeyframeNotFound(keyframe_id))?;

                if before.as_ref().is_some_and(|before| *before == after) {
                    return Ok(false);
                }
                if before.is_none()
                    && after.value == initial_value
                    && let Some(previous_next_entity_id) = inserted_next_entity_id_before
                {
                    remove_property_keyframe_by_id(
                        &mut self.project,
                        object_id,
                        property,
                        keyframe_id,
                    )?
                    .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                    self.project.next_entity_id = previous_next_entity_id;
                    return Ok(false);
                }

                self.history.push(PendingHistoryEntry::new(
                    "Edit Animated Property",
                    HistoryPayload::PropertyKeyframeValueChanged {
                        object_id,
                        property,
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
            ActiveTransaction::Position { object_id, before } => {
                let index = self.object_index(object_id)?;
                *self.project.composition.objects[index]
                    .transform
                    .position
                    .base_value_mut() = before;
            }
            ActiveTransaction::Positions { before } => {
                for (object_id, position) in before {
                    let index = self.object_index(object_id)?;
                    *self.project.composition.objects[index]
                        .transform
                        .position
                        .base_value_mut() = position;
                }
            }
            ActiveTransaction::DirectPositions {
                targets,
                next_entity_id_before,
            } => {
                for target in targets {
                    match target {
                        DirectPositionTarget::Base { object_id, before } => {
                            let index = self.object_index(object_id)?;
                            *self.project.composition.objects[index]
                                .transform
                                .position
                                .base_value_mut() = before;
                        }
                        DirectPositionTarget::Keyframe {
                            object_id,
                            keyframe_id,
                            before,
                            initial: _,
                        } => {
                            if let Some(before) = before {
                                set_property_keyframe_value(
                                    &mut self.project,
                                    object_id,
                                    AnimatableProperty::Position,
                                    keyframe_id,
                                    before.value,
                                )?
                                .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                            } else {
                                remove_property_keyframe_by_id(
                                    &mut self.project,
                                    object_id,
                                    AnimatableProperty::Position,
                                    keyframe_id,
                                )?
                                .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                            }
                        }
                    }
                }
                self.project.next_entity_id = next_entity_id_before;
            }
            ActiveTransaction::Scale { object_id, before } => {
                let index = self.object_index(object_id)?;
                *self.project.composition.objects[index]
                    .transform
                    .scale
                    .base_value_mut() = before;
            }
            ActiveTransaction::Rotation { object_id, before } => {
                let index = self.object_index(object_id)?;
                *self.project.composition.objects[index]
                    .transform
                    .rotation_degrees
                    .base_value_mut() = before;
            }
            ActiveTransaction::PropertyKeyframeValue {
                object_id,
                property,
                keyframe_id,
                before,
                initial_value: _,
                inserted_next_entity_id_before,
            } => {
                if let Some(before) = before {
                    set_property_keyframe_value(
                        &mut self.project,
                        object_id,
                        property,
                        keyframe_id,
                        before.value,
                    )?
                    .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                } else {
                    remove_property_keyframe_by_id(
                        &mut self.project,
                        object_id,
                        property,
                        keyframe_id,
                    )?
                    .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                    if let Some(previous_next_entity_id) = inserted_next_entity_id_before {
                        self.project.next_entity_id = previous_next_entity_id;
                    }
                }
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
                self.project
                    .composition
                    .objects
                    .push(object.as_ref().clone());
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
                    HistoryPayload::ObjectDeleted {
                        index,
                        object: Box::new(object),
                    },
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
            EditCommand::SetObjectVisible { object_id, visible } => {
                let index = self.object_index(object_id)?;
                let before = self.project.composition.objects[index].visible;
                if before == visible {
                    return Ok(None);
                }

                self.project.composition.objects[index].visible = visible;
                Ok(Some(PendingHistoryEntry::new(
                    if visible {
                        "Show Object"
                    } else {
                        "Hide Object"
                    },
                    HistoryPayload::ObjectVisibilityChanged {
                        object_id,
                        before,
                        after: visible,
                    },
                )))
            }
            EditCommand::SetObjectLocked { object_id, locked } => {
                let index = self.object_index(object_id)?;
                let before = self.project.composition.objects[index].locked;
                if before == locked {
                    return Ok(None);
                }

                self.project.composition.objects[index].locked = locked;
                Ok(Some(PendingHistoryEntry::new(
                    if locked {
                        "Lock Object"
                    } else {
                        "Unlock Object"
                    },
                    HistoryPayload::ObjectLockedChanged {
                        object_id,
                        before,
                        after: locked,
                    },
                )))
            }
            EditCommand::SetPropertyBase {
                object_id,
                property,
                value,
            } => {
                let before = property_base_value(&self.project, object_id, property)?;
                if before == value {
                    return Ok(None);
                }

                set_property_base_value(&mut self.project, object_id, property, value)?;
                if let Err(error) = self.project.validate() {
                    set_property_base_value(&mut self.project, object_id, property, before)?;
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Edit Property",
                    HistoryPayload::PropertyBaseChanged {
                        change: PropertyBaseChange {
                            object_id,
                            property,
                            before,
                            after: value,
                        },
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
            EditCommand::InsertPropertyKeyframe {
                object_id,
                property,
                keyframe,
            } => {
                if keyframe.id.get() != self.project.next_entity_id {
                    return Err(EditError::HistoryInvariant(
                        "keyframe id must match next_entity_id",
                    ));
                }

                let next_entity_id = self
                    .project
                    .next_entity_id
                    .checked_add(1)
                    .ok_or(EditError::IdAllocation(IdAllocationError::Exhausted))?;
                insert_property_keyframe(&mut self.project, object_id, property, keyframe)?;

                let previous_next_entity_id = self.project.next_entity_id;
                self.project.next_entity_id = next_entity_id;
                if let Err(error) = self.project.validate() {
                    let _ = remove_property_keyframe_by_id(
                        &mut self.project,
                        object_id,
                        property,
                        keyframe.id,
                    );
                    self.project.next_entity_id = previous_next_entity_id;
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Add Keyframe",
                    HistoryPayload::PropertyKeyframeInserted {
                        object_id,
                        property,
                        keyframe,
                    },
                )))
            }
            EditCommand::RemovePropertyKeyframe {
                object_id,
                property,
                keyframe,
            } => {
                let keyframe_count = property_keyframe_count(&self.project, object_id, property)?;
                let base_before = if keyframe_count == 1 {
                    Some(property_base_value(&self.project, object_id, property)?)
                } else {
                    None
                };

                let removed = remove_property_keyframe_by_id(
                    &mut self.project,
                    object_id,
                    property,
                    keyframe.id,
                )?
                .ok_or(EditError::KeyframeNotFound(keyframe.id))?;

                if removed != keyframe {
                    insert_property_keyframe(&mut self.project, object_id, property, removed)?;
                    return Err(EditError::HistoryInvariant(
                        "removed keyframe payload mismatch",
                    ));
                }

                let base_after = if keyframe_count == 1 {
                    set_property_base_value(
                        &mut self.project,
                        object_id,
                        property,
                        keyframe.value,
                    )?;
                    Some(keyframe.value)
                } else {
                    None
                };

                if let Err(error) = self.project.validate() {
                    if let Some(base_before) = base_before {
                        let _ = set_property_base_value(
                            &mut self.project,
                            object_id,
                            property,
                            base_before,
                        );
                    }
                    let _ =
                        insert_property_keyframe(&mut self.project, object_id, property, keyframe);
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Remove Keyframe",
                    HistoryPayload::PropertyKeyframeRemoved {
                        object_id,
                        property,
                        keyframe,
                        base_before,
                        base_after,
                    },
                )))
            }
            EditCommand::InsertPropertyKeyframes { entries } => {
                if entries.is_empty() {
                    return Ok(None);
                }

                let mut allocator = EntityIdAllocator::new(self.project.next_entity_id)?;
                let mut destinations = HashSet::with_capacity(entries.len());

                for (object_id, property, keyframe) in &entries {
                    let expected_id = allocator.allocate_keyframe()?;
                    if keyframe.id != expected_id {
                        return Err(EditError::HistoryInvariant(
                            "inserted keyframe ids must be fresh and sequential",
                        ));
                    }
                    if !destinations.insert((*object_id, *property, keyframe.tick)) {
                        return Err(EditError::HistoryInvariant(
                            "duplicate destination in keyframe insert batch",
                        ));
                    }
                }

                let previous_next_entity_id = self.project.next_entity_id;
                let mut records = Vec::with_capacity(entries.len());

                for (object_id, property, keyframe) in entries {
                    let replaced = crate::property::remove_property_keyframe_at_tick(
                        &mut self.project,
                        object_id,
                        property,
                        keyframe.tick,
                    )?;

                    if let Err(error) =
                        insert_property_keyframe(&mut self.project, object_id, property, keyframe)
                    {
                        if let Some(replaced) = replaced {
                            let _ = insert_property_keyframe(
                                &mut self.project,
                                object_id,
                                property,
                                replaced,
                            );
                        }
                        for record in records.iter().rev() {
                            let record: &PropertyKeyframeInsertRecord = record;
                            let _ = remove_property_keyframe_by_id(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                record.keyframe.id,
                            );
                            if let Some(replaced) = record.replaced {
                                let _ = insert_property_keyframe(
                                    &mut self.project,
                                    record.object_id,
                                    record.property,
                                    replaced,
                                );
                            }
                        }
                        return Err(EditError::PropertyAccess(error));
                    }

                    records.push(PropertyKeyframeInsertRecord {
                        object_id,
                        property,
                        keyframe,
                        replaced,
                    });
                }

                self.project.next_entity_id = allocator.next_entity_id();
                if let Err(error) = self.project.validate() {
                    self.project.next_entity_id = previous_next_entity_id;
                    for record in records.iter().rev() {
                        let _ = remove_property_keyframe_by_id(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe.id,
                        );
                        if let Some(replaced) = record.replaced {
                            let _ = insert_property_keyframe(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                replaced,
                            );
                        }
                    }
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Paste Keyframes",
                    HistoryPayload::PropertyKeyframesInserted { records },
                )))
            }
            EditCommand::DeletePropertyKeyframes { keyframe_ids } => {
                let mut unique_ids = HashSet::with_capacity(keyframe_ids.len());
                let mut located = Vec::with_capacity(keyframe_ids.len());

                for keyframe_id in keyframe_ids {
                    if !unique_ids.insert(keyframe_id) {
                        continue;
                    }

                    let located_keyframe = locate_property_keyframe(&self.project, keyframe_id)
                        .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                    located.push(located_keyframe);
                }

                if located.is_empty() {
                    return Ok(None);
                }

                let mut grouped: Vec<(ObjectId, AnimatableProperty, Vec<PropertyKeyframe>)> =
                    Vec::new();

                for located_keyframe in located {
                    if let Some((_, _, keys)) =
                        grouped.iter_mut().find(|(object_id, property, _)| {
                            *object_id == located_keyframe.object_id
                                && *property == located_keyframe.property
                        })
                    {
                        keys.push(located_keyframe.keyframe);
                    } else {
                        grouped.push((
                            located_keyframe.object_id,
                            located_keyframe.property,
                            vec![located_keyframe.keyframe],
                        ));
                    }
                }

                let mut records = Vec::new();
                let mut base_changes = Vec::new();

                for (object_id, property, mut keys) in grouped {
                    keys.sort_by_key(|keyframe| keyframe.tick);
                    let total_before = property_keyframe_count(&self.project, object_id, property)?;
                    let base_before = property_base_value(&self.project, object_id, property)?;

                    for keyframe in &keys {
                        let removed = remove_property_keyframe_by_id(
                            &mut self.project,
                            object_id,
                            property,
                            keyframe.id,
                        )?
                        .ok_or(EditError::KeyframeNotFound(keyframe.id))?;

                        records.push(PropertyKeyframeDeleteRecord {
                            object_id,
                            property,
                            keyframe: removed,
                        });
                    }

                    if keys.len() == total_before {
                        let after = keys.last().expect("non-empty delete group").value;
                        set_property_base_value(&mut self.project, object_id, property, after)?;
                        base_changes.push(PropertyBaseChange {
                            object_id,
                            property,
                            before: base_before,
                            after,
                        });
                    }
                }

                if let Err(error) = self.project.validate() {
                    for change in &base_changes {
                        let _ = set_property_base_value(
                            &mut self.project,
                            change.object_id,
                            change.property,
                            change.before,
                        );
                    }
                    for record in &records {
                        let _ = insert_property_keyframe(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe,
                        );
                    }
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Delete Keyframes",
                    HistoryPayload::PropertyKeyframesDeleted {
                        records,
                        base_changes,
                    },
                )))
            }
            EditCommand::MovePropertyKeyframes { moves } => {
                let mut resolved = Vec::with_capacity(moves.len());
                let mut ids = HashSet::with_capacity(moves.len());
                let mut destinations = HashSet::with_capacity(moves.len());

                for movement in moves {
                    if !ids.insert(movement.keyframe_id) {
                        return Err(EditError::HistoryInvariant(
                            "duplicate keyframe id in move batch",
                        ));
                    }
                    let before = property_keyframe_by_id(
                        &self.project,
                        movement.object_id,
                        movement.property,
                        movement.keyframe_id,
                    )?
                    .ok_or(EditError::KeyframeNotFound(movement.keyframe_id))?;
                    if before.tick == movement.target_tick {
                        continue;
                    }
                    if !destinations.insert((
                        movement.object_id,
                        movement.property,
                        movement.target_tick,
                    )) {
                        return Err(EditError::HistoryInvariant(
                            "duplicate keyframe destination in move batch",
                        ));
                    }
                    resolved.push((movement, before));
                }

                if resolved.is_empty() {
                    return Ok(None);
                }

                let mut removed_sources = Vec::with_capacity(resolved.len());
                for (movement, before) in &resolved {
                    let removed = remove_property_keyframe_by_id(
                        &mut self.project,
                        movement.object_id,
                        movement.property,
                        movement.keyframe_id,
                    )?
                    .ok_or(EditError::KeyframeNotFound(movement.keyframe_id))?;
                    if removed != *before {
                        for (object_id, property, keyframe) in removed_sources.drain(..) {
                            let _ = insert_property_keyframe(
                                &mut self.project,
                                object_id,
                                property,
                                keyframe,
                            );
                        }
                        return Err(EditError::HistoryInvariant("move source payload mismatch"));
                    }
                    removed_sources.push((movement.object_id, movement.property, removed));
                }

                let mut records = Vec::with_capacity(resolved.len());
                for (movement, before) in &resolved {
                    let replaced = crate::property::remove_property_keyframe_at_tick(
                        &mut self.project,
                        movement.object_id,
                        movement.property,
                        movement.target_tick,
                    )?;
                    let after = PropertyKeyframe {
                        tick: movement.target_tick,
                        ..*before
                    };

                    if let Err(error) = insert_property_keyframe(
                        &mut self.project,
                        movement.object_id,
                        movement.property,
                        after,
                    ) {
                        for record in records.iter().rev() {
                            let record: &PropertyKeyframeMoveRecord = record;
                            let _ = remove_property_keyframe_by_id(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                record.after.id,
                            );
                            if let Some(replaced) = record.replaced {
                                let _ = insert_property_keyframe(
                                    &mut self.project,
                                    record.object_id,
                                    record.property,
                                    replaced,
                                );
                            }
                        }
                        if let Some(replaced) = replaced {
                            let _ = insert_property_keyframe(
                                &mut self.project,
                                movement.object_id,
                                movement.property,
                                replaced,
                            );
                        }
                        for (object_id, property, keyframe) in &removed_sources {
                            let _ = insert_property_keyframe(
                                &mut self.project,
                                *object_id,
                                *property,
                                *keyframe,
                            );
                        }
                        return Err(EditError::PropertyAccess(error));
                    }

                    records.push(PropertyKeyframeMoveRecord {
                        object_id: movement.object_id,
                        property: movement.property,
                        before: *before,
                        after,
                        replaced,
                    });
                }

                if let Err(error) = self.project.validate() {
                    for record in records.iter().rev() {
                        let _ = remove_property_keyframe_by_id(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.after.id,
                        );
                        if let Some(replaced) = record.replaced {
                            let _ = insert_property_keyframe(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                replaced,
                            );
                        }
                    }
                    for (object_id, property, keyframe) in &removed_sources {
                        let _ = insert_property_keyframe(
                            &mut self.project,
                            *object_id,
                            *property,
                            *keyframe,
                        );
                    }
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Move Keyframes",
                    HistoryPayload::PropertyKeyframesMoved { records },
                )))
            }
            EditCommand::SetPropertyKeyframeInterpolations {
                keyframe_ids,
                interpolation,
            } => {
                let mut unique_ids = HashSet::with_capacity(keyframe_ids.len());
                let mut records = Vec::with_capacity(keyframe_ids.len());

                for keyframe_id in keyframe_ids {
                    if !unique_ids.insert(keyframe_id) {
                        continue;
                    }

                    let located = locate_property_keyframe(&self.project, keyframe_id)
                        .ok_or(EditError::KeyframeNotFound(keyframe_id))?;
                    if located.keyframe.interpolation == interpolation {
                        continue;
                    }

                    records.push(PropertyKeyframeInterpolationRecord {
                        object_id: located.object_id,
                        property: located.property,
                        keyframe_id,
                        before: located.keyframe.interpolation,
                        after: interpolation,
                    });
                }

                if records.is_empty() {
                    return Ok(None);
                }

                for (index, record) in records.iter().enumerate() {
                    match set_property_keyframe_interpolation(
                        &mut self.project,
                        record.object_id,
                        record.property,
                        record.keyframe_id,
                        record.after,
                    ) {
                        Ok(Some(before)) if before == record.before => {}
                        Ok(Some(before)) => {
                            let _ = set_property_keyframe_interpolation(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                record.keyframe_id,
                                before,
                            );
                            for applied in records[..index].iter().rev() {
                                let _ = set_property_keyframe_interpolation(
                                    &mut self.project,
                                    applied.object_id,
                                    applied.property,
                                    applied.keyframe_id,
                                    applied.before,
                                );
                            }
                            return Err(EditError::HistoryInvariant(
                                "keyframe interpolation source mismatch",
                            ));
                        }
                        Ok(None) => {
                            for applied in records[..index].iter().rev() {
                                let _ = set_property_keyframe_interpolation(
                                    &mut self.project,
                                    applied.object_id,
                                    applied.property,
                                    applied.keyframe_id,
                                    applied.before,
                                );
                            }
                            return Err(EditError::KeyframeNotFound(record.keyframe_id));
                        }
                        Err(error) => {
                            for applied in records[..index].iter().rev() {
                                let _ = set_property_keyframe_interpolation(
                                    &mut self.project,
                                    applied.object_id,
                                    applied.property,
                                    applied.keyframe_id,
                                    applied.before,
                                );
                            }
                            return Err(EditError::PropertyAccess(error));
                        }
                    }
                }

                if let Err(error) = self.project.validate() {
                    for record in records.iter().rev() {
                        let _ = set_property_keyframe_interpolation(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe_id,
                            record.before,
                        );
                    }
                    return Err(EditError::InvalidProject(error));
                }

                Ok(Some(PendingHistoryEntry::new(
                    "Change Keyframe Easing",
                    HistoryPayload::PropertyKeyframeInterpolationsChanged { records },
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
                    .insert(*index, object.as_ref().clone());
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
                HistoryPayload::ObjectVisibilityChanged {
                    object_id,
                    before,
                    after,
                },
                direction,
            ) => {
                let value = *direction.pick(before, after);
                let index = self.object_index(*object_id)?;
                self.project.composition.objects[index].visible = value;
            }
            (
                HistoryPayload::ObjectLockedChanged {
                    object_id,
                    before,
                    after,
                },
                direction,
            ) => {
                let value = *direction.pick(before, after);
                let index = self.object_index(*object_id)?;
                self.project.composition.objects[index].locked = value;
            }
            (HistoryPayload::PropertyBaseChanged { change }, direction) => {
                set_property_base_value(
                    &mut self.project,
                    change.object_id,
                    change.property,
                    *direction.pick(&change.before, &change.after),
                )?;
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
            (HistoryPayload::PositionsBaseChanged { changes }, direction) => {
                for (object_id, before, after) in changes {
                    let value = *direction.pick(before, after);
                    let index = self.object_index(*object_id)?;
                    *self.project.composition.objects[index]
                        .transform
                        .position
                        .base_value_mut() = value;
                }
            }
            (
                HistoryPayload::DirectPositionsChanged {
                    base_changes,
                    keyframe_changes,
                },
                direction,
            ) => {
                for (object_id, before, after) in base_changes {
                    let value = *direction.pick(before, after);
                    let index = self.object_index(*object_id)?;
                    *self.project.composition.objects[index]
                        .transform
                        .position
                        .base_value_mut() = value;
                }

                for record in keyframe_changes {
                    match (&record.before, direction) {
                        (Some(before), HistoryDirection::Undo) => {
                            set_property_keyframe_value(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                before.id,
                                before.value,
                            )?
                            .ok_or(EditError::KeyframeNotFound(before.id))?;
                        }
                        (Some(_), HistoryDirection::Redo) => {
                            set_property_keyframe_value(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                record.after.id,
                                record.after.value,
                            )?
                            .ok_or(EditError::KeyframeNotFound(record.after.id))?;
                        }
                        (None, HistoryDirection::Undo) => {
                            remove_property_keyframe_by_id(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                record.after.id,
                            )?
                            .ok_or(EditError::KeyframeNotFound(record.after.id))?;
                        }
                        (None, HistoryDirection::Redo) => {
                            insert_property_keyframe(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                record.after,
                            )?;
                        }
                    }
                }
            }
            (
                HistoryPayload::ScaleBaseChanged {
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
                    .scale
                    .base_value_mut() = value;
            }
            (
                HistoryPayload::RotationBaseChanged {
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
                    .rotation_degrees
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
            (
                HistoryPayload::PropertyKeyframeInserted {
                    object_id,
                    property,
                    keyframe,
                },
                HistoryDirection::Undo,
            ) => {
                remove_property_keyframe_by_id(
                    &mut self.project,
                    *object_id,
                    *property,
                    keyframe.id,
                )?
                .ok_or(EditError::KeyframeNotFound(keyframe.id))?;
            }
            (
                HistoryPayload::PropertyKeyframeInserted {
                    object_id,
                    property,
                    keyframe,
                },
                HistoryDirection::Redo,
            ) => {
                insert_property_keyframe(&mut self.project, *object_id, *property, *keyframe)?;
            }
            (
                HistoryPayload::PropertyKeyframeRemoved {
                    object_id,
                    property,
                    keyframe,
                    base_before,
                    base_after: _,
                },
                HistoryDirection::Undo,
            ) => {
                if let Some(base_before) = base_before {
                    set_property_base_value(
                        &mut self.project,
                        *object_id,
                        *property,
                        *base_before,
                    )?;
                }
                insert_property_keyframe(&mut self.project, *object_id, *property, *keyframe)?;
            }
            (
                HistoryPayload::PropertyKeyframeRemoved {
                    object_id,
                    property,
                    keyframe,
                    base_before: _,
                    base_after,
                },
                HistoryDirection::Redo,
            ) => {
                remove_property_keyframe_by_id(
                    &mut self.project,
                    *object_id,
                    *property,
                    keyframe.id,
                )?
                .ok_or(EditError::KeyframeNotFound(keyframe.id))?;
                if let Some(base_after) = base_after {
                    set_property_base_value(&mut self.project, *object_id, *property, *base_after)?;
                }
            }
            (HistoryPayload::PropertyKeyframesInserted { records }, direction) => match direction {
                HistoryDirection::Undo => {
                    for record in records.iter().rev() {
                        remove_property_keyframe_by_id(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe.id,
                        )?
                        .ok_or(EditError::KeyframeNotFound(record.keyframe.id))?;
                        if let Some(replaced) = record.replaced {
                            insert_property_keyframe(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                replaced,
                            )?;
                        }
                    }
                }
                HistoryDirection::Redo => {
                    for record in records {
                        let replaced = crate::property::remove_property_keyframe_at_tick(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe.tick,
                        )?;
                        if replaced != record.replaced {
                            return Err(EditError::HistoryInvariant(
                                "redo paste collision payload mismatch",
                            ));
                        }
                        insert_property_keyframe(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe,
                        )?;
                    }
                }
            },
            (
                HistoryPayload::PropertyKeyframesDeleted {
                    records,
                    base_changes,
                },
                direction,
            ) => match direction {
                HistoryDirection::Undo => {
                    for change in base_changes {
                        set_property_base_value(
                            &mut self.project,
                            change.object_id,
                            change.property,
                            change.before,
                        )?;
                    }
                    for record in records {
                        insert_property_keyframe(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe,
                        )?;
                    }
                }
                HistoryDirection::Redo => {
                    for record in records {
                        remove_property_keyframe_by_id(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.keyframe.id,
                        )?
                        .ok_or(EditError::KeyframeNotFound(record.keyframe.id))?;
                    }
                    for change in base_changes {
                        set_property_base_value(
                            &mut self.project,
                            change.object_id,
                            change.property,
                            change.after,
                        )?;
                    }
                }
            },
            (HistoryPayload::PropertyKeyframesMoved { records }, direction) => match direction {
                HistoryDirection::Undo => {
                    for record in records.iter().rev() {
                        remove_property_keyframe_by_id(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.after.id,
                        )?
                        .ok_or(EditError::KeyframeNotFound(record.after.id))?;
                        if let Some(replaced) = record.replaced {
                            insert_property_keyframe(
                                &mut self.project,
                                record.object_id,
                                record.property,
                                replaced,
                            )?;
                        }
                    }
                    for record in records {
                        insert_property_keyframe(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.before,
                        )?;
                    }
                }
                HistoryDirection::Redo => {
                    for record in records {
                        remove_property_keyframe_by_id(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.before.id,
                        )?
                        .ok_or(EditError::KeyframeNotFound(record.before.id))?;
                    }
                    for record in records {
                        let replaced = crate::property::remove_property_keyframe_at_tick(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.after.tick,
                        )?;
                        if replaced != record.replaced {
                            return Err(EditError::HistoryInvariant(
                                "redo collision payload mismatch",
                            ));
                        }
                        insert_property_keyframe(
                            &mut self.project,
                            record.object_id,
                            record.property,
                            record.after,
                        )?;
                    }
                }
            },
            (
                HistoryPayload::PropertyKeyframeValueChanged {
                    object_id,
                    property,
                    before,
                    after,
                },
                direction,
            ) => match (before, direction) {
                (Some(before), HistoryDirection::Undo) => {
                    set_property_keyframe_value(
                        &mut self.project,
                        *object_id,
                        *property,
                        before.id,
                        before.value,
                    )?
                    .ok_or(EditError::KeyframeNotFound(before.id))?;
                }
                (Some(_), HistoryDirection::Redo) => {
                    set_property_keyframe_value(
                        &mut self.project,
                        *object_id,
                        *property,
                        after.id,
                        after.value,
                    )?
                    .ok_or(EditError::KeyframeNotFound(after.id))?;
                }
                (None, HistoryDirection::Undo) => {
                    remove_property_keyframe_by_id(
                        &mut self.project,
                        *object_id,
                        *property,
                        after.id,
                    )?
                    .ok_or(EditError::KeyframeNotFound(after.id))?;
                }
                (None, HistoryDirection::Redo) => {
                    insert_property_keyframe(&mut self.project, *object_id, *property, *after)?;
                }
            },
            (HistoryPayload::PropertyKeyframeInterpolationsChanged { records }, direction) => {
                for record in records {
                    let (expected, target) = match direction {
                        HistoryDirection::Undo => (record.after, record.before),
                        HistoryDirection::Redo => (record.before, record.after),
                    };
                    let observed = set_property_keyframe_interpolation(
                        &mut self.project,
                        record.object_id,
                        record.property,
                        record.keyframe_id,
                        target,
                    )?
                    .ok_or(EditError::KeyframeNotFound(record.keyframe_id))?;
                    if observed != expected {
                        return Err(EditError::HistoryInvariant(
                            "keyframe interpolation history mismatch",
                        ));
                    }
                }
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
        property::{AnimatableProperty, PropertyValue, property_base_value},
        time::{BpmMicros, GridOffsetNs, MusicalTick, TempoMap, TimeSignature},
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
    fn generic_property_base_edit_is_undoable() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let scale = Vec2::new(2.0, 3.0).expect("scale");

        assert_eq!(
            editor.execute(EditCommand::SetPropertyBase {
                object_id,
                property: AnimatableProperty::Scale,
                value: PropertyValue::Vec2(scale),
            }),
            Ok(true)
        );
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(scale))
        );
        assert_eq!(editor.history_len(), 1);

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(
                Vec2::new(1.0, 1.0).expect("original scale")
            ))
        );

        assert_eq!(editor.redo(), Ok(true));
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(scale))
        );
    }

    #[test]
    fn object_visibility_and_lock_edits_are_undoable() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        assert_eq!(
            editor.execute(EditCommand::SetObjectVisible {
                object_id,
                visible: false,
            }),
            Ok(true)
        );
        assert!(!editor.project().composition.objects[0].visible);

        assert_eq!(
            editor.execute(EditCommand::SetObjectLocked {
                object_id,
                locked: true,
            }),
            Ok(true)
        );
        assert!(editor.project().composition.objects[0].locked);
        assert_eq!(editor.history_len(), 2);

        assert_eq!(editor.undo(), Ok(true));
        assert!(!editor.project().composition.objects[0].locked);
        assert_eq!(editor.undo(), Ok(true));
        assert!(editor.project().composition.objects[0].visible);

        assert_eq!(editor.redo(), Ok(true));
        assert!(!editor.project().composition.objects[0].visible);
        assert_eq!(editor.redo(), Ok(true));
        assert!(editor.project().composition.objects[0].locked);
    }

    #[test]
    fn batch_insert_allocates_fresh_ids_and_restores_collision_on_undo() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let existing = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(240),
                crate::property::PropertyValue::Scalar(0.9),
            )
            .expect("insert")
            .expect("existing");

        let ids = editor
            .insert_property_keyframes(vec![
                super::PropertyKeyframeDraft {
                    object_id,
                    property: crate::property::AnimatableProperty::Opacity,
                    tick: MusicalTick::new(240),
                    value: crate::property::PropertyValue::Scalar(0.2),
                    interpolation: Interpolation::Linear,
                },
                super::PropertyKeyframeDraft {
                    object_id,
                    property: crate::property::AnimatableProperty::Opacity,
                    tick: MusicalTick::new(480),
                    value: crate::property::PropertyValue::Scalar(0.4),
                    interpolation: Interpolation::Hold,
                },
            ])
            .expect("batch insert");

        assert_eq!(ids.len(), 2);
        assert!(ids[0].get() > existing.get());
        let at_240 = crate::property::property_keyframe_at_tick(
            editor.project(),
            object_id,
            crate::property::AnimatableProperty::Opacity,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("key");
        assert_eq!(at_240.id, ids[0]);

        assert_eq!(editor.undo(), Ok(true));
        let restored = crate::property::property_keyframe_at_tick(
            editor.project(),
            object_id,
            crate::property::AnimatableProperty::Opacity,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("restored");
        assert_eq!(restored.id, existing);
        assert!(
            crate::property::property_keyframe_at_tick(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(480),
            )
            .expect("property")
            .is_none()
        );
    }

    #[test]
    fn batch_keyframe_interpolation_change_is_one_undoable_history_entry() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let first = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(0),
                crate::property::PropertyValue::Scalar(0.1),
            )
            .expect("insert")
            .expect("first");
        let second = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(240),
                crate::property::PropertyValue::Scalar(0.5),
            )
            .expect("insert")
            .expect("second");
        let third = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(480),
                crate::property::PropertyValue::Scalar(0.9),
            )
            .expect("insert")
            .expect("third");
        let history_before = editor.history_len();

        assert_eq!(
            editor.set_property_keyframe_interpolations(vec![first, second], Interpolation::Hold,),
            Ok(true)
        );
        assert_eq!(editor.history_len(), history_before + 1);
        assert_eq!(
            crate::property::locate_property_keyframe(editor.project(), first)
                .expect("first")
                .keyframe
                .interpolation,
            Interpolation::Hold
        );
        assert_eq!(
            crate::property::locate_property_keyframe(editor.project(), second)
                .expect("second")
                .keyframe
                .interpolation,
            Interpolation::Hold
        );
        assert_eq!(
            crate::property::locate_property_keyframe(editor.project(), third)
                .expect("third")
                .keyframe
                .interpolation,
            Interpolation::Linear
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            crate::property::locate_property_keyframe(editor.project(), first)
                .expect("first")
                .keyframe
                .interpolation,
            Interpolation::Linear
        );
        assert_eq!(
            crate::property::locate_property_keyframe(editor.project(), second)
                .expect("second")
                .keyframe
                .interpolation,
            Interpolation::Linear
        );

        assert_eq!(editor.redo(), Ok(true));
        assert_eq!(
            crate::property::locate_property_keyframe(editor.project(), first)
                .expect("first")
                .keyframe
                .interpolation,
            Interpolation::Hold
        );
    }

    #[test]
    fn compound_keyframe_delete_is_one_entry_and_latest_deleted_value_becomes_static() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let first = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(0),
                crate::property::PropertyValue::Scalar(0.2),
            )
            .expect("insert")
            .expect("first");
        let second = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(240),
                crate::property::PropertyValue::Scalar(0.8),
            )
            .expect("insert")
            .expect("second");
        let history_before = editor.history_len();

        assert_eq!(
            editor.delete_property_keyframes(vec![second, first]),
            Ok(true)
        );
        assert_eq!(editor.history_len(), history_before + 1);
        assert_eq!(
            crate::property::property_keyframe_count(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(0)
        );
        assert_eq!(
            crate::property::property_base_value(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(crate::property::PropertyValue::Scalar(0.8))
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            crate::property::property_keyframe_count(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(2)
        );
        assert_eq!(
            crate::property::property_base_value(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(crate::property::PropertyValue::Scalar(1.0))
        );
    }

    #[test]
    fn multi_key_move_uses_one_history_entry_and_incoming_wins_collision() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let first = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(0),
                crate::property::PropertyValue::Scalar(0.1),
            )
            .expect("insert")
            .expect("first");
        let second = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(240),
                crate::property::PropertyValue::Scalar(0.2),
            )
            .expect("insert")
            .expect("second");
        let replaced = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(720),
                crate::property::PropertyValue::Scalar(0.9),
            )
            .expect("insert")
            .expect("replaced");
        let history_before = editor.history_len();

        assert_eq!(
            editor.move_property_keyframes(vec![
                super::PropertyKeyframeMove {
                    object_id,
                    property: crate::property::AnimatableProperty::Opacity,
                    keyframe_id: first,
                    target_tick: MusicalTick::new(480),
                },
                super::PropertyKeyframeMove {
                    object_id,
                    property: crate::property::AnimatableProperty::Opacity,
                    keyframe_id: second,
                    target_tick: MusicalTick::new(720),
                },
            ]),
            Ok(true)
        );
        assert_eq!(editor.history_len(), history_before + 1);
        let at_720 = crate::property::property_keyframe_at_tick(
            editor.project(),
            object_id,
            crate::property::AnimatableProperty::Opacity,
            MusicalTick::new(720),
        )
        .expect("property")
        .expect("moved key");
        assert_eq!(at_720.id, second);
        assert_ne!(at_720.id, replaced);

        assert_eq!(editor.undo(), Ok(true));
        let restored = crate::property::property_keyframe_at_tick(
            editor.project(),
            object_id,
            crate::property::AnimatableProperty::Opacity,
            MusicalTick::new(720),
        )
        .expect("property")
        .expect("restored collision");
        assert_eq!(restored.id, replaced);
        assert!(
            crate::property::property_keyframe_at_tick(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(0),
            )
            .expect("property")
            .is_some()
        );
        assert!(
            crate::property::property_keyframe_at_tick(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(240),
            )
            .expect("property")
            .is_some()
        );
    }

    #[test]
    fn moving_property_keyframe_is_one_undoable_history_entry() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let keyframe_id = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(0),
                crate::property::PropertyValue::Scalar(0.5),
            )
            .expect("insert")
            .expect("keyframe");
        let history_before_move = editor.history_len();

        assert_eq!(
            editor.move_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                keyframe_id,
                MusicalTick::new(240),
            ),
            Ok(true)
        );
        assert_eq!(editor.history_len(), history_before_move + 1);
        assert!(
            crate::property::property_keyframe_at_tick(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(240),
            )
            .expect("property")
            .is_some()
        );

        assert_eq!(editor.undo(), Ok(true));
        assert!(
            crate::property::property_keyframe_at_tick(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(0),
            )
            .expect("property")
            .is_some()
        );
    }

    #[test]
    fn removing_final_property_keyframe_promotes_removed_value_to_static_base() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let keyframe_id = editor
            .create_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                MusicalTick::new(240),
                crate::property::PropertyValue::Scalar(0.25),
            )
            .expect("insert")
            .expect("keyframe");
        let keyframe = crate::property::property_keyframe_at_tick(
            editor.project(),
            object_id,
            crate::property::AnimatableProperty::Opacity,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("keyframe");
        assert_eq!(keyframe.id, keyframe_id);

        assert_eq!(
            editor.remove_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Opacity,
                keyframe,
            ),
            Ok(true)
        );
        assert_eq!(
            crate::property::property_keyframe_count(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(0)
        );
        assert_eq!(
            crate::property::property_base_value(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(crate::property::PropertyValue::Scalar(0.25))
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            crate::property::property_base_value(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(crate::property::PropertyValue::Scalar(1.0))
        );
        assert_eq!(
            crate::property::property_keyframe_count(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Opacity,
            ),
            Ok(1)
        );
    }

    #[test]
    fn first_property_keyframe_uses_shared_allocator_and_undo_preserves_allocator() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");
        let keyframe_id = editor
            .create_first_property_keyframe(
                object_id,
                crate::property::AnimatableProperty::Position,
                MusicalTick::new(240),
            )
            .expect("keyframe edit")
            .expect("first keyframe");

        assert_eq!(keyframe_id.get(), 2);
        assert_eq!(editor.project().next_entity_id, 3);
        assert_eq!(
            crate::property::property_keyframe_count(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Position,
            ),
            Ok(1)
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            crate::property::property_keyframe_count(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Position,
            ),
            Ok(0)
        );
        assert_eq!(editor.project().next_entity_id, 3);

        assert_eq!(editor.redo(), Ok(true));
        assert_eq!(
            crate::property::property_keyframe_count(
                editor.project(),
                object_id,
                crate::property::AnimatableProperty::Position,
            ),
            Ok(1)
        );
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
    fn multi_position_drag_applies_shared_delta_as_one_history_entry() {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let mut first = object(1, "A");
        first.transform.position =
            Animated::new_static(Vec2::new(10.0, 20.0).expect("first position"));
        let mut second = object(2, "B");
        second.transform.position =
            Animated::new_static(Vec2::new(-5.0, 40.0).expect("second position"));
        project.composition.objects.extend([first, second]);
        project.next_entity_id = 3;
        let mut editor = ProjectEditor::new(project).expect("valid project");

        editor
            .begin_multi_position_transaction(&[
                ObjectId::new(2).expect("second"),
                ObjectId::new(1).expect("first"),
            ])
            .expect("begin multi-position transaction");
        editor
            .update_multi_position_transaction(Vec2::new(30.0, -10.0).expect("delta"))
            .expect("preview move");
        assert_eq!(editor.history_len(), 0);

        assert_eq!(editor.commit_transaction(), Ok(true));
        assert_eq!(editor.history_len(), 1);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(40.0, 10.0).expect("first moved")
        );
        assert_eq!(
            *editor.project().composition.objects[1]
                .transform
                .position
                .base_value(),
            Vec2::new(25.0, 30.0).expect("second moved")
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(10.0, 20.0).expect("first restored")
        );
        assert_eq!(
            *editor.project().composition.objects[1]
                .transform
                .position
                .base_value(),
            Vec2::new(-5.0, 40.0).expect("second restored")
        );
    }

    #[test]
    fn one_rotation_drag_transaction_creates_one_history_entry() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        editor
            .begin_rotation_transaction(object_id)
            .expect("begin rotation transaction");
        for rotation in [15.0, 45.0, 190.0, 405.0] {
            editor
                .update_rotation_transaction(rotation)
                .expect("preview rotation");
        }

        assert_eq!(editor.history_len(), 0);
        assert_eq!(editor.commit_transaction(), Ok(true));
        assert_eq!(editor.history_len(), 1);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .rotation_degrees
                .base_value(),
            405.0
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .rotation_degrees
                .base_value(),
            0.0
        );
        assert_eq!(editor.redo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .rotation_degrees
                .base_value(),
            405.0
        );
    }

    #[test]
    fn cancelled_rotation_transaction_restores_before_state() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        editor
            .begin_rotation_transaction(object_id)
            .expect("begin rotation transaction");
        editor
            .update_rotation_transaction(-123.5)
            .expect("preview rotation");
        assert_eq!(editor.cancel_transaction(), Ok(true));

        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .rotation_degrees
                .base_value(),
            0.0
        );
        assert_eq!(editor.history_len(), 0);
    }

    #[test]
    fn one_scale_drag_transaction_creates_one_history_entry() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        editor
            .begin_scale_transaction(object_id)
            .expect("begin scale transaction");
        for scale in [(1.25, 0.75), (1.5, 0.5), (-2.0, 1.25)] {
            editor
                .update_scale_transaction(Vec2::new(scale.0, scale.1).expect("finite scale"))
                .expect("preview scale");
        }

        assert_eq!(editor.history_len(), 0);
        assert_eq!(editor.commit_transaction(), Ok(true));
        assert_eq!(editor.history_len(), 1);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .scale
                .base_value(),
            Vec2::new(-2.0, 1.25).expect("committed scale")
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .scale
                .base_value(),
            Vec2::new(1.0, 1.0).expect("restored scale")
        );
        assert_eq!(editor.redo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .scale
                .base_value(),
            Vec2::new(-2.0, 1.25).expect("redone scale")
        );
    }

    #[test]
    fn cancelled_scale_transaction_restores_before_state() {
        let mut editor = editor_with_object();
        let object_id = ObjectId::new(1).expect("object id");

        editor
            .begin_scale_transaction(object_id)
            .expect("begin scale transaction");
        editor
            .update_scale_transaction(Vec2::new(3.0, -0.5).expect("finite scale"))
            .expect("preview scale");
        assert_eq!(editor.cancel_transaction(), Ok(true));

        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .scale
                .base_value(),
            Vec2::new(1.0, 1.0).expect("restored scale")
        );
        assert_eq!(editor.history_len(), 0);
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
