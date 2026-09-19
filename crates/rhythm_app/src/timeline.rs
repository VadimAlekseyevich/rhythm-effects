use crate::editor_session::EditorSession;
use rhythm_core::time::{
    BeatDivision, DurationNs, MusicalTick, PPQ, ProjectTimeNs, TempoMap,
    floor_tick_position_to_grid,
};
use rhythm_engine::waveform::{WaveformData, WaveformSlice};

const RULER_ROW_HEIGHT: f32 = 28.0;
const WAVEFORM_ROW_HEIGHT: f32 = 64.0;
const MIN_RULER_LABEL_SPACING_PX: f32 = 72.0;
const MAX_GRID_LINES_PER_FRAME: usize = 100_000;

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
        let offset_ns = (i128::from(project_time.get()) - i128::from(self.start_time.get())) as f64;
        let normalized = offset_ns / span_ns;
        self.rect.left() + (normalized * f64::from(self.rect.width())) as f32
    }

    #[must_use]
    pub fn x_to_project_time(self, x: f32) -> ProjectTimeNs {
        let normalized = ((x - self.rect.left()) / self.rect.width()).clamp(0.0, 1.0) as f64;
        let start_ns = i128::from(self.start_time.get());
        let span_ns = i128::from(self.end_time.get()) - start_ns;
        let offset_ns = (span_ns as f64 * normalized).round() as i128;
        let project_ns =
            (start_ns + offset_ns).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        ProjectTimeNs::new(project_ns)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MusicalGridLineKind {
    Bar,
    Beat,
    Subdivision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MusicalGridLine {
    tick: MusicalTick,
    kind: MusicalGridLineKind,
}

pub fn draw_timeline(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    tempo_map: &TempoMap,
    project_duration: DurationNs,
    waveform: Option<&WaveformData>,
) {
    let (mut start_time, mut end_time) = session.timeline_range(project_duration);

    let (ruler_rect, ruler_response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), RULER_ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    let (waveform_rect, waveform_response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), WAVEFORM_ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );

    let grid_rect = egui::Rect::from_min_max(
        egui::pos2(ruler_rect.left(), ruler_rect.top()),
        egui::pos2(waveform_rect.right(), waveform_rect.bottom()),
    );

    let initial_transform = TimelineTransform::new(ruler_rect, start_time, end_time);
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let pointer_over_timeline = pointer.is_some_and(|position| grid_rect.contains(position));

    if pointer_over_timeline {
        let (zoom_delta, scroll_delta, modifiers) = ui.input(|input| {
            (
                input.zoom_delta(),
                input.smooth_scroll_delta(),
                input.modifiers,
            )
        });

        if modifiers.ctrl && (zoom_delta - 1.0).abs() >= 0.000_1 {
            if let (Some(transform), Some(pointer)) = (initial_transform, pointer) {
                let anchor = transform.x_to_project_time(pointer.x);
                session.zoom_timeline(project_duration, anchor, zoom_delta);
            }
        } else if modifiers.shift {
            let horizontal_delta = if scroll_delta.x.abs() > f32::EPSILON {
                scroll_delta.x
            } else {
                scroll_delta.y
            };
            session.pan_timeline_points(project_duration, horizontal_delta, ruler_rect.width());
        }
    }

    let middle_drag_delta = if ruler_response.dragged_by(egui::PointerButton::Middle) {
        ruler_response.drag_delta().x
    } else if waveform_response.dragged_by(egui::PointerButton::Middle) {
        waveform_response.drag_delta().x
    } else {
        0.0
    };
    if middle_drag_delta.abs() > f32::EPSILON {
        session.pan_timeline_points(project_duration, middle_drag_delta, ruler_rect.width());
    }

    (start_time, end_time) = session.timeline_range(project_duration);
    let Some(ruler_transform) = TimelineTransform::new(ruler_rect, start_time, end_time) else {
        return;
    };
    let Some(waveform_transform) = TimelineTransform::new(waveform_rect, start_time, end_time)
    else {
        return;
    };

    if (ruler_response.clicked() || ruler_response.dragged_by(egui::PointerButton::Primary))
        && let Some(pointer) = ruler_response.interact_pointer_pos()
    {
        session.seek_paused(ruler_transform.x_to_project_time(pointer.x));
    }

    draw_waveform(ui, waveform_rect, waveform_transform, waveform);

    draw_musical_grid(
        ui,
        grid_rect,
        ruler_transform,
        tempo_map,
        session.authoring_division(),
    );
    draw_time_ruler(ui, ruler_rect, ruler_transform);
    draw_playhead(ui, grid_rect, ruler_transform, session.playhead());
}

fn draw_playhead(
    ui: &egui::Ui,
    rect: egui::Rect,
    transform: TimelineTransform,
    playhead: ProjectTimeNs,
) {
    let x = transform.project_time_to_x(playhead);
    if x < rect.left() || x > rect.right() {
        return;
    }

    let color = ui.visuals().selection.stroke.color;
    ui.painter().line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(2.0, color),
    );
    ui.painter()
        .circle_filled(egui::pos2(x, rect.top() + 4.0), 4.0, color);
}

