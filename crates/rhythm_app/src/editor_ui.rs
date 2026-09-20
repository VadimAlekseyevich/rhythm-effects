use crate::{
    editor_session::{EditorSession, PreviewQuality},
    timeline::draw_timeline,
    viewport::{objects_intersecting_box, pick_topmost_object},
};
use rhythm_core::{
    domain::Vec2,
    geometry::LocalBounds2d,
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
                let mut follow_playhead = session.follow_playhead();
                if ui
                    .toggle_value(&mut follow_playhead, "Follow Playhead")
                    .changed()
                {
                    session.set_follow_playhead(follow_playhead);
                }
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
            ui.heading("Timeline");
            ui.separator();
            draw_timeline(ui, session, project, waveform);
            ui.take_available_space();
        });

    egui::Panel::left("objects")
        .resizable(true)
        .default_size(240.0)
        .show(ui, |ui| {
            ui.heading("Objects");
            ui.separator();
            ui.label("Composition objects will appear here.");
            ui.take_available_space();
        });

    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(320.0)
        .show(ui, |ui| {
            ui.heading("Inspector");
            ui.separator();
            ui.label("Selected object properties will appear here.");
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
        ui.separator();

        let viewport_rect = ui.available_rect_before_wrap();
        let pan_response = ui.interact(
            viewport_rect,
            ui.id().with("viewport_middle_pan"),
            egui::Sense::drag(),
        );
        if pan_response.dragged_by(egui::PointerButton::Middle) {
            let delta = ui.input(|input| input.pointer.delta());
            session.pan_viewport_points([delta.x, delta.y]);
        }

        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            && viewport_rect.contains(pointer)
        {
            let scroll_y = ui.input(|input| input.smooth_scroll_delta().y);
            if scroll_y.abs() > f32::EPSILON {
                let zoom_factor = (scroll_y * 0.002).exp();
                let anchor = [
                    pointer.x - viewport_rect.center().x,
                    pointer.y - viewport_rect.center().y,
                ];
                session.zoom_viewport_around_anchor(zoom_factor, anchor);
            }
        }

        if let Some(texture_id) = composition_texture_id {
            let preview_size =
                fit_composition_preview(viewport_rect.size()) * session.viewport_zoom();
            let pan = session.viewport_pan_points();
            let preview_rect = egui::Rect::from_center_size(
                viewport_rect.center() + egui::vec2(pan[0], pan[1]),
                preview_size,
            );
            let response = ui.put(
                preview_rect,
                egui::Image::from_texture(egui::load::SizedTexture::new(
                    texture_id,
                    egui::vec2(1920.0, 1080.0),
                ))
                .fit_to_exact_size(preview_size)
                .sense(egui::Sense::click()),
            );

            let screen_to_composition = |point: egui::Pos2| -> Option<Vec2> {
                if response.rect.width() <= 0.0
                    || response.rect.height() <= 0.0
                    || !response.rect.contains(point)
                {
                    return None;
                }

                let normalized_x =
                    ((point.x - response.rect.left()) / response.rect.width()).clamp(0.0, 1.0);
                let normalized_y =
                    ((point.y - response.rect.top()) / response.rect.height()).clamp(0.0, 1.0);
                Vec2::new(
                    normalized_x * project.settings.composition_width as f32,
                    normalized_y * project.settings.composition_height as f32,
                )
                .ok()
            };

            if response.clicked()
                && let Some(pointer) = response.interact_pointer_pos()
                && let Some(composition_point) = screen_to_composition(pointer)
                && let Ok(scene) =
                    rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
            {
                let picked = pick_topmost_object(project, &scene, composition_point, |_, _| None);
                let ctrl = ui.input(|input| input.modifiers.ctrl);
                if ctrl {
                    if let Some(object_id) = picked {
                        session.toggle_object_selection(object_id);
                    }
                } else {
                    session.replace_object_selection(picked);
                }
            }

            let (primary_pressed, primary_down, primary_released, pointer_pos, press_origin) =
                ui.input(|input| {
                    (
                        input.pointer.primary_pressed(),
                        input.pointer.primary_down(),
                        input.pointer.primary_released(),
                        input.pointer.interact_pos(),
                        input.pointer.press_origin(),
                    )
                });

            if primary_pressed
                && let Some(origin) = press_origin
                && let Some(composition_origin) = screen_to_composition(origin)
                && let Ok(scene) =
                    rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
                && pick_topmost_object(project, &scene, composition_origin, |_, _| None).is_none()
            {
                session.begin_viewport_box_selection([origin.x, origin.y]);
            }

            if primary_down
                && let Some(pointer) = pointer_pos
                && session.viewport_box_selection().is_some()
            {
                let clamped = egui::pos2(
                    pointer.x.clamp(response.rect.left(), response.rect.right()),
                    pointer.y.clamp(response.rect.top(), response.rect.bottom()),
                );
                session.update_viewport_box_selection([clamped.x, clamped.y]);
            }

            if let Some((start, current)) = session.viewport_box_selection() {
                let selection_rect = egui::Rect::from_two_pos(
                    egui::pos2(start[0], start[1]),
                    egui::pos2(current[0], current[1]),
                )
                .intersect(response.rect);
                if selection_rect.is_positive() {
                    ui.painter().rect_stroke(
                        selection_rect,
                        0.0,
                        egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().rect_filled(
                        selection_rect,
                        0.0,
                        ui.visuals().selection.bg_fill.gamma_multiply(0.12),
                    );
                }
            }

            if primary_released
                && let Some((start, current)) = session.take_viewport_box_selection()
            {
                let screen_rect = egui::Rect::from_two_pos(
                    egui::pos2(start[0], start[1]),
                    egui::pos2(current[0], current[1]),
                )
                .intersect(response.rect);
                if let (Some(min), Some(max)) = (
                    screen_to_composition(screen_rect.min),
                    screen_to_composition(screen_rect.max),
                ) && let Ok(scene) =
                    rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
                {
                    let selection_bounds = LocalBounds2d::new(
                        min,
                        Vec2::new(max.x() - min.x(), max.y() - min.y())
                            .expect("viewport selection bounds are finite"),
                    );
                    let selected =
                        objects_intersecting_box(project, &scene, selection_bounds, |_, _| None);
                    session.replace_object_selection_many(selected);
                }
            }
        } else {
            ui.painter().text(
                viewport_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Composition preview unavailable",
                egui::FontId::proportional(15.0),
                ui.visuals().text_color(),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::fit_composition_preview;

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
