use crate::{
    editor_session::{
        EditorSession, PreviewQuality, ViewportGroupPositionAction, ViewportPositionAction,
        ViewportRotationAction, ViewportScaleAction,
    },
    timeline::{draw_easing_preset_menu, draw_timeline},
};
use rhythm_core::{
    domain::Vec2,
    editor::{EditCommand, EditError, ProjectEditor},
    hit_test::{OutlineKind, object_outline, pick_topmost_object, select_objects_in_box},
    ids::ObjectId,
    project::{Object, ObjectContent, Project},
    property::{
        AnimatableProperty, PropertyValue, evaluate_property_at_tick, property_base_value,
        property_keyframe_at_tick, property_keyframe_count,
    },
    time::{ProjectTimeNs, snap_tick_position_to_grid},
};

fn fit_composition_preview(available: egui::Vec2, composition_size: [u32; 2]) -> egui::Vec2 {
    if available.x <= 0.0 || available.y <= 0.0 || composition_size.contains(&0) {
        return egui::Vec2::ZERO;
    }

    let composition_aspect = composition_size[0] as f32 / composition_size[1] as f32;
    let available_aspect = available.x / available.y;
    if available_aspect > composition_aspect {
        egui::vec2(available.y * composition_aspect, available.y)
    } else {
        egui::vec2(available.x, available.x / composition_aspect)
    }
}

/// Converts a click inside the displayed composition texture into project pixels.
/// Must use the image's actual response rectangle, not the whole viewport panel.
fn composition_point_from_preview(
    rect: egui::Rect,
    pointer: egui::Pos2,
    size: [u32; 2],
) -> Option<Vec2> {
    if !rect.contains(pointer)
        || rect.width() <= 0.0
        || rect.height() <= 0.0
        || size[0] == 0
        || size[1] == 0
    {
        return None;
    }
    let u = (pointer.x - rect.min.x) / rect.width();
    let v = (pointer.y - rect.min.y) / rect.height();
    Vec2::new(u * size[0] as f32, v * size[1] as f32).ok()
}

/// Use the exact preview camera for unbounded object moves. Unlike marquee,
/// Position drags may extend beyond the composition boundary.
fn unbounded_composition_point(
    rect: egui::Rect,
    pointer: egui::Pos2,
    size: [u32; 2],
) -> Option<Vec2> {
    if !pointer.x.is_finite()
        || !pointer.y.is_finite()
        || rect.width() <= 0.0
        || rect.height() <= 0.0
        || size.contains(&0)
    {
        return None;
    }
    let x = (pointer.x - rect.min.x) / rect.width() * size[0] as f32;
    let y = (pointer.y - rect.min.y) / rect.height() * size[1] as f32;
    Vec2::new(x, y).ok()
}

/// Maps a pointer that can leave the preview during dragging back to its
/// nearest point inside the displayed image. Only used once the drag began
/// inside the composition; letterbox chrome cannot start marquee selection.
/// Translate the fitted composition without changing project object coordinates.
fn panned_preview_rect(
    canvas: egui::Rect,
    fitted_size: egui::Vec2,
    pan_points: [f32; 2],
) -> egui::Rect {
    egui::Rect::from_center_size(
        canvas.center() + egui::vec2(pan_points[0], pan_points[1]),
        fitted_size,
    )
}

fn clamped_drag_point(rect: egui::Rect, pointer: egui::Pos2, size: [u32; 2]) -> Option<Vec2> {
    let position = egui::pos2(
        pointer.x.clamp(rect.min.x, rect.max.x),
        pointer.y.clamp(rect.min.y, rect.max.y),
    );
    composition_point_from_preview(rect, position, size)
}

/// Converts composition pixels into displayed preview logical points.
fn preview_point(rect: egui::Rect, point: Vec2, size: [u32; 2]) -> egui::Pos2 {
    egui::pos2(
        rect.min.x + point.x() * rect.width() / size[0] as f32,
        rect.min.y + point.y() * rect.height() / size[1] as f32,
    )
}

fn draw_marquee(
    ui: &egui::Ui,
    preview: egui::Rect,
    clip: egui::Rect,
    start: Vec2,
    current: Vec2,
    size: [u32; 2],
) {
    let rect = egui::Rect::from_two_pos(
        preview_point(preview, start, size),
        preview_point(preview, current, size),
    );
    let painter = ui.painter().with_clip_rect(preview.intersect(clip));
    painter.rect_filled(
        rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(95, 185, 240, 26),
    );
    let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(130, 210, 255));
    painter.add(egui::Shape::line(
        vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
            rect.left_top(),
        ],
        stroke,
    ));
}

/// Union of visible, selected Rectangle/Ellipse outlines in composition
/// space. Image and Text require runtime bounds which are not available yet;
/// never fabricate selection bounds for those object kinds.
fn selected_outline_bounds(
    project: &Project,
    session: &EditorSession,
    continuous_tick: f64,
) -> Option<(Vec2, Vec2)> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for object in &project.composition.objects {
        if !object.visible || !session.is_object_selected(object.id) {
            continue;
        }
        let Some((min, max)) =
            object_outline(object, continuous_tick).and_then(|shape| shape.composition_bounds())
        else {
            continue;
        };
        bounds = Some(match bounds {
            Some((x0, y0, x1, y1)) => (
                x0.min(min.x()),
                y0.min(min.y()),
                x1.max(max.x()),
                y1.max(max.y()),
            ),
            None => (min.x(), min.y(), max.x(), max.y()),
        });
    }
    let (min_x, min_y, max_x, max_y) = bounds?;
    Some((Vec2::new(min_x, min_y).ok()?, Vec2::new(max_x, max_y).ok()?))
}

/// Projects composition-space bounds through the actual pan/zoom camera
/// image rectangle. Returns none for empty or invalid geometry.
fn selection_rect_on_preview(
    preview: egui::Rect,
    bounds: (Vec2, Vec2),
    composition_size: [u32; 2],
) -> Option<egui::Rect> {
    let (min, max) = bounds;
    if composition_size.contains(&0) || min.x() >= max.x() || min.y() >= max.y() {
        return None;
    }
    let a = preview_point(preview, min, composition_size);
    let b = preview_point(preview, max, composition_size);
    if !a.x.is_finite() || !a.y.is_finite() || !b.x.is_finite() || !b.y.is_finite() {
        return None;
    }
    Some(egui::Rect::from_min_max(a, b))
}

/// Draws a crisp AABB independently of the transformed wireframe outline.
fn paint_selection_bounds(painter: &egui::Painter, bounds: egui::Rect, stroke: egui::Stroke) {
    painter.add(egui::Shape::line(
        vec![
            bounds.left_top(),
            bounds.right_top(),
            bounds.right_bottom(),
            bounds.left_bottom(),
            bounds.left_top(),
        ],
        stroke,
    ));
}

/// Corners are in local normalized shape coordinates, not the rotated AABB.
const SCALE_CORNERS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

fn editable_scale_object<'a>(project: &'a Project, session: &EditorSession) -> Option<&'a Object> {
    if session.selected_objects().len() != 1 {
        return None;
    }
    let id = session.selected_object()?;
    let object = project
        .composition
        .objects
        .iter()
        .find(|object| object.id == id)?;
    if !object.visible || object.locked || !object.transform.scale.keyframes().is_empty() {
        return None;
    }
    match object.content {
        ObjectContent::Rectangle(_) | ObjectContent::Ellipse(_) => Some(object),
        _ => None,
    }
}

/// Rotation is independently editable from Scale: animated Scale may have
/// a static Rotation and vice versa. Only one visible, unlocked shape qualifies.
fn editable_rotation_object<'a>(
    project: &'a Project,
    session: &EditorSession,
) -> Option<&'a Object> {
    if session.selected_objects().len() != 1 {
        return None;
    }
    let id = session.selected_object()?;
    let object = project
        .composition
        .objects
        .iter()
        .find(|object| object.id == id)?;
    if !object.visible || object.locked || !object.transform.rotation_degrees.keyframes().is_empty()
    {
        return None;
    }
    match object.content {
        ObjectContent::Rectangle(_) | ObjectContent::Ellipse(_) => Some(object),
        _ => None,
    }
}

/// Place a rotation knob 28 egui logical points outward from the transformed
/// local top edge. The offset and 11-point hit radius do not scale with zoom.
/// Mirrored Y scales reverse the local outward normal, as the visible shape does.
fn rotation_handle_on_preview(
    object: &Object,
    preview: egui::Rect,
    composition_size: [u32; 2],
    continuous_tick: f64,
) -> Option<(egui::Pos2, egui::Pos2)> {
    if composition_size.contains(&0) {
        return None;
    }
    let geometry = object_outline(object, continuous_tick)?;
    let top = preview_point(preview, geometry.point(0.5, 0.0)?, composition_size);
    let bottom = preview_point(preview, geometry.point(0.5, 1.0)?, composition_size);
    let outward = top - bottom;
    let length = outward.length();
    if !length.is_finite() || length <= 1e-3 {
        return None;
    }
    let knob = top + outward / length * 28.0;
    if !knob.x.is_finite() || !knob.y.is_finite() {
        return None;
    }
    Some((top, knob))
}

/// Unlike corner handles, the rotation knob may lie just outside the
/// composition texture, but never outside the editor canvas.
fn selected_rotation_handle_hit(
    project: &Project,
    session: &EditorSession,
    preview: egui::Rect,
    canvas: egui::Rect,
    continuous_tick: f64,
    pointer: egui::Pos2,
) -> Option<ObjectId> {
    if !canvas.contains(pointer) {
        return None;
    }
    let object = editable_rotation_object(project, session)?;
    let composition_size = [
        project.settings.composition_width,
        project.settings.composition_height,
    ];
    let (_, knob) = rotation_handle_on_preview(object, preview, composition_size, continuous_tick)?;
    (knob.distance_sq(pointer) <= 121.0).then_some(object.id)
}

/// Screen-point-sized hit boxes remain invariant under viewport zoom/pan.
/// A corner coinciding exactly with Anchor cannot produce a scale delta.
fn selected_scale_corner_hit(
    project: &Project,
    session: &EditorSession,
    preview: egui::Rect,
    canvas: egui::Rect,
    continuous_tick: f64,
    pointer: egui::Pos2,
) -> Option<(ObjectId, [f32; 2])> {
    if !preview.intersect(canvas).contains(pointer) {
        return None;
    }
    let object = editable_scale_object(project, session)?;
    let geometry = object_outline(object, continuous_tick)?;
    let anchor = scale_anchor(project, object, continuous_tick)?;
    let size = [
        project.settings.composition_width,
        project.settings.composition_height,
    ];
    let mut nearest: Option<([f32; 2], f32)> = None;
    for corner in SCALE_CORNERS {
        if corner == [anchor.x(), anchor.y()] {
            continue;
        }
        let Some(point) = geometry.point(f64::from(corner[0]), f64::from(corner[1])) else {
            continue;
        };
        let center = preview_point(preview, point, size);
        let distance_sq = center.distance_sq(pointer);
        if distance_sq <= 100.0 && nearest.is_none_or(|(_, d)| distance_sq < d) {
            nearest = Some((corner, distance_sq));
        }
    }
    nearest.map(|(corner, _)| (object.id, corner))
}