fn draw_time_ruler(ui: &egui::Ui, rect: egui::Rect, transform: TimelineTransform) {
    let interval_ns = choose_ruler_interval_ns(transform);
    let first = transform.start_time().get().div_euclid(interval_ns) * interval_ns;
    let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let subtle = color.gamma_multiply(0.45);
    let mut tick_time = first;
    let mut rendered = 0_usize;

    while tick_time <= transform.end_time().get() && rendered < 10_000 {
        if tick_time >= transform.start_time().get() {
            let project_time = ProjectTimeNs::new(tick_time);
            let x = transform.project_time_to_x(project_time);
            ui.painter().line_segment(
                [
                    egui::pos2(x, rect.bottom() - 7.0),
                    egui::pos2(x, rect.bottom()),
                ],
                egui::Stroke::new(1.0, subtle),
            );
            ui.painter().text(
                egui::pos2(x + 3.0, rect.top() + 2.0),
                egui::Align2::LEFT_TOP,
                format_ruler_label(project_time, interval_ns),
                egui::FontId::monospace(11.0),
                color,
            );
        }

        let Some(next) = tick_time.checked_add(interval_ns) else {
            break;
        };
        tick_time = next;
        rendered += 1;
    }
}

fn choose_ruler_interval_ns(transform: TimelineTransform) -> i64 {
    const INTERVALS: [i64; 14] = [
        100_000_000,
        250_000_000,
        500_000_000,
        1_000_000_000,
        2_000_000_000,
        5_000_000_000,
        10_000_000_000,
        30_000_000_000,
        60_000_000_000,
        120_000_000_000,
        300_000_000_000,
        600_000_000_000,
        1_800_000_000_000,
        3_600_000_000_000,
    ];

    for interval in INTERVALS {
        let x0 = transform.project_time_to_x(transform.start_time());
        let x1 = transform.project_time_to_x(ProjectTimeNs::new(
            transform.start_time().get().saturating_add(interval),
        ));
        if (x1 - x0).abs() >= MIN_RULER_LABEL_SPACING_PX {
            return interval;
        }
    }

    INTERVALS[INTERVALS.len() - 1]
}

fn format_ruler_label(project_time: ProjectTimeNs, interval_ns: i64) -> String {
    let total_ns = project_time.get().max(0);
    let total_seconds = total_ns as f64 / 1_000_000_000.0;

    if total_seconds >= 60.0 {
        let minutes = (total_seconds / 60.0).floor() as u64;
        let seconds = total_seconds - minutes as f64 * 60.0;
        format!("{minutes}:{seconds:04.1}")
    } else if interval_ns < 1_000_000_000 {
        format!("{total_seconds:.1}s")
    } else {
        format!("{total_seconds:.0}s")
    }
}

