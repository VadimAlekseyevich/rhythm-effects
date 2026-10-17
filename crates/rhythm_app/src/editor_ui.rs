use crate::{
    editor_session::{EditorSession, PreviewQuality},
    timeline::{draw_easing_preset_menu, draw_timeline},
};
use rhythm_core::{
    domain::Vec2,
    hit_test::{OutlineKind, object_outline, pick_topmost_object, select_objects_in_box},
    ids::ObjectId,
    project::Project,
    time::ProjectTimeNs,
};

fn fit_composition_preview(available: egui::Vec2) -> egui::Vec2 {
    const COMPOSITION_ASPECT: f32 = 1920.0 / 1080.0;

    if available.x <= 0.0 || available.y <= 0.0 {
        return egui::Vec2::ZERO;
    }

    let available_aspect = available.x / available.y;
    if available_aspect > COMPOSITION_ASPECT {
        egui::vec2(available.y * COMPOSITION_ASPECT, available.y)
    } else {
        egui::vec2(available.x, available.x / COMPOSITION_ASPECT)
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
    add_rectangle_requested: &mut bool,
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
                *add_rectangle_requested = true;
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
                } else {
                    ui.label("Transform controls are not wired yet.");
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
        ui.small("Click to select; Ctrl+click toggles objects without clearing the others.");
        ui.small("Drag from empty space to box-select; hold Ctrl to toggle intersecting objects.");
        ui.small("Middle-drag pans; wheel zooms around cursor (12.5%–1600% of Fit).");
        if ui.small_button("Reset View").clicked() {
            session.reset_viewport_camera();
        }
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
                    let (pointer, scroll) = ui.input(|input| {
                        (input.pointer.hover_pos(), input.smooth_scroll_delta().y)
                    });
                    if let Some(pointer) = pointer
                        && scroll.is_finite()
                        && scroll != 0.0
                    {
                        let multiplier = (f64::from(scroll) * 0.003)
                            .clamp(-4.0, 4.0)
                            .exp() as f32;
                        session.zoom_viewport_at(
                            multiplier,
                            [
                                pointer.x - canvas.center().x,
                                pointer.y - canvas.center().y,
                            ],
                        );
                    }
                }

                let preview_size =
                    fit_composition_preview(canvas.size()) * session.viewport_zoom();
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
                let tick = project
                    .tempo_map
                    .continuous_tick_position(session.playhead())
                    .unwrap_or(0.0);
                draw_editor_outlines(ui, preview, canvas, session, project, tick);
                let composition_size = [
                    project.settings.composition_width,
                    project.settings.composition_height,
                ];
                // A marquee can only begin with the mouse pressed on empty
                // composition geometry. Drags begun on an object are reserved
                // for the future move gizmo, not misinterpreted as box selection.
                if response.drag_started_by(egui::PointerButton::Primary)
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    let start_pointer = ui
                        .input(|input| input.pointer.press_origin())
                        .unwrap_or(pointer);
                    if let Some(start) =
                        composition_point_from_preview(preview, start_pointer, composition_size)
                        && pick_topmost_object(
                            &project.composition.objects,
                            tick,
                            start,
                            |_| None,
                            |_| None,
                        )
                        .is_none()
                    {
                        session.begin_viewport_box_selection(
                            start,
                            ui.input(|input| input.modifiers.ctrl),
                        );
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary)
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(current) = clamped_drag_point(preview, pointer, composition_size)
                {
                    session.update_viewport_box_selection(current);
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
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
        apply_object_click, clamped_drag_point, composition_point_from_preview,
        fit_composition_preview, panned_preview_rect, preview_point,
    };
    use crate::editor_session::EditorSession;
    use rhythm_core::ids::ObjectId;

    #[test]
    fn pointer_anchored_zoom_preserves_composition_point_and_outline_mapping() {
        let canvas = egui::Rect::from_min_size(
            egui::pos2(100.0, 100.0),
            egui::vec2(960.0, 600.0),
        );
        let fit_size = fit_composition_preview(canvas.size());
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
        let fitted = fit_composition_preview(canvas.size());
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
        let size = fit_composition_preview(canvas.size());
        assert!(!panned_preview_rect(canvas, size, [2000.0, 0.0]).intersects(canvas));
        assert_eq!(
            panned_preview_rect(canvas, size, [0.0, 0.0]).center(),
            canvas.center()
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
        let wide = fit_composition_preview(egui::vec2(1000.0, 400.0));
        let tall = fit_composition_preview(egui::vec2(400.0, 1000.0));

        assert!((wide.x / wide.y - 16.0 / 9.0).abs() < 0.0001);
        assert!((tall.x / tall.y - 16.0 / 9.0).abs() < 0.0001);
        assert!(wide.x <= 1000.0 && wide.y <= 400.0);
        assert!(tall.x <= 400.0 && tall.y <= 1000.0);
    }
}
