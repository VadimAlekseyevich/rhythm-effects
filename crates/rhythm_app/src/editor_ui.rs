use crate::{
    editor_session::{EditorSession, PreviewQuality, ViewportPositionAction},
    timeline::{draw_easing_preset_menu, draw_timeline},
};
use rhythm_core::{
    domain::Vec2,
    editor::{EditCommand, EditError, ProjectEditor},
    hit_test::{OutlineKind, object_outline, pick_topmost_object, select_objects_in_box},
    ids::ObjectId,
    project::Project,
    time::ProjectTimeNs,
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

fn inspector_position_id(ui: &egui::Ui, object_id: ObjectId, axis: PositionAxis) -> egui::Id {
    ui.make_persistent_id(("inspector_position", object_id.get(), axis.label()))
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

fn draw_inspector_position_field(
    ui: &mut egui::Ui,
    object_id: ObjectId,
    axis: PositionAxis,
    current: f32,
    actions: &mut EditorUiActions,
) {
    let id = inspector_position_id(ui, object_id, axis);
    let mut draft = ui
        .ctx()
        .data(|data| data.get_temp::<InspectorNumericDraft>(id))
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
            .data_mut(|data| data.remove::<InspectorNumericDraft>(id));
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
            .data_mut(|data| data.remove::<InspectorNumericDraft>(id));
        if enter {
            ui.memory_mut(|memory| memory.surrender_focus(id));
        }
    } else if response.has_focus() {
        ui.ctx().data_mut(|data| data.insert_temp(id, draft));
    } else {
        ui.ctx()
            .data_mut(|data| data.remove::<InspectorNumericDraft>(id));
    }
}

fn flush_inspector_draft_on_selection_change(
    ui: &egui::Ui,
    project: &Project,
    selected_object: Option<ObjectId>,
    actions: &mut EditorUiActions,
) {
    let marker = ui.make_persistent_id("last_inspector_position_object");
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
            let id = inspector_position_id(ui, old_id, axis);
            let draft = ui.ctx().data_mut(|data| {
                let draft = data.get_temp::<InspectorNumericDraft>(id);
                data.remove::<InspectorNumericDraft>(id);
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
    pub viewport_position_actions: Vec<ViewportPositionAction>,
    pub inspector_position_edits: Vec<InspectorPositionEdit>,
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
            for object in &project.composition.objects {
                let label = format!(
                    "{}{}{}",
                    if !object.visible { "[hidden] " } else { "" },
                    if object.locked { "[locked] " } else { "" },
                    object.name,
                );
                if ui
                    .selectable_label(session.is_object_selected(object.id), label)
                    .on_hover_text("Click to select; Ctrl+click to toggle selection")
                    .clicked()
                {
                    apply_object_click(
                        session,
                        Some(object.id),
                        ui.input(|input| input.modifiers.ctrl),
                    );
                }
            }
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
                    ui.small("Multi-object transform editing is not wired yet.");
                } else if object.locked {
                    ui.small("Locked object: editing is disabled.");
                } else if !object.transform.position.keyframes().is_empty() {
                    ui.small("Animated Position: edit through the keyframe timeline.");
                } else {
                    ui.label("Transform — Position (px)");
                    let base = *object.transform.position.base_value();
                    ui.horizontal(|ui| {
                        draw_inspector_position_field(
                            ui,
                            object.id,
                            PositionAxis::X,
                            base.x(),
                            actions,
                        );
                        draw_inspector_position_field(
                            ui,
                            object.id,
                            PositionAxis::Y,
                            base.y(),
                            actions,
                        );
                    });
                    ui.small("Enter / focus loss to commit · Escape to cancel.");
                    ui.small("Scale, Rotation, Anchor and Opacity controls are pending.");
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
            "Drag a single unlocked Rectangle/Ellipse to move its static Position (one Undo step).",
        );
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
                // An object drag edits exactly one *static* Position with an
                // existing ProjectEditor transaction. A drag from empty
                // composition space is a marquee. Ctrl and multi-object
                // drags are reserved for dedicated later interactions.
                if response.drag_started_by(egui::PointerButton::Primary)
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    let origin = ui
                        .input(|input| input.pointer.press_origin())
                        .unwrap_or(pointer);
                    if let Some(start) =
                        composition_point_from_preview(preview, origin, composition_size)
                    {
                        let picked = pick_topmost_object(
                            &project.composition.objects,
                            tick,
                            start,
                            |_| None,
                            |_| None,
                        );
                        if let Some(id) = picked {
                            if !ui.input(|input| input.modifiers.ctrl)
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
                            session.begin_viewport_box_selection(
                                start,
                                ui.input(|input| input.modifiers.ctrl),
                            );
                        }
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary)
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    if let Some(point) =
                        unbounded_composition_point(preview, pointer, composition_size)
                        && let Some(position) = session.update_viewport_position_drag(point)
                    {
                        actions
                            .viewport_position_actions
                            .push(ViewportPositionAction::Update(position));
                    }
                    if let Some(current) = clamped_drag_point(preview, pointer, composition_size) {
                        session.update_viewport_box_selection(current);
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
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
        InspectorNumericDraft, InspectorPositionEdit, PositionAxis, apply_inspector_position_edits,
        apply_object_click, clamped_drag_point, committed_number, composition_point_from_preview,
        fit_composition_preview, panned_preview_rect, preview_point, selected_outline_bounds,
        selection_rect_on_preview, unbounded_composition_point,
    };
    use crate::editor_session::EditorSession;
    use rhythm_core::ids::ObjectId;

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
