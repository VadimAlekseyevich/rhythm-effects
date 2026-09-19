use rhythm_core::time::{BeatDivision, ProjectTimeNs};

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
}

#[cfg(test)]
mod tests {
    use super::{EditorSession, PreviewQuality};
    use rhythm_core::time::{BeatDivision, ProjectTimeNs};

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
