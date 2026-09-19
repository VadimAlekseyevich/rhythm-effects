use std::collections::HashSet;

use rhythm_core::{
    ids::KeyframeId,
    time::{
        BeatDivision, DurationNs, MVP_BEAT_DIVISIONS, MusicalTick, PPQ, ProjectTimeNs, TempoMap,
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineBoxSelection {
    start: [f32; 2],
    current: [f32; 2],
    ctrl_toggle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimelineView {
    start: ProjectTimeNs,
    end: ProjectTimeNs,
}

#[derive(Debug)]
pub struct EditorSession {
    pub preview_quality: PreviewQuality,
    playhead: ProjectTimeNs,
    authoring_division: BeatDivision,
    timeline_view: Option<TimelineView>,
    selected_keyframes: HashSet<KeyframeId>,
    timeline_box_selection: Option<TimelineBoxSelection>,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self {
            preview_quality: PreviewQuality::Auto,
            playhead: ProjectTimeNs::new(0),
            authoring_division: BeatDivision::new(4).expect("1/4 beat is an accepted MVP grid"),
            timeline_view: None,
            selected_keyframes: HashSet::new(),
            timeline_box_selection: None,
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
    pub fn is_keyframe_selected(&self, keyframe_id: KeyframeId) -> bool {
        self.selected_keyframes.contains(&keyframe_id)
    }

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
        true
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
    use super::{EditorSession, PreviewQuality};
    use rhythm_core::time::{
        BeatDivision, BpmMicros, DurationNs, GridOffsetNs, ProjectTimeNs, TempoMap, TimeSignature,
    };

    fn tempo_120() -> TempoMap {
        TempoMap::with_initial_tempo(
            GridOffsetNs::new(0),
            BpmMicros::new(120_000_000).expect("BPM"),
            TimeSignature::default(),
        )
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
