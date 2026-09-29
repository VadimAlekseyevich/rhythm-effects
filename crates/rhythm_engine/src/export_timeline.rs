//! Exact, non-accumulating export frame timestamps and default full range.
//! The start of frame N is computed directly from rational FPS in i128 math.

use rhythm_core::{
    time::{FrameRate, ProjectTimeNs, TimeConversionError, project_time_for_frame},
};

use crate::export_job::ExportJobSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTimelineError {
    EmptyComposition,
    FrameCountOverflow,
    FrameIndexOutOfRange,
    TimeOverflow,
}

/// The default MVP export is half-open [0, composition duration), never a
/// capture of the editor's current playhead or a mutable preview range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportFrameTimeline {
    output_fps: FrameRate,
    end: ProjectTimeNs,
    frame_count: u64,
}

impl ExportFrameTimeline {
    pub fn full_composition(
        snapshot: &ExportJobSnapshot,
        output_fps: FrameRate,
    ) -> Result<Self, ExportTimelineError> {
        let duration_ns = snapshot.project().settings.duration.get();
        if duration_ns == 0 {
            return Err(ExportTimelineError::EmptyComposition);
        }
        let end_ns = i64::try_from(duration_ns)
            .map_err(|_| ExportTimelineError::TimeOverflow)?;

        // Number of frame STARTS with exact rational timestamp < duration.
        // Ceil(duration * fps_numerator / (fps_denominator * 1e9)).
        // Multiplication fits in u128 for validated V1 duration/rate bounds.
        let numerator = u128::from(duration_ns) * u128::from(output_fps.numerator());
        let denominator = u128::from(output_fps.denominator()) * 1_000_000_000_u128;
        let frame_count = numerator.div_ceil(denominator);
        let frame_count =
            u64::try_from(frame_count).map_err(|_| ExportTimelineError::FrameCountOverflow)?;

        Ok(Self {
            output_fps,
            end: ProjectTimeNs::new(end_ns),
            frame_count,
        })
    }

    #[must_use]
    pub const fn start(&self) -> ProjectTimeNs {
        ProjectTimeNs::new(0)
    }

    #[must_use]
    pub const fn end_exclusive(&self) -> ProjectTimeNs {
        self.end
    }