fn scale_anchor(project: &Project, object: &Object, tick: f64) -> Option<Vec2> {
    match evaluate_property_at_tick(project, object.id, AnimatableProperty::Anchor, tick).ok()? {
        PropertyValue::Vec2(anchor) => Some(anchor),
        _ => None,
    }
}

fn scale_rotation(project: &Project, object: &Object, tick: f64) -> Option<f32> {
    match evaluate_property_at_tick(project, object.id, AnimatableProperty::Rotation, tick).ok()? {
        PropertyValue::Scalar(rotation) => Some(rotation),
        _ => None,
    }
}

fn scale_shape_size(project: &Project, object: &Object, tick: f64) -> Option<Vec2> {
    let property = match object.content {
        ObjectContent::Rectangle(_) => AnimatableProperty::RectangleSize,
        ObjectContent::Ellipse(_) => AnimatableProperty::EllipseSize,
        _ => return None,
    };
    match evaluate_property_at_tick(project, object.id, property, tick).ok()? {
        PropertyValue::Vec2(size) => Some(size),
        _ => None,
    }
}

/// Viewport-only selection bounds and actual transform pivot markers.
/// Rendered after the wireframes, clipped to the canvas; no Project mutations,
/// GPU readback or guessed Image/Text bounds. Locked objects selected from the
/// Objects panel can still show their noninteractive selection overlays.
fn draw_selection_overlays(
    ui: &egui::Ui,
    preview: egui::Rect,
    canvas: egui::Rect,
    session: &EditorSession,
    project: &Project,
    continuous_tick: f64,
) {
    let composition_size = [
        project.settings.composition_width,
        project.settings.composition_height,
    ];
    if composition_size.contains(&0) {
        return;
    }
    let painter = ui.painter().with_clip_rect(preview.intersect(canvas));
    let selected_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 210, 80));
    let pivot_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 210, 80));
    let mut shown = 0_usize;

    for object in &project.composition.objects {
        if !object.visible || !session.is_object_selected(object.id) {
            continue;
        }
        let Some(geometry) = object_outline(object, continuous_tick) else {
            continue;
        };
        let Some(bounds) = geometry
            .composition_bounds()
            .and_then(|bounds| selection_rect_on_preview(preview, bounds, composition_size))
        else {
            continue;
        };

        paint_selection_bounds(&painter, bounds, selected_stroke);
        shown += 1;

        // Corner handles follow local transformed geometry, not the AABB.
        // Only one visible, unlocked, static-scale shape may be resized.
        if editable_scale_object(project, session).is_some_and(|shape| shape.id == object.id)
            && let Some(anchor) = scale_anchor(project, object, continuous_tick)
        {
            for corner in SCALE_CORNERS {
                if corner == [anchor.x(), anchor.y()] {
                    continue;
                }
                if let Some(point) = geometry.point(f64::from(corner[0]), f64::from(corner[1])) {
                    let center = preview_point(preview, point, composition_size);
                    if center.x.is_finite() && center.y.is_finite() {
                        painter.rect_filled(
                            egui::Rect::from_center_size(center, egui::vec2(10.0, 10.0)),
                            1.0,
                            egui::Color32::BLACK,
                        );
                        painter.rect_filled(
                            egui::Rect::from_center_size(center, egui::vec2(7.0, 7.0)),
                            0.5,
                            egui::Color32::from_rgb(255, 210, 80),
                        );
                    }
                }
            }
        }

        if editable_rotation_object(project, session).is_some_and(|shape| shape.id == object.id)
            && let Some((top, knob)) =
                rotation_handle_on_preview(object, preview, composition_size, continuous_tick)
        {
            // This knob deliberately extends beyond the texture when a shape
            // touches the top edge; only the containing editor canvas clips it.
            let rotation_painter = ui.painter().with_clip_rect(canvas);
            rotation_painter.line_segment(
                [top, knob],
                egui::Stroke::new(1.5, egui::Color32::from_rgb(130, 210, 255)),
            );
            rotation_painter.circle_filled(knob, 7.0, egui::Color32::BLACK);
            rotation_painter.circle_filled(knob, 4.5, egui::Color32::from_rgb(130, 210, 255));
        }

        // Position marks the actual transform Anchor, not the visual center
        // when the normalized Anchor is noncentral. Keep marker size constant
        // in screen points regardless of camera zoom and device DPI.
        let pivot = preview_point(preview, geometry.anchor_position(), composition_size);
        if pivot.x.is_finite() && pivot.y.is_finite() {
            painter.circle_stroke(pivot, 5.0, egui::Stroke::new(3.0, egui::Color32::BLACK));
            painter.circle_stroke(pivot, 5.0, pivot_stroke);
            painter.line_segment(
                [pivot + egui::vec2(-8.0, 0.0), pivot + egui::vec2(8.0, 0.0)],
                pivot_stroke,
            );
            painter.line_segment(
                [pivot + egui::vec2(0.0, -8.0), pivot + egui::vec2(0.0, 8.0)],
                pivot_stroke,
            );
        }
    }

    // Multiple selected objects share one additional group context, while
    // each individual object retains its own pivot marker.
    if shown > 1
        && let Some(bounds) = selected_outline_bounds(project, session, continuous_tick)
        && let Some(bounds) = selection_rect_on_preview(preview, bounds, composition_size)
    {
        paint_selection_bounds(
            &painter,
            bounds,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(130, 210, 255)),
        );
    }
}

/// Single dispatch point for viewport and Objects-panel click semantics.
fn apply_object_click(session: &mut EditorSession, picked: Option<ObjectId>, ctrl: bool) {
    if ctrl {
        session.toggle_viewport_object(picked);
    } else {
        session.select_viewport_object(picked);
    }
}

/// Transparent editor-only object outlines, visibly distinct from rendered output.
/// This bridge will be removed once the composition GPU renderer draws content.
fn draw_editor_outlines(
    ui: &egui::Ui,
    preview: egui::Rect,
    clip: egui::Rect,
    session: &EditorSession,
    project: &Project,
    continuous_tick: f64,
) {
    let width = project.settings.composition_width as f32;
    let height = project.settings.composition_height as f32;
    let painter = ui.painter().with_clip_rect(preview.intersect(clip));
    for object in &project.composition.objects {
        if !object.visible {
            continue;
        }
        let Some(geometry) = object_outline(object, continuous_tick) else {
            continue;
        };
        let selected = session.is_object_selected(object.id);
        let stroke = egui::Stroke::new(
            if selected { 2.5 } else { 1.0 },
            if selected {
                egui::Color32::from_rgb(255, 210, 80)
            } else if object.locked {
                egui::Color32::from_gray(105)
            } else {
                egui::Color32::from_rgb(95, 185, 240)
            },
        );
        let count = match geometry.kind {
            OutlineKind::Rectangle => 4,
            OutlineKind::Ellipse => 40,
        };
        let mut points = Vec::with_capacity(count + 1);
        for i in 0..count {
            let (u, v) = match geometry.kind {
                OutlineKind::Rectangle => match i {
                    0 => (0.0, 0.0),
                    1 => (1.0, 0.0),
                    2 => (1.0, 1.0),
                    _ => (0.0, 1.0),
                },
                OutlineKind::Ellipse => {
                    let radians = std::f64::consts::TAU * (i as f64) / (count as f64);
                    (0.5 + 0.5 * radians.cos(), 0.5 + 0.5 * radians.sin())
                }
            };
            let Some(point) = geometry.point(u, v) else {
                points.clear();
                break;
            };
            points.push(egui::pos2(
                preview.min.x + point.x() / width * preview.width(),
                preview.min.y + point.y() / height * preview.height(),
            ));
        }
        if points.len() == count {
            points.push(points[0]);
            painter.add(egui::Shape::line(points, stroke));
        }
    }
}

/// A commit of one static Inspector Position axis; updates recompute the
/// untouched axis from the current ProjectEditor state, never from a stale UI snapshot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorPositionEdit {
    pub object_id: ObjectId,
    pub axis: PositionAxis,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionAxis {
    X,
    Y,
}

impl PositionAxis {
    const fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
        }
    }
}

#[derive(Clone, Debug)]
struct InspectorNumericDraft {
    text: String,
    dirty: bool,
}

fn inspector_position_id(object_id: ObjectId, axis: PositionAxis) -> egui::Id {
    // Stable across different nested egui panels/horizontal layouts, so a
    // selection change can flush the same draft that TextEdit produced.
    egui::Id::new(("inspector_position", object_id.get(), axis.label()))
}

fn committed_number(draft: &InspectorNumericDraft) -> Option<f32> {
    if !draft.dirty {
        return None;
    }
    draft
        .text
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

/// One valid focus-loss/Enter commit becomes one EditCommand and one Undo step.
/// Editing animated, locked or missing objects is deliberately not supported yet.
pub fn apply_inspector_position_edits(
    editor: &mut ProjectEditor,
    edits: impl IntoIterator<Item = InspectorPositionEdit>,
) -> Result<bool, EditError> {
    let mut changed = false;
    for edit in edits {
        let Some(object) = editor
            .project()
            .composition
            .objects
            .iter()
            .find(|object| object.id == edit.object_id)
        else {
            continue;
        };
        if object.locked || !object.transform.position.keyframes().is_empty() {
            continue;
        }
        let current = *object.transform.position.base_value();
        let (x, y) = match edit.axis {
            PositionAxis::X => (edit.value, current.y()),
            PositionAxis::Y => (current.x(), edit.value),
        };
        let Ok(position) = Vec2::new(x, y) else {
            continue;
        };
        changed |= editor.execute(EditCommand::SetPositionBase {
            object_id: edit.object_id,
            value: position,
        })?;
    }
    Ok(changed)
}

/// A numeric Inspector commit for any static transform property. Vec2 axes
/// are edited separately but each one preserves the other current component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorTransformEdit {
    pub object_id: ObjectId,
    pub property: AnimatableProperty,
    pub axis: Option<PositionAxis>,
    /// Value in Inspector display units (percent for Scale/Anchor/Opacity).
    pub display_value: f32,
}

