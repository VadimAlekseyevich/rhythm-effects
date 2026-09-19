use rhythm_core::time::ProjectTimeNs;
use rhythm_engine::waveform::{WaveformData, WaveformSlice};

const WAVEFORM_ROW_HEIGHT: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineTransform {
    rect: egui::Rect,
    start_time: ProjectTimeNs,
    end_time: ProjectTimeNs,
}

impl TimelineTransform {
    #[must_use]
    pub fn new(
        rect: egui::Rect,
        start_time: ProjectTimeNs,
        end_time: ProjectTimeNs,
    ) -> Option<Self> {
        if rect.width() <= 0.0 || end_time <= start_time {
            return None;
        }

        Some(Self {
            rect,
            start_time,
            end_time,
        })
    }

    #[must_use]
    pub const fn start_time(self) -> ProjectTimeNs {
        self.start_time
    }

    #[must_use]
    pub const fn end_time(self) -> ProjectTimeNs {
        self.end_time
    }

    #[must_use]
    pub const fn rect(self) -> egui::Rect {
        self.rect
    }

    #[must_use]
    pub fn project_time_to_x(self, project_time: ProjectTimeNs) -> f32 {
        let span_ns = (i128::from(self.end_time.get()) - i128::from(self.start_time.get())) as f64;
        let offset_ns =
            (i128::from(project_time.get()) - i128::from(self.start_time.get())) as f64;
        let normalized = offset_ns / span_ns;
        self.rect.left() + (normalized * f64::from(self.rect.width())) as f32
    }

    #[must_use]
    pub fn x_to_project_time(self, x: f32) -> ProjectTimeNs {
        let normalized =
            ((x - self.rect.left()) / self.rect.width()).clamp(0.0, 1.0) as f64;
        let start_ns = i128::from(self.start_time.get());
        let span_ns = i128::from(self.end_time.get()) - start_ns;
        let offset_ns = (span_ns as f64 * normalized).round() as i128;
        let project_ns = (start_ns + offset_ns)
            .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        ProjectTimeNs::new(project_ns)
    }
}

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

    let Some(end_time) =
        project_time_for_source_frame(source_frames, waveform.pyramid.source_sample_rate)
    else {
        return;
    };
    let Some(transform) =
        TimelineTransform::new(rect, ProjectTimeNs::new(0), end_time)
    else {
        return;
    };

    let level_index = waveform.choose_level_for_view(0, source_frames, rect.width());
    let Some(slice) = waveform.visible_slice(level_index, 0, source_frames) else {
        return;
    };

    let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let mesh = build_waveform_mesh(
        slice,
        waveform.pyramid.source_sample_rate,
        transform,
        color,
    );
    if !mesh.indices.is_empty() {
        ui.painter().add(egui::Shape::mesh(mesh));
    }
}

fn build_waveform_mesh(
    slice: WaveformSlice<'_>,
    source_sample_rate: u32,
    transform: TimelineTransform,
    color: egui::Color32,
) -> egui::epaint::Mesh {
    let mut mesh = egui::epaint::Mesh::default();
    let rect = transform.rect();
    if slice.peaks.is_empty()
        || source_sample_rate == 0
        || rect.width() <= 0.0
        || rect.height() <= 0.0
    {
        return mesh;
    }

    let center_y = rect.center().y;
    let half_height = rect.height() * 0.5;

    for (offset, peak) in slice.peaks.iter().enumerate() {
        let peak_index = slice.first_peak_index.saturating_add(offset);
        let Ok(peak_index) = u64::try_from(peak_index) else {
            break;
        };
        let peak_start = peak_index.saturating_mul(slice.frames_per_peak);
        let peak_end = peak_start.saturating_add(slice.frames_per_peak);

        let Some(peak_start_time) =
            project_time_for_source_frame(peak_start, source_sample_rate)
        else {
            continue;
        };
        let Some(peak_end_time) = project_time_for_source_frame(peak_end, source_sample_rate) else {
            continue;
        };

        let x0 = transform.project_time_to_x(peak_start_time).max(rect.left());
        let mut x1 = transform.project_time_to_x(peak_end_time).min(rect.right());
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

fn project_time_for_source_frame(frame: u64, sample_rate: u32) -> Option<ProjectTimeNs> {
    if sample_rate == 0 {
        return None;
    }

    let nanos = u128::from(frame)
        .checked_mul(1_000_000_000)?
        .checked_div(u128::from(sample_rate))?;
    Some(ProjectTimeNs::new(i64::try_from(nanos).ok()?))
}

#[cfg(test)]
mod tests {
    use super::{TimelineTransform, build_waveform_mesh};
    use rhythm_core::time::ProjectTimeNs;
    use rhythm_engine::waveform::{WavePeak, WaveformSlice};

    #[test]
    fn timeline_transform_round_trips_project_time_and_x() {
        let rect =
            egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 64.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(1_000_000_000),
            ProjectTimeNs::new(3_000_000_000),
        )
        .expect("valid transform");

        assert_eq!(
            transform.project_time_to_x(ProjectTimeNs::new(2_000_000_000)),
            60.0
        );
        assert_eq!(
            transform.x_to_project_time(60.0),
            ProjectTimeNs::new(2_000_000_000)
        );
        assert_eq!(
            transform.x_to_project_time(-100.0),
            ProjectTimeNs::new(1_000_000_000)
        );
        assert_eq!(
            transform.x_to_project_time(1_000.0),
            ProjectTimeNs::new(3_000_000_000)
        );
    }

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
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(0),
            ProjectTimeNs::new(128_000_000),
        )
        .expect("valid transform");

        let mesh = build_waveform_mesh(slice, 1_000, transform, egui::Color32::WHITE);

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
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(256_000_000),
            ProjectTimeNs::new(512_000_000),
        )
        .expect("valid transform");

        let mesh = build_waveform_mesh(slice, 1_000, transform, egui::Color32::WHITE);
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
