use crate::{
    editor_session::{EditorSession, PreviewQuality},
    timeline::{draw_easing_preset_menu, draw_timeline},
};
use rhythm_core::{
    domain::Vec2,
    hit_test::{OutlineKind, object_outline, pick_topmost_object},
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
    session: &EditorSession,
    project: &Project,
    continuous_tick: f64,
) {
    let width = project.settings.composition_width as f32;
    let height = project.settings.composition_height as f32;
    let painter = ui.painter().with_clip_rect(preview);
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
        ui.separator();
        ui.centered_and_justified(|ui| {
            if let Some(texture_id) = composition_texture_id {
                let preview_size = fit_composition_preview(ui.available_size());
                let response = ui.add(
                    egui::Image::from_texture(egui::load::SizedTexture::new(
                        texture_id,
                        egui::vec2(1920.0, 1080.0),
                    ))
                    .fit_to_exact_size(preview_size)
                    .sense(egui::Sense::click()),
                );
                // Static objects remain pickable in projects without a tempo map.
                // No audio-clock or timeline playback is inferred.
                let tick = project
                    .tempo_map
                    .continuous_tick_position(session.playhead())
                    .unwrap_or(0.0);
                draw_editor_outlines(ui, response.rect, session, project, tick);
                if response.clicked()
                    && let Some(pointer) = response.interact_pointer_pos()
                    && let Some(point) = composition_point_from_preview(
                        response.rect,
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
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{apply_object_click, composition_point_from_preview, fit_composition_preview};
    use crate::editor_session::EditorSession;
    use rhythm_core::ids::ObjectId;

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