    #[must_use]
    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }

    #[must_use]
    pub const fn output_fps(&self) -> FrameRate {
        self.output_fps
    }

    /// Uses the core wide-integer implementation's nearest-nanosecond,
    /// ties-away-from-zero rounding. Seeking N in any order yields exactly
    /// the same timestamp, without an accumulated frame-delta clock.
    pub fn frame_time(&self, frame_index: u64) -> Result<ProjectTimeNs, ExportTimelineError> {
        if frame_index >= self.frame_count {
            return Err(ExportTimelineError::FrameIndexOutOfRange);
        }
        project_time_for_frame(frame_index, self.output_fps).map_err(|error| match error {
            TimeConversionError::Overflow => ExportTimelineError::TimeOverflow,
            TimeConversionError::TempoUnavailable | TimeConversionError::NonFiniteTickPosition => {
                ExportTimelineError::TimeOverflow
            }
        })
    }

    pub fn evaluate_frame(
        &self,
        snapshot: &ExportJobSnapshot,
        frame_index: u64,
    ) -> Result<crate::scene_eval::EvaluatedScene, ExportFrameError> {
        let time = self.frame_time(frame_index).map_err(ExportFrameError::Timeline)?;
        snapshot
            .evaluate_at(time)
            .map_err(ExportFrameError::Scene)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportFrameError {
    Timeline(ExportTimelineError),
    Scene(crate::scene_eval::SceneEvaluationError),
}

#[cfg(test)]
mod tests {
    use super::{ExportFrameTimeline, ExportTimelineError};
    use crate::{
        export_job::ExportJobSnapshot,
        reference_scenes::five_effects_reference_scene,
    };
    use rhythm_core::{
        editor::ProjectEditor,
        time::{DurationNs, FrameRate, ProjectTimeNs},
    };

    fn captured(duration_ns: u64) -> ExportJobSnapshot {
        let mut scene = five_effects_reference_scene();
        scene.project.settings.duration = DurationNs::new(duration_ns);
        ExportJobSnapshot::capture(
            &ProjectEditor::new(scene.project).expect("valid project"),
        )
        .expect("snapshot")
    }

    #[test]
    fn standard_60fps_full_range_has_exact_frame_count_and_end_exclusive() {
        let snapshot = captured(10_000_000_000);
        let timeline = ExportFrameTimeline::full_composition(
            &snapshot,
            FrameRate::new(60, 1).expect("60 FPS"),
        )
        .expect("default range");
        assert_eq!(timeline.start(), ProjectTimeNs::new(0));
        assert_eq!(timeline.end_exclusive(), ProjectTimeNs::new(10_000_000_000));
        assert_eq!(timeline.frame_count(), 600);
        assert_eq!(timeline.frame_time(0), Ok(ProjectTimeNs::new(0)));
        assert_eq!(timeline.frame_time(1), Ok(ProjectTimeNs::new(16_666_667)));
        assert_eq!(timeline.frame_time(2), Ok(ProjectTimeNs::new(33_333_333)));
        assert_eq!(timeline.frame_time(3), Ok(ProjectTimeNs::new(50_000_000)));
        assert_eq!(
            timeline.frame_time(599),
            Ok(ProjectTimeNs::new(9_983_333_333))
        );
        assert_eq!(
            timeline.frame_time(600),
            Err(ExportTimelineError::FrameIndexOutOfRange)
        );
        assert_eq!(timeline.evaluate_frame(&snapshot, 0), timeline.evaluate_frame(&snapshot, 0));
    }

    #[test]
    fn fractional_fps_uses_direct_frame_index_without_accumulated_drift() {
        let snapshot = captured(10_000_000_000);
        let rate = FrameRate::new(30_000, 1_001).expect("rational FPS");
        let timeline = ExportFrameTimeline::full_composition(&snapshot, rate).expect("range");
        assert_eq!(timeline.frame_count(), 300);
        assert_eq!(timeline.frame_time(0), Ok(ProjectTimeNs::new(0)));
        assert_eq!(timeline.frame_time(1), Ok(ProjectTimeNs::new(33_366_667)));
        assert_eq!(timeline.frame_time(2), Ok(ProjectTimeNs::new(66_733_333)));
        assert_eq!(timeline.frame_time(299), Ok(ProjectTimeNs::new(9_976_633_333)));
        assert_eq!(
            timeline.frame_time(300),
            Err(ExportTimelineError::FrameIndexOutOfRange)
        );

        let first = timeline.frame_time(200).expect("direct seek");
        assert_eq!(timeline.frame_time(1), Ok(ProjectTimeNs::new(33_366_667)));
        assert_eq!(timeline.frame_time(200), Ok(first));
        assert_eq!(
            timeline.frame_time(200),
            rhythm_core::time::project_time_for_frame(200, rate)
                .map_err(|_| ExportTimelineError::TimeOverflow)
        );
    }

    #[test]
    fn short_non_frame_aligned_duration_rounds_count_up_without_extra_end_frame() {
        let snapshot = captured(1_000_000_001);
        let timeline = ExportFrameTimeline::full_composition(
            &snapshot,
            FrameRate::new(30, 1).expect("30 FPS"),
        )
        .expect("range");
        assert_eq!(timeline.frame_count(), 31);
        assert_eq!(timeline.frame_time(30), Ok(ProjectTimeNs::new(1_000_000_000)));
        assert_eq!(
            timeline.frame_time(31),
            Err(ExportTimelineError::FrameIndexOutOfRange)
        );
    }

    #[test]
    fn changing_export_fps_does_not_modify_composition_or_snapshot() {
        let snapshot = captured(10_000_000_000);
        let original = snapshot.project().clone();
        let first = ExportFrameTimeline::full_composition(
            &snapshot,
            FrameRate::new(60, 1).expect("60"),
        )
        .expect("60 range");
        let second = ExportFrameTimeline::full_composition(
            &snapshot,
            FrameRate::new(25, 1).expect("25"),
        )
        .expect("25 range");
        assert_eq!(first.frame_count(), 600);
        assert_eq!(second.frame_count(), 250);
        assert_eq!(snapshot.project(), &original);
        assert_eq!(snapshot.project().settings.frame_rate, FrameRate::new(60, 1).expect("60"));
    }
}
