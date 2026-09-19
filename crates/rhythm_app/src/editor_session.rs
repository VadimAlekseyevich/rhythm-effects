use rhythm_core::time::{
    BeatDivision, DurationNs, MVP_BEAT_DIVISIONS, MusicalTick, PPQ, ProjectTimeNs, TempoMap,
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

#[derive(Debug)]
pub struct EditorSession {
    pub preview_quality: PreviewQuality,
    playhead: ProjectTimeNs,
    authoring_division: BeatDivision,
}

impl Default for EditorSession {
    fn default() -> Self {
        Self {
            preview_quality: PreviewQuality::Auto,
            playhead: ProjectTimeNs::new(0),
            authoring_division: BeatDivision::new(4).expect("1/4 beat is an accepted MVP grid"),
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

    pub fn change_authoring_division(&mut self, finer: bool) -> bool {
        let current = self.authoring_division.parts_per_beat();
        let Some(index) = MVP_BEAT_DIVISIONS
            .iter()
            .position(|division| *division == current)
        else {
            return false;
        };

        let next_index = if finer {
            index.checked_add(1).filter(|next| *next < MVP_BEAT_DIVISIONS.len())
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
