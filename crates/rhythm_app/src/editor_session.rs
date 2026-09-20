use std::collections::HashSet;

use rhythm_core::{
    editor::{EditError, ProjectEditor, PropertyKeyframeMove},
    ids::{KeyframeId, ObjectId},
    property::{
        AnimatableProperty, PropertyValue, evaluate_property_at_tick, locate_property_keyframe,
        property_keyframe_at_tick, property_keyframe_count,
    },
    time::{
        BeatDivision, DurationNs, MVP_BEAT_DIVISIONS, MusicalTick, PPQ, ProjectTimeNs, TempoMap,
        snap_tick_position_to_grid,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewQuality {
    #[default]
    Auto,
    Full,
    Half,
    Quarter,
}

impl PreviewQuality {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Full, Self::Half, Self::Quarter];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Full => "Full",
            Self::Half => "Half",
            Self::Quarter => "Quarter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineBoxSelection {
    start: [f32; 2],
    current: [f32; 2],
    ctrl_toggle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyframeDragMember {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub keyframe_id: KeyframeId,
    pub original_tick: MusicalTick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyframeDrag {
    anchor_keyframe_id: KeyframeId,
    anchor_original_tick: MusicalTick,
    anchor_target_tick: MusicalTick,
    members: Vec<KeyframeDragMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingKeyframeMove {
    moves: Vec<PropertyKeyframeMove>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimelineView {
    start: ProjectTimeNs,
    end: ProjectTimeNs,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyframeCopyEntry {
    pub source_object_id: ObjectId,
    pub source_property: AnimatableProperty,
    pub relative_tick: i64,
    pub value: PropertyValue,
    pub interpolation: rhythm_core::animation::Interpolation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeCopyPacket {
    pub entries: Vec<KeyframeCopyEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusedProperty {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
}

#[derive(Debug)]
pub struct EditorSession {
    pub preview_quality: PreviewQuality,
    playhead: ProjectTimeNs,
    authoring_division: BeatDivision,
    timeline_view: Option<TimelineView>,
    selected_keyframes: HashSet<KeyframeId>,
    focused_property: Option<FocusedProperty>,
    keyframe_drag: Option<KeyframeDrag>,
    pending_keyframe_move: Option<PendingKeyframeMove>,
    keyframe_clipboard: Option<KeyframeCopyPacket>,
    timeline_box_selection: Option<TimelineBoxSelection>,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self {
            preview_quality: PreviewQuality::Auto,
            playhead: ProjectTimeNs::new(0),
            authoring_division: BeatDivision::new(4).expect("1/4 beat is an accepted MVP grid"),
            timeline_view: None,
            selected_keyframes: HashSet::new(),
            focused_property: None,
            keyframe_drag: None,
            pending_keyframe_move: None,
            keyframe_clipboard: None,
            timeline_box_selection: None,
        }
    }
}

impl EditorSession {
    #[must_use]
    pub const fn playhead(&self) -> ProjectTimeNs {
        self.playhead
    }

    pub const fn seek_paused(&mut self, project_time: ProjectTimeNs) {
        self.playhead = project_time;
    }

    #[must_use]
    pub const fn authoring_division(&self) -> BeatDivision {
        self.authoring_division
    }

    #[must_use]
    pub const fn focused_property(&self) -> Option<FocusedProperty> {
        self.focused_property
    }

    pub const fn focus_property(&mut self, object_id: ObjectId, property: AnimatableProperty) {
        self.focused_property = Some(FocusedProperty {
            object_id,
            property,
        });
    }

    pub fn keyframe_action(&mut self, editor: &mut ProjectEditor) -> Result<bool, EditError> {
        let Some(focused) = self.focused_property else {
            return Ok(false);
        };

        let (resolved_tick, resolved_time) = {
            let project = editor.project();
            let Ok(continuous_tick) = project.tempo_map.continuous_tick_position(self.playhead)
            else {
                return Ok(false);
            };
            let Ok(tick) = snap_tick_position_to_grid(continuous_tick, self.authoring_division)
            else {
                return Ok(false);
            };
            let Ok(project_time) = project.tempo_map.project_time_for_tick(tick) else {
                return Ok(false);
            };

            (tick, project_time)
        };

        if let Some(existing) = property_keyframe_at_tick(
            editor.project(),
            focused.object_id,
            focused.property,
            resolved_tick,
        )? {
            let changed =
                editor.remove_property_keyframe(focused.object_id, focused.property, existing)?;
            if changed {
                self.selected_keyframes.remove(&existing.id);
                self.playhead = resolved_time;
            }
            return Ok(changed);
        }

        let value =
            if property_keyframe_count(editor.project(), focused.object_id, focused.property)? == 0
            {
                rhythm_core::property::property_base_value(
                    editor.project(),
                    focused.object_id,
                    focused.property,
                )?
            } else {
                evaluate_property_at_tick(
                    editor.project(),
                    focused.object_id,
                    focused.property,
                    resolved_tick.get() as f64,
                )?
            };

        let Some(keyframe_id) = editor.create_property_keyframe(
            focused.object_id,
            focused.property,
            resolved_tick,
            value,
        )?
        else {
            return Ok(false);
        };

        self.playhead = resolved_time;
        self.select_only_keyframe(keyframe_id);
        Ok(true)
    }

    pub fn begin_keyframe_drag(
        &mut self,
        anchor_keyframe_id: KeyframeId,
        members: Vec<KeyframeDragMember>,
    ) -> bool {
        let Some(anchor) = members
            .iter()
            .find(|member| member.keyframe_id == anchor_keyframe_id)
            .copied()
        else {
            return false;
        };

        self.keyframe_drag = Some(KeyframeDrag {
            anchor_keyframe_id,
            anchor_original_tick: anchor.original_tick,
            anchor_target_tick: anchor.original_tick,
            members,
        });
        self.pending_keyframe_move = None;
        true
    }

    pub fn update_keyframe_drag(&mut self, keyframe_id: KeyframeId, target_tick: MusicalTick) {
        let Some(drag) = self.keyframe_drag.as_mut() else {
            return;
        };
        if drag.anchor_keyframe_id != keyframe_id {
            return;
        }

        let delta = target_tick.get() - drag.anchor_original_tick.get();
        if drag
            .members
            .iter()
            .all(|member| member.original_tick.get().checked_add(delta).is_some())
        {
            drag.anchor_target_tick = target_tick;
        }
    }

    #[must_use]
    pub fn keyframe_drag_preview_tick(&self, keyframe_id: KeyframeId) -> Option<MusicalTick> {
        let drag = self.keyframe_drag.as_ref()?;
        let member = drag
            .members
            .iter()
            .find(|member| member.keyframe_id == keyframe_id)?;
        let delta = drag.anchor_target_tick.get() - drag.anchor_original_tick.get();
        member
            .original_tick
            .get()
            .checked_add(delta)
            .map(MusicalTick::new)
    }

    pub fn finish_keyframe_drag(&mut self, keyframe_id: KeyframeId) -> bool {
        let Some(drag) = self.keyframe_drag.take() else {
            return false;
        };
        if drag.anchor_keyframe_id != keyframe_id {
            self.keyframe_drag = Some(drag);
            return false;
        }

        let delta = drag.anchor_target_tick.get() - drag.anchor_original_tick.get();
        if delta == 0 {
            return false;
        }

        let moves = drag
            .members
            .into_iter()
            .filter_map(|member| {
                let target_tick = member.original_tick.get().checked_add(delta)?;
                Some(PropertyKeyframeMove {
                    object_id: member.object_id,
                    property: member.property,
                    keyframe_id: member.keyframe_id,
                    target_tick: MusicalTick::new(target_tick),
                })
            })
            .collect();

        self.pending_keyframe_move = Some(PendingKeyframeMove { moves });
        true
    }

    pub fn cancel_keyframe_drag(&mut self) -> bool {
        let had_drag = self.keyframe_drag.take().is_some();
        self.pending_keyframe_move = None;
        had_drag
    }

    pub fn move_selected_keyframes_by_ticks(
        &mut self,
        editor: &mut ProjectEditor,
        delta_ticks: i64,
    ) -> Result<bool, EditError> {
        if delta_ticks == 0 || self.selected_keyframes.is_empty() {
            return Ok(false);
        }

        let mut moves = Vec::with_capacity(self.selected_keyframes.len());
        for keyframe_id in self.selected_keyframe_ids() {
            let Some(located) = locate_property_keyframe(editor.project(), keyframe_id) else {
                return Err(EditError::KeyframeNotFound(keyframe_id));
            };
            let target_tick = located
                .keyframe
                .tick
                .get()
                .checked_add(delta_ticks)
                .ok_or(EditError::HistoryInvariant("keyframe tick overflow"))?;
            moves.push(PropertyKeyframeMove {
                object_id: located.object_id,
                property: located.property,
                keyframe_id,
                target_tick: MusicalTick::new(target_tick),
            });
        }

        editor.move_property_keyframes(moves)
    }

    pub fn move_selected_keyframes_by_grid(
        &mut self,
        editor: &mut ProjectEditor,
        direction: i64,
    ) -> Result<bool, EditError> {
        let delta = self
            .authoring_division
            .ticks_per_step()
            .checked_mul(direction)
            .ok_or(EditError::HistoryInvariant("grid movement overflow"))?;
        self.move_selected_keyframes_by_ticks(editor, delta)
    }

    pub fn move_selected_keyframes_by_beat(
        &mut self,
        editor: &mut ProjectEditor,
        direction: i64,
    ) -> Result<bool, EditError> {
        let delta = PPQ
            .checked_mul(direction)
            .ok_or(EditError::HistoryInvariant("beat movement overflow"))?;
        self.move_selected_keyframes_by_ticks(editor, delta)
    }

    pub fn copy_selected_keyframes(&mut self, project: &rhythm_core::project::Project) -> bool {
        let mut located = Vec::with_capacity(self.selected_keyframes.len());
        for keyframe_id in self.selected_keyframe_ids() {
            let Some(keyframe) = locate_property_keyframe(project, keyframe_id) else {
                return false;
            };
            located.push(keyframe);
        }

        let Some(anchor_tick) = located
            .iter()
            .map(|located| located.keyframe.tick.get())
            .min()
        else {
            return false;
        };

        let mut entries: Vec<_> = located
            .into_iter()
            .map(|located| KeyframeCopyEntry {
                source_object_id: located.object_id,
                source_property: located.property,
                relative_tick: located.keyframe.tick.get() - anchor_tick,
                value: located.keyframe.value,
                interpolation: located.keyframe.interpolation,
            })
            .collect();

        entries.sort_by_key(|entry| {
            (
                entry.relative_tick,
                entry.source_object_id.get(),
                format!("{:?}", entry.source_property),
            )
        });

        self.keyframe_clipboard = Some(KeyframeCopyPacket { entries });
        true
    }

    #[must_use]
    pub fn keyframe_clipboard(&self) -> Option<&KeyframeCopyPacket> {
        self.keyframe_clipboard.as_ref()
    }

    pub fn delete_selected_keyframes(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let selected = self.selected_keyframe_ids();
        if selected.is_empty() {
            return Ok(false);
        }

        let changed = editor.delete_property_keyframes(selected)?;
        if changed {
            self.clear_keyframe_selection();
        }
        Ok(changed)
    }

    pub fn commit_pending_keyframe_move(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(pending) = self.pending_keyframe_move.take() else {
            return Ok(false);
        };

        editor.move_property_keyframes(pending.moves)
    }

    #[must_use]
    pub fn selected_keyframe_ids(&self) -> Vec<KeyframeId> {
        self.selected_keyframes.iter().copied().collect()
    }

    #[must_use]
    pub fn is_keyframe_selected(&self, keyframe_id: KeyframeId) -> bool {
        self.selected_keyframes.contains(&keyframe_id)
    }

    #[must_use]
    pub fn selected_keyframe_count(&self) -> usize {
        self.selected_keyframes.len()
    }

    pub fn select_only_keyframe(&mut self, keyframe_id: KeyframeId) {
        self.selected_keyframes.clear();
        self.selected_keyframes.insert(keyframe_id);
    }

    pub fn toggle_keyframe_selection(&mut self, keyframe_id: KeyframeId) {
        if !self.selected_keyframes.remove(&keyframe_id) {
            self.selected_keyframes.insert(keyframe_id);
        }
    }

    pub fn clear_keyframe_selection(&mut self) {
        self.selected_keyframes.clear();
    }

    pub fn replace_keyframe_selection(
        &mut self,
        keyframe_ids: impl IntoIterator<Item = KeyframeId>,
    ) {
        self.selected_keyframes.clear();
        self.selected_keyframes.extend(keyframe_ids);
    }

    pub fn toggle_keyframe_selection_many(
        &mut self,
        keyframe_ids: impl IntoIterator<Item = KeyframeId>,
    ) {
        for keyframe_id in keyframe_ids {
            self.toggle_keyframe_selection(keyframe_id);
        }
    }

    pub fn begin_timeline_box_selection(&mut self, start: [f32; 2], ctrl_toggle: bool) {
        self.timeline_box_selection = Some(TimelineBoxSelection {
            start,
            current: start,
            ctrl_toggle,
        });
    }

    pub fn update_timeline_box_selection(&mut self, current: [f32; 2]) {
        if let Some(selection) = self.timeline_box_selection.as_mut() {
            selection.current = current;
        }
    }

    #[must_use]
    pub fn timeline_box_selection(&self) -> Option<([f32; 2], [f32; 2], bool)> {
        self.timeline_box_selection
            .map(|selection| (selection.start, selection.current, selection.ctrl_toggle))
    }

    pub fn take_timeline_box_selection(&mut self) -> Option<([f32; 2], [f32; 2], bool)> {
        self.timeline_box_selection
            .take()
            .map(|selection| (selection.start, selection.current, selection.ctrl_toggle))
    }

    pub const fn set_authoring_division(&mut self, division: BeatDivision) {
        self.authoring_division = division;
    }

    pub fn step_playhead_grid(
        &mut self,
        tempo_map: &TempoMap,
        duration: DurationNs,
        direction: i64,
    ) -> bool {
        self.step_playhead_by_ticks(
            tempo_map,
            duration,
            self.authoring_division.ticks_per_step(),
            direction,
        )
    }

    pub fn step_playhead_beat(
        &mut self,
        tempo_map: &TempoMap,
        duration: DurationNs,
        direction: i64,
    ) -> bool {
        self.step_playhead_by_ticks(tempo_map, duration, PPQ, direction)
    }

    pub fn step_playhead_bar(
        &mut self,
        tempo_map: &TempoMap,
        duration: DurationNs,
        direction: i64,
    ) -> bool {
        let Some(segment) = tempo_map.initial_segment() else {
            return false;
        };
        let meter = segment.meter();
        let numerator = i64::from(meter.numerator());
        let denominator = i64::from(meter.denominator());
        let Some(ticks_per_bar) = PPQ
            .checked_mul(numerator)
            .and_then(|value| value.checked_mul(4))
            .map(|value| value / denominator)
        else {
            return false;
        };
        if ticks_per_bar <= 0 {
            return false;
        }

        self.step_playhead_by_ticks(tempo_map, duration, ticks_per_bar, direction)
    }

    #[must_use]
    pub fn timeline_range(&self, duration: DurationNs) -> (ProjectTimeNs, ProjectTimeNs) {
        let duration_ns = i64::try_from(duration.get()).unwrap_or(i64::MAX).max(1);
        let Some(view) = self.timeline_view else {
            return (ProjectTimeNs::new(0), ProjectTimeNs::new(duration_ns));
        };

        let span = (view.end.get() - view.start.get()).clamp(1, duration_ns);
        let start = view.start.get().clamp(0, duration_ns - span);
        (
            ProjectTimeNs::new(start),
            ProjectTimeNs::new(start.saturating_add(span)),
        )
    }

    pub fn zoom_timeline(
        &mut self,
        duration: DurationNs,
        anchor: ProjectTimeNs,
        zoom_factor: f32,
    ) -> bool {
        if !zoom_factor.is_finite() || zoom_factor <= 0.0 || (zoom_factor - 1.0).abs() < 0.000_1 {
            return false;
        }

        let duration_ns = i64::try_from(duration.get()).unwrap_or(i64::MAX).max(1);
        let (start, end) = self.timeline_range(duration);
        let old_span = (end.get() - start.get()).max(1);
        const MIN_TIMELINE_SPAN_NS: i64 = 1_000_000;
        let min_span = MIN_TIMELINE_SPAN_NS.min(duration_ns);
        let new_span = ((old_span as f64) / f64::from(zoom_factor))
            .round()
            .clamp(min_span as f64, duration_ns as f64) as i64;
        if new_span == old_span {
            return false;
        }

        let anchor_ns = anchor.get().clamp(start.get(), end.get());
        let anchor_ratio = (anchor_ns - start.get()) as f64 / old_span as f64;
        let desired_start = (anchor_ns as f64 - anchor_ratio * new_span as f64).round() as i64;
        let new_start = desired_start.clamp(0, duration_ns - new_span);
        let new_end = new_start.saturating_add(new_span);
        self.timeline_view = Some(TimelineView {
            start: ProjectTimeNs::new(new_start),
            end: ProjectTimeNs::new(new_end),
        });
        true
    }

    pub fn pan_timeline_points(
        &mut self,
        duration: DurationNs,
        content_delta_points: f32,
        viewport_width_points: f32,
    ) -> bool {
        if !content_delta_points.is_finite()
            || !viewport_width_points.is_finite()
            || viewport_width_points <= 0.0
            || content_delta_points.abs() < f32::EPSILON
        {
            return false;
        }

        let duration_ns = i64::try_from(duration.get()).unwrap_or(i64::MAX).max(1);
        let (start, end) = self.timeline_range(duration);
        let span = (end.get() - start.get()).max(1);
        if span >= duration_ns {
            return false;
        }

        let delta_ns = (-(f64::from(content_delta_points)) / f64::from(viewport_width_points)
            * span as f64)
            .round() as i64;
        if delta_ns == 0 {
            return false;
        }

        let new_start = start
            .get()
            .saturating_add(delta_ns)
            .clamp(0, duration_ns - span);
        if new_start == start.get() {
            return false;
        }

        self.timeline_view = Some(TimelineView {
            start: ProjectTimeNs::new(new_start),
            end: ProjectTimeNs::new(new_start.saturating_add(span)),
        });
        true
    }

    pub fn change_authoring_division(&mut self, finer: bool) -> bool {
        let current = self.authoring_division.parts_per_beat();
        let Some(index) = MVP_BEAT_DIVISIONS
            .iter()
            .position(|division| *division == current)
        else {
            return false;
        };

        let next_index = if finer {
            index
                .checked_add(1)
                .filter(|next| *next < MVP_BEAT_DIVISIONS.len())
        } else {
            index.checked_sub(1)
        };
        let Some(next_index) = next_index else {
            return false;
        };
        let Some(division) = BeatDivision::new(MVP_BEAT_DIVISIONS[next_index]) else {
            return false;
        };

        self.authoring_division = division;
        true
    }

    fn step_playhead_by_ticks(
        &mut self,
        tempo_map: &TempoMap,
        duration: DurationNs,
        tick_step: i64,
        direction: i64,
    ) -> bool {
        if tick_step <= 0 || direction == 0 {
            return false;
        }

        let Ok(position) = tempo_map.continuous_tick_position(self.playhead) else {
            return false;
        };
        let step = tick_step as f64;
        let target = if direction > 0 {
            ((position / step).floor() + 1.0) * step
        } else {
            ((position / step).ceil() - 1.0) * step
        };
        if !target.is_finite() || target < i64::MIN as f64 || target > i64::MAX as f64 {
            return false;
        }

        let Ok(project_time) = tempo_map.project_time_for_tick(MusicalTick::new(target as i64))
        else {
            return false;
        };
        let duration_ns = i64::try_from(duration.get()).unwrap_or(i64::MAX);
        let clamped = ProjectTimeNs::new(project_time.get().clamp(0, duration_ns));
        let changed = clamped != self.playhead;
        self.playhead = clamped;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorSession, PreviewQuality};
    use rhythm_core::time::{
        BeatDivision, BpmMicros, DurationNs, GridOffsetNs, ProjectTimeNs, TempoMap, TimeSignature,
    };

    fn tempo_120() -> TempoMap {
        TempoMap::with_initial_tempo(
            GridOffsetNs::new(0),
            BpmMicros::new(120_000_000).expect("BPM"),
            TimeSignature::default(),
        )
    }

    fn editor_with_object_for_drag() -> rhythm_core::editor::ProjectEditor {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            ids::ObjectId,
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, TempoMap},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = Project::new(
            "Drag Test",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id: object_id,
            name: "Rectangle".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 100.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 2;
        rhythm_core::editor::ProjectEditor::new(project).expect("valid project")
    }

    #[test]
    fn box_selection_replace_and_ctrl_toggle_are_applied_on_commit() {
        let first = rhythm_core::ids::KeyframeId::new(21).expect("key id");
        let second = rhythm_core::ids::KeyframeId::new(22).expect("key id");
        let third = rhythm_core::ids::KeyframeId::new(23).expect("key id");
        let mut session = EditorSession::default();

        session.replace_keyframe_selection([first, second]);
        assert_eq!(session.selected_keyframe_count(), 2);

        session.toggle_keyframe_selection_many([second, third]);
        assert!(session.is_keyframe_selected(first));
        assert!(!session.is_keyframe_selected(second));
        assert!(session.is_keyframe_selected(third));

        session.begin_timeline_box_selection([10.0, 20.0], true);
        session.update_timeline_box_selection([30.0, 40.0]);
        assert_eq!(
            session.take_timeline_box_selection(),
            Some(([10.0, 20.0], [30.0, 40.0], true))
        );
        assert!(session.timeline_box_selection().is_none());
    }

    #[test]
    fn single_and_ctrl_toggle_keyframe_selection_use_stable_ids() {
        let first = rhythm_core::ids::KeyframeId::new(11).expect("key id");
        let second = rhythm_core::ids::KeyframeId::new(12).expect("key id");
        let mut session = EditorSession::default();

        session.select_only_keyframe(first);
        assert!(session.is_keyframe_selected(first));
        assert_eq!(session.selected_keyframe_count(), 1);

        session.select_only_keyframe(second);
        assert!(!session.is_keyframe_selected(first));
        assert!(session.is_keyframe_selected(second));

        session.toggle_keyframe_selection(first);
        assert!(session.is_keyframe_selected(first));
        assert!(session.is_keyframe_selected(second));
        assert_eq!(session.selected_keyframe_count(), 2);

        session.toggle_keyframe_selection(second);
        assert!(!session.is_keyframe_selected(second));
        assert_eq!(session.selected_keyframe_count(), 1);
    }

    #[test]
    fn timeline_zoom_keeps_pointer_anchor_stable() {
        let duration = DurationNs::new(10_000_000_000);
        let mut session = EditorSession::default();

        assert!(session.zoom_timeline(duration, ProjectTimeNs::new(5_000_000_000), 2.0,));
        assert_eq!(
            session.timeline_range(duration),
            (
                ProjectTimeNs::new(2_500_000_000),
                ProjectTimeNs::new(7_500_000_000),
            )
        );
    }

    #[test]
    fn timeline_pan_uses_content_motion_and_clamps_to_project() {
        let duration = DurationNs::new(10_000_000_000);
        let mut session = EditorSession::default();
        assert!(session.zoom_timeline(duration, ProjectTimeNs::new(5_000_000_000), 2.0,));

        assert!(session.pan_timeline_points(duration, -100.0, 1_000.0));
        assert_eq!(
            session.timeline_range(duration),
            (
                ProjectTimeNs::new(3_000_000_000),
                ProjectTimeNs::new(8_000_000_000),
            )
        );

        assert!(session.pan_timeline_points(duration, 10_000.0, 1_000.0));
        assert_eq!(
            session.timeline_range(duration),
            (ProjectTimeNs::new(0), ProjectTimeNs::new(5_000_000_000),)
        );
    }

    #[test]
    fn copy_packet_preserves_values_easing_and_relative_ticks() {
        use rhythm_core::{
            animation::Interpolation,
            property::{AnimatableProperty, PropertyValue},
            time::MusicalTick,
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let first = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(240),
                PropertyValue::Scalar(0.25),
            )
            .expect("insert")
            .expect("first");
        let second = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(720),
                PropertyValue::Scalar(0.75),
            )
            .expect("insert")
            .expect("second");

        let mut session = EditorSession::default();
        session.replace_keyframe_selection([second, first]);

        assert!(session.copy_selected_keyframes(editor.project()));
        let packet = session.keyframe_clipboard().expect("copy packet");
        assert_eq!(packet.entries.len(), 2);
        assert_eq!(packet.entries[0].relative_tick, 0);
        assert_eq!(packet.entries[0].value, PropertyValue::Scalar(0.25));
        assert_eq!(packet.entries[0].interpolation, Interpolation::Linear);
        assert_eq!(packet.entries[1].relative_tick, 480);
        assert_eq!(packet.entries[1].value, PropertyValue::Scalar(0.75));
    }

    #[test]
    fn selected_key_keyboard_moves_use_exact_grid_and_beat_ticks() {
        use rhythm_core::{
            property::{AnimatableProperty, property_keyframe_at_tick},
            time::MusicalTick,
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let keyframe_id = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(0),
                rhythm_core::property::PropertyValue::Scalar(0.5),
            )
            .expect("insert key")
            .expect("key id");

        let mut session = EditorSession::default();
        session.select_only_keyframe(keyframe_id);

        assert_eq!(
            session.move_selected_keyframes_by_grid(&mut editor, 1),
            Ok(true)
        );
        assert!(
            property_keyframe_at_tick(
                editor.project(),
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(240),
            )
            .expect("property")
            .is_some()
        );

        assert_eq!(
            session.move_selected_keyframes_by_beat(&mut editor, 1),
            Ok(true)
        );
        assert!(
            property_keyframe_at_tick(
                editor.project(),
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(1_200),
            )
            .expect("property")
            .is_some()
        );
        assert!(session.is_keyframe_selected(keyframe_id));
    }

    #[test]
    fn multi_key_drag_applies_anchor_delta_to_every_member() {
        let object_id = ObjectId::new(1).expect("object id");
        let first = KeyframeId::new(2).expect("key id");
        let second = KeyframeId::new(3).expect("key id");
        let mut session = EditorSession::default();

        assert!(session.begin_keyframe_drag(
            first,
            vec![
                KeyframeDragMember {
                    object_id,
                    property: AnimatableProperty::Opacity,
                    keyframe_id: first,
                    original_tick: MusicalTick::new(0),
                },
                KeyframeDragMember {
                    object_id,
                    property: AnimatableProperty::Opacity,
                    keyframe_id: second,
                    original_tick: MusicalTick::new(240),
                },
            ],
        ));
        session.update_keyframe_drag(first, MusicalTick::new(480));

        assert_eq!(
            session.keyframe_drag_preview_tick(first),
            Some(MusicalTick::new(480))
        );
        assert_eq!(
            session.keyframe_drag_preview_tick(second),
            Some(MusicalTick::new(720))
        );
        assert!(session.finish_keyframe_drag(first));
    }

    #[test]
    fn keyframe_drag_preview_can_be_cancelled_without_project_mutation() {
        let object_id = ObjectId::new(1).expect("object id");
        let keyframe_id = KeyframeId::new(2).expect("key id");
        let mut session = EditorSession::default();

        assert!(session.begin_keyframe_drag(
            keyframe_id,
            vec![KeyframeDragMember {
                object_id,
                property: AnimatableProperty::Opacity,
                keyframe_id,
                original_tick: MusicalTick::new(0),
            }],
        ));
        session.update_keyframe_drag(keyframe_id, MusicalTick::new(240));

        assert_eq!(
            session.keyframe_drag_preview_tick(keyframe_id),
            Some(MusicalTick::new(240))
        );
        assert!(session.cancel_keyframe_drag());
        assert_eq!(session.keyframe_drag_preview_tick(keyframe_id), None);
        assert_eq!(
            session.commit_pending_keyframe_move(&mut editor_with_object_for_drag()),
            Ok(false)
        );
    }

    #[test]
    fn k_on_existing_resolved_key_removes_it() {
        use rhythm_core::{
            animation::{Animated, Interpolation, Keyframe},
            domain::{LinearRgba, Vec2},
            ids::{KeyframeId, ObjectId},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            property::{AnimatableProperty, property_base_value, property_keyframe_count},
            time::{BpmMicros, GridOffsetNs, MusicalTick, TempoMap, TimeSignature},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = Project::new(
            "Remove Key",
            ProjectSettings::default(),
            TempoMap::with_initial_tempo(
                GridOffsetNs::new(0),
                BpmMicros::new(120_000_000).expect("valid BPM"),
                TimeSignature::default(),
            ),
        );
        project.composition.objects.push(Object {
            id: object_id,
            name: "Rectangle".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::with_keyframes(
                    1.0,
                    vec![Keyframe::new(
                        KeyframeId::new(2).expect("key id"),
                        MusicalTick::new(240),
                        0.4,
                        Interpolation::Linear,
                    )],
                )
                .expect("keys"),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 100.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 3;

        let mut editor = rhythm_core::editor::ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(120_000_000));
        session.focus_property(object_id, AnimatableProperty::Opacity);

        assert_eq!(session.keyframe_action(&mut editor), Ok(true));
        assert_eq!(
            property_keyframe_count(editor.project(), object_id, AnimatableProperty::Opacity),
            Ok(0)
        );
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Opacity),
            Ok(rhythm_core::property::PropertyValue::Scalar(0.4))
        );
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
    }

    #[test]
    fn k_on_animated_property_without_key_inserts_evaluated_resolved_value() {
        use rhythm_core::{
            animation::{Animated, Interpolation, Keyframe},
            domain::{LinearRgba, Vec2},
            ids::{KeyframeId, ObjectId},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            property::{AnimatableProperty, PropertyValue, property_keyframe_at_tick},
            time::{BpmMicros, GridOffsetNs, MusicalTick, TempoMap, TimeSignature},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = Project::new(
            "Insert Evaluated Key",
            ProjectSettings::default(),
            TempoMap::with_initial_tempo(
                GridOffsetNs::new(0),
                BpmMicros::new(120_000_000).expect("valid BPM"),
                TimeSignature::default(),
            ),
        );
        project.composition.objects.push(Object {
            id: object_id,
            name: "Rectangle".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::with_keyframes(
                    Vec2::new(0.0, 0.0).expect("base"),
                    vec![
                        Keyframe::new(
                            KeyframeId::new(2).expect("key id"),
                            MusicalTick::new(0),
                            Vec2::new(0.0, 0.0).expect("position"),
                            Interpolation::Linear,
                        ),
                        Keyframe::new(
                            KeyframeId::new(3).expect("key id"),
                            MusicalTick::new(480),
                            Vec2::new(100.0, 0.0).expect("position"),
                            Interpolation::Linear,
                        ),
                    ],
                )
                .expect("keys"),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 100.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 4;

        let mut editor = rhythm_core::editor::ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(130_000_000));
        session.focus_property(object_id, AnimatableProperty::Position);

        assert_eq!(session.keyframe_action(&mut editor), Ok(true));
        let keyframe = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Position,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("inserted key");
        assert_eq!(
            keyframe.value,
            PropertyValue::Vec2(Vec2::new(50.0, 0.0).expect("midpoint"))
        );
        assert_eq!(keyframe.id.get(), 4);
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
    }

    #[test]
    fn k_on_static_focused_property_creates_first_key_and_moves_playhead() {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            ids::ObjectId,
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            property::{AnimatableProperty, property_keyframe_at_tick},
            time::{BpmMicros, GridOffsetNs, MusicalTick, TempoMap, TimeSignature},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = Project::new(
            "Key Test",
            ProjectSettings::default(),
            TempoMap::with_initial_tempo(
                GridOffsetNs::new(0),
                BpmMicros::new(120_000_000).expect("valid BPM"),
                TimeSignature::default(),
            ),
        );
        project.composition.objects.push(Object {
            id: object_id,
            name: "Rectangle".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(10.0, 20.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 100.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 2;

        let mut editor = rhythm_core::editor::ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(70_000_000));
        session.focus_property(object_id, AnimatableProperty::Position);

        assert_eq!(session.keyframe_action(&mut editor), Ok(true));
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        let keyframe = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Position,
            MusicalTick::new(240),
        )
        .expect("property access")
        .expect("first keyframe");
        assert_eq!(keyframe.id.get(), 2);
        assert!(session.is_keyframe_selected(keyframe.id));
    }

    #[test]
    fn playhead_keyboard_steps_follow_grid_beat_and_bar() {
        let tempo = tempo_120();
        let duration = DurationNs::new(10_000_000_000);
        let mut session = EditorSession::default();

        assert!(session.step_playhead_grid(&tempo, duration, 1));
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        assert!(session.step_playhead_grid(&tempo, duration, -1));
        assert_eq!(session.playhead(), ProjectTimeNs::new(0));

        assert!(session.step_playhead_beat(&tempo, duration, 1));
        assert_eq!(session.playhead(), ProjectTimeNs::new(500_000_000));

        session.seek_paused(ProjectTimeNs::new(0));
        assert!(session.step_playhead_bar(&tempo, duration, 1));
        assert_eq!(session.playhead(), ProjectTimeNs::new(2_000_000_000));
    }

    #[test]
    fn playhead_step_from_off_grid_moves_to_next_boundary() {
        let tempo = tempo_120();
        let duration = DurationNs::new(10_000_000_000);
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(100_000_000));

        assert!(session.step_playhead_grid(&tempo, duration, 1));
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));

        session.seek_paused(ProjectTimeNs::new(100_000_000));
        assert!(session.step_playhead_grid(&tempo, duration, -1));
        assert_eq!(session.playhead(), ProjectTimeNs::new(0));
    }

    #[test]
    fn authoring_division_moves_through_accepted_mvp_set() {
        let mut session = EditorSession::default();
        assert_eq!(session.authoring_division().parts_per_beat(), 4);

        assert!(session.change_authoring_division(true));
        assert_eq!(session.authoring_division().parts_per_beat(), 6);
        assert!(session.change_authoring_division(false));
        assert_eq!(session.authoring_division().parts_per_beat(), 4);
    }

    #[test]
    fn paused_seek_updates_only_session_playhead() {
        let mut session = EditorSession::default();
        assert_eq!(session.playhead().get(), 0);

        session.seek_paused(ProjectTimeNs::new(1_250_000_000));

        assert_eq!(session.playhead().get(), 1_250_000_000);
    }

    #[test]
    fn authoring_grid_defaults_to_one_quarter_beat() {
        let mut session = EditorSession::default();
        assert_eq!(session.authoring_division().parts_per_beat(), 4);

        session.set_authoring_division(BeatDivision::new(16).expect("accepted division"));
        assert_eq!(session.authoring_division().parts_per_beat(), 16);
    }

    #[test]
    fn preview_quality_defaults_to_auto() {
        assert_eq!(
            EditorSession::default().preview_quality,
            PreviewQuality::Auto
        );
    }

    #[test]
    fn preview_quality_contains_all_mvp_modes() {
        let modes = [
            PreviewQuality::Auto,
            PreviewQuality::Full,
            PreviewQuality::Half,
            PreviewQuality::Quarter,
        ];

        assert_eq!(modes.len(), 4);
    }
}