/// Animated Inspector edits update the nearest authoring-grid key, never
/// the static base, and move the playhead after a successful commit.
pub fn apply_inspector_animated_transform_edits(
    session: &mut EditorSession,
    editor: &mut ProjectEditor,
    edits: impl IntoIterator<Item = InspectorTransformEdit>,
) -> Result<bool, EditError> {
    let mut changed = false;
    for edit in edits {
        let project = editor.project();
        let Some(object) = project
            .composition
            .objects
            .iter()
            .find(|object| object.id == edit.object_id)
        else {
            continue;
        };
        if object.locked || property_keyframe_count(project, edit.object_id, edit.property)? == 0 {
            continue;
        }
        let Ok(continuous_tick) = project
            .tempo_map
            .continuous_tick_position(session.playhead())
        else {
            continue;
        };
        let Ok(resolved_tick) =
            snap_tick_position_to_grid(continuous_tick, session.authoring_division())
        else {
            continue;
        };
        let Ok(resolved_time) = project.tempo_map.project_time_for_tick(resolved_tick) else {
            continue;
        };
        let existing =
            property_keyframe_at_tick(project, edit.object_id, edit.property, resolved_tick)?;
        let current = if let Some(key) = existing {
            key.value
        } else {
            evaluate_property_at_tick(project, edit.object_id, edit.property, continuous_tick)?
        };
        let is_percent = matches!(
            edit.property,
            AnimatableProperty::Scale | AnimatableProperty::Anchor | AnimatableProperty::Opacity
        );
        let scalar = if is_percent {
            edit.display_value / 100.0
        } else {
            edit.display_value
        };
        if !scalar.is_finite() {
            continue;
        }
        let replacement = match (current, edit.axis) {
            (PropertyValue::Vec2(previous), Some(axis)) => {
                let (x, y) = match axis {
                    PositionAxis::X => (scalar, previous.y()),
                    PositionAxis::Y => (previous.x(), scalar),
                };
                let Ok(value) = Vec2::new(x, y) else { continue };
                PropertyValue::Vec2(value)
            }
            (PropertyValue::Scalar(_), None) => PropertyValue::Scalar(scalar),
            _ => continue,
        };
        let applied = if let Some(key) = existing {
            editor.execute(EditCommand::SetPropertyKeyframeValue {
                object_id: edit.object_id,
                property: edit.property,
                keyframe_id: key.id,
                value: replacement,
            })?
        } else {
            editor
                .create_property_keyframe(
                    edit.object_id,
                    edit.property,
                    resolved_tick,
                    replacement,
                )?
                .is_some()
        };
        if applied {
            session.seek_paused(resolved_time);
        }
        changed |= applied;
    }
    Ok(changed)
}

pub fn apply_inspector_transform_edits(
    editor: &mut ProjectEditor,
    edits: impl IntoIterator<Item = InspectorTransformEdit>,
) -> Result<bool, EditError> {
    let mut changed = false;
    for edit in edits {
        let Some(object) = editor
            .project()
            .composition
            .objects
            .iter()
            .find(|object| object.id == edit.object_id)
        else {
            continue;
        };
        if object.locked
            || property_keyframe_count(editor.project(), edit.object_id, edit.property)? != 0
        {
            continue;
        }
        let current = property_base_value(editor.project(), edit.object_id, edit.property)?;
        let is_percent = matches!(
            edit.property,
            AnimatableProperty::Scale | AnimatableProperty::Anchor | AnimatableProperty::Opacity
        );
        let value = if is_percent {
            edit.display_value / 100.0
        } else {
            edit.display_value
        };
        if !value.is_finite() {
            continue;
        }
        let replacement = match (current, edit.axis) {
            (PropertyValue::Vec2(previous), Some(axis)) => {
                let (x, y) = match axis {
                    PositionAxis::X => (value, previous.y()),
                    PositionAxis::Y => (previous.x(), value),
                };
                let Ok(value) = Vec2::new(x, y) else {
                    continue;
                };
                PropertyValue::Vec2(value)
            }
            (PropertyValue::Scalar(_), None) => PropertyValue::Scalar(value),
            _ => continue,
        };
        changed |= editor.execute(EditCommand::SetPropertyBase {
            object_id: edit.object_id,
            property: edit.property,
            value: replacement,
        })?;
    }
    Ok(changed)
}

fn inspector_transform_id(
    object_id: ObjectId,
    property: AnimatableProperty,
    axis: Option<PositionAxis>,
) -> egui::Id {
    egui::Id::new((
        "inspector_transform",
        object_id.get(),
        property,
        axis.map(PositionAxis::label),
    ))
}

fn draw_inspector_transform_field(
    ui: &mut egui::Ui,
    object_id: ObjectId,
    property: AnimatableProperty,
    axis: Option<PositionAxis>,
    current: f32,
    animated: bool,
    actions: &mut EditorUiActions,
) -> bool {
    let id = inspector_transform_id(object_id, property, axis);
    let draft_id = id.with("draft");
    let mut draft = ui
        .ctx()
        .data(|data| data.get_temp::<InspectorNumericDraft>(draft_id))
        .unwrap_or_else(|| InspectorNumericDraft {
            text: current.to_string(),
            dirty: false,
        });
    if let Some(axis) = axis {
        ui.label(axis.label());
    }
    let previously_focused = ui.memory(|memory| memory.has_focus(id));
    let response = ui.add(
        egui::TextEdit::singleline(&mut draft.text)
            .id(id)
            .desired_width(62.0),
    );
    draft.dirty |= response.changed();
    let escape = (previously_focused || response.has_focus())
        && ui.input(|input| input.key_pressed(egui::Key::Escape));
    let enter = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if escape {
        ui.memory_mut(|memory| memory.surrender_focus(id));
        ui.ctx()
            .data_mut(|data| data.remove::<InspectorNumericDraft>(draft_id));
    } else if enter || response.lost_focus() {
        if let Some(value) = committed_number(&draft) {
            let edit = InspectorTransformEdit {
                object_id,
                property,
                axis,
                display_value: value,
            };
            if animated {
                actions.inspector_animated_edits.push(edit);
            } else {
                actions.inspector_transform_edits.push(edit);
            }
        }
        ui.ctx()
            .data_mut(|data| data.remove::<InspectorNumericDraft>(draft_id));
        if enter {
            ui.memory_mut(|memory| memory.surrender_focus(id));
        }
    } else if response.has_focus() {
        ui.ctx().data_mut(|data| data.insert_temp(draft_id, draft));
    } else {
        ui.ctx()
            .data_mut(|data| data.remove::<InspectorNumericDraft>(draft_id));
    }
    response.has_focus()
}

fn draw_inspector_transform_row(
    ui: &mut egui::Ui,
    project: &Project,
    session: &mut EditorSession,
    object: &Object,
    property: AnimatableProperty,
    actions: &mut EditorUiActions,
) {
    let label = match property {
        AnimatableProperty::Position => "Position",
        AnimatableProperty::Scale => "Scale %",
        AnimatableProperty::Rotation => "Rotation °",
        AnimatableProperty::Anchor => "Anchor %",
        AnimatableProperty::Opacity => "Opacity %",
        _ => return,
    };
    let Ok(count) = property_keyframe_count(project, object.id, property) else {
        return;
    };
    let tick = project
        .tempo_map
        .continuous_tick_position(session.playhead())
        .unwrap_or(0.0);
    let value = if count == 0 {
        property_base_value(project, object.id, property)
    } else {
        evaluate_property_at_tick(project, object.id, property, tick)
    };
    let Ok(value) = value else {
        return;
    };
    let editable = !object.locked && (count == 0 || project.tempo_map.initial_segment().is_some());
    let keyframe_tick = project
        .tempo_map
        .continuous_tick_position(session.playhead())
        .ok()
        .and_then(|tick| snap_tick_position_to_grid(tick, session.authoring_division()).ok());
    let at_key = keyframe_tick.is_some_and(|tick| {
        property_keyframe_at_tick(project, object.id, property, tick)
            .ok()
            .flatten()
            .is_some()
    });
    let (key_label, key_hint) = if count == 0 {
        (
            "◇ Static",
            "Static property: create first key at nearest grid position",
        )
    } else if at_key {
        (
            "◆ Key",
            "Key exists at current grid position: remove that key",
        )
    } else {
        (
            "◇ Anim",
            "Animated property: create a key at nearest grid position",
        )
    };
    ui.horizontal(|ui| {
        ui.label(label);
        match value {
            PropertyValue::Vec2(v) if editable => {
                let display = |value: f32| {
                    if matches!(
                        property,
                        AnimatableProperty::Scale | AnimatableProperty::Anchor
                    ) {
                        value * 100.0
                    } else {
                        value
                    }
                };
                if property == AnimatableProperty::Position && count == 0 {
                    if draw_inspector_position_field(ui, object.id, PositionAxis::X, v.x(), actions)
                    {
                        session.focus_property(object.id, property);
                    }
                    if draw_inspector_position_field(ui, object.id, PositionAxis::Y, v.y(), actions)
                    {
                        session.focus_property(object.id, property);
                    }
                } else {
                    if draw_inspector_transform_field(
                        ui,
                        object.id,
                        property,
                        Some(PositionAxis::X),
                        display(v.x()),
                        count != 0,
                        actions,
                    ) {
                        session.focus_property(object.id, property);
                    }
                    if draw_inspector_transform_field(
                        ui,
                        object.id,
                        property,
                        Some(PositionAxis::Y),
                        display(v.y()),
                        count != 0,
                        actions,
                    ) {
                        session.focus_property(object.id, property);
                    }
                }
            }
            PropertyValue::Scalar(v) if editable => {
                let display = if property == AnimatableProperty::Opacity {
                    v * 100.0
                } else {
                    v
                };
                if draw_inspector_transform_field(
                    ui,
                    object.id,
                    property,
                    None,
                    display,
                    count != 0,
                    actions,
                ) {
                    session.focus_property(object.id, property);
                }
            }
            PropertyValue::Vec2(v) => {
                ui.label(format!("X {:.2}  Y {:.2}", v.x(), v.y()));
            }
            PropertyValue::Scalar(v) => {
                ui.label(format!("{v:.2}"));
            }
            PropertyValue::Color(_) => {}
        }
        let key = ui
            .add_enabled(
                !object.locked && keyframe_tick.is_some(),
                egui::Button::new(key_label),
            )
            .on_hover_text(key_hint);
        if key.clicked() {
            session.focus_property(object.id, property);
            actions
                .inspector_keyframe_requests
                .push((object.id, property));
        }
    });
}

fn draw_inspector_position_field(
    ui: &mut egui::Ui,
    object_id: ObjectId,
    axis: PositionAxis,
    current: f32,
    actions: &mut EditorUiActions,
) -> bool {
    let id = inspector_position_id(object_id, axis);
    // TextEdit owns `id` for its internal cursor/selection state; keep
    // application draft data in a separate key to avoid type collisions.
    let draft_id = id.with("draft");
    let mut draft = ui
        .ctx()
        .data(|data| data.get_temp::<InspectorNumericDraft>(draft_id))
        .unwrap_or_else(|| InspectorNumericDraft {
            text: current.to_string(),
            dirty: false,
        });
    ui.label(axis.label());
    let previously_focused = ui.memory(|memory| memory.has_focus(id));
    let response = ui.add(
        egui::TextEdit::singleline(&mut draft.text)
            .id(id)
            .desired_width(96.0),
    );
    draft.dirty |= response.changed();
    let escape = (previously_focused || response.has_focus())
        && ui.input(|input| input.key_pressed(egui::Key::Escape));
    let enter = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));

    if escape {
        ui.memory_mut(|memory| memory.surrender_focus(id));
        ui.ctx()
            .data_mut(|data| data.remove::<InspectorNumericDraft>(draft_id));
    } else if response.lost_focus() || enter {
        if let Some(value) = committed_number(&draft) {
            actions
                .inspector_position_edits
                .push(InspectorPositionEdit {
                    object_id,
                    axis,
                    value,
                });
        }
        ui.ctx()
            .data_mut(|data| data.remove::<InspectorNumericDraft>(draft_id));
        if enter {
            ui.memory_mut(|memory| memory.surrender_focus(id));
        }
    } else if response.has_focus() {
        ui.ctx().data_mut(|data| data.insert_temp(draft_id, draft));
    } else {
        ui.ctx()
            .data_mut(|data| data.remove::<InspectorNumericDraft>(draft_id));
    }
    response.has_focus()
}