fn draw_musical_grid(
    ui: &egui::Ui,
    rect: egui::Rect,
    transform: TimelineTransform,
    tempo_map: &TempoMap,
    division: BeatDivision,
) {
    let Some(segment) = tempo_map.initial_segment() else {
        return;
    };
    let lines = collect_musical_grid_lines(tempo_map, division, transform);
    if lines.is_empty() {
        return;
    }

    let base_color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let subdivision_color = base_color.gamma_multiply(0.16);
    let beat_color = base_color.gamma_multiply(0.32);
    let bar_color = base_color.gamma_multiply(0.58);

    let subdivision_spacing = grid_spacing_px(tempo_map, division.ticks_per_step(), transform);
    let beat_spacing = grid_spacing_px(tempo_map, PPQ, transform);
    let ticks_per_bar = ticks_per_bar(segment.meter());
    let bar_spacing = grid_spacing_px(tempo_map, ticks_per_bar, transform);
    let draw_subdivisions = subdivision_spacing >= 4.0;
    let draw_beats = beat_spacing >= 5.0;
    let draw_bar_labels = bar_spacing >= MIN_RULER_LABEL_SPACING_PX;

    for line in lines {
        if line.kind == MusicalGridLineKind::Subdivision && !draw_subdivisions {
            continue;
        }
        if line.kind == MusicalGridLineKind::Beat && !draw_beats {
            continue;
        }

        let Ok(project_time) = tempo_map.project_time_for_tick(line.tick) else {
            continue;
        };
        let x = transform.project_time_to_x(project_time);
        if x < rect.left() || x > rect.right() {
            continue;
        }

        let (width, color) = match line.kind {
            MusicalGridLineKind::Bar => (1.5, bar_color),
            MusicalGridLineKind::Beat => (1.0, beat_color),
            MusicalGridLineKind::Subdivision => (0.5, subdivision_color),
        };
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(width, color),
        );

        if line.kind == MusicalGridLineKind::Bar && draw_bar_labels && ticks_per_bar > 0 {
            let bar_index = line.tick.get().div_euclid(ticks_per_bar) + 1;
            ui.painter().text(
                egui::pos2(x + 3.0, rect.top() + 14.0),
                egui::Align2::LEFT_TOP,
                format!("B{bar_index}"),
                egui::FontId::monospace(10.0),
                bar_color,
            );
        }
    }
}

fn collect_musical_grid_lines(
    tempo_map: &TempoMap,
    division: BeatDivision,
    transform: TimelineTransform,
) -> Vec<MusicalGridLine> {
    let Some(segment) = tempo_map.initial_segment() else {
        return Vec::new();
    };
    let Ok(start_tick_position) = tempo_map.continuous_tick_position(transform.start_time()) else {
        return Vec::new();
    };
    let Ok(end_tick_position) = tempo_map.continuous_tick_position(transform.end_time()) else {
        return Vec::new();
    };

    let Ok(mut tick) = floor_tick_position_to_grid(start_tick_position, division) else {
        return Vec::new();
    };
    let step = division.ticks_per_step();
    let last_tick = end_tick_position.ceil() as i64;
    let bar_ticks = ticks_per_bar(segment.meter());
    if step <= 0 || bar_ticks <= 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for _ in 0..MAX_GRID_LINES_PER_FRAME {
        if tick.get() > last_tick {
            break;
        }

        let kind = if tick.get().rem_euclid(bar_ticks) == 0 {
            MusicalGridLineKind::Bar
        } else if tick.get().rem_euclid(PPQ) == 0 {
            MusicalGridLineKind::Beat
        } else {
            MusicalGridLineKind::Subdivision
        };
        lines.push(MusicalGridLine { tick, kind });

        let Some(next) = tick.get().checked_add(step) else {
            break;
        };
        tick = MusicalTick::new(next);
    }

    lines
}

fn ticks_per_bar(meter: rhythm_core::time::TimeSignature) -> i64 {
    let numerator = i64::from(meter.numerator());
    let denominator = i64::from(meter.denominator());
    PPQ.saturating_mul(4)
        .saturating_mul(numerator)
        .checked_div(denominator)
        .unwrap_or(0)
}

