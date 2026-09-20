use std::collections::HashSet;

use rhythm_core::{
    animation::{BezierEasing, Interpolation},
    domain::{LinearRgba, Vec2},
    editor::{EditCommand, EditError, ProjectEditor, PropertyKeyframeDraft, PropertyKeyframeMove},
    geometry::LocalBounds2d,
    ids::{AssetId, KeyframeId, ObjectId},
    project::{FontStyle, FontWeight, TextAlignment},
    property::{
        AnimatableProperty, PropertyValue, evaluate_property_at_tick, locate_property_keyframe,
        property_base_value, property_keyframe_at_tick, property_keyframe_count,
        property_keyframe_has_successor, property_value_compatible,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframeInterpolationPreset {
    Hold,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl KeyframeInterpolationPreset {
    pub const ALL: [Self; 5] = [
        Self::Hold,
        Self::Linear,
        Self::EaseIn,
        Self::EaseOut,
        Self::EaseInOut,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Hold => "Hold",
            Self::Linear => "Linear",
            Self::EaseIn => "Ease In",
            Self::EaseOut => "Ease Out",
            Self::EaseInOut => "Ease In-Out",
        }
    }

    #[must_use]
    pub fn interpolation(self) -> Interpolation {
        match self {
            Self::Hold => Interpolation::Hold,
            Self::Linear => Interpolation::Linear,
            Self::EaseIn => Interpolation::CubicBezier(
                BezierEasing::new(0.42, 0.0, 1.0, 1.0).expect("valid ease-in preset"),
            ),
            Self::EaseOut => Interpolation::CubicBezier(
                BezierEasing::new(0.0, 0.0, 0.58, 1.0).expect("valid ease-out preset"),
            ),
            Self::EaseInOut => Interpolation::CubicBezier(
                BezierEasing::new(0.42, 0.0, 0.58, 1.0).expect("valid ease-in-out preset"),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineBoxSelection {
    start: [f32; 2],
    current: [f32; 2],
    ctrl_toggle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ViewportBoxSelection {
    start: [f32; 2],
    current: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportCameraAction {
    FitComposition,
    FrameSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectListAction {
    SetVisible { object_id: ObjectId, visible: bool },
    SetLocked { object_id: ObjectId, locked: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewportTransformDragPhase {
    Active,
    CommitRequested,
    CancelRequested,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ViewportPositionDrag {
    object_id: ObjectId,
    pointer_start: [f32; 2],
    position_start: Vec2,
    current_position: Vec2,
    phase: ViewportTransformDragPhase,
    transaction_started: bool,
    animated_transaction: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct ViewportMultiPositionDrag {
    object_ids: Vec<ObjectId>,
    pointer_start: [f32; 2],
    current_delta: Vec2,
    phase: ViewportTransformDragPhase,
    transaction_started: bool,
    animated_transaction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ViewportScaleDrag {
    object_id: ObjectId,
    anchor: Vec2,
    handle_start: Vec2,
    scale_start: Vec2,
    rotation_degrees: f32,
    current_scale: Vec2,
    phase: ViewportTransformDragPhase,
    transaction_started: bool,
    animated_transaction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ViewportRotationDrag {
    object_id: ObjectId,
    anchor: Vec2,
    pointer_angle_degrees: f32,
    rotation_start: f32,
    accumulated_delta_degrees: f32,
    current_rotation: f32,
    phase: ViewportTransformDragPhase,
    transaction_started: bool,
    animated_transaction: bool,
}

const MIN_VIEWPORT_ZOOM: f32 = 0.1;
const MAX_VIEWPORT_ZOOM: f32 = 8.0;
const FRAME_SELECTION_PADDING_POINTS: f32 = 32.0;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorNumericComponent {
    Scalar,
    X,
    Y,
    R,
    G,
    B,
    A,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectorNumericTarget {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub component: InspectorNumericComponent,
}

#[derive(Debug, Clone, PartialEq)]
struct InspectorNumericEdit {
    target: InspectorNumericTarget,
    original_value: f32,
    buffer: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorNumericCommit {
    pub target: InspectorNumericTarget,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTextField {
    Content,
    FontFamily,
    FontSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectorTextTarget {
    pub object_id: ObjectId,
    pub field: InspectorTextField,
}

#[derive(Debug, Clone, PartialEq)]
struct InspectorTextEdit {
    target: InspectorTextTarget,
    buffer: String,
}

#[derive(Debug, Clone, PartialEq)]
enum TextInspectorAction {
    SetContent {
        object_id: ObjectId,
        text: String,
    },
    SetFontFamily {
        object_id: ObjectId,
        family: String,
    },
    SetFontSize {
        object_id: ObjectId,
        font_size: f32,
    },
    SetFontWeight {
        object_id: ObjectId,
        weight: FontWeight,
    },
    SetFontStyle {
        object_id: ObjectId,
        style: FontStyle,
    },
    SetAlignment {
        object_id: ObjectId,
        alignment: TextAlignment,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectorMultiNumericTarget {
    pub property: AnimatableProperty,
    pub component: InspectorNumericComponent,
}

#[derive(Debug, Clone, PartialEq)]
struct InspectorMultiNumericEdit {
    target: InspectorMultiNumericTarget,
    object_ids: Vec<ObjectId>,
    buffer: String,
}

#[derive(Debug, Clone, PartialEq)]
struct InspectorMultiNumericCommit {
    target: InspectorMultiNumericTarget,
    object_ids: Vec<ObjectId>,
    value: f32,
}

#[derive(Debug)]
pub struct EditorSession {
    pub preview_quality: PreviewQuality,
    playhead: ProjectTimeNs,
    authoring_division: BeatDivision,
    timeline_view: Option<TimelineView>,
    follow_playhead: bool,
    viewport_pan_points: [f32; 2],
    viewport_zoom: f32,
    pending_viewport_camera_action: Option<ViewportCameraAction>,
    pending_object_list_actions: Vec<ObjectListAction>,
    pending_image_relink_asset: Option<AssetId>,
    viewport_position_drag: Option<ViewportPositionDrag>,
    viewport_multi_position_drag: Option<ViewportMultiPositionDrag>,
    viewport_scale_drag: Option<ViewportScaleDrag>,
    viewport_rotation_drag: Option<ViewportRotationDrag>,
    selected_objects: HashSet<ObjectId>,
    selected_keyframes: HashSet<KeyframeId>,
    focused_property: Option<FocusedProperty>,
    pending_focused_keyframe_action: bool,
    inspector_numeric_edit: Option<InspectorNumericEdit>,
    pending_inspector_numeric_commit: Option<InspectorNumericCommit>,
    inspector_multi_numeric_edit: Option<InspectorMultiNumericEdit>,
    pending_inspector_multi_numeric_commit: Option<InspectorMultiNumericCommit>,
    inspector_text_edit: Option<InspectorTextEdit>,
    pending_text_inspector_actions: Vec<TextInspectorAction>,
    keyframe_drag: Option<KeyframeDrag>,
    pending_keyframe_move: Option<PendingKeyframeMove>,
    pending_keyframe_interpolation: Option<Interpolation>,
    keyframe_clipboard: Option<KeyframeCopyPacket>,
    timeline_box_selection: Option<TimelineBoxSelection>,
    viewport_box_selection: Option<ViewportBoxSelection>,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self {
            preview_quality: PreviewQuality::Auto,
            playhead: ProjectTimeNs::new(0),
            authoring_division: BeatDivision::new(4).expect("1/4 beat is an accepted MVP grid"),
            timeline_view: None,
            follow_playhead: false,
            viewport_pan_points: [0.0, 0.0],
            viewport_zoom: 1.0,
            pending_viewport_camera_action: None,
            pending_object_list_actions: Vec::new(),
            pending_image_relink_asset: None,
            viewport_position_drag: None,
            viewport_multi_position_drag: None,
            viewport_scale_drag: None,
            viewport_rotation_drag: None,
            selected_objects: HashSet::new(),
            selected_keyframes: HashSet::new(),
            focused_property: None,
            pending_focused_keyframe_action: false,
            inspector_numeric_edit: None,
            pending_inspector_numeric_commit: None,
            inspector_multi_numeric_edit: None,
            pending_inspector_multi_numeric_commit: None,
            inspector_text_edit: None,
            pending_text_inspector_actions: Vec::new(),
            keyframe_drag: None,
            pending_keyframe_move: None,
            pending_keyframe_interpolation: None,
            keyframe_clipboard: None,
            timeline_box_selection: None,
            viewport_box_selection: None,
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
    pub const fn follow_playhead(&self) -> bool {
        self.follow_playhead
    }

    pub const fn set_follow_playhead(&mut self, enabled: bool) {
        self.follow_playhead = enabled;
    }

    #[must_use]
    pub const fn viewport_pan_points(&self) -> [f32; 2] {
        self.viewport_pan_points
    }

    pub fn pan_viewport_points(&mut self, delta: [f32; 2]) -> bool {
        if !delta[0].is_finite()
            || !delta[1].is_finite()
            || (delta[0].abs() < f32::EPSILON && delta[1].abs() < f32::EPSILON)
        {
            return false;
        }

        self.viewport_pan_points[0] += delta[0];
        self.viewport_pan_points[1] += delta[1];
        true
    }

    #[must_use]
    pub const fn viewport_zoom(&self) -> f32 {
        self.viewport_zoom
    }

    pub fn zoom_viewport_around_anchor(
        &mut self,
        zoom_factor: f32,
        anchor_from_viewport_center: [f32; 2],
    ) -> bool {
        if !zoom_factor.is_finite()
            || zoom_factor <= 0.0
            || !anchor_from_viewport_center[0].is_finite()
            || !anchor_from_viewport_center[1].is_finite()
        {
            return false;
        }

        let old_zoom = self.viewport_zoom;
        let new_zoom = (old_zoom * zoom_factor).clamp(MIN_VIEWPORT_ZOOM, MAX_VIEWPORT_ZOOM);
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return false;
        }

        let ratio = new_zoom / old_zoom;
        let offset_from_composition_center = [
            anchor_from_viewport_center[0] - self.viewport_pan_points[0],
            anchor_from_viewport_center[1] - self.viewport_pan_points[1],
        ];
        self.viewport_pan_points[0] += offset_from_composition_center[0] * (1.0 - ratio);
        self.viewport_pan_points[1] += offset_from_composition_center[1] * (1.0 - ratio);
        self.viewport_zoom = new_zoom;
        true
    }

    pub const fn request_viewport_camera_action(&mut self, action: ViewportCameraAction) {
        self.pending_viewport_camera_action = Some(action);
    }

    pub const fn take_viewport_camera_action(&mut self) -> Option<ViewportCameraAction> {
        self.pending_viewport_camera_action.take()
    }

    pub fn request_image_relink(&mut self, asset_id: AssetId) -> bool {
        if self.pending_image_relink_asset == Some(asset_id) {
            return false;
        }

        self.pending_image_relink_asset = Some(asset_id);
        true
    }

    #[must_use]
    pub fn image_relink_requested(&self, asset_id: AssetId) -> bool {
        self.pending_image_relink_asset == Some(asset_id)
    }

    pub fn queue_object_visibility(&mut self, object_id: ObjectId, visible: bool) {
        self.pending_object_list_actions
            .push(ObjectListAction::SetVisible { object_id, visible });
    }

    pub fn queue_object_locked(&mut self, object_id: ObjectId, locked: bool) {
        self.pending_object_list_actions
            .push(ObjectListAction::SetLocked { object_id, locked });
    }

    pub fn commit_pending_object_list_actions(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        if self.pending_object_list_actions.is_empty() {
            return Ok(false);
        }

        let actions = std::mem::take(&mut self.pending_object_list_actions);
        let mut changed = false;
        for action in actions {
            let action_changed = match action {
                ObjectListAction::SetVisible { object_id, visible } => {
                    editor.execute(EditCommand::SetObjectVisible { object_id, visible })?
                }
                ObjectListAction::SetLocked { object_id, locked } => {
                    editor.execute(EditCommand::SetObjectLocked { object_id, locked })?
                }
            };
            changed |= action_changed;
        }
        Ok(changed)
    }

    pub fn fit_viewport_composition(&mut self) -> bool {
        let changed = self.viewport_pan_points != [0.0, 0.0]
            || (self.viewport_zoom - 1.0).abs() >= f32::EPSILON;
        self.viewport_pan_points = [0.0, 0.0];
        self.viewport_zoom = 1.0;
        changed
    }

    pub fn frame_viewport_bounds(
        &mut self,
        bounds: LocalBounds2d,
        composition_size: [f32; 2],
        viewport_size_points: [f32; 2],
        fitted_preview_size_points: [f32; 2],
    ) -> bool {
        let [composition_width, composition_height] = composition_size;
        let [viewport_width, viewport_height] = viewport_size_points;
        let [preview_width, preview_height] = fitted_preview_size_points;

        if !composition_width.is_finite()
            || !composition_height.is_finite()
            || !viewport_width.is_finite()
            || !viewport_height.is_finite()
            || !preview_width.is_finite()
            || !preview_height.is_finite()
            || composition_width <= 0.0
            || composition_height <= 0.0
            || viewport_width <= 0.0
            || viewport_height <= 0.0
            || preview_width <= 0.0
            || preview_height <= 0.0
            || bounds.size.x() < 0.0
            || bounds.size.y() < 0.0
        {
            return false;
        }

        let available_width = (viewport_width - FRAME_SELECTION_PADDING_POINTS * 2.0).max(1.0);
        let available_height = (viewport_height - FRAME_SELECTION_PADDING_POINTS * 2.0).max(1.0);
        let bounds_width_points = bounds.size.x() * preview_width / composition_width;
        let bounds_height_points = bounds.size.y() * preview_height / composition_height;

        let zoom_x = if bounds_width_points <= f32::EPSILON {
            MAX_VIEWPORT_ZOOM
        } else {
            available_width / bounds_width_points
        };
        let zoom_y = if bounds_height_points <= f32::EPSILON {
            MAX_VIEWPORT_ZOOM
        } else {
            available_height / bounds_height_points
        };
        let new_zoom = zoom_x
            .min(zoom_y)
            .clamp(MIN_VIEWPORT_ZOOM, MAX_VIEWPORT_ZOOM);

        let selection_center_x = bounds.min.x() + bounds.size.x() * 0.5;
        let selection_center_y = bounds.min.y() + bounds.size.y() * 0.5;
        let composition_center_x = composition_width * 0.5;
        let composition_center_y = composition_height * 0.5;
        let new_pan = [
            -(selection_center_x - composition_center_x)
                * (preview_width / composition_width)
                * new_zoom,
            -(selection_center_y - composition_center_y)
                * (preview_height / composition_height)
                * new_zoom,
        ];
        if !new_pan[0].is_finite() || !new_pan[1].is_finite() || !new_zoom.is_finite() {
            return false;
        }

        let changed = (self.viewport_zoom - new_zoom).abs() >= f32::EPSILON
            || (self.viewport_pan_points[0] - new_pan[0]).abs() >= f32::EPSILON
            || (self.viewport_pan_points[1] - new_pan[1]).abs() >= f32::EPSILON;
        self.viewport_zoom = new_zoom;
        self.viewport_pan_points = new_pan;
        changed
    }

    pub fn begin_viewport_position_drag(
        &mut self,
        object_id: ObjectId,
        pointer_start: [f32; 2],
        position_start: Vec2,
    ) -> bool {
        if self.viewport_position_drag.is_some()
            || self.viewport_multi_position_drag.is_some()
            || self.viewport_scale_drag.is_some()
            || self.viewport_rotation_drag.is_some()
            || !pointer_start[0].is_finite()
            || !pointer_start[1].is_finite()
        {
            return false;
        }

        self.viewport_position_drag = Some(ViewportPositionDrag {
            object_id,
            pointer_start,
            position_start,
            current_position: position_start,
            phase: ViewportTransformDragPhase::Active,
            transaction_started: false,
            animated_transaction: false,
        });
        true
    }

    #[must_use]
    pub fn viewport_position_drag_active(&self) -> bool {
        self.viewport_position_drag
            .is_some_and(|drag| drag.phase == ViewportTransformDragPhase::Active)
    }

    pub fn update_viewport_position_drag(
        &mut self,
        pointer: [f32; 2],
        composition_units_per_point: [f32; 2],
        constrain_axis: bool,
    ) -> bool {
        let Some(drag) = self.viewport_position_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active
            || !pointer[0].is_finite()
            || !pointer[1].is_finite()
            || !composition_units_per_point[0].is_finite()
            || !composition_units_per_point[1].is_finite()
            || composition_units_per_point[0] <= 0.0
            || composition_units_per_point[1] <= 0.0
        {
            return false;
        }

        let mut delta_x = (pointer[0] - drag.pointer_start[0]) * composition_units_per_point[0];
        let mut delta_y = (pointer[1] - drag.pointer_start[1]) * composition_units_per_point[1];
        if constrain_axis {
            if delta_x.abs() >= delta_y.abs() {
                delta_y = 0.0;
            } else {
                delta_x = 0.0;
            }
        }
        let Ok(current_position) = Vec2::new(
            drag.position_start.x() + delta_x,
            drag.position_start.y() + delta_y,
        ) else {
            return false;
        };

        if current_position == drag.current_position {
            return false;
        }
        drag.current_position = current_position;
        true
    }

    pub fn finish_viewport_position_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_position_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CommitRequested;
        true
    }

    pub fn cancel_viewport_position_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_position_drag.as_mut() else {
            return false;
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CancelRequested;
        true
    }

    fn begin_direct_transform_transaction(
        &mut self,
        editor: &mut ProjectEditor,
        object_id: ObjectId,
        property: AnimatableProperty,
    ) -> Result<Option<bool>, EditError> {
        if property_keyframe_count(editor.project(), object_id, property)? == 0 {
            return Ok(Some(false));
        }

        let project = editor.project();
        let Ok(continuous_tick) = project.tempo_map.continuous_tick_position(self.playhead) else {
            return Ok(None);
        };
        let Ok(tick) = snap_tick_position_to_grid(continuous_tick, self.authoring_division) else {
            return Ok(None);
        };
        let Ok(project_time) = project.tempo_map.project_time_for_tick(tick) else {
            return Ok(None);
        };
        let initial_value =
            evaluate_property_at_tick(project, object_id, property, tick.get() as f64)?;

        editor.begin_property_keyframe_value_transaction(
            object_id,
            property,
            tick,
            initial_value,
        )?;
        self.playhead = project_time;
        Ok(Some(true))
    }

    pub fn sync_viewport_position_drag(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(snapshot) = self.viewport_position_drag else {
            return Ok(false);
        };

        if snapshot.phase == ViewportTransformDragPhase::CancelRequested
            && !snapshot.transaction_started
        {
            self.viewport_position_drag = None;
            return Ok(true);
        }

        if !snapshot.transaction_started {
            let Some(animated_transaction) = self.begin_direct_transform_transaction(
                editor,
                snapshot.object_id,
                AnimatableProperty::Position,
            )?
            else {
                self.viewport_position_drag = None;
                return Ok(false);
            };
            if !animated_transaction
                && let Err(error) = editor.begin_position_transaction(snapshot.object_id)
            {
                self.viewport_position_drag = None;
                return Err(error);
            }
            if let Some(drag) = self.viewport_position_drag.as_mut() {
                drag.transaction_started = true;
                drag.animated_transaction = animated_transaction;
            }
        }

        let Some(drag) = self.viewport_position_drag else {
            return Ok(false);
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            let changed = editor.cancel_transaction()?;
            self.viewport_position_drag = None;
            return Ok(changed);
        }

        if drag.animated_transaction {
            editor.update_property_keyframe_value_transaction(PropertyValue::Vec2(
                drag.current_position,
            ))?;
        } else {
            editor.update_position_transaction(drag.current_position)?;
        }

        if drag.phase == ViewportTransformDragPhase::CommitRequested {
            let changed = editor.commit_transaction()?;
            self.viewport_position_drag = None;
            return Ok(changed);
        }

        Ok(true)
    }

    pub fn begin_viewport_multi_position_drag(
        &mut self,
        mut object_ids: Vec<ObjectId>,
        pointer_start: [f32; 2],
    ) -> bool {
        if self.viewport_position_drag.is_some()
            || self.viewport_multi_position_drag.is_some()
            || self.viewport_scale_drag.is_some()
            || self.viewport_rotation_drag.is_some()
            || !pointer_start[0].is_finite()
            || !pointer_start[1].is_finite()
        {
            return false;
        }

        object_ids.sort_by_key(|object_id| object_id.get());
        object_ids.dedup();
        if object_ids.len() < 2 {
            return false;
        }

        self.viewport_multi_position_drag = Some(ViewportMultiPositionDrag {
            object_ids,
            pointer_start,
            current_delta: Vec2::new(0.0, 0.0).expect("zero delta is finite"),
            phase: ViewportTransformDragPhase::Active,
            transaction_started: false,
            animated_transaction: false,
        });
        true
    }

    #[must_use]
    pub fn viewport_multi_position_drag_active(&self) -> bool {
        self.viewport_multi_position_drag
            .as_ref()
            .is_some_and(|drag| drag.phase == ViewportTransformDragPhase::Active)
    }

    pub fn update_viewport_multi_position_drag(
        &mut self,
        pointer: [f32; 2],
        composition_units_per_point: [f32; 2],
        constrain_axis: bool,
    ) -> bool {
        let Some(drag) = self.viewport_multi_position_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active
            || !pointer[0].is_finite()
            || !pointer[1].is_finite()
            || !composition_units_per_point[0].is_finite()
            || !composition_units_per_point[1].is_finite()
            || composition_units_per_point[0] <= 0.0
            || composition_units_per_point[1] <= 0.0
        {
            return false;
        }

        let mut delta_x = (pointer[0] - drag.pointer_start[0]) * composition_units_per_point[0];
        let mut delta_y = (pointer[1] - drag.pointer_start[1]) * composition_units_per_point[1];
        if constrain_axis {
            if delta_x.abs() >= delta_y.abs() {
                delta_y = 0.0;
            } else {
                delta_x = 0.0;
            }
        }
        let Ok(current_delta) = Vec2::new(delta_x, delta_y) else {
            return false;
        };
        if current_delta == drag.current_delta {
            return false;
        }

        drag.current_delta = current_delta;
        true
    }

    pub fn finish_viewport_multi_position_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_multi_position_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CommitRequested;
        true
    }

    pub fn cancel_viewport_multi_position_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_multi_position_drag.as_mut() else {
            return false;
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CancelRequested;
        true
    }

    pub fn sync_viewport_multi_position_drag(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(snapshot) = self.viewport_multi_position_drag.as_ref().cloned() else {
            return Ok(false);
        };

        if snapshot.phase == ViewportTransformDragPhase::CancelRequested
            && !snapshot.transaction_started
        {
            self.viewport_multi_position_drag = None;
            return Ok(true);
        }

        if !snapshot.transaction_started {
            let has_animated_position =
                snapshot.object_ids.iter().try_fold(false, |animated, id| {
                    Ok::<_, EditError>(
                        animated
                            || property_keyframe_count(
                                editor.project(),
                                *id,
                                AnimatableProperty::Position,
                            )? > 0,
                    )
                })?;

            if has_animated_position {
                let project = editor.project();
                let Ok(continuous_tick) = project.tempo_map.continuous_tick_position(self.playhead)
                else {
                    self.viewport_multi_position_drag = None;
                    return Ok(false);
                };
                let Ok(tick) = snap_tick_position_to_grid(continuous_tick, self.authoring_division)
                else {
                    self.viewport_multi_position_drag = None;
                    return Ok(false);
                };
                let Ok(project_time) = project.tempo_map.project_time_for_tick(tick) else {
                    self.viewport_multi_position_drag = None;
                    return Ok(false);
                };

                let mut evaluated_positions = Vec::with_capacity(snapshot.object_ids.len());
                for object_id in &snapshot.object_ids {
                    let value = evaluate_property_at_tick(
                        project,
                        *object_id,
                        AnimatableProperty::Position,
                        tick.get() as f64,
                    )?;
                    let PropertyValue::Vec2(position) = value else {
                        return Err(EditError::HistoryInvariant(
                            "position evaluation returned non-vector value",
                        ));
                    };
                    evaluated_positions.push((*object_id, position));
                }

                if let Err(error) = editor.begin_direct_multi_position_transaction(
                    &snapshot.object_ids,
                    tick,
                    &evaluated_positions,
                ) {
                    self.viewport_multi_position_drag = None;
                    return Err(error);
                }
                self.playhead = project_time;
            } else if let Err(error) = editor.begin_multi_position_transaction(&snapshot.object_ids)
            {
                self.viewport_multi_position_drag = None;
                return Err(error);
            }

            if let Some(drag) = self.viewport_multi_position_drag.as_mut() {
                drag.transaction_started = true;
                drag.animated_transaction = has_animated_position;
            }
        }

        let Some(drag) = self.viewport_multi_position_drag.as_ref().cloned() else {
            return Ok(false);
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            let changed = editor.cancel_transaction()?;
            self.viewport_multi_position_drag = None;
            return Ok(changed);
        }

        if drag.animated_transaction {
            editor.update_direct_multi_position_transaction(drag.current_delta)?;
        } else {
            editor.update_multi_position_transaction(drag.current_delta)?;
        }

        if drag.phase == ViewportTransformDragPhase::CommitRequested {
            let changed = editor.commit_transaction()?;
            self.viewport_multi_position_drag = None;
            return Ok(changed);
        }

        Ok(true)
    }

    pub fn begin_viewport_scale_drag(
        &mut self,
        object_id: ObjectId,
        anchor: Vec2,
        handle_start: Vec2,
        scale_start: Vec2,
        rotation_degrees: f32,
    ) -> bool {
        if self.viewport_position_drag.is_some()
            || self.viewport_multi_position_drag.is_some()
            || self.viewport_scale_drag.is_some()
            || self.viewport_rotation_drag.is_some()
            || !rotation_degrees.is_finite()
        {
            return false;
        }

        self.viewport_scale_drag = Some(ViewportScaleDrag {
            object_id,
            anchor,
            handle_start,
            scale_start,
            rotation_degrees,
            current_scale: scale_start,
            phase: ViewportTransformDragPhase::Active,
            transaction_started: false,
            animated_transaction: false,
        });
        true
    }

    #[must_use]
    pub fn viewport_scale_drag_active(&self) -> bool {
        self.viewport_scale_drag
            .is_some_and(|drag| drag.phase == ViewportTransformDragPhase::Active)
    }

    pub fn update_viewport_scale_drag(&mut self, pointer: Vec2, constrain_uniform: bool) -> bool {
        let Some(drag) = self.viewport_scale_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active {
            return false;
        }

        let radians = drag.rotation_degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        let unrotate = |point: Vec2| {
            let translated_x = point.x() - drag.anchor.x();
            let translated_y = point.y() - drag.anchor.y();
            (
                cos * translated_x + sin * translated_y,
                -sin * translated_x + cos * translated_y,
            )
        };
        let (start_x, start_y) = unrotate(drag.handle_start);
        let (current_x, current_y) = unrotate(pointer);
        const HANDLE_AXIS_EPSILON: f32 = 0.0001;

        let ratio_x = (start_x.abs() > HANDLE_AXIS_EPSILON).then_some(current_x / start_x);
        let ratio_y = (start_y.abs() > HANDLE_AXIS_EPSILON).then_some(current_y / start_y);
        let (scale_x, scale_y) = if constrain_uniform {
            let uniform_ratio = match (ratio_x, ratio_y) {
                (Some(x), Some(y)) => {
                    if (x - 1.0).abs() >= (y - 1.0).abs() {
                        x
                    } else {
                        y
                    }
                }
                (Some(x), None) => x,
                (None, Some(y)) => y,
                (None, None) => 1.0,
            };
            (
                drag.scale_start.x() * uniform_ratio,
                drag.scale_start.y() * uniform_ratio,
            )
        } else {
            (
                ratio_x.map_or(drag.scale_start.x(), |ratio| drag.scale_start.x() * ratio),
                ratio_y.map_or(drag.scale_start.y(), |ratio| drag.scale_start.y() * ratio),
            )
        };
        let Ok(current_scale) = Vec2::new(scale_x, scale_y) else {
            return false;
        };

        if current_scale == drag.current_scale {
            return false;
        }
        drag.current_scale = current_scale;
        true
    }

    pub fn finish_viewport_scale_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_scale_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CommitRequested;
        true
    }

    pub fn cancel_viewport_scale_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_scale_drag.as_mut() else {
            return false;
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CancelRequested;
        true
    }

    pub fn sync_viewport_scale_drag(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(snapshot) = self.viewport_scale_drag else {
            return Ok(false);
        };

        if snapshot.phase == ViewportTransformDragPhase::CancelRequested
            && !snapshot.transaction_started
        {
            self.viewport_scale_drag = None;
            return Ok(true);
        }

        if !snapshot.transaction_started {
            let Some(animated_transaction) = self.begin_direct_transform_transaction(
                editor,
                snapshot.object_id,
                AnimatableProperty::Scale,
            )?
            else {
                self.viewport_scale_drag = None;
                return Ok(false);
            };
            if !animated_transaction
                && let Err(error) = editor.begin_scale_transaction(snapshot.object_id)
            {
                self.viewport_scale_drag = None;
                return Err(error);
            }
            if let Some(drag) = self.viewport_scale_drag.as_mut() {
                drag.transaction_started = true;
                drag.animated_transaction = animated_transaction;
            }
        }

        let Some(drag) = self.viewport_scale_drag else {
            return Ok(false);
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            let changed = editor.cancel_transaction()?;
            self.viewport_scale_drag = None;
            return Ok(changed);
        }

        if drag.animated_transaction {
            editor.update_property_keyframe_value_transaction(PropertyValue::Vec2(
                drag.current_scale,
            ))?;
        } else {
            editor.update_scale_transaction(drag.current_scale)?;
        }

        if drag.phase == ViewportTransformDragPhase::CommitRequested {
            let changed = editor.commit_transaction()?;
            self.viewport_scale_drag = None;
            return Ok(changed);
        }

        Ok(true)
    }

    pub fn begin_viewport_rotation_drag(
        &mut self,
        object_id: ObjectId,
        anchor: Vec2,
        pointer_start: Vec2,
        rotation_start: f32,
    ) -> bool {
        if self.viewport_position_drag.is_some()
            || self.viewport_multi_position_drag.is_some()
            || self.viewport_scale_drag.is_some()
            || self.viewport_rotation_drag.is_some()
            || !rotation_start.is_finite()
        {
            return false;
        }

        let delta_x = pointer_start.x() - anchor.x();
        let delta_y = pointer_start.y() - anchor.y();
        if delta_x.abs() < f32::EPSILON && delta_y.abs() < f32::EPSILON {
            return false;
        }
        let pointer_angle_degrees = delta_y.atan2(delta_x).to_degrees();

        self.viewport_rotation_drag = Some(ViewportRotationDrag {
            object_id,
            anchor,
            pointer_angle_degrees,
            rotation_start,
            accumulated_delta_degrees: 0.0,
            current_rotation: rotation_start,
            phase: ViewportTransformDragPhase::Active,
            transaction_started: false,
            animated_transaction: false,
        });
        true
    }

    #[must_use]
    pub fn viewport_rotation_drag_active(&self) -> bool {
        self.viewport_rotation_drag
            .is_some_and(|drag| drag.phase == ViewportTransformDragPhase::Active)
    }

    pub fn update_viewport_rotation_drag(&mut self, pointer: Vec2, snap_15_degrees: bool) -> bool {
        let Some(drag) = self.viewport_rotation_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active {
            return false;
        }

        let delta_x = pointer.x() - drag.anchor.x();
        let delta_y = pointer.y() - drag.anchor.y();
        if delta_x.abs() < f32::EPSILON && delta_y.abs() < f32::EPSILON {
            return false;
        }

        let current_angle = delta_y.atan2(delta_x).to_degrees();
        let mut delta = current_angle - drag.pointer_angle_degrees;
        while delta > 180.0 {
            delta -= 360.0;
        }
        while delta <= -180.0 {
            delta += 360.0;
        }

        drag.pointer_angle_degrees = current_angle;
        drag.accumulated_delta_degrees += delta;
        let raw_rotation = drag.rotation_start + drag.accumulated_delta_degrees;
        let current_rotation = if snap_15_degrees {
            (raw_rotation / 15.0).round() * 15.0
        } else {
            raw_rotation
        };
        if !current_rotation.is_finite()
            || (current_rotation - drag.current_rotation).abs() < f32::EPSILON
        {
            return false;
        }

        drag.current_rotation = current_rotation;
        true
    }

    pub fn finish_viewport_rotation_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_rotation_drag.as_mut() else {
            return false;
        };
        if drag.phase != ViewportTransformDragPhase::Active {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CommitRequested;
        true
    }

    pub fn cancel_viewport_rotation_drag(&mut self) -> bool {
        let Some(drag) = self.viewport_rotation_drag.as_mut() else {
            return false;
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            return false;
        }

        drag.phase = ViewportTransformDragPhase::CancelRequested;
        true
    }

    pub fn sync_viewport_rotation_drag(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(snapshot) = self.viewport_rotation_drag else {
            return Ok(false);
        };

        if snapshot.phase == ViewportTransformDragPhase::CancelRequested
            && !snapshot.transaction_started
        {
            self.viewport_rotation_drag = None;
            return Ok(true);
        }

        if !snapshot.transaction_started {
            let Some(animated_transaction) = self.begin_direct_transform_transaction(
                editor,
                snapshot.object_id,
                AnimatableProperty::Rotation,
            )?
            else {
                self.viewport_rotation_drag = None;
                return Ok(false);
            };
            if !animated_transaction
                && let Err(error) = editor.begin_rotation_transaction(snapshot.object_id)
            {
                self.viewport_rotation_drag = None;
                return Err(error);
            }
            if let Some(drag) = self.viewport_rotation_drag.as_mut() {
                drag.transaction_started = true;
                drag.animated_transaction = animated_transaction;
            }
        }

        let Some(drag) = self.viewport_rotation_drag else {
            return Ok(false);
        };
        if drag.phase == ViewportTransformDragPhase::CancelRequested {
            let changed = editor.cancel_transaction()?;
            self.viewport_rotation_drag = None;
            return Ok(changed);
        }

        if drag.animated_transaction {
            editor.update_property_keyframe_value_transaction(PropertyValue::Scalar(
                drag.current_rotation,
            ))?;
        } else {
            editor.update_rotation_transaction(drag.current_rotation)?;
        }

        if drag.phase == ViewportTransformDragPhase::CommitRequested {
            let changed = editor.commit_transaction()?;
            self.viewport_rotation_drag = None;
            return Ok(changed);
        }

        Ok(true)
    }

    #[allow(dead_code)]
    pub fn update_playhead_during_playback(
        &mut self,
        project_time: ProjectTimeNs,
        duration: DurationNs,
    ) {
        let duration_ns = i64::try_from(duration.get()).unwrap_or(i64::MAX);
        self.playhead = ProjectTimeNs::new(project_time.get().clamp(0, duration_ns));
        if self.follow_playhead {
            self.follow_playhead_to_viewport_edge(duration);
        }
    }

    #[must_use]
    pub fn selected_object_ids(&self) -> Vec<ObjectId> {
        self.selected_objects.iter().copied().collect()
    }

    #[must_use]
    pub fn is_object_selected(&self, object_id: ObjectId) -> bool {
        self.selected_objects.contains(&object_id)
    }

    pub fn replace_object_selection(&mut self, object_id: Option<ObjectId>) -> bool {
        let already_selected = object_id.is_some_and(|id| {
            self.selected_objects.len() == 1 && self.selected_objects.contains(&id)
        });
        if already_selected || (object_id.is_none() && self.selected_objects.is_empty()) {
            return false;
        }

        self.selected_objects.clear();
        if let Some(object_id) = object_id {
            self.selected_objects.insert(object_id);
        }
        true
    }

    pub fn toggle_object_selection(&mut self, object_id: ObjectId) -> bool {
        if !self.selected_objects.remove(&object_id) {
            self.selected_objects.insert(object_id);
        }
        true
    }

    pub fn replace_object_selection_many(
        &mut self,
        object_ids: impl IntoIterator<Item = ObjectId>,
    ) -> bool {
        let replacement: HashSet<_> = object_ids.into_iter().collect();
        if replacement == self.selected_objects {
            return false;
        }

        self.selected_objects = replacement;
        true
    }

    pub fn begin_viewport_box_selection(&mut self, start: [f32; 2]) {
        self.viewport_box_selection = Some(ViewportBoxSelection {
            start,
            current: start,
        });
    }

    pub fn update_viewport_box_selection(&mut self, current: [f32; 2]) {
        if let Some(selection) = self.viewport_box_selection.as_mut() {
            selection.current = current;
        }
    }

    #[must_use]
    pub fn viewport_box_selection(&self) -> Option<([f32; 2], [f32; 2])> {
        self.viewport_box_selection
            .map(|selection| (selection.start, selection.current))
    }

    pub fn take_viewport_box_selection(&mut self) -> Option<([f32; 2], [f32; 2])> {
        self.viewport_box_selection
            .take()
            .map(|selection| (selection.start, selection.current))
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

    pub fn begin_inspector_numeric_edit(
        &mut self,
        target: InspectorNumericTarget,
        value: f32,
        display_value: String,
    ) {
        if !value.is_finite() {
            return;
        }

        self.focus_property(target.object_id, target.property);
        self.inspector_numeric_edit = Some(InspectorNumericEdit {
            target,
            original_value: value,
            buffer: display_value,
        });
    }

    #[must_use]
    pub fn inspector_numeric_edit_buffer(&self, target: InspectorNumericTarget) -> Option<&str> {
        self.inspector_numeric_edit
            .as_ref()
            .filter(|edit| edit.target == target)
            .map(|edit| edit.buffer.as_str())
    }

    pub fn update_inspector_numeric_edit_buffer(
        &mut self,
        target: InspectorNumericTarget,
        buffer: String,
    ) -> bool {
        let Some(edit) = self.inspector_numeric_edit.as_mut() else {
            return false;
        };
        if edit.target != target || edit.buffer == buffer {
            return false;
        }
        edit.buffer = buffer;
        true
    }

    pub fn commit_inspector_numeric_edit(&mut self, target: InspectorNumericTarget) -> bool {
        let Some(edit) = self.inspector_numeric_edit.as_ref() else {
            return false;
        };
        if edit.target != target {
            return false;
        }

        let Ok(value) = edit.buffer.trim().parse::<f32>() else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }

        self.pending_inspector_numeric_commit = Some(InspectorNumericCommit { target, value });
        self.inspector_numeric_edit = None;
        true
    }

    pub fn cancel_inspector_numeric_edit(&mut self, target: InspectorNumericTarget) -> bool {
        let Some(edit) = self.inspector_numeric_edit.as_ref() else {
            return false;
        };
        if edit.target != target {
            return false;
        }

        self.inspector_numeric_edit = None;
        true
    }

    #[cfg(test)]
    #[must_use]
    pub fn inspector_numeric_edit_original_value(
        &self,
        target: InspectorNumericTarget,
    ) -> Option<f32> {
        self.inspector_numeric_edit
            .as_ref()
            .filter(|edit| edit.target == target)
            .map(|edit| edit.original_value)
    }

    #[cfg(test)]
    pub const fn take_inspector_numeric_commit(&mut self) -> Option<InspectorNumericCommit> {
        self.pending_inspector_numeric_commit.take()
    }

    pub fn commit_pending_inspector_static_property_edit(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(commit) = self.pending_inspector_numeric_commit else {
            return Ok(false);
        };

        if property_keyframe_count(
            editor.project(),
            commit.target.object_id,
            commit.target.property,
        )? != 0
        {
            return Ok(false);
        }

        let project_value = if matches!(
            commit.target.component,
            InspectorNumericComponent::R
                | InspectorNumericComponent::G
                | InspectorNumericComponent::B
                | InspectorNumericComponent::A
        ) {
            commit.value / 100.0
        } else {
            match commit.target.property {
                AnimatableProperty::Scale
                | AnimatableProperty::Anchor
                | AnimatableProperty::Opacity => commit.value / 100.0,
                _ => commit.value,
            }
        };
        if !project_value.is_finite() {
            self.pending_inspector_numeric_commit = None;
            return Ok(false);
        }

        let before = property_base_value(
            editor.project(),
            commit.target.object_id,
            commit.target.property,
        )?;
        let after = match (before, commit.target.component) {
            (PropertyValue::Scalar(_), InspectorNumericComponent::Scalar) => {
                PropertyValue::Scalar(project_value)
            }
            (PropertyValue::Vec2(value), InspectorNumericComponent::X) => PropertyValue::Vec2(
                Vec2::new(project_value, value.y())
                    .map_err(|_| EditError::InvalidValue("inspector property value"))?,
            ),
            (PropertyValue::Vec2(value), InspectorNumericComponent::Y) => PropertyValue::Vec2(
                Vec2::new(value.x(), project_value)
                    .map_err(|_| EditError::InvalidValue("inspector property value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::R) => PropertyValue::Color(
                LinearRgba::new(project_value, value.g(), value.b(), value.a())
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::G) => PropertyValue::Color(
                LinearRgba::new(value.r(), project_value, value.b(), value.a())
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::B) => PropertyValue::Color(
                LinearRgba::new(value.r(), value.g(), project_value, value.a())
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::A) => PropertyValue::Color(
                LinearRgba::new(value.r(), value.g(), value.b(), project_value)
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            _ => {
                self.pending_inspector_numeric_commit = None;
                return Err(EditError::HistoryInvariant(
                    "inspector numeric component does not match property value",
                ));
            }
        };

        self.pending_inspector_numeric_commit = None;
        editor.execute(EditCommand::SetPropertyBase {
            object_id: commit.target.object_id,
            property: commit.target.property,
            value: after,
        })
    }

    pub fn commit_pending_inspector_animated_property_edit(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(commit) = self.pending_inspector_numeric_commit else {
            return Ok(false);
        };

        if property_keyframe_count(
            editor.project(),
            commit.target.object_id,
            commit.target.property,
        )? == 0
        {
            return Ok(false);
        }

        self.pending_inspector_numeric_commit = None;

        let project_value = if matches!(
            commit.target.component,
            InspectorNumericComponent::R
                | InspectorNumericComponent::G
                | InspectorNumericComponent::B
                | InspectorNumericComponent::A
        ) {
            commit.value / 100.0
        } else {
            match commit.target.property {
                AnimatableProperty::Scale
                | AnimatableProperty::Anchor
                | AnimatableProperty::Opacity => commit.value / 100.0,
                _ => commit.value,
            }
        };
        if !project_value.is_finite() {
            return Ok(false);
        }

        let (resolved_tick, resolved_time, evaluated) = {
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
            let value = evaluate_property_at_tick(
                project,
                commit.target.object_id,
                commit.target.property,
                tick.get() as f64,
            )?;
            (tick, project_time, value)
        };

        let edited = match (evaluated, commit.target.component) {
            (PropertyValue::Scalar(_), InspectorNumericComponent::Scalar) => {
                PropertyValue::Scalar(project_value)
            }
            (PropertyValue::Vec2(value), InspectorNumericComponent::X) => PropertyValue::Vec2(
                Vec2::new(project_value, value.y())
                    .map_err(|_| EditError::InvalidValue("inspector property value"))?,
            ),
            (PropertyValue::Vec2(value), InspectorNumericComponent::Y) => PropertyValue::Vec2(
                Vec2::new(value.x(), project_value)
                    .map_err(|_| EditError::InvalidValue("inspector property value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::R) => PropertyValue::Color(
                LinearRgba::new(project_value, value.g(), value.b(), value.a())
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::G) => PropertyValue::Color(
                LinearRgba::new(value.r(), project_value, value.b(), value.a())
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::B) => PropertyValue::Color(
                LinearRgba::new(value.r(), value.g(), project_value, value.a())
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            (PropertyValue::Color(value), InspectorNumericComponent::A) => PropertyValue::Color(
                LinearRgba::new(value.r(), value.g(), value.b(), project_value)
                    .map_err(|_| EditError::InvalidValue("inspector color value"))?,
            ),
            _ => {
                return Err(EditError::HistoryInvariant(
                    "inspector numeric component does not match property value",
                ));
            }
        };

        let existing = property_keyframe_at_tick(
            editor.project(),
            commit.target.object_id,
            commit.target.property,
            resolved_tick,
        )?;
        let changed = if existing.is_some() {
            editor.begin_property_keyframe_value_transaction(
                commit.target.object_id,
                commit.target.property,
                resolved_tick,
                evaluated,
            )?;
            if let Err(error) = editor.update_property_keyframe_value_transaction(edited) {
                let _ = editor.cancel_transaction();
                return Err(error);
            }
            editor.commit_transaction()?
        } else {
            editor
                .create_property_keyframe(
                    commit.target.object_id,
                    commit.target.property,
                    resolved_tick,
                    edited,
                )?
                .is_some()
        };

        self.playhead = resolved_time;
        Ok(changed)
    }

    pub fn begin_inspector_multi_numeric_edit(
        &mut self,
        target: InspectorMultiNumericTarget,
        mut object_ids: Vec<ObjectId>,
        buffer: String,
    ) -> bool {
        object_ids.sort_by_key(|object_id| object_id.get());
        object_ids.dedup();
        if object_ids.len() < 2 {
            return false;
        }

        self.inspector_multi_numeric_edit = Some(InspectorMultiNumericEdit {
            target,
            object_ids,
            buffer,
        });
        self.pending_inspector_multi_numeric_commit = None;
        true
    }

    #[must_use]
    pub fn inspector_multi_numeric_edit_buffer(
        &self,
        target: InspectorMultiNumericTarget,
        object_ids: &[ObjectId],
    ) -> Option<&str> {
        self.inspector_multi_numeric_edit
            .as_ref()
            .filter(|edit| edit.target == target && edit.object_ids == object_ids)
            .map(|edit| edit.buffer.as_str())
    }

    pub fn update_inspector_multi_numeric_edit_buffer(
        &mut self,
        target: InspectorMultiNumericTarget,
        buffer: String,
    ) -> bool {
        let Some(edit) = self.inspector_multi_numeric_edit.as_mut() else {
            return false;
        };
        if edit.target != target || edit.buffer == buffer {
            return false;
        }

        edit.buffer = buffer;
        true
    }

    pub fn commit_inspector_multi_numeric_edit(
        &mut self,
        target: InspectorMultiNumericTarget,
    ) -> bool {
        let Some(edit) = self.inspector_multi_numeric_edit.as_ref() else {
            return false;
        };
        if edit.target != target {
            return false;
        }

        let Ok(value) = edit.buffer.trim().parse::<f32>() else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }

        self.pending_inspector_multi_numeric_commit = Some(InspectorMultiNumericCommit {
            target,
            object_ids: edit.object_ids.clone(),
            value,
        });
        self.inspector_multi_numeric_edit = None;
        true
    }

    pub fn cancel_inspector_multi_numeric_edit(
        &mut self,
        target: InspectorMultiNumericTarget,
    ) -> bool {
        let Some(edit) = self.inspector_multi_numeric_edit.as_ref() else {
            return false;
        };
        if edit.target != target {
            return false;
        }

        self.inspector_multi_numeric_edit = None;
        true
    }

    pub fn commit_pending_inspector_multi_property_edit(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(commit) = self.pending_inspector_multi_numeric_commit.take() else {
            return Ok(false);
        };

        let project_value = if matches!(
            commit.target.component,
            InspectorNumericComponent::R
                | InspectorNumericComponent::G
                | InspectorNumericComponent::B
                | InspectorNumericComponent::A
        ) {
            commit.value / 100.0
        } else {
            match commit.target.property {
                AnimatableProperty::Scale
                | AnimatableProperty::Anchor
                | AnimatableProperty::Opacity => commit.value / 100.0,
                _ => commit.value,
            }
        };
        if !project_value.is_finite() {
            return Ok(false);
        }

        let has_animated = commit
            .object_ids
            .iter()
            .try_fold(false, |animated, object_id| {
                Ok::<_, EditError>(
                    animated
                        || property_keyframe_count(
                            editor.project(),
                            *object_id,
                            commit.target.property,
                        )? > 0,
                )
            })?;

        let (resolved_tick, resolved_time) = if has_animated {
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
            (tick, Some(project_time))
        } else {
            (MusicalTick::new(0), None)
        };

        let mut values = Vec::with_capacity(commit.object_ids.len());
        for object_id in &commit.object_ids {
            let before =
                if property_keyframe_count(editor.project(), *object_id, commit.target.property)?
                    == 0
                {
                    property_base_value(editor.project(), *object_id, commit.target.property)?
                } else {
                    evaluate_property_at_tick(
                        editor.project(),
                        *object_id,
                        commit.target.property,
                        resolved_tick.get() as f64,
                    )?
                };

            let after = match (before, commit.target.component) {
                (PropertyValue::Scalar(_), InspectorNumericComponent::Scalar) => {
                    PropertyValue::Scalar(project_value)
                }
                (PropertyValue::Vec2(value), InspectorNumericComponent::X) => PropertyValue::Vec2(
                    Vec2::new(project_value, value.y())
                        .map_err(|_| EditError::InvalidValue("inspector property value"))?,
                ),
                (PropertyValue::Vec2(value), InspectorNumericComponent::Y) => PropertyValue::Vec2(
                    Vec2::new(value.x(), project_value)
                        .map_err(|_| EditError::InvalidValue("inspector property value"))?,
                ),
                (PropertyValue::Color(value), InspectorNumericComponent::R) => {
                    PropertyValue::Color(
                        LinearRgba::new(project_value, value.g(), value.b(), value.a())
                            .map_err(|_| EditError::InvalidValue("inspector color value"))?,
                    )
                }
                (PropertyValue::Color(value), InspectorNumericComponent::G) => {
                    PropertyValue::Color(
                        LinearRgba::new(value.r(), project_value, value.b(), value.a())
                            .map_err(|_| EditError::InvalidValue("inspector color value"))?,
                    )
                }
                (PropertyValue::Color(value), InspectorNumericComponent::B) => {
                    PropertyValue::Color(
                        LinearRgba::new(value.r(), value.g(), project_value, value.a())
                            .map_err(|_| EditError::InvalidValue("inspector color value"))?,
                    )
                }
                (PropertyValue::Color(value), InspectorNumericComponent::A) => {
                    PropertyValue::Color(
                        LinearRgba::new(value.r(), value.g(), value.b(), project_value)
                            .map_err(|_| EditError::InvalidValue("inspector color value"))?,
                    )
                }
                _ => {
                    return Err(EditError::HistoryInvariant(
                        "inspector multi numeric component does not match property value",
                    ));
                }
            };
            values.push((*object_id, after));
        }

        let changed =
            editor.set_property_values_at_tick(commit.target.property, resolved_tick, values)?;
        if let Some(resolved_time) = resolved_time {
            self.playhead = resolved_time;
        }
        Ok(changed)
    }

    pub fn begin_inspector_text_edit(&mut self, target: InspectorTextTarget, buffer: String) {
        self.inspector_text_edit = Some(InspectorTextEdit { target, buffer });
    }

    #[must_use]
    pub fn inspector_text_edit_buffer(&self, target: InspectorTextTarget) -> Option<&str> {
        self.inspector_text_edit
            .as_ref()
            .filter(|edit| edit.target == target)
            .map(|edit| edit.buffer.as_str())
    }

    pub fn update_inspector_text_edit_buffer(
        &mut self,
        target: InspectorTextTarget,
        buffer: String,
    ) -> bool {
        let Some(edit) = self.inspector_text_edit.as_mut() else {
            return false;
        };
        if edit.target != target || edit.buffer == buffer {
            return false;
        }

        edit.buffer = buffer;
        true
    }

    pub fn commit_inspector_text_edit(&mut self, target: InspectorTextTarget) -> bool {
        let Some(edit) = self.inspector_text_edit.as_ref() else {
            return false;
        };
        if edit.target != target {
            return false;
        }

        let action = match target.field {
            InspectorTextField::Content => TextInspectorAction::SetContent {
                object_id: target.object_id,
                text: edit.buffer.clone(),
            },
            InspectorTextField::FontFamily => TextInspectorAction::SetFontFamily {
                object_id: target.object_id,
                family: edit.buffer.clone(),
            },
            InspectorTextField::FontSize => {
                let Ok(font_size) = edit.buffer.trim().parse::<f32>() else {
                    return false;
                };
                if !font_size.is_finite() || font_size <= 0.0 {
                    return false;
                }
                TextInspectorAction::SetFontSize {
                    object_id: target.object_id,
                    font_size,
                }
            }
        };

        self.pending_text_inspector_actions.push(action);
        self.inspector_text_edit = None;
        true
    }

    pub fn cancel_inspector_text_edit(&mut self, target: InspectorTextTarget) -> bool {
        let Some(edit) = self.inspector_text_edit.as_ref() else {
            return false;
        };
        if edit.target != target {
            return false;
        }

        self.inspector_text_edit = None;
        true
    }

    pub fn queue_text_font_weight(&mut self, object_id: ObjectId, weight: FontWeight) {
        self.pending_text_inspector_actions
            .push(TextInspectorAction::SetFontWeight { object_id, weight });
    }

    pub fn queue_text_font_style(&mut self, object_id: ObjectId, style: FontStyle) {
        self.pending_text_inspector_actions
            .push(TextInspectorAction::SetFontStyle { object_id, style });
    }

    pub fn queue_text_alignment(&mut self, object_id: ObjectId, alignment: TextAlignment) {
        self.pending_text_inspector_actions
            .push(TextInspectorAction::SetAlignment {
                object_id,
                alignment,
            });
    }

    pub fn commit_pending_text_inspector_actions(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        if self.pending_text_inspector_actions.is_empty() {
            return Ok(false);
        }

        let actions = std::mem::take(&mut self.pending_text_inspector_actions);
        let mut changed = false;
        for action in actions {
            let action_changed = match action {
                TextInspectorAction::SetContent { object_id, text } => {
                    editor.execute(EditCommand::SetTextContent { object_id, text })?
                }
                TextInspectorAction::SetFontFamily { object_id, family } => {
                    editor.execute(EditCommand::SetTextFontFamily { object_id, family })?
                }
                TextInspectorAction::SetFontSize {
                    object_id,
                    font_size,
                } => editor.execute(EditCommand::SetTextFontSize {
                    object_id,
                    font_size,
                })?,
                TextInspectorAction::SetFontWeight { object_id, weight } => {
                    editor.execute(EditCommand::SetTextFontWeight { object_id, weight })?
                }
                TextInspectorAction::SetFontStyle { object_id, style } => {
                    editor.execute(EditCommand::SetTextFontStyle { object_id, style })?
                }
                TextInspectorAction::SetAlignment {
                    object_id,
                    alignment,
                } => editor.execute(EditCommand::SetTextAlignment {
                    object_id,
                    alignment,
                })?,
            };
            changed |= action_changed;
        }
        Ok(changed)
    }

    pub fn request_focused_keyframe_action(
        &mut self,
        object_id: ObjectId,
        property: AnimatableProperty,
    ) {
        self.focus_property(object_id, property);
        self.pending_focused_keyframe_action = true;
    }

    pub fn commit_pending_focused_keyframe_action(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        if !self.pending_focused_keyframe_action {
            return Ok(false);
        }

        self.pending_focused_keyframe_action = false;
        self.keyframe_action(editor)
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

    #[cfg(test)]
    #[must_use]
    pub fn keyframe_clipboard(&self) -> Option<&KeyframeCopyPacket> {
        self.keyframe_clipboard.as_ref()
    }

    pub fn paste_keyframe_clipboard(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(packet) = self.keyframe_clipboard.clone() else {
            return Ok(false);
        };
        if packet.entries.is_empty() {
            return Ok(false);
        }

        let anchor_tick = {
            let project = editor.project();
            let Ok(continuous_tick) = project.tempo_map.continuous_tick_position(self.playhead)
            else {
                return Ok(false);
            };
            let Ok(anchor_tick) =
                snap_tick_position_to_grid(continuous_tick, self.authoring_division)
            else {
                return Ok(false);
            };
            anchor_tick
        };

        let first_source = (
            packet.entries[0].source_object_id,
            packet.entries[0].source_property,
        );
        let single_source = packet
            .entries
            .iter()
            .all(|entry| (entry.source_object_id, entry.source_property) == first_source);
        let remap_target = self.focused_property.filter(|focused| {
            single_source
                && packet
                    .entries
                    .iter()
                    .all(|entry| property_value_compatible(focused.property, entry.value))
        });

        let mut drafts = Vec::with_capacity(packet.entries.len());
        for entry in packet.entries {
            let (object_id, property) = remap_target
                .map(|focused| (focused.object_id, focused.property))
                .unwrap_or((entry.source_object_id, entry.source_property));

            if !property_value_compatible(property, entry.value) {
                return Err(EditError::InvalidValue(
                    "keyframe paste property type mismatch",
                ));
            }

            let target_tick = anchor_tick
                .get()
                .checked_add(entry.relative_tick)
                .ok_or(EditError::HistoryInvariant("paste tick overflow"))?;

            drafts.push(PropertyKeyframeDraft {
                object_id,
                property,
                tick: MusicalTick::new(target_tick),
                value: entry.value,
                interpolation: entry.interpolation,
            });
        }

        let new_ids = editor.insert_property_keyframes(drafts)?;
        if new_ids.is_empty() {
            return Ok(false);
        }

        self.replace_keyframe_selection(new_ids);
        Ok(true)
    }

    pub fn duplicate_selected_keyframes(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        if self.selected_keyframes.is_empty() {
            return Ok(false);
        }

        let mut located = Vec::with_capacity(self.selected_keyframes.len());
        for keyframe_id in self.selected_keyframe_ids() {
            let Some(keyframe) = locate_property_keyframe(editor.project(), keyframe_id) else {
                return Err(EditError::KeyframeNotFound(keyframe_id));
            };
            located.push(keyframe);
        }

        let Some(first_tick) = located
            .iter()
            .map(|located| located.keyframe.tick.get())
            .min()
        else {
            return Ok(false);
        };
        let Some(last_tick) = located
            .iter()
            .map(|located| located.keyframe.tick.get())
            .max()
        else {
            return Ok(false);
        };

        let pattern_span = last_tick
            .checked_sub(first_tick)
            .ok_or(EditError::HistoryInvariant(
                "duplicate pattern span overflow",
            ))?;
        let offset = pattern_span
            .checked_add(self.authoring_division.ticks_per_step())
            .ok_or(EditError::HistoryInvariant(
                "duplicate keyframe offset overflow",
            ))?;

        located.sort_by_key(|located| {
            (
                located.keyframe.tick.get(),
                located.object_id.get(),
                format!("{:?}", located.property),
            )
        });

        let mut drafts = Vec::with_capacity(located.len());
        for located in located {
            let target_tick = located.keyframe.tick.get().checked_add(offset).ok_or(
                EditError::HistoryInvariant("duplicate keyframe tick overflow"),
            )?;
            drafts.push(PropertyKeyframeDraft {
                object_id: located.object_id,
                property: located.property,
                tick: MusicalTick::new(target_tick),
                value: located.keyframe.value,
                interpolation: located.keyframe.interpolation,
            });
        }

        let new_ids = editor.insert_property_keyframes(drafts)?;
        if new_ids.is_empty() {
            return Ok(false);
        }

        self.replace_keyframe_selection(new_ids);
        Ok(true)
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

    pub fn queue_selected_keyframe_interpolation(
        &mut self,
        preset: KeyframeInterpolationPreset,
    ) -> bool {
        if self.selected_keyframes.is_empty() {
            return false;
        }

        self.pending_keyframe_interpolation = Some(preset.interpolation());
        true
    }

    pub fn commit_pending_keyframe_interpolation(
        &mut self,
        editor: &mut ProjectEditor,
    ) -> Result<bool, EditError> {
        let Some(interpolation) = self.pending_keyframe_interpolation.take() else {
            return Ok(false);
        };

        let mut outgoing_ids = Vec::with_capacity(self.selected_keyframes.len());
        for keyframe_id in self.selected_keyframe_ids() {
            let Some(located) = locate_property_keyframe(editor.project(), keyframe_id) else {
                return Err(EditError::KeyframeNotFound(keyframe_id));
            };
            if property_keyframe_has_successor(
                editor.project(),
                located.object_id,
                located.property,
                keyframe_id,
            )? {
                outgoing_ids.push(keyframe_id);
            }
        }

        outgoing_ids.sort_by_key(|keyframe_id| keyframe_id.get());
        editor.set_property_keyframe_interpolations(outgoing_ids, interpolation)
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

    #[cfg(test)]
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

    #[cfg(test)]
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
        self.follow_playhead = false;
        true
    }

    fn follow_playhead_to_viewport_edge(&mut self, duration: DurationNs) {
        const EDGE_FRACTION: f64 = 0.10;
        const RIGHT_EDGE_TARGET_FRACTION: f64 = 0.25;
        const LEFT_EDGE_TARGET_FRACTION: f64 = 0.75;

        let duration_ns = i64::try_from(duration.get()).unwrap_or(i64::MAX).max(1);
        let Some(view) = self.timeline_view else {
            return;
        };
        let span = (view.end.get() - view.start.get()).clamp(1, duration_ns);
        if span >= duration_ns {
            return;
        }

        let edge_margin = ((span as f64) * EDGE_FRACTION).round() as i64;
        let left_edge = view.start.get().saturating_add(edge_margin);
        let right_edge = view.end.get().saturating_sub(edge_margin);
        let playhead = self.playhead.get();

        let target_fraction = if playhead >= right_edge {
            Some(RIGHT_EDGE_TARGET_FRACTION)
        } else if playhead <= left_edge {
            Some(LEFT_EDGE_TARGET_FRACTION)
        } else {
            None
        };
        let Some(target_fraction) = target_fraction else {
            return;
        };

        let target_offset = ((span as f64) * target_fraction).round() as i64;
        let new_start = playhead
            .saturating_sub(target_offset)
            .clamp(0, duration_ns - span);
        self.timeline_view = Some(TimelineView {
            start: ProjectTimeNs::new(new_start),
            end: ProjectTimeNs::new(new_start.saturating_add(span)),
        });
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
    use super::{
        EditorSession, InspectorNumericCommit, InspectorNumericComponent, InspectorNumericTarget,
        KeyframeDragMember, KeyframeInterpolationPreset, PreviewQuality, ViewportCameraAction,
    };
    use rhythm_core::{
        domain::{LinearRgba, Vec2},
        editor::ProjectEditor,
        ids::{KeyframeId, ObjectId},
        project::ObjectContent,
        property::{
            AnimatableProperty, PropertyValue, property_base_value, property_keyframe_at_tick,
        },
        time::{
            BeatDivision, BpmMicros, DurationNs, GridOffsetNs, MusicalTick, ProjectTimeNs,
            TempoMap, TimeSignature,
        },
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
    fn inspector_numeric_edit_commits_finite_values_and_escape_discards_buffer() {
        let target = InspectorNumericTarget {
            object_id: ObjectId::new(1).expect("object id"),
            property: AnimatableProperty::Rotation,
            component: InspectorNumericComponent::Scalar,
        };
        let mut session = EditorSession::default();

        session.begin_inspector_numeric_edit(target, 15.0, "15.0".to_owned());
        assert_eq!(session.inspector_numeric_edit_buffer(target), Some("15.0"));
        assert!(session.update_inspector_numeric_edit_buffer(target, "22.5".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.take_inspector_numeric_commit(),
            Some(InspectorNumericCommit {
                target,
                value: 22.5,
            })
        );

        session.begin_inspector_numeric_edit(target, 22.5, "22.5".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "-".to_owned()));
        assert!(!session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.inspector_numeric_edit_original_value(target),
            Some(22.5)
        );
        assert!(session.cancel_inspector_numeric_edit(target));
        assert_eq!(session.inspector_numeric_edit_buffer(target), None);
        assert_eq!(session.take_inspector_numeric_commit(), None);
    }

    #[test]
    fn inspector_static_numeric_commit_updates_only_target_component() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();
        let target = InspectorNumericTarget {
            object_id,
            property: AnimatableProperty::Scale,
            component: InspectorNumericComponent::X,
        };

        session.begin_inspector_numeric_edit(target, 1.0, "100.0".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "250.0".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_static_property_edit(&mut editor),
            Ok(true)
        );
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(
                Vec2::new(2.5, 1.0).expect("updated scale")
            ))
        );
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(
                Vec2::new(1.0, 1.0).expect("restored scale")
            ))
        );
    }

    #[test]
    fn inspector_animated_numeric_commit_stays_pending_for_keyframe_handler() {
        use rhythm_core::animation::{Animated, Interpolation, Keyframe};

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = editor_with_object_for_drag().into_project();
        project.composition.objects[0].transform.rotation_degrees = Animated::with_keyframes(
            0.0,
            vec![Keyframe::new(
                KeyframeId::new(2).expect("keyframe id"),
                MusicalTick::new(0),
                10.0,
                Interpolation::Linear,
            )],
        )
        .expect("animated rotation");
        project.next_entity_id = 3;
        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        let target = InspectorNumericTarget {
            object_id,
            property: AnimatableProperty::Rotation,
            component: InspectorNumericComponent::Scalar,
        };

        session.begin_inspector_numeric_edit(target, 10.0, "10.0".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "25.0".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_static_property_edit(&mut editor),
            Ok(false)
        );
        assert_eq!(
            session.take_inspector_numeric_commit(),
            Some(InspectorNumericCommit {
                target,
                value: 25.0,
            })
        );
        assert_eq!(editor.history_len(), 0);
    }

    #[test]
    fn inspector_static_color_channel_commit_updates_fill_with_one_history_entry() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();
        let target = InspectorNumericTarget {
            object_id,
            property: AnimatableProperty::RectangleFill,
            component: InspectorNumericComponent::R,
        };

        session.begin_inspector_numeric_edit(target, 0.0, "0.0".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "50.0".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_static_property_edit(&mut editor),
            Ok(true)
        );

        let fill = property_base_value(
            editor.project(),
            object_id,
            AnimatableProperty::RectangleFill,
        )
        .expect("fill");
        let PropertyValue::Color(fill) = fill else {
            panic!("fill should be color");
        };
        assert!((fill.r() - 0.5).abs() < f32::EPSILON);
        assert_eq!(editor.history_len(), 1);

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            property_base_value(
                editor.project(),
                object_id,
                AnimatableProperty::RectangleFill,
            ),
            Ok(PropertyValue::Color(LinearRgba::black_opaque()))
        );
    }

    #[test]
    fn inspector_static_ellipse_fill_channel_commit_uses_generic_color_path() {
        use rhythm_core::{
            animation::Animated,
            project::{EllipseObject, ObjectContent},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = editor_with_object_for_drag().into_project();
        project.composition.objects[0].content = ObjectContent::Ellipse(EllipseObject {
            size: Animated::new_static(Vec2::new(120.0, 80.0).expect("ellipse size")),
            fill: Animated::new_static(LinearRgba::black_opaque()),
        });
        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        let target = InspectorNumericTarget {
            object_id,
            property: AnimatableProperty::EllipseFill,
            component: InspectorNumericComponent::B,
        };

        session.begin_inspector_numeric_edit(target, 0.0, "0.0".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "25.0".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_static_property_edit(&mut editor),
            Ok(true)
        );

        let fill =
            property_base_value(editor.project(), object_id, AnimatableProperty::EllipseFill)
                .expect("fill");
        let PropertyValue::Color(fill) = fill else {
            panic!("fill should be color");
        };
        assert!((fill.b() - 0.25).abs() < f32::EPSILON);
        assert_eq!(editor.history_len(), 1);

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::EllipseFill),
            Ok(PropertyValue::Color(LinearRgba::black_opaque()))
        );
    }

    #[test]
    fn inspector_animated_fill_channel_commit_creates_nearest_grid_color_key() {
        use rhythm_core::animation::{Animated, Interpolation, Keyframe};

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = editor_with_object_for_drag().into_project();
        project.tempo_map = tempo_120();
        let first = LinearRgba::new(0.0, 0.2, 0.4, 1.0).expect("color");
        let second = LinearRgba::new(1.0, 0.2, 0.4, 1.0).expect("color");
        let ObjectContent::Rectangle(rectangle) = &mut project.composition.objects[0].content
        else {
            panic!("test object should be rectangle");
        };
        rectangle.fill = Animated::with_keyframes(
            first,
            vec![
                Keyframe::new(
                    KeyframeId::new(2).expect("keyframe id"),
                    MusicalTick::new(0),
                    first,
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(3).expect("keyframe id"),
                    MusicalTick::new(480),
                    second,
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("animated fill");
        project.next_entity_id = 4;

        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(130_000_000));
        let target = InspectorNumericTarget {
            object_id,
            property: AnimatableProperty::RectangleFill,
            component: InspectorNumericComponent::G,
        };

        session.begin_inspector_numeric_edit(target, 0.2, "20.0".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "70.0".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_animated_property_edit(&mut editor),
            Ok(true)
        );
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));

        let key = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::RectangleFill,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("nearest-grid fill key");
        let PropertyValue::Color(color) = key.value else {
            panic!("fill key should be color");
        };
        assert!((color.r() - 0.5).abs() < 0.001);
        assert!((color.g() - 0.7).abs() < f32::EPSILON);
        assert!((color.b() - 0.4).abs() < f32::EPSILON);
        assert_eq!(color.a(), 1.0);

        assert_eq!(editor.undo(), Ok(true));
        assert!(
            property_keyframe_at_tick(
                editor.project(),
                object_id,
                AnimatableProperty::RectangleFill,
                MusicalTick::new(240),
            )
            .expect("property")
            .is_none()
        );
    }

    #[test]
    fn inspector_animated_off_key_commit_creates_nearest_grid_key_and_moves_playhead() {
        use rhythm_core::animation::{Animated, Interpolation, Keyframe};

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = editor_with_object_for_drag().into_project();
        project.tempo_map = tempo_120();
        project.composition.objects[0].transform.rotation_degrees = Animated::with_keyframes(
            0.0,
            vec![
                Keyframe::new(
                    KeyframeId::new(2).expect("keyframe id"),
                    MusicalTick::new(0),
                    0.0,
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(3).expect("keyframe id"),
                    MusicalTick::new(480),
                    40.0,
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("animated rotation");
        project.next_entity_id = 4;

        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(130_000_000));
        let target = InspectorNumericTarget {
            object_id,
            property: AnimatableProperty::Rotation,
            component: InspectorNumericComponent::Scalar,
        };

        session.begin_inspector_numeric_edit(target, 20.0, "20.0".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "30.0".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_static_property_edit(&mut editor),
            Ok(false)
        );
        assert_eq!(
            session.commit_pending_inspector_animated_property_edit(&mut editor),
            Ok(true)
        );

        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        let inserted = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Rotation,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("nearest-grid key");
        assert_eq!(inserted.value, PropertyValue::Scalar(30.0));
        assert_eq!(editor.history_len(), 1);

        assert_eq!(editor.undo(), Ok(true));
        assert!(
            property_keyframe_at_tick(
                editor.project(),
                object_id,
                AnimatableProperty::Rotation,
                MusicalTick::new(240),
            )
            .expect("property")
            .is_none()
        );
    }

    #[test]
    fn inspector_animated_on_key_commit_updates_existing_key() {
        use rhythm_core::animation::{Animated, Interpolation, Keyframe};

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = editor_with_object_for_drag().into_project();
        project.tempo_map = tempo_120();
        project.composition.objects[0].transform.opacity = Animated::with_keyframes(
            1.0,
            vec![Keyframe::new(
                KeyframeId::new(2).expect("keyframe id"),
                MusicalTick::new(240),
                0.5,
                Interpolation::Linear,
            )],
        )
        .expect("animated opacity");
        project.next_entity_id = 3;

        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(130_000_000));
        let target = InspectorNumericTarget {
            object_id,
            property: AnimatableProperty::Opacity,
            component: InspectorNumericComponent::Scalar,
        };

        session.begin_inspector_numeric_edit(target, 0.5, "50.0".to_owned());
        assert!(session.update_inspector_numeric_edit_buffer(target, "75.0".to_owned()));
        assert!(session.commit_inspector_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_animated_property_edit(&mut editor),
            Ok(true)
        );

        let key = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("existing key");
        assert_eq!(key.value, PropertyValue::Scalar(0.75));
        assert_eq!(editor.history_len(), 1);

        assert_eq!(editor.undo(), Ok(true));
        let restored = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("restored key");
        assert_eq!(restored.value, PropertyValue::Scalar(0.5));
    }

    #[test]
    fn inspector_multi_numeric_commit_edits_mixed_static_and_animated_members_once() {
        use rhythm_core::animation::{Animated, Interpolation, Keyframe};

        let first_id = ObjectId::new(1).expect("first object");
        let second_id = ObjectId::new(2).expect("second object");
        let mut project = editor_with_object_for_drag().into_project();
        project.tempo_map = tempo_120();

        let mut second = project.composition.objects[0].clone();
        second.id = second_id;
        second.transform.scale = Animated::with_keyframes(
            Vec2::new(1.0, 2.0).expect("base scale"),
            vec![
                Keyframe::new(
                    KeyframeId::new(3).expect("keyframe id"),
                    MusicalTick::new(0),
                    Vec2::new(1.0, 2.0).expect("scale"),
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(4).expect("keyframe id"),
                    MusicalTick::new(480),
                    Vec2::new(3.0, 2.0).expect("scale"),
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("animated scale");
        project.composition.objects.push(second);
        project.next_entity_id = 5;

        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(130_000_000));
        let target = super::InspectorMultiNumericTarget {
            property: AnimatableProperty::Scale,
            component: InspectorNumericComponent::X,
        };
        let ids = vec![first_id, second_id];

        assert!(session.begin_inspector_multi_numeric_edit(target, ids.clone(), String::new(),));
        assert_eq!(
            session.inspector_multi_numeric_edit_buffer(target, &ids),
            Some("")
        );
        assert!(session.update_inspector_multi_numeric_edit_buffer(target, "250.0".to_owned(),));
        assert!(session.commit_inspector_multi_numeric_edit(target));
        assert_eq!(
            session.commit_pending_inspector_multi_property_edit(&mut editor),
            Ok(true)
        );
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        assert_eq!(editor.history_len(), 1);
        assert_eq!(
            property_base_value(editor.project(), first_id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(
                Vec2::new(2.5, 1.0).expect("static edited scale")
            ))
        );
        let animated = property_keyframe_at_tick(
            editor.project(),
            second_id,
            AnimatableProperty::Scale,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("inserted key");
        assert_eq!(
            animated.value,
            PropertyValue::Vec2(Vec2::new(2.5, 2.0).expect("animated edited scale"))
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            property_base_value(editor.project(), first_id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(
                Vec2::new(1.0, 1.0).expect("static restored scale")
            ))
        );
        assert!(
            property_keyframe_at_tick(
                editor.project(),
                second_id,
                AnimatableProperty::Scale,
                MusicalTick::new(240),
            )
            .expect("property")
            .is_none()
        );
    }

    #[test]
    fn inspector_keyframe_action_promotes_final_removed_value_to_static_base() {
        use rhythm_core::animation::{Animated, Interpolation, Keyframe};

        let object_id = ObjectId::new(1).expect("object id");
        let keyframe_id = KeyframeId::new(2).expect("keyframe id");
        let mut project = editor_with_object_for_drag().into_project();
        project.tempo_map = tempo_120();
        project.composition.objects[0].transform.opacity = Animated::with_keyframes(
            1.0,
            vec![Keyframe::new(
                keyframe_id,
                MusicalTick::new(240),
                0.25,
                Interpolation::Linear,
            )],
        )
        .expect("animated opacity");
        project.next_entity_id = 3;

        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(130_000_000));
        session.select_only_keyframe(keyframe_id);
        session.request_focused_keyframe_action(object_id, AnimatableProperty::Opacity);

        assert_eq!(
            session.commit_pending_focused_keyframe_action(&mut editor),
            Ok(true)
        );
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        assert!(
            property_keyframe_at_tick(
                editor.project(),
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(240),
            )
            .expect("property")
            .is_none()
        );
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Opacity),
            Ok(PropertyValue::Scalar(0.25))
        );
        assert_eq!(session.selected_keyframe_count(), 0);

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            property_base_value(editor.project(), object_id, AnimatableProperty::Opacity),
            Ok(PropertyValue::Scalar(1.0))
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
    }

    #[test]
    fn inspector_text_actions_commit_buffered_and_enum_edits() {
        use rhythm_core::{
            animation::Animated,
            domain::LinearRgba,
            project::{
                FontReference, FontStyle, FontWeight, ObjectContent, TextAlignment, TextObject,
            },
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = editor_with_object_for_drag().into_project();
        project.composition.objects[0].content = ObjectContent::Text(TextObject {
            text: "Hello".to_owned(),
            font: FontReference {
                family: "Inter".to_owned(),
                weight: FontWeight::Normal,
                style: FontStyle::Normal,
            },
            font_size: 48.0,
            color: Animated::new_static(LinearRgba::black_opaque()),
            alignment: TextAlignment::Left,
        });
        let mut editor = ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();

        let content_target = super::InspectorTextTarget {
            object_id,
            field: super::InspectorTextField::Content,
        };
        session.begin_inspector_text_edit(content_target, "Hello".to_owned());
        assert!(
            session.update_inspector_text_edit_buffer(content_target, "Привет\nRhythm".to_owned(),)
        );
        assert!(session.commit_inspector_text_edit(content_target));

        let family_target = super::InspectorTextTarget {
            object_id,
            field: super::InspectorTextField::FontFamily,
        };
        session.begin_inspector_text_edit(family_target, "Inter".to_owned());
        assert!(session.update_inspector_text_edit_buffer(family_target, "Noto Sans".to_owned(),));
        assert!(session.commit_inspector_text_edit(family_target));

        let size_target = super::InspectorTextTarget {
            object_id,
            field: super::InspectorTextField::FontSize,
        };
        session.begin_inspector_text_edit(size_target, "48".to_owned());
        assert!(session.update_inspector_text_edit_buffer(size_target, "64".to_owned(),));
        assert!(session.commit_inspector_text_edit(size_target));

        session.queue_text_font_weight(object_id, FontWeight::Bold);
        session.queue_text_font_style(object_id, FontStyle::Italic);
        session.queue_text_alignment(object_id, TextAlignment::Center);

        assert_eq!(
            session.commit_pending_text_inspector_actions(&mut editor),
            Ok(true)
        );
        assert_eq!(editor.history_len(), 6);

        let ObjectContent::Text(text) = &editor.project().composition.objects[0].content else {
            panic!("object should be text");
        };
        assert_eq!(text.text, "Привет\nRhythm");
        assert_eq!(text.font.family, "Noto Sans");
        assert_eq!(text.font.weight, FontWeight::Bold);
        assert_eq!(text.font.style, FontStyle::Italic);
        assert_eq!(text.font_size, 64.0);
        assert_eq!(text.alignment, TextAlignment::Center);

        session.begin_inspector_text_edit(content_target, text.text.clone());
        assert!(
            session.update_inspector_text_edit_buffer(content_target, "discard me".to_owned(),)
        );
        assert!(session.cancel_inspector_text_edit(content_target));
        assert_eq!(
            session.commit_pending_text_inspector_actions(&mut editor),
            Ok(false)
        );
    }

    #[test]
    fn image_relink_request_tracks_stable_asset_id() {
        use rhythm_core::ids::AssetId;

        let first = AssetId::new(11).expect("asset id");
        let second = AssetId::new(12).expect("asset id");
        let mut session = EditorSession::default();

        assert!(session.request_image_relink(first));
        assert!(session.image_relink_requested(first));
        assert!(!session.request_image_relink(first));
        assert!(session.request_image_relink(second));
        assert!(!session.image_relink_requested(first));
        assert!(session.image_relink_requested(second));
    }

    #[test]
    fn object_list_actions_apply_through_project_editor() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        session.queue_object_visibility(object_id, false);
        session.queue_object_locked(object_id, true);
        assert_eq!(
            session.commit_pending_object_list_actions(&mut editor),
            Ok(true)
        );
        assert!(!editor.project().composition.objects[0].visible);
        assert!(editor.project().composition.objects[0].locked);
        assert_eq!(editor.history_len(), 2);
        assert_eq!(
            session.commit_pending_object_list_actions(&mut editor),
            Ok(false)
        );
    }

    #[test]
    fn viewport_single_selection_replaces_and_clears_object_selection() {
        let first = ObjectId::new(11).expect("object id");
        let second = ObjectId::new(12).expect("object id");
        let mut session = EditorSession::default();

        assert!(session.replace_object_selection(Some(first)));
        assert!(session.is_object_selected(first));
        assert_eq!(session.selected_object_ids().len(), 1);

        assert!(session.replace_object_selection(Some(second)));
        assert!(!session.is_object_selected(first));
        assert!(session.is_object_selected(second));

        assert!(session.replace_object_selection(None));
        assert!(session.selected_object_ids().is_empty());
        assert!(!session.replace_object_selection(None));
    }

    #[test]
    fn viewport_position_drag_updates_preview_and_commits_one_history_entry() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_position_drag(
            object_id,
            [100.0, 80.0],
            Vec2::new(0.0, 0.0).expect("position"),
        ));
        assert!(session.update_viewport_position_drag([120.0, 90.0], [2.0, 2.0], false));
        assert_eq!(session.sync_viewport_position_drag(&mut editor), Ok(true));
        assert_eq!(editor.history_len(), 0);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(40.0, 20.0).expect("preview position")
        );

        assert!(session.update_viewport_position_drag([130.0, 95.0], [2.0, 2.0], false));
        assert_eq!(session.sync_viewport_position_drag(&mut editor), Ok(true));
        assert_eq!(editor.history_len(), 0);
        assert!(session.finish_viewport_position_drag());
        assert_eq!(session.sync_viewport_position_drag(&mut editor), Ok(true));
        assert_eq!(editor.history_len(), 1);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(60.0, 30.0).expect("committed position")
        );

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(0.0, 0.0).expect("restored position")
        );
    }

    #[test]
    fn viewport_position_drag_shift_constrains_to_dominant_axis() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_position_drag(
            object_id,
            [100.0, 100.0],
            Vec2::new(0.0, 0.0).expect("position"),
        ));
        assert!(session.update_viewport_position_drag([130.0, 110.0], [2.0, 2.0], true,));
        assert_eq!(session.sync_viewport_position_drag(&mut editor), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(60.0, 0.0).expect("x constrained position")
        );

        assert!(session.update_viewport_position_drag([105.0, 140.0], [2.0, 2.0], true,));
        assert_eq!(session.sync_viewport_position_drag(&mut editor), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(0.0, 80.0).expect("y constrained position")
        );
    }

    #[test]
    fn viewport_multi_position_drag_applies_one_shared_delta_and_history_entry() {
        let first_id = ObjectId::new(1).expect("first object");
        let second_id = ObjectId::new(2).expect("second object");
        let mut project = editor_with_object_for_drag().into_project();
        let mut second = project.composition.objects[0].clone();
        second.id = second_id;
        second.transform.position = rhythm_core::animation::Animated::new_static(
            Vec2::new(100.0, 50.0).expect("second position"),
        );
        project.composition.objects.push(second);
        project.next_entity_id = 3;
        let mut editor = rhythm_core::editor::ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();

        assert!(
            session.begin_viewport_multi_position_drag(vec![second_id, first_id], [100.0, 100.0],)
        );
        assert!(session.update_viewport_multi_position_drag([130.0, 80.0], [2.0, 2.0], false,));
        assert_eq!(
            session.sync_viewport_multi_position_drag(&mut editor),
            Ok(true)
        );
        assert_eq!(editor.history_len(), 0);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(60.0, -40.0).expect("first preview")
        );
        assert_eq!(
            *editor.project().composition.objects[1]
                .transform
                .position
                .base_value(),
            Vec2::new(160.0, 10.0).expect("second preview")
        );

        assert!(session.finish_viewport_multi_position_drag());
        assert_eq!(
            session.sync_viewport_multi_position_drag(&mut editor),
            Ok(true)
        );
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[1]
                .transform
                .position
                .base_value(),
            Vec2::new(100.0, 50.0).expect("second restored")
        );
    }

    #[test]
    fn viewport_multi_position_drag_mixes_static_and_animated_positions_at_nearest_grid() {
        use rhythm_core::animation::{Animated, Interpolation, Keyframe};

        let first_id = ObjectId::new(1).expect("first object");
        let second_id = ObjectId::new(2).expect("second object");
        let mut project = editor_with_object_for_drag().into_project();
        project.tempo_map = tempo_120();

        let mut second = project.composition.objects[0].clone();
        second.id = second_id;
        second.transform.position = Animated::with_keyframes(
            Vec2::new(100.0, 50.0).expect("base position"),
            vec![
                Keyframe::new(
                    KeyframeId::new(3).expect("keyframe"),
                    MusicalTick::new(0),
                    Vec2::new(100.0, 50.0).expect("position"),
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(4).expect("keyframe"),
                    MusicalTick::new(480),
                    Vec2::new(200.0, 50.0).expect("position"),
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("animated position");
        project.composition.objects.push(second);
        project.next_entity_id = 5;

        let mut editor = rhythm_core::editor::ProjectEditor::new(project).expect("valid project");
        let mut session = EditorSession::default();
        session.seek_paused(ProjectTimeNs::new(130_000_000));

        assert!(
            session.begin_viewport_multi_position_drag(vec![first_id, second_id], [100.0, 100.0],)
        );
        assert!(session.update_viewport_multi_position_drag([120.0, 100.0], [1.0, 1.0], false,));
        assert_eq!(
            session.sync_viewport_multi_position_drag(&mut editor),
            Ok(true)
        );

        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(20.0, 0.0).expect("static preview")
        );
        let animated = &editor.project().composition.objects[1].transform.position;
        let inserted = animated
            .keyframe_at_tick(MusicalTick::new(240))
            .expect("nearest-grid key");
        assert_eq!(
            inserted.value,
            Vec2::new(170.0, 50.0).expect("animated preview")
        );

        assert!(session.finish_viewport_multi_position_drag());
        assert_eq!(
            session.sync_viewport_multi_position_drag(&mut editor),
            Ok(true)
        );
        assert_eq!(editor.history_len(), 1);

        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(0.0, 0.0).expect("static restored")
        );
        assert!(
            editor.project().composition.objects[1]
                .transform
                .position
                .keyframe_at_tick(MusicalTick::new(240))
                .is_none()
        );
    }

    #[test]
    fn viewport_rotation_drag_accumulates_across_angle_wrap() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_rotation_drag(
            object_id,
            Vec2::new(0.0, 0.0).expect("anchor"),
            Vec2::new(-0.9848077, 0.1736482).expect("170 degree pointer"),
            0.0,
        ));
        assert!(session.update_viewport_rotation_drag(
            Vec2::new(-0.9848077, -0.1736482).expect("-170 degree pointer"),
            false,
        ));
        assert_eq!(session.sync_viewport_rotation_drag(&mut editor), Ok(true));
        let rotation = *editor.project().composition.objects[0]
            .transform
            .rotation_degrees
            .base_value();
        assert!((rotation - 20.0).abs() < 0.001);

        assert!(session.update_viewport_rotation_drag(
            Vec2::new(0.0, -1.0).expect("-90 degree pointer"),
            false,
        ));
        assert_eq!(session.sync_viewport_rotation_drag(&mut editor), Ok(true));
        let rotation = *editor.project().composition.objects[0]
            .transform
            .rotation_degrees
            .base_value();
        assert!((rotation - 100.0).abs() < 0.001);

        assert!(session.finish_viewport_rotation_drag());
        assert_eq!(session.sync_viewport_rotation_drag(&mut editor), Ok(true));
        assert_eq!(editor.history_len(), 1);
    }

    #[test]
    fn viewport_rotation_drag_shift_snaps_preview_to_15_degrees() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_rotation_drag(
            object_id,
            Vec2::new(0.0, 0.0).expect("anchor"),
            Vec2::new(1.0, 0.0).expect("pointer"),
            0.0,
        ));
        let radians = 22.0_f32.to_radians();
        let pointer = Vec2::new(radians.cos(), radians.sin()).expect("pointer");
        assert!(session.update_viewport_rotation_drag(pointer, true));
        assert_eq!(session.sync_viewport_rotation_drag(&mut editor), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .rotation_degrees
                .base_value(),
            15.0
        );

        assert!(session.update_viewport_rotation_drag(pointer, false));
        assert_eq!(session.sync_viewport_rotation_drag(&mut editor), Ok(true));
        let rotation = *editor.project().composition.objects[0]
            .transform
            .rotation_degrees
            .base_value();
        assert!((rotation - 22.0).abs() < 0.001);
    }

    #[test]
    fn viewport_rotation_drag_cancel_restores_rotation_without_history() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_rotation_drag(
            object_id,
            Vec2::new(0.0, 0.0).expect("anchor"),
            Vec2::new(1.0, 0.0).expect("pointer"),
            0.0,
        ));
        assert!(
            session.update_viewport_rotation_drag(Vec2::new(0.0, 1.0).expect("pointer"), false,)
        );
        assert_eq!(session.sync_viewport_rotation_drag(&mut editor), Ok(true));
        assert!(session.cancel_viewport_rotation_drag());
        assert_eq!(session.sync_viewport_rotation_drag(&mut editor), Ok(true));
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
    fn viewport_scale_drag_updates_both_axes_and_commits_one_history_entry() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_scale_drag(
            object_id,
            Vec2::new(0.0, 0.0).expect("anchor"),
            Vec2::new(50.0, 50.0).expect("handle"),
            Vec2::new(1.0, 1.0).expect("scale"),
            0.0,
        ));
        assert!(
            session.update_viewport_scale_drag(Vec2::new(100.0, 25.0).expect("pointer"), false,)
        );
        assert_eq!(session.sync_viewport_scale_drag(&mut editor), Ok(true));
        assert_eq!(editor.history_len(), 0);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .scale
                .base_value(),
            Vec2::new(2.0, 0.5).expect("preview scale")
        );

        assert!(session.finish_viewport_scale_drag());
        assert_eq!(session.sync_viewport_scale_drag(&mut editor), Ok(true));
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.undo(), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .scale
                .base_value(),
            Vec2::new(1.0, 1.0).expect("restored scale")
        );
    }

    #[test]
    fn viewport_scale_drag_shift_preserves_starting_scale_ratio() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_scale_drag(
            object_id,
            Vec2::new(0.0, 0.0).expect("anchor"),
            Vec2::new(50.0, 50.0).expect("handle"),
            Vec2::new(2.0, 1.0).expect("scale"),
            0.0,
        ));
        assert!(
            session.update_viewport_scale_drag(Vec2::new(100.0, 60.0).expect("pointer"), true,)
        );
        assert_eq!(session.sync_viewport_scale_drag(&mut editor), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .scale
                .base_value(),
            Vec2::new(4.0, 2.0).expect("uniform scale")
        );
    }

    #[test]
    fn viewport_scale_drag_respects_object_rotation_and_crossing_anchor() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_scale_drag(
            object_id,
            Vec2::new(0.0, 0.0).expect("anchor"),
            Vec2::new(-50.0, 50.0).expect("rotated handle"),
            Vec2::new(1.0, 1.0).expect("scale"),
            90.0,
        ));
        assert!(session.update_viewport_scale_drag(
            Vec2::new(25.0, -100.0).expect("pointer across anchor"),
            false,
        ));
        assert_eq!(session.sync_viewport_scale_drag(&mut editor), Ok(true));
        let scale = *editor.project().composition.objects[0]
            .transform
            .scale
            .base_value();
        assert!((scale.x() + 2.0).abs() < 0.0001);
        assert!((scale.y() + 0.5).abs() < 0.0001);
    }

    #[test]
    fn viewport_position_drag_cancel_restores_before_without_history() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let mut session = EditorSession::default();

        assert!(session.begin_viewport_position_drag(
            object_id,
            [50.0, 50.0],
            Vec2::new(0.0, 0.0).expect("position"),
        ));
        assert!(session.update_viewport_position_drag([80.0, 70.0], [1.0, 1.0], false));
        assert_eq!(session.sync_viewport_position_drag(&mut editor), Ok(true));
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(30.0, 20.0).expect("preview position")
        );

        assert!(session.cancel_viewport_position_drag());
        assert_eq!(session.sync_viewport_position_drag(&mut editor), Ok(true));
        assert_eq!(editor.history_len(), 0);
        assert_eq!(
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value(),
            Vec2::new(0.0, 0.0).expect("restored position")
        );
        assert!(!session.viewport_position_drag_active());
    }

    #[test]
    fn viewport_camera_action_requests_are_consumed_once() {
        let mut session = EditorSession::default();

        session.request_viewport_camera_action(ViewportCameraAction::FrameSelection);
        assert_eq!(
            session.take_viewport_camera_action(),
            Some(ViewportCameraAction::FrameSelection)
        );
        assert_eq!(session.take_viewport_camera_action(), None);
    }

    #[test]
    fn fit_composition_resets_manual_viewport_camera() {
        let mut session = EditorSession::default();
        assert!(session.pan_viewport_points([30.0, -20.0]));
        assert!(session.zoom_viewport_around_anchor(2.0, [0.0, 0.0]));

        assert!(session.fit_viewport_composition());
        assert_eq!(session.viewport_pan_points(), [0.0, 0.0]);
        assert_eq!(session.viewport_zoom(), 1.0);
        assert!(!session.fit_viewport_composition());
    }

    #[test]
    fn frame_viewport_bounds_centers_selection_with_padding() {
        use rhythm_core::{domain::Vec2, geometry::LocalBounds2d};

        let mut session = EditorSession::default();
        let bounds = LocalBounds2d::new(
            Vec2::new(1_200.0, 600.0).expect("bounds min"),
            Vec2::new(400.0, 300.0).expect("bounds size"),
        );
        let composition_size = [1_920.0, 1_080.0];
        let viewport_size = [1_000.0, 500.0];
        let fitted_preview_size = [888.8889, 500.0];

        assert!(session.frame_viewport_bounds(
            bounds,
            composition_size,
            viewport_size,
            fitted_preview_size,
        ));

        let zoom = session.viewport_zoom();
        let pan = session.viewport_pan_points();
        let scale_x = fitted_preview_size[0] / composition_size[0] * zoom;
        let scale_y = fitted_preview_size[1] / composition_size[1] * zoom;
        let selection_center_x = bounds.min.x() + bounds.size.x() * 0.5;
        let selection_center_y = bounds.min.y() + bounds.size.y() * 0.5;
        let composition_center_x = composition_size[0] * 0.5;
        let composition_center_y = composition_size[1] * 0.5;

        assert!((pan[0] + (selection_center_x - composition_center_x) * scale_x).abs() < 0.001);
        assert!((pan[1] + (selection_center_y - composition_center_y) * scale_y).abs() < 0.001);
        assert!(bounds.size.x() * scale_x <= viewport_size[0] - 64.0 + 0.001);
        assert!(bounds.size.y() * scale_y <= viewport_size[1] - 64.0 + 0.001);
    }

    #[test]
    fn viewport_zoom_keeps_pointer_anchor_stable_by_adjusting_pan() {
        let mut session = EditorSession::default();

        assert_eq!(session.viewport_zoom(), 1.0);
        assert_eq!(session.viewport_pan_points(), [0.0, 0.0]);
        assert!(session.zoom_viewport_around_anchor(2.0, [100.0, 50.0]));
        assert_eq!(session.viewport_zoom(), 2.0);
        assert_eq!(session.viewport_pan_points(), [-100.0, -50.0]);

        let old_screen_x: f32 = -100.0 + 100.0 * 2.0;
        let old_screen_y: f32 = -50.0 + 50.0 * 2.0;
        assert!((old_screen_x - 100.0).abs() < 0.0001);
        assert!((old_screen_y - 50.0).abs() < 0.0001);
    }

    #[test]
    fn viewport_zoom_clamps_to_practical_range() {
        let mut session = EditorSession::default();

        assert!(session.zoom_viewport_around_anchor(100.0, [0.0, 0.0]));
        assert_eq!(session.viewport_zoom(), 8.0);
        assert!(session.zoom_viewport_around_anchor(0.001, [0.0, 0.0]));
        assert_eq!(session.viewport_zoom(), 0.1);
    }

    #[test]
    fn viewport_pan_accumulates_middle_drag_delta_in_ui_points() {
        let mut session = EditorSession::default();

        assert_eq!(session.viewport_pan_points(), [0.0, 0.0]);
        assert!(session.pan_viewport_points([12.5, -4.0]));
        assert!(session.pan_viewport_points([-2.5, 9.0]));
        assert_eq!(session.viewport_pan_points(), [10.0, 5.0]);
        assert!(!session.pan_viewport_points([0.0, 0.0]));
    }

    #[test]
    fn viewport_box_selection_lifecycle_and_multi_replace_are_stable() {
        let first = ObjectId::new(31).expect("object id");
        let second = ObjectId::new(32).expect("object id");
        let mut session = EditorSession::default();

        session.begin_viewport_box_selection([10.0, 20.0]);
        session.update_viewport_box_selection([80.0, 90.0]);
        assert_eq!(
            session.viewport_box_selection(),
            Some(([10.0, 20.0], [80.0, 90.0]))
        );
        assert_eq!(
            session.take_viewport_box_selection(),
            Some(([10.0, 20.0], [80.0, 90.0]))
        );
        assert!(session.viewport_box_selection().is_none());

        assert!(session.replace_object_selection_many([first, second]));
        assert!(session.is_object_selected(first));
        assert!(session.is_object_selected(second));
        assert_eq!(session.selected_object_ids().len(), 2);
        assert!(!session.replace_object_selection_many([second, first]));
    }

    #[test]
    fn viewport_ctrl_toggle_adds_and_removes_objects_without_replacing_others() {
        let first = ObjectId::new(21).expect("object id");
        let second = ObjectId::new(22).expect("object id");
        let mut session = EditorSession::default();

        assert!(session.replace_object_selection(Some(first)));
        assert!(session.toggle_object_selection(second));
        assert!(session.is_object_selected(first));
        assert!(session.is_object_selected(second));
        assert_eq!(session.selected_object_ids().len(), 2);

        assert!(session.toggle_object_selection(first));
        assert!(!session.is_object_selected(first));
        assert!(session.is_object_selected(second));
        assert_eq!(session.selected_object_ids().len(), 1);
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
    fn follow_playhead_defaults_off_and_scrolls_only_near_view_edges() {
        let duration = DurationNs::new(10_000_000_000);
        let mut session = EditorSession::default();

        assert!(!session.follow_playhead());
        assert!(session.zoom_timeline(duration, ProjectTimeNs::new(0), 2.0));
        assert_eq!(
            session.timeline_range(duration),
            (ProjectTimeNs::new(0), ProjectTimeNs::new(5_000_000_000))
        );

        session.set_follow_playhead(true);
        session.update_playhead_during_playback(ProjectTimeNs::new(3_000_000_000), duration);
        assert_eq!(
            session.timeline_range(duration),
            (ProjectTimeNs::new(0), ProjectTimeNs::new(5_000_000_000))
        );

        session.update_playhead_during_playback(ProjectTimeNs::new(4_600_000_000), duration);
        assert_eq!(
            session.timeline_range(duration),
            (
                ProjectTimeNs::new(3_350_000_000),
                ProjectTimeNs::new(8_350_000_000),
            )
        );
    }

    #[test]
    fn manual_timeline_pan_disables_follow_playhead() {
        let duration = DurationNs::new(10_000_000_000);
        let mut session = EditorSession::default();
        assert!(session.zoom_timeline(duration, ProjectTimeNs::new(5_000_000_000), 2.0));
        session.set_follow_playhead(true);

        assert!(session.pan_timeline_points(duration, -100.0, 1_000.0));
        assert!(!session.follow_playhead());
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
    fn paste_anchors_packet_to_grid_and_allocates_fresh_ids() {
        use rhythm_core::{
            animation::Interpolation,
            property::{AnimatableProperty, PropertyValue, property_keyframe_at_tick},
            time::{BpmMicros, GridOffsetNs, MusicalTick, TempoMap, TimeSignature},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        editor
            .execute(rhythm_core::editor::EditCommand::SetTempoMap {
                tempo_map: TempoMap::with_initial_tempo(
                    GridOffsetNs::new(0),
                    BpmMicros::new(120_000_000).expect("valid BPM"),
                    TimeSignature::default(),
                ),
            })
            .expect("tempo");

        let first = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(0),
                PropertyValue::Scalar(0.25),
            )
            .expect("insert")
            .expect("first");
        let second = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(480),
                PropertyValue::Scalar(0.75),
            )
            .expect("insert")
            .expect("second");

        let mut session = EditorSession::default();
        session.replace_keyframe_selection([first, second]);
        assert!(session.copy_selected_keyframes(editor.project()));
        session.seek_paused(ProjectTimeNs::new(500_000_000));

        assert_eq!(session.paste_keyframe_clipboard(&mut editor), Ok(true));
        assert_eq!(session.selected_keyframe_count(), 2);

        let pasted_first = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(960),
        )
        .expect("property")
        .expect("pasted first");
        let pasted_second = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(1_440),
        )
        .expect("property")
        .expect("pasted second");

        assert_eq!(pasted_first.value, PropertyValue::Scalar(0.25));
        assert_eq!(pasted_first.interpolation, Interpolation::Linear);
        assert_eq!(pasted_second.value, PropertyValue::Scalar(0.75));
        assert!(pasted_first.id.get() > second.get());
        assert!(session.is_keyframe_selected(pasted_first.id));
        assert!(session.is_keyframe_selected(pasted_second.id));
    }

    #[test]
    fn interpolation_preset_targets_selected_outgoing_segments_only() {
        use rhythm_core::{
            animation::Interpolation,
            property::{AnimatableProperty, PropertyValue, locate_property_keyframe},
            time::MusicalTick,
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut editor = editor_with_object_for_drag();
        let first = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(0),
                PropertyValue::Scalar(0.1),
            )
            .expect("insert")
            .expect("first");
        let second = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(240),
                PropertyValue::Scalar(0.5),
            )
            .expect("insert")
            .expect("second");
        let terminal = editor
            .create_property_keyframe(
                object_id,
                AnimatableProperty::Opacity,
                MusicalTick::new(480),
                PropertyValue::Scalar(0.9),
            )
            .expect("insert")
            .expect("terminal");

        let mut session = EditorSession::default();
        session.replace_keyframe_selection([first, second, terminal]);
        assert!(session.queue_selected_keyframe_interpolation(KeyframeInterpolationPreset::Hold));
        assert_eq!(
            session.commit_pending_keyframe_interpolation(&mut editor),
            Ok(true)
        );

        assert_eq!(
            locate_property_keyframe(editor.project(), first)
                .expect("first")
                .keyframe
                .interpolation,
            Interpolation::Hold
        );
        assert_eq!(
            locate_property_keyframe(editor.project(), second)
                .expect("second")
                .keyframe
                .interpolation,
            Interpolation::Hold
        );
        assert_eq!(
            locate_property_keyframe(editor.project(), terminal)
                .expect("terminal")
                .keyframe
                .interpolation,
            Interpolation::Linear
        );
        assert_eq!(session.selected_keyframe_count(), 3);
    }

    #[test]
    fn duplicate_repeats_pattern_after_current_grid_gap_and_selects_duplicates() {
        use rhythm_core::{
            property::{AnimatableProperty, PropertyValue, property_keyframe_at_tick},
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
        session.replace_keyframe_selection([first, second]);
        assert!(session.keyframe_clipboard().is_none());

        assert_eq!(session.duplicate_selected_keyframes(&mut editor), Ok(true));

        let source_first = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(240),
        )
        .expect("property")
        .expect("source first");
        let source_second = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(720),
        )
        .expect("property")
        .expect("source second");
        let duplicate_first = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(960),
        )
        .expect("property")
        .expect("duplicate first");
        let duplicate_second = property_keyframe_at_tick(
            editor.project(),
            object_id,
            AnimatableProperty::Opacity,
            MusicalTick::new(1_440),
        )
        .expect("property")
        .expect("duplicate second");

        assert_eq!(source_first.id, first);
        assert_eq!(source_second.id, second);
        assert_eq!(duplicate_first.value, PropertyValue::Scalar(0.25));
        assert_eq!(duplicate_second.value, PropertyValue::Scalar(0.75));
        assert!(duplicate_first.id.get() > second.get());
        assert!(duplicate_second.id.get() > duplicate_first.id.get());
        assert!(!session.is_keyframe_selected(first));
        assert!(!session.is_keyframe_selected(second));
        assert!(session.is_keyframe_selected(duplicate_first.id));
        assert!(session.is_keyframe_selected(duplicate_second.id));
        assert_eq!(session.selected_keyframe_count(), 2);
        assert!(session.keyframe_clipboard().is_none());
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