fn flush_inspector_draft_on_selection_change(
    ui: &egui::Ui,
    project: &Project,
    selected_object: Option<ObjectId>,
    actions: &mut EditorUiActions,
) {
    let marker = egui::Id::new("last_inspector_position_object");
    let previous = ui.ctx().data(|data| data.get_temp::<ObjectId>(marker));
    if previous == selected_object {
        return;
    }
    if let Some(old_id) = previous {
        let editable = project.composition.objects.iter().any(|object| {
            object.id == old_id
                && !object.locked
                && object.transform.position.keyframes().is_empty()
        });
        for axis in [PositionAxis::X, PositionAxis::Y] {
            let draft_id = inspector_position_id(old_id, axis).with("draft");
            let draft = ui.ctx().data_mut(|data| {
                let draft = data.get_temp::<InspectorNumericDraft>(draft_id);
                data.remove::<InspectorNumericDraft>(draft_id);
                draft
            });
            if editable && let Some(value) = draft.as_ref().and_then(committed_number) {
                actions
                    .inspector_position_edits
                    .push(InspectorPositionEdit {
                        object_id: old_id,
                        axis,
                        value,
                    });
            }
        }
        for property in [
            AnimatableProperty::Position,
            AnimatableProperty::Scale,
            AnimatableProperty::Rotation,
            AnimatableProperty::Anchor,
            AnimatableProperty::Opacity,
        ] {
            let editable = project
                .composition
                .objects
                .iter()
                .any(|object| object.id == old_id && !object.locked);
            let animated =
                property_keyframe_count(project, old_id, property).is_ok_and(|count| count > 0);
            let axes = if matches!(
                property,
                AnimatableProperty::Position
                    | AnimatableProperty::Scale
                    | AnimatableProperty::Anchor
            ) {
                vec![Some(PositionAxis::X), Some(PositionAxis::Y)]
            } else {
                vec![None]
            };
            for axis in axes {
                let draft_id = inspector_transform_id(old_id, property, axis).with("draft");
                let draft = ui.ctx().data_mut(|data| {
                    let draft = data.get_temp::<InspectorNumericDraft>(draft_id);
                    data.remove::<InspectorNumericDraft>(draft_id);
                    draft
                });
                if editable && let Some(display_value) = draft.as_ref().and_then(committed_number) {
                    let edit = InspectorTransformEdit {
                        object_id: old_id,
                        property,
                        axis,
                        display_value,
                    };
                    if animated {
                        actions.inspector_animated_edits.push(edit);
                    } else if property != AnimatableProperty::Position {
                        actions.inspector_transform_edits.push(edit);
                    }
                }
            }
        }
    }
    ui.ctx().data_mut(|data| {
        if let Some(selected_object) = selected_object {
            data.insert_temp(marker, selected_object);
        } else {
            data.remove::<ObjectId>(marker);
        }
    });
}

#[derive(Debug, Default)]
pub struct EditorUiActions {
    pub add_rectangle_requested: bool,
    pub object_list_edits: Vec<EditCommand>,
    pub viewport_position_actions: Vec<ViewportPositionAction>,
    pub viewport_group_position_actions: Vec<ViewportGroupPositionAction>,
    pub viewport_scale_actions: Vec<ViewportScaleAction>,
    pub viewport_rotation_actions: Vec<ViewportRotationAction>,
    pub inspector_position_edits: Vec<InspectorPositionEdit>,
    pub inspector_transform_edits: Vec<InspectorTransformEdit>,
    pub inspector_animated_edits: Vec<InspectorTransformEdit>,
    pub inspector_keyframe_requests: Vec<(ObjectId, AnimatableProperty)>,
    pub undo_requested: bool,
    pub redo_requested: bool,
}

#[derive(Debug, Clone)]
pub struct DiagnosticsView {
    pub adapter_name: String,
    pub backend: String,
    pub window_size: [u32; 2],
    pub frame_time_ms: f32,
}

pub fn configure_theme(context: &egui::Context) {
    context.set_theme(egui::Theme::Dark);
    context.set_visuals(egui::Visuals::dark());

    context.global_style_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(16.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(13.0, egui::FontFamily::Proportional),
        );
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.interact_size = egui::vec2(32.0, 34.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
    });
}

