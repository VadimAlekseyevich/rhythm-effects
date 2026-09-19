use rhythm_engine::waveform::{WaveformData, WaveformSlice};

const WAVEFORM_ROW_HEIGHT: f32 = 64.0;

pub fn draw_timeline_waveform(ui: &mut egui::Ui, waveform: Option<&WaveformData>) {
    let desired_size = egui::vec2(ui.available_width(), WAVEFORM_ROW_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let Some(waveform) = waveform else {
        return;
    };
    let source_frames = waveform.pyramid.source_frame_count;
    if source_frames == 0 || rect.width() <= 0.0 {
        return;
    }

    let level_index = waveform.choose_level_for_view(0, source_frames, rect.width());
    let Some(slice) = waveform.visible_slice(level_index, 0, source_frames) else {
        return;
    };

    let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let mesh = build_waveform_mesh(rect, slice, 0, source_frames, color);
    if !mesh.indices.is_empty() {
        ui.painter().add(egui::Shape::mesh(mesh));
    }
}

fn build_waveform_mesh(
    rect: egui::Rect,
    slice: WaveformSlice<'_>,
    visible_start_frame: u64,
    visible_end_frame: u64,
    color: egui::Color32,
) -> egui::epaint::Mesh {
    let mut mesh = egui::epaint::Mesh::default();
    if slice.peaks.is_empty()
        || visible_end_frame <= visible_start_frame
        || rect.width() <= 0.0
        || rect.height() <= 0.0
    {
        return mesh;
    }

    let visible_span = visible_end_frame - visible_start_frame;
    let center_y = rect.center().y;
    let half_height = rect.height() * 0.5;

    for (offset, peak) in slice.peaks.iter().enumerate() {
        let peak_index = slice.first_peak_index.saturating_add(offset);
        let Ok(peak_index) = u64::try_from(peak_index) else {
            break;
        };
        let peak_start = peak_index.saturating_mul(slice.frames_per_peak);
        let peak_end = peak_start.saturating_add(slice.frames_per_peak);

        let clipped_start = peak_start.max(visible_start_frame);
        let clipped_end = peak_end.min(visible_end_frame);
        if clipped_end <= clipped_start {
            continue;
        }

        let x0 = frame_to_x(clipped_start, visible_start_frame, visible_span, rect);
        let mut x1 = frame_to_x(clipped_end, visible_start_frame, visible_span, rect);
        x1 = x1.max(x0 + 0.75).min(rect.right());
        if x1 <= x0 {
            continue;
        }

        let amplitude_min = peak.min.clamp(-1.0, 1.0);
        let amplitude_max = peak.max.clamp(-1.0, 1.0);
        let mut top = center_y - amplitude_max * half_height;
        let mut bottom = center_y - amplitude_min * half_height;

        if bottom - top < 1.0 {
            let middle = (top + bottom) * 0.5;
            top = (middle - 0.5).max(rect.top());
            bottom = (middle + 0.5).min(rect.bottom());
        }

        let peak_rect = egui::Rect::from_min_max(
            egui::pos2(x0, top.max(rect.top())),
            egui::pos2(x1, bottom.min(rect.bottom())),
        );
        mesh.add_colored_rect(peak_rect, color);
    }

    mesh
}

fn frame_to_x(frame: u64, visible_start_frame: u64, visible_span: u64, rect: egui::Rect) -> f32 {
    let relative = frame.saturating_sub(visible_start_frame);
    let normalized = relative as f64 / visible_span as f64;
    rect.left() + (normalized * f64::from(rect.width())) as f32
}

#[cfg(test)]
mod tests {
    use super::build_waveform_mesh;
    use rhythm_engine::waveform::{WavePeak, WaveformSlice};

    #[test]
    fn waveform_uses_one_mesh_for_all_visible_peaks() {
        let peaks = [
            WavePeak {
                min: -1.0,
                max: 0.5,
            },
            WavePeak {
                min: -0.25,
                max: 0.75,
            },
        ];
        let slice = WaveformSlice {
            level_index: 0,
            first_peak_index: 0,
            frames_per_peak: 64,
            peaks: &peaks,
        };
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(128.0, 64.0));

        let mesh = build_waveform_mesh(rect, slice, 0, 128, egui::Color32::WHITE);

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.indices.len(), 12);
    }

    #[test]
    fn waveform_mesh_maps_visible_range_to_panel_width() {
        let peaks = [WavePeak {
            min: -0.5,
            max: 0.5,
        }];
        let slice = WaveformSlice {
            level_index: 2,
            first_peak_index: 1,
            frames_per_peak: 256,
            peaks: &peaks,
        };
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 64.0));

        let mesh = build_waveform_mesh(rect, slice, 256, 512, egui::Color32::WHITE);
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.pos.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.pos.x)
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(min_x, rect.left());
        assert_eq!(max_x, rect.right());
    }
}
