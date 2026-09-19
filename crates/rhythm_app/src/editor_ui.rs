use crate::editor_session::{EditorSession, PreviewQuality};

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
    diagnostics: &DiagnosticsView,
    composition_texture_id: Option<egui::TextureId>,
) {
    egui::Panel::top("transport_rhythm")
        .exact_size(54.0)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("Rhythm Effects");
                ui.separator();
                ui.label("Transport");
                ui.separator();
                ui.label("BPM —");
                ui.label("4/4");
                ui.label("Grid 1/4");
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
            ui.label("Musical timeline placeholder");
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
        ui.centered_and_justified(|ui| {
            if let Some(texture_id) = composition_texture_id {
                let preview_size = fit_composition_preview(ui.available_size());
                ui.add(
                    egui::Image::from_texture(egui::load::SizedTexture::new(
                        texture_id,
                        egui::vec2(1920.0, 1080.0),
                    ))
                    .fit_to_exact_size(preview_size),
                );
            } else {
                ui.label("Composition preview unavailable");
            }
        });
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