fn grid_spacing_px(tempo_map: &TempoMap, tick_delta: i64, transform: TimelineTransform) -> f32 {
    let Ok(a) = tempo_map.project_time_for_tick(MusicalTick::new(0)) else {
        return 0.0;
    };
    let Ok(b) = tempo_map.project_time_for_tick(MusicalTick::new(tick_delta)) else {
        return 0.0;
    };
    (transform.project_time_to_x(b) - transform.project_time_to_x(a)).abs()
}

fn draw_waveform(
    ui: &egui::Ui,
    rect: egui::Rect,
    transform: TimelineTransform,
    waveform: Option<&WaveformData>,
) {
    let Some(waveform) = waveform else {
        return;
    };
    let sample_rate = waveform.pyramid.source_sample_rate;
    let source_frames = waveform.pyramid.source_frame_count;
    if sample_rate == 0 || source_frames == 0 {
        return;
    }

    let visible_start_frame = source_frame_for_project_time(transform.start_time(), sample_rate);
    let visible_end_frame =
        source_frame_for_project_time(transform.end_time(), sample_rate).min(source_frames);
    if visible_end_frame <= visible_start_frame {
        return;
    }

    let level_index =
        waveform.choose_level_for_view(visible_start_frame, visible_end_frame, rect.width());
    let Some(slice) = waveform.visible_slice(level_index, visible_start_frame, visible_end_frame)
    else {
        return;
    };

    let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let mesh = build_waveform_mesh(slice, sample_rate, transform, color);
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

        let Some(peak_start_time) = project_time_for_source_frame(peak_start, source_sample_rate)
        else {
            continue;
        };
        let Some(peak_end_time) = project_time_for_source_frame(peak_end, source_sample_rate)
        else {
            continue;
        };

        let x0 = transform
            .project_time_to_x(peak_start_time)
            .max(rect.left());
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

fn source_frame_for_project_time(project_time: ProjectTimeNs, sample_rate: u32) -> u64 {
    if project_time.get() <= 0 || sample_rate == 0 {
        return 0;
    }

    let frames =
        (project_time.get() as u128).saturating_mul(u128::from(sample_rate)) / 1_000_000_000_u128;
    u64::try_from(frames).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        MusicalGridLineKind, TimelineTransform, build_waveform_mesh, collect_musical_grid_lines,
    };
    use rhythm_core::time::{
        BeatDivision, BpmMicros, GridOffsetNs, ProjectTimeNs, TempoMap, TimeSignature,
    };
    use rhythm_engine::waveform::{WavePeak, WaveformSlice};

    #[test]
    fn ruler_midpoint_maps_to_continuous_project_time() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1_000.0, 28.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(0),
            ProjectTimeNs::new(10_000_000_000),
        )
        .expect("transform");

        let mapped = transform.x_to_project_time(333.3).get();
        assert!((mapped - 3_333_000_000).abs() < 2_000);
    }

    #[test]
    fn timeline_transform_round_trips_project_time_and_x() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 64.0));
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
    fn four_four_grid_classifies_bars_beats_and_subdivisions() {
        let tempo = TempoMap::with_initial_tempo(
            GridOffsetNs::new(0),
            BpmMicros::new(120_000_000).expect("BPM"),
            TimeSignature::default(),
        );
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 100.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(0),
            ProjectTimeNs::new(2_000_000_000),
        )
        .expect("transform");
        let lines =
            collect_musical_grid_lines(&tempo, BeatDivision::new(4).expect("division"), transform);

        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == MusicalGridLineKind::Bar)
                .count(),
            2
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == MusicalGridLineKind::Beat)
                .count(),
            3
        );
        assert!(
            lines
                .iter()
                .filter(|line| line.kind == MusicalGridLineKind::Subdivision)
                .count()
                >= 12
        );
    }

    #[test]
    fn unset_tempo_has_no_musical_grid() {
        let tempo = TempoMap::unset(GridOffsetNs::new(0));
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 100.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(0),
            ProjectTimeNs::new(10_000_000_000),
        )
        .expect("transform");

        assert!(
            collect_musical_grid_lines(&tempo, BeatDivision::new(4).expect("division"), transform,)
                .is_empty()
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
        let transform =
            TimelineTransform::new(rect, ProjectTimeNs::new(0), ProjectTimeNs::new(128_000_000))
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