pub fn draw_editor_shell(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    project: &Project,
    diagnostics: &DiagnosticsView,
    composition_texture_id: Option<egui::TextureId>,
    waveform: Option<&rhythm_engine::waveform::WaveformData>,
    actions: &mut EditorUiActions,
) {
    // All three editing surfaces use the same stable object-ID selection.
    // Prune IDs deleted by commands/Undo before drawing any of the panels.
    session.sync_object_selection(project);
    egui::Panel::top("transport_rhythm")
        .exact_size(54.0)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("Rhythm Effects");
                ui.separator();
                ui.label("Transport");
                if ui.button("Start").clicked() {
                    session.seek_paused(ProjectTimeNs::new(0));
                }
                let playhead_seconds = session.playhead().get() as f64 / 1_000_000_000.0;
                ui.label(format!("Playhead {playhead_seconds:.3}s"));
                ui.separator();
                if let Some(segment) = project.tempo_map.initial_segment() {
                    ui.label(format!("BPM {}", segment.bpm().format_decimal()));
                    ui.label(format!(
                        "{}/{}",
                        segment.meter().numerator(),
                        segment.meter().denominator()
                    ));
                } else {
                    ui.label("BPM —");
                    ui.label("4/4");
                }
                ui.label(format!(
                    "Grid 1/{}",
                    session.authoring_division().parts_per_beat()
                ));
                ui.separator();
                egui::ComboBox::from_id_salt("preview_quality")
                    .selected_text(session.preview_quality.label())
                    .show_ui(ui, |ui| {
                        for quality in PreviewQuality::ALL {
                            ui.selectable_value(
                                &mut session.preview_quality,
                                quality,
                                quality.label(),
                            );
                        }
                    });
            });
        });

    egui::Panel::bottom("timeline")
        .resizable(true)
        .default_size(300.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Timeline");
                ui.add_enabled_ui(session.selected_keyframe_count() > 0, |ui| {
                    ui.menu_button("Easing", |ui| draw_easing_preset_menu(ui, session));
                });
                let mut follow = session.follow_playhead_enabled();
                if ui
                    .checkbox(&mut follow, "Follow Playhead")
                    .on_hover_text("Scroll near timeline edges during audio playback. Manual pan turns this off.")
                    .changed()
                {
                    session.set_follow_playhead_enabled(follow);
                }
            });
            ui.separator();
            draw_timeline(ui, session, project, waveform);
            ui.take_available_space();
        });

    egui::Panel::left("objects")
        .resizable(true)
        .default_size(240.0)
        .show(ui, |ui| {
            ui.heading("Objects");
            if ui.button("+ Rectangle").clicked() {
                actions.add_rectangle_requested = true;
            }
            ui.separator();
            if project.composition.objects.is_empty() {
                ui.label("No objects yet. Add a Rectangle to try selection.");
            }
            ui.small("V = visible · H = hidden · U = unlocked · L = locked. Double-click name to rename.");
            egui::ScrollArea::vertical()
                .id_salt("object_list_scroll")
                .show(ui, |ui| {
                    for object in &project.composition.objects {
                        let edit_id = egui::Id::new(("object_name_edit", object.id.get()));
                        let draft_id = edit_id.with("draft");
                        ui.horizontal(|ui| {
                            let visibility = if object.visible { "V" } else { "H" };
                            if ui
                                .small_button(visibility)
                                .on_hover_text(if object.visible {
                                    "Hide object (Undo available)"
                                } else {
                                    "Show object (Undo available)"
                                })
                                .clicked()
                            {
                                actions.object_list_edits.push(EditCommand::SetObjectVisible {
                                    object_id: object.id,
                                    visible: !object.visible,
                                });
                            }
                            let locking = if object.locked { "L" } else { "U" };
                            if ui
                                .small_button(locking)
                                .on_hover_text(if object.locked {
                                    "Unlock object (Undo available)"
                                } else {
                                    "Lock object (Undo available)"
                                })
                                .clicked()
                            {
                                actions.object_list_edits.push(EditCommand::SetObjectLocked {
                                    object_id: object.id,
                                    locked: !object.locked,
                                });
                            }
                            if let Some(mut draft) =
                                ui.ctx().data(|data| data.get_temp::<String>(draft_id))
                            {
                                let field = ui.add(
                                    egui::TextEdit::singleline(&mut draft)
                                        .id(edit_id)
                                        .desired_width(150.0),
                                );
                                let escape = field.has_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Escape));
                                let enter = field.has_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                                if escape {
                                    ui.ctx().data_mut(|data| data.remove::<String>(draft_id));
                                    ui.memory_mut(|memory| memory.surrender_focus(edit_id));
                                } else if enter || field.lost_focus() {
                                    let renamed = draft.trim();
                                    if !renamed.is_empty() && renamed != object.name {
                                        actions.object_list_edits.push(EditCommand::RenameObject {
                                            object_id: object.id,
                                            name: renamed.to_owned(),
                                        });
                                    }
                                    ui.ctx().data_mut(|data| data.remove::<String>(draft_id));
                                    if enter {
                                        ui.memory_mut(|memory| memory.surrender_focus(edit_id));
                                    }
                                } else {
                                    ui.ctx().data_mut(|data| data.insert_temp(draft_id, draft));
                                }
                            } else {
                                let response = ui
                                    .selectable_label(
                                        session.is_object_selected(object.id),
                                        &object.name,
                                    )
                                    .on_hover_text("Click to select; Ctrl+click toggles; double-click to rename");
                                if response.clicked() {
                                    apply_object_click(
                                        session,
                                        Some(object.id),
                                        ui.input(|input| input.modifiers.ctrl),
                                    );
                                }
                                if response.double_clicked() {
                                    ui.ctx().data_mut(|data| {
                                        data.insert_temp(draft_id, object.name.clone());
                                    });
                                    ui.memory_mut(|memory| memory.request_focus(edit_id));
                                }
                            }
                        });
                    }
                });
            ui.take_available_space();
        });

    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(320.0)
        .show(ui, |ui| {
            ui.heading("Inspector");
            ui.separator();
            let selected = if session.selected_objects().len() == 1 {
                session.selected_object()
            } else {
                None
            };
            flush_inspector_draft_on_selection_change(ui, project, selected, actions);
            ui.horizontal(|ui| {
                if ui.button("Undo").clicked() {
                    actions.undo_requested = true;
                }
                if ui.button("Redo").clicked() {
                    actions.redo_requested = true;
                }
            });
            ui.separator();
            if let Some(object) = session.selected_object().and_then(|id| {
                project
                    .composition
                    .objects
                    .iter()
                    .find(|object| object.id == id)
            }) {
                ui.label(format!(
                    "Selected objects: {}",
                    session.selected_objects().len()
                ));
                ui.label(format!("Primary: {}", object.name));
                ui.label(format!("Primary ID: {}", object.id.get()));
                if session.selected_objects().len() > 1 {
                    ui.small("Multi-object mixed transforms are tracked as AI-188.");
                } else {
                    ui.heading("Transform");
                    if object.locked {
                        ui.small("Locked: values are read-only.");
                    }
                    for property in [
                        AnimatableProperty::Position,
                        AnimatableProperty::Scale,
                        AnimatableProperty::Rotation,
                        AnimatableProperty::Anchor,
                        AnimatableProperty::Opacity,
                    ] {
                        draw_inspector_transform_row(
                            ui, project, session, object, property, actions,
                        );
                    }
                    ui.small("Enter / focus loss commits · Escape restores. Values in %, ° or px.");
                    ui.small(
                        "◇ Static · ◇ Anim · ◆ Key; animated edits resolve to the nearest grid.",
                    );
                }
            } else {
                ui.label("No object selected.");
            }
            ui.add_space(16.0);
            ui.separator();
            ui.heading("Diagnostics");
            ui.label(format!("GPU: {}", diagnostics.adapter_name));
            ui.label(format!("Backend: {}", diagnostics.backend));
            ui.label(format!(
                "Window: {} × {}",
                diagnostics.window_size[0], diagnostics.window_size[1]
            ));
            ui.label(format!("Frame: {:.2} ms", diagnostics.frame_time_ms));
            ui.take_available_space();
        });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("Viewport");
        ui.small("Editor-only wireframes; actual object GPU rendering is not wired yet.");
        ui.small(
            "Selected objects show yellow bounds and pivot crosses; blue bounds enclose groups.",
        );
        ui.small("Click to select; Ctrl+click toggles objects without clearing the others.");
        ui.small("Drag from empty space to box-select; hold Ctrl to toggle intersecting objects.");
        ui.small(
            "Drag an unlocked Rectangle/Ellipse to move; drag a selected member to move a multi-selection together.",
        );
        ui.small("Drag yellow corner squares to adjust static Scale; hold Shift to preserve proportions.");
        ui.small("Drag the blue circular handle to rotate; hold Shift to snap to 15°.");
        ui.small("Middle-drag pans; wheel zooms around cursor (12.5%–1600% of Fit).");
        let mut frame_selection_requested = false;
        ui.horizontal(|ui| {
            if ui.button("Fit Composition").clicked() {
                session.reset_viewport_camera();
            }
            if ui
                .add_enabled(
                    !session.selected_objects().is_empty(),
                    egui::Button::new("Frame Selection"),
                )
                .on_hover_text(
                    "Frame selected Rectangle/Ellipse outlines; image/text layout is not ready",
                )
                .clicked()
            {
                frame_selection_requested = true;
            }
        });
        ui.separator();
        {
            if let Some(texture_id) = composition_texture_id {
                // The whole panel handles pan, including its letterbox regions.
                let (canvas, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
                if response.drag_started_by(egui::PointerButton::Middle) {
                    session.begin_viewport_pan();
                }
                if response.dragged_by(egui::PointerButton::Middle)
                    && let Some(delta) = ui.input(|input| input.pointer.total_drag_delta())
                {
                    session.update_viewport_pan([delta.x, delta.y]);
                }
                if response.drag_stopped_by(egui::PointerButton::Middle) {
                    session.end_viewport_pan();
                }

                // Wheel is handled only when hovering the viewport, not
                // timeline or property panels. Keep the composition point
                // under the cursor fixed when updating zoom and camera pan.
                if response.hovered() {
                    let (pointer, scroll) = ui
                        .input(|input| (input.pointer.hover_pos(), input.smooth_scroll_delta().y));
                    if let Some(pointer) = pointer
                        && scroll.is_finite()
                        && scroll != 0.0
                    {
                        let multiplier = (f64::from(scroll) * 0.003).clamp(-4.0, 4.0).exp() as f32;
                        session.zoom_viewport_at(
                            multiplier,
                            [pointer.x - canvas.center().x, pointer.y - canvas.center().y],
                        );
                    }
                }

                let composition_size = [
                    project.settings.composition_width,
                    project.settings.composition_height,
                ];
                let fitted_size = fit_composition_preview(canvas.size(), composition_size);
                let tick = project
                    .tempo_map
                    .continuous_tick_position(session.playhead())
                    .unwrap_or(0.0);
                if frame_selection_requested
                    && let Some(bounds) = selected_outline_bounds(project, session, tick)
                {
                    session.frame_viewport_bounds(
                        bounds,
                        [canvas.width(), canvas.height()],
                        [fitted_size.x, fitted_size.y],
                        composition_size,
                    );
                }
                let preview_size = fitted_size * session.viewport_zoom();
                let preview =
                    panned_preview_rect(canvas, preview_size, session.viewport_pan_points());
                ui.painter().with_clip_rect(canvas).image(
                    texture_id,
                    preview,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                // Static objects remain pickable in projects without a tempo map.
                // No audio-clock or timeline playback is inferred.
                draw_editor_outlines(ui, preview, canvas, session, project, tick);
                draw_selection_overlays(ui, preview, canvas, session, project, tick);
                // Drags on an already selected member move an entire eligible
                // multi-selection by one composition-space delta. Otherwise
                // a single static Position moves or empty space starts a box.
                if response.drag_started_by(egui::PointerButton::Primary)
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    let origin = ui
                        .input(|input| input.pointer.press_origin())
                        .unwrap_or(pointer);
                    let ctrl = ui.input(|input| input.modifiers.ctrl);
                    let rotation_hit = if ctrl {
                        None
                    } else {
                        selected_rotation_handle_hit(
                            project, session, preview, canvas, tick, origin,
                        )
                    };
                    if let Some(id) = rotation_hit {
                        if let Some(object) = editable_rotation_object(project, session)
                            && let Some(pivot) =
                                object_outline(object, tick).map(|shape| shape.anchor_position())
                            && let Some(start) =
                                unbounded_composition_point(preview, origin, composition_size)
                            && session.begin_viewport_rotation_drag(
                                start,
                                pivot,
                                *object.transform.rotation_degrees.base_value(),
                            )
                        {
                            actions
                                .viewport_rotation_actions
                                .push(ViewportRotationAction::Begin(id));
                        }
                    } else if let Some(start) =
                        composition_point_from_preview(preview, origin, composition_size)
                    {
                        let scale_hit = if ctrl {
                            None
                        } else {
                            selected_scale_corner_hit(
                                project, session, preview, canvas, tick, origin,
                            )
                        };
                        if let Some((id, corner)) = scale_hit {
                            if let Some(object) = editable_scale_object(project, session)
                                && let Some(size) = scale_shape_size(project, object, tick)
                                && let Some(anchor) = scale_anchor(project, object, tick)
                                && let Some(rotation) = scale_rotation(project, object, tick)
                                && session.begin_viewport_scale_drag(
                                    start,
                                    *object.transform.scale.base_value(),
                                    size,
                                    anchor,
                                    rotation,
                                    corner,
                                )
                            {
                                actions
                                    .viewport_scale_actions
                                    .push(ViewportScaleAction::Begin(id));
                            }
                        } else {
                            let picked = pick_topmost_object(
                                &project.composition.objects,
                                tick,
                                start,
                                |_| None,
                                |_| None,
                            );
                            if let Some(id) = picked {
                                let group_eligible = session.selected_objects().len() > 1
                                    && session.is_object_selected(id)
                                    && session.selected_objects().iter().all(|member_id| {
                                        project.composition.objects.iter().any(|member| {
                                            member.id == *member_id
                                                && member.visible
                                                && !member.locked
                                                && member.transform.position.keyframes().is_empty()
                                        })
                                    });
                                if !ctrl && group_eligible
                                    && session.begin_viewport_group_position_drag(start)
                                {
                                    actions.viewport_group_position_actions.push(
                                        ViewportGroupPositionAction::Begin(
                                            session.selected_objects().to_vec(),
                                        ),
                                    );
                                } else if !ctrl
                                    && session.selected_objects().len() <= 1
                                    && let Some(object) = project
                                        .composition
                                        .objects
                                        .iter()
                                        .find(|object| object.id == id)
                                    && object.transform.position.keyframes().is_empty()
                                    && session.begin_viewport_position_drag(
                                        start,
                                        *object.transform.position.base_value(),
                                    )
                                {
                                    session.select_viewport_object(Some(id));
                                    actions
                                        .viewport_position_actions
                                        .push(ViewportPositionAction::Begin(id));
                                }
                            } else {
                                session.begin_viewport_box_selection(start, ctrl);
                            }
                        }
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary)
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    if let Some(point) =
                        unbounded_composition_point(preview, pointer, composition_size)
                    {
                        if session.viewport_rotation_drag_active() {
                            if let Some(degrees) = session.update_viewport_rotation_drag(
                                point,
                                ui.input(|input| input.modifiers.shift),
                            ) {
                                actions
                                    .viewport_rotation_actions
                                    .push(ViewportRotationAction::Update(degrees));
                            }
                        } else if let Some(delta) =
                            session.update_viewport_group_position_drag(point)
                        {
                            actions.viewport_group_position_actions.push(
                                ViewportGroupPositionAction::Update(delta),
                            );
                        } else if let Some(scale) = session.update_viewport_scale_drag(
                            point,
                            ui.input(|input| input.modifiers.shift),
                        ) {
                            actions
                                .viewport_scale_actions
                                .push(ViewportScaleAction::Update(scale));
                        } else if let Some(position) = session.update_viewport_position_drag(point)
                        {
                            actions
                                .viewport_position_actions
                                .push(ViewportPositionAction::Update(position));
                        }
                    }
                    if let Some(current) = clamped_drag_point(preview, pointer, composition_size) {
                        session.update_viewport_box_selection(current);
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    if session.viewport_rotation_drag_active() {
                        if let Some(pointer) = response.interact_pointer_pos()
                            && let Some(point) =
                                unbounded_composition_point(preview, pointer, composition_size)
                            && let Some(degrees) = session.update_viewport_rotation_drag(
                                point,
                                ui.input(|input| input.modifiers.shift),
                            )
                        {
                            actions
                                .viewport_rotation_actions
                                .push(ViewportRotationAction::Update(degrees));
                        }
                        if session.finish_viewport_rotation_drag() {
                            actions
                                .viewport_rotation_actions
                                .push(ViewportRotationAction::Commit);
                        }
                    }
                    if session.viewport_group_position_drag_active() {
                        if let Some(pointer) = response.interact_pointer_pos()
                            && let Some(point) =
                                unbounded_composition_point(preview, pointer, composition_size)
                            && let Some(delta) = session.update_viewport_group_position_drag(point)
                        {
                            actions.viewport_group_position_actions.push(
                                ViewportGroupPositionAction::Update(delta),
                            );
                        }
                        if session.finish_viewport_group_position_drag() {
                            actions.viewport_group_position_actions.push(
                                ViewportGroupPositionAction::Commit,
                            );
                        }
                    }
                    if session.viewport_scale_drag_active() {
                        if let Some(pointer) = response.interact_pointer_pos()
                            && let Some(point) =
                                unbounded_composition_point(preview, pointer, composition_size)
                            && let Some(scale) = session.update_viewport_scale_drag(
                            point,
                            ui.input(|input| input.modifiers.shift),
                        )
                        {
                            actions
                                .viewport_scale_actions
                                .push(ViewportScaleAction::Update(scale));
                        }
                        if session.finish_viewport_scale_drag() {
                            actions
                                .viewport_scale_actions
                                .push(ViewportScaleAction::Commit);
                        }
                    }
                    if session.viewport_position_drag_active() {
                        if let Some(pointer) = response.interact_pointer_pos()
                            && let Some(point) =
                                unbounded_composition_point(preview, pointer, composition_size)
                            && let Some(position) = session.update_viewport_position_drag(point)
                        {
                            actions
                                .viewport_position_actions
                                .push(ViewportPositionAction::Update(position));
                        }
                        if session.finish_viewport_position_drag() {
                            actions
                                .viewport_position_actions
                                .push(ViewportPositionAction::Commit);
                        }
                    }
                    if let Some(pointer) = response.interact_pointer_pos()
                        && let Some(current) =
                            clamped_drag_point(preview, pointer, composition_size)
                    {
                        session.update_viewport_box_selection(current);
                    }
                    if let Some((start, current, ctrl_toggle)) =
                        session.take_viewport_box_selection()
                    {
                        let picked = select_objects_in_box(
                            &project.composition.objects,
                            tick,
                            start,
                            current,
                            |_| None, // No image intrinsic metadata cache yet.
                            |_| None, // No composition font-layout cache yet.
                        );
                        session.apply_viewport_box_selection(picked, ctrl_toggle);
                    }
                }
                if let Some((start, current, _)) = session.viewport_box_selection() {
                    draw_marquee(ui, preview, canvas, start, current, composition_size);
                }
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(point) = composition_point_from_preview(
                        preview,
                        pointer,
                        [
                            project.settings.composition_width,
                            project.settings.composition_height,
                        ],
                    )
                {
                    let picked = pick_topmost_object(
                        &project.composition.objects,
                        tick,
                        point,
                        |_| None, // No prepared image dimensions cache yet.
                        |_| None, // No composition font-layout cache yet.
                    );
                    apply_object_click(session, picked, ui.input(|input| input.modifiers.ctrl));
                }
            } else {
                ui.label("Composition preview unavailable");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{
        InspectorNumericDraft, InspectorPositionEdit, InspectorTransformEdit, PositionAxis,
        apply_inspector_animated_transform_edits, apply_inspector_position_edits,
        apply_inspector_transform_edits, apply_object_click, clamped_drag_point, committed_number,
        composition_point_from_preview, fit_composition_preview, panned_preview_rect,
        preview_point, rotation_handle_on_preview, selected_outline_bounds,
        selected_rotation_handle_hit, selected_scale_corner_hit, selection_rect_on_preview,
        unbounded_composition_point,
    };
    use crate::editor_session::EditorSession;
    use rhythm_core::ids::ObjectId;

    #[test]
    fn object_list_and_viewport_clicks_share_inspector_primary_selection() {
        let mut session = EditorSession::default();
        let first = ObjectId::new(1).expect("first");
        let second = ObjectId::new(2).expect("second");
        apply_object_click(&mut session, Some(first), false); // Objects panel
        assert_eq!(session.selected_object(), Some(first));
        apply_object_click(&mut session, Some(second), true); // Viewport Ctrl-click
        assert_eq!(session.selected_objects(), &[first, second]);
        assert_eq!(session.selected_object(), Some(second)); // Inspector primary
        apply_object_click(&mut session, Some(first), true); // Objects Ctrl-toggle
        assert_eq!(session.selected_objects(), &[second]);
        apply_object_click(&mut session, None, true); // Empty Ctrl-click keeps selection
        assert_eq!(session.selected_object(), Some(second));
        apply_object_click(&mut session, None, false); // Viewport empty click clears it
        assert_eq!(session.selected_object(), None);
    }

    #[test]
    fn position_text_accepts_only_changed_finite_numeric_values() {
        let draft = |text: &str, dirty| InspectorNumericDraft {
            text: text.to_owned(),
            dirty,
        };
        assert_eq!(committed_number(&draft(" -123.25 ", true)), Some(-123.25));
        assert_eq!(committed_number(&draft("123", false)), None);
        for invalid in ["", "-", "1.2.3", "NaN", "inf", "1e999"] {
            assert_eq!(committed_number(&draft(invalid, true)), None);
        }
    }

    fn animated_inspector_fixture() -> (EditorSession, rhythm_core::editor::ProjectEditor, ObjectId)
    {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{BpmMicros, GridOffsetNs, TempoMap, TimeSignature},
        };
        let v = |x, y| Vec2::new(x, y).expect("finite");
        let id = ObjectId::new(1).expect("id");
        let mut project = Project::new(
            "Animated Inspector",
            ProjectSettings::default(),
            TempoMap::with_initial_tempo(
                GridOffsetNs::new(0),
                BpmMicros::new(120_000_000).expect("tempo"),
                TimeSignature::default(),
            ),
        );
        project.composition.objects.push(Object {
            id,
            name: "Shape".into(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(v(100.0, 200.0)),
                Animated::new_static(v(1.0, 2.0)),
                Animated::new_static(0.0),
                Animated::new_static(v(0.5, 0.5)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(v(100.0, 50.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 2;
        let editor = rhythm_core::editor::ProjectEditor::new(project).expect("project");
        let mut session = EditorSession::default();
        session.seek_paused(rhythm_core::time::ProjectTimeNs::new(130_000_000));
        (session, editor, id)
    }

    #[test]
    fn animated_inspector_off_key_creates_resolved_key_and_on_key_updates_in_place() {
        use rhythm_core::{
            property::{
                AnimatableProperty, PropertyValue, property_base_value, property_keyframe_at_tick,
                property_keyframe_count,
            },
            time::{MusicalTick, ProjectTimeNs},
        };
        let v = |x, y| rhythm_core::domain::Vec2::new(x, y).expect("finite");
        let (mut session, mut editor, id) = animated_inspector_fixture();
        let property = AnimatableProperty::Scale;
        editor
            .create_property_keyframe(
                id,
                property,
                MusicalTick::new(0),
                PropertyValue::Vec2(v(1.0, 2.0)),
            )
            .expect("first key");
        editor
            .create_property_keyframe(
                id,
                property,
                MusicalTick::new(480),
                PropertyValue::Vec2(v(1.0, 2.0)),
            )
            .expect("second key");
        let history = editor.history_len();
        let edit = |axis, display_value| InspectorTransformEdit {
            object_id: id,
            property,
            axis: Some(axis),
            display_value,
        };
        assert!(
            apply_inspector_animated_transform_edits(
                &mut session,
                &mut editor,
                [edit(PositionAxis::X, 250.0)],
            )
            .expect("off-key edit")
        );
        let first =
            property_keyframe_at_tick(editor.project(), id, property, MusicalTick::new(240))
                .expect("property")
                .expect("inserted key");
        assert_eq!(first.value, PropertyValue::Vec2(v(2.5, 2.0)));
        assert_eq!(
            property_keyframe_count(editor.project(), id, property),
            Ok(3)
        );
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        assert_eq!(editor.history_len(), history + 1);
        assert_eq!(
            property_base_value(editor.project(), id, property),
            Ok(PropertyValue::Vec2(v(1.0, 2.0))),
        );

        assert!(
            apply_inspector_animated_transform_edits(
                &mut session,
                &mut editor,
                [edit(PositionAxis::Y, 300.0)],
            )
            .expect("existing-key edit")
        );
        let updated =
            property_keyframe_at_tick(editor.project(), id, property, MusicalTick::new(240))
                .expect("property")
                .expect("edited key");
        assert_eq!(updated.id, first.id);
        assert_eq!(updated.interpolation, first.interpolation);
        assert_eq!(updated.value, PropertyValue::Vec2(v(2.5, 3.0)));
        assert_eq!(editor.history_len(), history + 2);
        assert!(editor.undo().expect("undo existing-key edit"));
        let restored =
            property_keyframe_at_tick(editor.project(), id, property, MusicalTick::new(240))
                .expect("property")
                .expect("restored key");
        assert_eq!(restored.value, first.value);
        assert_eq!(restored.id, first.id);
        assert!(editor.undo().expect("undo newly created key"));
        assert_eq!(
            property_keyframe_count(editor.project(), id, property),
            Ok(2)
        );
        assert!(editor.redo().expect("redo new key"));
        assert!(editor.redo().expect("redo value"));
        editor.project().validate().expect("valid");
    }

    #[test]
    fn inspector_keyframe_button_removes_last_key_to_static_value_and_can_undo() {
        use rhythm_core::{
            property::{
                AnimatableProperty, PropertyValue, property_base_value, property_keyframe_count,
            },
            time::{MusicalTick, ProjectTimeNs},
        };
        let (mut session, mut editor, id) = animated_inspector_fixture();
        let property = AnimatableProperty::Opacity;
        editor
            .create_property_keyframe(
                id,
                property,
                MusicalTick::new(240),
                PropertyValue::Scalar(0.65),
            )
            .expect("insert key");
        let history = editor.history_len();
        session.focus_property(id, property);
        assert!(
            session
                .keyframe_action(&mut editor)
                .expect("remove final key")
        );
        assert_eq!(
            property_keyframe_count(editor.project(), id, property),
            Ok(0)
        );
        assert_eq!(
            property_base_value(editor.project(), id, property),
            Ok(PropertyValue::Scalar(0.65))
        );
        assert_eq!(session.playhead(), ProjectTimeNs::new(125_000_000));
        assert_eq!(editor.history_len(), history + 1);
        assert!(editor.undo().expect("undo removal"));
        assert_eq!(
            property_keyframe_count(editor.project(), id, property),
            Ok(1)
        );
        assert_eq!(
            property_base_value(editor.project(), id, property),
            Ok(PropertyValue::Scalar(1.0))
        );
        assert!(editor.redo().expect("redo removal"));
        assert_eq!(
            property_keyframe_count(editor.project(), id, property),
            Ok(0)
        );
        assert_eq!(
            property_base_value(editor.project(), id, property),
            Ok(PropertyValue::Scalar(0.65))
        );
        editor.project().validate().expect("valid");
    }

    #[test]
    fn inspector_transform_controls_commit_percent_units_as_static_base_values() {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            editor::ProjectEditor,
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            property::{AnimatableProperty, PropertyValue, property_base_value},
            time::{GridOffsetNs, TempoMap},
        };
        let v = |x, y| Vec2::new(x, y).expect("finite");
        let id = ObjectId::new(1).expect("id");
        let mut project = Project::new(
            "Inspector transforms",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id,
            name: "Shape".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(v(100.0, 200.0)),
                Animated::new_static(v(1.0, 1.0)),
                Animated::new_static(0.0),
                Animated::new_static(v(0.5, 0.5)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(v(100.0, 50.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 2;
        let mut editor = ProjectEditor::new(project).expect("project");
        let edit = |property, axis, display_value| InspectorTransformEdit {
            object_id: id,
            property,
            axis,
            display_value,
        };
        let changes = [
            edit(AnimatableProperty::Scale, Some(PositionAxis::X), -150.0),
            edit(AnimatableProperty::Scale, Some(PositionAxis::Y), 225.0),
            edit(AnimatableProperty::Rotation, None, 37.5),
            edit(AnimatableProperty::Anchor, Some(PositionAxis::X), 25.0),
            edit(AnimatableProperty::Opacity, None, 60.0),
        ];
        assert!(apply_inspector_transform_edits(&mut editor, changes).expect("commit"));
        assert_eq!(editor.history_len(), changes.len());
        assert_eq!(
            property_base_value(editor.project(), id, AnimatableProperty::Scale),
            Ok(PropertyValue::Vec2(v(-1.5, 2.25)))
        );
        assert_eq!(
            property_base_value(editor.project(), id, AnimatableProperty::Rotation),
            Ok(PropertyValue::Scalar(37.5))
        );
        assert_eq!(
            property_base_value(editor.project(), id, AnimatableProperty::Anchor),
            Ok(PropertyValue::Vec2(v(0.25, 0.5)))
        );
        assert_eq!(
            property_base_value(editor.project(), id, AnimatableProperty::Opacity),
            Ok(PropertyValue::Scalar(0.6))
        );
        assert!(editor.undo().expect("undo opacity"));
        assert_eq!(
            property_base_value(editor.project(), id, AnimatableProperty::Opacity),
            Ok(PropertyValue::Scalar(1.0))
        );
        assert!(editor.redo().expect("redo opacity"));
        editor.project().validate().expect("valid");
    }

    #[test]
    fn inspector_position_axis_commits_preserve_other_axis_with_individual_undo() {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            editor::ProjectEditor,
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, TempoMap},
        };

        let v = |x, y| Vec2::new(x, y).expect("finite");
        let mut project = Project::new(
            "Inspector integration",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let id = ObjectId::new(1).expect("id");
        project.composition.objects.push(Object {
            id,
            name: "Test".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(v(100.0, 200.0)),
                Animated::new_static(v(1.0, 1.0)),
                Animated::new_static(0.0),
                Animated::new_static(v(0.5, 0.5)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(v(100.0, 50.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 2;
        let mut editor = ProjectEditor::new(project).expect("project");
        let position = |editor: &ProjectEditor| {
            *editor.project().composition.objects[0]
                .transform
                .position
                .base_value()
        };
        assert!(
            apply_inspector_position_edits(
                &mut editor,
                [
                    InspectorPositionEdit {
                        object_id: id,
                        axis: PositionAxis::X,
                        value: 320.0
                    },
                    InspectorPositionEdit {
                        object_id: id,
                        axis: PositionAxis::Y,
                        value: -50.0
                    },
                ],
            )
            .expect("edits")
        );
        assert_eq!(position(&editor), v(320.0, -50.0));
        assert_eq!(editor.history_len(), 2);
        assert!(
            !apply_inspector_position_edits(
                &mut editor,
                [InspectorPositionEdit {
                    object_id: id,
                    axis: PositionAxis::Y,
                    value: -50.0
                }],
            )
            .expect("no-op")
        );
        assert!(
            !apply_inspector_position_edits(
                &mut editor,
                [InspectorPositionEdit {
                    object_id: id,
                    axis: PositionAxis::X,
                    value: f32::NAN
                }],
            )
            .expect("invalid")
        );
        assert_eq!(editor.history_len(), 2);
        assert!(editor.undo().expect("undo y"));
        assert_eq!(position(&editor), v(320.0, 200.0));
        assert!(editor.undo().expect("undo x"));
        assert_eq!(position(&editor), v(100.0, 200.0));
        assert!(editor.redo().expect("redo x"));
        assert!(editor.redo().expect("redo y"));
        assert_eq!(position(&editor), v(320.0, -50.0));
        editor.project().validate().expect("project remains valid");
    }

    #[test]
    fn scale_corner_hit_uses_actual_rotated_shape_not_aabb_and_skips_ineligible() {
        use rhythm_core::{
            animation::{Animated, Interpolation, Keyframe},
            domain::{LinearRgba, Vec2},
            hit_test::object_outline,
            ids::KeyframeId,
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, MusicalTick, TempoMap},
        };
        let v = |x, y| Vec2::new(x, y).expect("finite");
        let id = ObjectId::new(1).expect("object");
        let mut project = Project::new(
            "scale pick",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id,
            name: "shape".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(v(900.0, 500.0)),
                Animated::new_static(v(-1.0, 2.0)),
                Animated::new_static(90.0),
                Animated::new_static(v(0.5, 0.5)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(v(200.0, 100.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        let mut session = EditorSession::default();
        session.select_viewport_object(Some(id));
        let preview = egui::Rect::from_min_size(egui::pos2(40.0, 80.0), egui::vec2(960.0, 540.0));
        let geometry = object_outline(&project.composition.objects[0], 0.0).expect("geometry");
        let point = geometry.point(1.0, 1.0).expect("rotated corner");
        let pointer = preview_point(preview, point, [1920, 1080]);
        assert_eq!(
            selected_scale_corner_hit(&project, &session, preview, preview, 0.0, pointer),
            Some((id, [1.0, 1.0]))
        );
        project.composition.objects[0].locked = true;
        assert_eq!(
            selected_scale_corner_hit(&project, &session, preview, preview, 0.0, pointer),
            None
        );
        project.composition.objects[0].locked = false;
        project.composition.objects[0]
            .transform
            .scale
            .insert_keyframe(Keyframe::new(
                KeyframeId::new(2).expect("key"),
                MusicalTick::new(0),
                v(-1.0, 2.0),
                Interpolation::Linear,
            ))
            .expect("keyframe");
        assert_eq!(
            selected_scale_corner_hit(&project, &session, preview, preview, 0.0, pointer),
            None
        );
    }

    #[test]
    fn rotation_handle_tracks_rotated_mirrored_shape_and_selection_eligibility() {
        use rhythm_core::ids::KeyframeId;
        use rhythm_core::{
            animation::{Animated, Interpolation, Keyframe},
            domain::{LinearRgba, Vec2},
            hit_test::object_outline,
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, MusicalTick, TempoMap},
        };
        let v = |x, y| Vec2::new(x, y).expect("finite");
        let id = ObjectId::new(1).expect("id");
        let mut project = Project::new(
            "rotation handle",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id,
            name: "rotated".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(v(960.0, 500.0)),
                Animated::new_static(v(-1.0, -2.0)),
                Animated::new_static(90.0),
                Animated::new_static(v(0.25, 0.75)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(v(200.0, 100.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        let mut session = EditorSession::default();
        session.select_viewport_object(Some(id));
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0));
        let preview = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(960.0, 540.0));
        let object = &project.composition.objects[0];
        let geometry = object_outline(object, 0.0).expect("geometry");
        let (top, knob) =
            rotation_handle_on_preview(object, preview, [1920, 1080], 0.0).expect("handle");
        assert!(((top - knob).length() - 28.0).abs() < 1e-3);
        let pivot = preview_point(preview, geometry.anchor_position(), [1920, 1080]);
        assert!(pivot.distance(knob) > 20.0);
        assert_eq!(
            selected_rotation_handle_hit(&project, &session, preview, canvas, 0.0, knob),
            Some(id)
        );
        assert_eq!(
            selected_rotation_handle_hit(
                &project,
                &session,
                preview,
                canvas,
                0.0,
                knob + egui::vec2(12.0, 0.0),
            ),
            None
        );
        let zoomed =
            egui::Rect::from_min_size(egui::pos2(-200.0, -200.0), egui::vec2(1920.0, 1080.0));
        let (top_zoomed, knob_zoomed) =
            rotation_handle_on_preview(object, zoomed, [1920, 1080], 0.0).expect("zoom handle");
        assert!(((top_zoomed - knob_zoomed).length() - 28.0).abs() < 1e-3);

        project.composition.objects[0].locked = true;
        assert_eq!(
            selected_rotation_handle_hit(&project, &session, preview, canvas, 0.0, knob),
            None
        );
        project.composition.objects[0].locked = false;
        project.composition.objects[0]
            .transform
            .rotation_degrees
            .insert_keyframe(Keyframe::new(
                KeyframeId::new(2).expect("key"),
                MusicalTick::new(0),
                90.0,
                Interpolation::Linear,
            ))
            .expect("insert");
        assert_eq!(
            selected_rotation_handle_hit(&project, &session, preview, canvas, 0.0, knob),
            None
        );
    }

    #[test]
    fn rotation_handle_is_pickable_in_canvas_outside_composition_preview() {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, TempoMap},
        };
        let v = |x, y| Vec2::new(x, y).expect("finite");
        let id = ObjectId::new(1).expect("id");
        let mut project = Project::new(
            "outside preview",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id,
            name: "top edge".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(v(960.0, 0.0)),
                Animated::new_static(v(1.0, 1.0)),
                Animated::new_static(0.0),
                Animated::new_static(v(0.5, 0.5)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(v(200.0, 100.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1150.0, 760.0));
        let preview = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(960.0, 540.0));
        let mut session = EditorSession::default();
        session.select_viewport_object(Some(id));
        let (_, knob) =
            rotation_handle_on_preview(&project.composition.objects[0], preview, [1920, 1080], 0.0)
                .expect("rotation knob");
        assert!(knob.y < preview.min.y && canvas.contains(knob));
        assert_eq!(
            selected_rotation_handle_hit(&project, &session, preview, canvas, 0.0, knob),
            Some(id)
        );
    }

    #[test]
    fn selection_bounds_projection_tracks_pan_zoom_and_pivot_mapping() {
        let canvas = egui::Rect::from_min_size(egui::pos2(200.0, 80.0), egui::vec2(960.0, 540.0));
        let preview = panned_preview_rect(canvas, egui::vec2(960.0, 540.0) * 2.0, [30.0, -45.0]);
        let min = rhythm_core::domain::Vec2::new(720.0, 400.0).expect("min");
        let max = rhythm_core::domain::Vec2::new(1200.0, 680.0).expect("max");
        let rect = selection_rect_on_preview(preview, (min, max), [1920, 1080])
            .expect("projected selected bounds");
        let expected_min = preview_point(preview, min, [1920, 1080]);
        let expected_max = preview_point(preview, max, [1920, 1080]);
        assert_eq!(rect.min, expected_min);
        assert_eq!(rect.max, expected_max);
        // The selected pivot/center maps through the same camera.
        let pivot = preview_point(
            preview,
            rhythm_core::domain::Vec2::new(960.0, 540.0).expect("pivot"),
            [1920, 1080],
        );
        assert_eq!(pivot, canvas.center() + egui::vec2(30.0, -45.0));
    }

    #[test]
    fn selection_bounds_reject_empty_invalid_and_missing_composition() {
        let preview = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(800.0, 600.0));
        let min = rhythm_core::domain::Vec2::new(50.0, 20.0).expect("min");
        let max = rhythm_core::domain::Vec2::new(50.0, 100.0).expect("max");
        assert!(selection_rect_on_preview(preview, (min, max), [1920, 1080]).is_none());
        let max = rhythm_core::domain::Vec2::new(200.0, 100.0).expect("max");
        assert!(selection_rect_on_preview(preview, (max, min), [1920, 1080]).is_none());
        assert!(selection_rect_on_preview(preview, (min, max), [0, 1080]).is_none());
    }

    #[test]
    fn fit_respects_non_widescreen_composition_dimensions() {
        let portrait = fit_composition_preview(egui::vec2(1000.0, 400.0), [1080, 1920]);
        assert!((portrait.x / portrait.y - 1080.0 / 1920.0).abs() < 0.0001);
        assert_eq!(
            fit_composition_preview(egui::vec2(1000.0, 400.0), [0, 1080]),
            egui::Vec2::ZERO
        );
    }

    #[test]
    fn selected_outline_union_skips_hidden_and_unavailable_objects() {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, TempoMap},
        };
        let mut project = Project::new(
            "Frame selection",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let id = ObjectId::new(1).expect("id");
        let mut rectangle = Object {
            id,
            name: "Visible".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(960.0, 540.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(400.0, 200.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        };
        let mut hidden = rectangle.clone();
        hidden.id = ObjectId::new(2).expect("id");
        hidden.visible = false;
        hidden.transform.position =
            Animated::new_static(Vec2::new(200.0, 200.0).expect("position"));
        project.composition.objects.push(rectangle.clone());
        project.composition.objects.push(hidden);
        let mut session = EditorSession::default();
        session.select_viewport_object(Some(id));
        session.toggle_viewport_object(Some(ObjectId::new(2).expect("id")));
        let (min, max) = selected_outline_bounds(&project, &session, 0.0).expect("outline union");
        assert_eq!(min, Vec2::new(760.0, 440.0).expect("min"));
        assert_eq!(max, Vec2::new(1160.0, 640.0).expect("max"));

        rectangle.visible = false;
        project.composition.objects[0] = rectangle;
        assert!(selected_outline_bounds(&project, &session, 0.0).is_none());
    }

    #[test]
    fn frame_selection_uses_camera_rectangle_and_respects_existing_pan() {
        let canvas = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(1000.0, 600.0));
        let fitted = fit_composition_preview(canvas.size(), [1920, 1080]);
        let mut session = EditorSession::default();
        session.begin_viewport_pan();
        session.update_viewport_pan([300.0, -200.0]);
        session.end_viewport_pan();
        assert!(session.frame_viewport_bounds(
            (
                rhythm_core::domain::Vec2::new(500.0, 300.0).expect("min"),
                rhythm_core::domain::Vec2::new(900.0, 500.0).expect("max"),
            ),
            [canvas.width(), canvas.height()],
            [fitted.x, fitted.y],
            [1920, 1080],
        ));
        let preview = panned_preview_rect(
            canvas,
            fitted * session.viewport_zoom(),
            session.viewport_pan_points(),
        );
        let center = rhythm_core::domain::Vec2::new(700.0, 400.0).expect("center");
        let projected = preview_point(preview, center, [1920, 1080]);
        assert!((projected.x - canvas.center().x).abs() < 0.01);
        assert!((projected.y - canvas.center().y).abs() < 0.01);
    }

    #[test]
    fn pointer_anchored_zoom_preserves_composition_point_and_outline_mapping() {
        let canvas = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(960.0, 600.0));
        let fit_size = fit_composition_preview(canvas.size(), [1920, 1080]);
        let mut session = EditorSession::default();
        // Include existing pan, avoiding a symmetric camera-only test.
        session.begin_viewport_pan();
        session.update_viewport_pan([45.0, -20.0]);
        session.end_viewport_pan();
        let before = panned_preview_rect(canvas, fit_size, session.viewport_pan_points());
        let pointer = before.center() + egui::vec2(90.0, 45.0);
        let composition = [1920, 1080];
        let original =
            composition_point_from_preview(before, pointer, composition).expect("inside image");

        assert!(session.zoom_viewport_at(
            1.75,
            [pointer.x - canvas.center().x, pointer.y - canvas.center().y],
        ));
        let after = panned_preview_rect(
            canvas,
            fit_size * session.viewport_zoom(),
            session.viewport_pan_points(),
        );
        let preserved =
            composition_point_from_preview(after, pointer, composition).expect("zoom anchor");
        assert!((preserved.x() - original.x()).abs() < 0.001);
        assert!((preserved.y() - original.y()).abs() < 0.001);
        let outline_point = preview_point(after, original, composition);
        assert!((outline_point.x - pointer.x).abs() < 0.001);
        assert!((outline_point.y - pointer.y).abs() < 0.001);
        session.reset_viewport_camera();
        assert_eq!(session.viewport_zoom(), 1.0);
        assert_eq!(session.viewport_pan_points(), [0.0, 0.0]);
    }

    #[test]
    fn camera_pan_shifts_image_and_hit_coordinates_together() {
        let canvas = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(1000.0, 800.0));
        let fitted = fit_composition_preview(canvas.size(), [1920, 1080]);
        let initial = panned_preview_rect(canvas, fitted, [0.0, 0.0]);
        let shifted = panned_preview_rect(canvas, fitted, [90.0, -70.0]);
        assert_eq!(shifted.min, initial.min + egui::vec2(90.0, -70.0));
        assert_eq!(shifted.size(), initial.size());

        let center = composition_point_from_preview(shifted, shifted.center(), [1920, 1080])
            .expect("panned image center");
        assert!((center.x() - 960.0).abs() < 0.01);
        assert!((center.y() - 540.0).abs() < 0.01);
        let earlier_center =
            composition_point_from_preview(shifted, initial.center(), [1920, 1080])
                .expect("image overlaps canvas center");
        assert!(earlier_center.x() < 960.0);
        assert!(earlier_center.y() > 540.0);
    }

    #[test]
    fn resetting_pan_can_recenter_an_offscreen_composition() {
        let canvas = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(800.0, 450.0));
        let size = fit_composition_preview(canvas.size(), [1920, 1080]);
        assert!(!panned_preview_rect(canvas, size, [2000.0, 0.0]).intersects(canvas));
        assert_eq!(
            panned_preview_rect(canvas, size, [0.0, 0.0]).center(),
            canvas.center()
        );
    }

    #[test]
    fn position_drag_pointer_mapping_respects_pan_zoom_and_allows_outside() {
        let canvas = egui::Rect::from_min_size(egui::pos2(300.0, 120.0), egui::vec2(960.0, 540.0));
        let preview = panned_preview_rect(canvas, egui::vec2(960.0, 540.0) * 2.0, [90.0, -40.0]);
        let start = egui::pos2(550.0, 250.0);
        let end = egui::pos2(610.0, 290.0);
        let a = unbounded_composition_point(preview, start, [1920, 1080]).expect("drag origin");
        let b = unbounded_composition_point(preview, end, [1920, 1080]).expect("drag target");
        assert!((b.x() - a.x() - 60.0).abs() < 0.01);
        assert!((b.y() - a.y() - 40.0).abs() < 0.01);
        assert!(
            unbounded_composition_point(preview, egui::pos2(-1000.0, -1000.0), [1920, 1080],)
                .is_some()
        );
    }

    #[test]
    fn marquee_uses_preview_coordinate_mapping_and_clamps_outside_drag() {
        let rect = egui::Rect::from_min_size(egui::pos2(200.0, 100.0), egui::vec2(960.0, 540.0));
        let size = [1920, 1080];
        let inside = egui::pos2(440.0, 235.0);
        let mapped = composition_point_from_preview(rect, inside, size).expect("inside");
        let round_trip = preview_point(rect, mapped, size);
        assert!((round_trip.x - inside.x).abs() < 0.01);
        assert!((round_trip.y - inside.y).abs() < 0.01);
        let clamped =
            clamped_drag_point(rect, egui::pos2(9999.0, 9999.0), size).expect("clamped to image");
        assert!((clamped.x() - 1920.0).abs() < 0.01);
        assert!((clamped.y() - 1080.0).abs() < 0.01);
        assert!(composition_point_from_preview(rect, egui::pos2(9999.0, 9999.0), size).is_none());
    }

    #[test]
    fn canvas_and_list_clicks_share_ctrl_toggle_semantics() {
        let a = ObjectId::new(10).expect("object");
        let b = ObjectId::new(11).expect("object");
        let mut session = EditorSession::default();
        apply_object_click(&mut session, Some(a), false);
        apply_object_click(&mut session, Some(b), true);
        assert_eq!(session.selected_objects(), &[a, b]);

        apply_object_click(&mut session, None, true);
        assert_eq!(session.selected_objects(), &[a, b]);
        apply_object_click(&mut session, Some(a), true);
        assert_eq!(session.selected_objects(), &[b]);

        apply_object_click(&mut session, None, false);
        assert_eq!(session.selected_object(), None);
        assert!(session.selected_objects().is_empty());
    }

    #[test]
    fn preview_click_coordinates_include_letterboxed_image_offset() {
        let rect = egui::Rect::from_min_size(egui::pos2(240.0, 100.0), egui::vec2(960.0, 540.0));
        let hit = composition_point_from_preview(rect, egui::pos2(720.0, 370.0), [1920, 1080])
            .expect("inside texture");
        assert!((hit.x() - 960.0).abs() < 0.01);
        assert!((hit.y() - 540.0).abs() < 0.01);
        assert!(
            composition_point_from_preview(rect, egui::pos2(200.0, 370.0), [1920, 1080],).is_none()
        );
        assert!(
            composition_point_from_preview(rect, egui::pos2(720.0, 370.0), [0, 1080],).is_none()
        );
    }

    #[test]
    fn preview_fit_preserves_composition_aspect() {
        let wide = fit_composition_preview(egui::vec2(1000.0, 400.0), [1920, 1080]);
        let tall = fit_composition_preview(egui::vec2(400.0, 1000.0), [1920, 1080]);

        assert!((wide.x / wide.y - 16.0 / 9.0).abs() < 0.0001);
        assert!((tall.x / tall.y - 16.0 / 9.0).abs() < 0.0001);
        assert!(wide.x <= 1000.0 && wide.y <= 400.0);
        assert!(tall.x <= 400.0 && tall.y <= 1000.0);
    }
}
