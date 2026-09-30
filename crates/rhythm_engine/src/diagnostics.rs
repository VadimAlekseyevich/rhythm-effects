//! Low-overhead diagnostics primitives used by the debug overlay and
//! performance acceptance runs. All values are observational only.

use std::{
    collections::VecDeque,
    mem::{size_of, size_of_val},
    time::Duration,
};

use rhythm_core::{
    animation::{Animated, Keyframe},
    project::{EffectKind, ObjectContent, Project},
};

use crate::waveform::{WavePeak, WaveformData, WaveformLevel};

pub const FRAME_TIMING_WINDOW: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimingSummary {
    pub latest_ms: f32,
    pub p50_ms: f32,
    pub p95_ms: f32,
    pub samples: usize,
}

impl Default for TimingSummary {
    fn default() -> Self {
        Self {
            latest_ms: 0.0,
            p50_ms: 0.0,
            p95_ms: 0.0,
            samples: 0,
        }
    }
}

/// Bounded rolling frame window. Sorting happens only when the overlay asks
/// for a summary; recording a frame is O(1) and never grows after capacity.
#[derive(Debug, Clone)]
pub struct TimingWindow {
    samples_ms: VecDeque<f32>,
    capacity: usize,
}

impl Default for TimingWindow {
    fn default() -> Self {
        Self::new(FRAME_TIMING_WINDOW)
    }
}

impl TimingWindow {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "timing window must retain at least one sample");
        Self {
            samples_ms: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.record_ms(duration.as_secs_f32() * 1000.0);
    }

    pub fn record_ms(&mut self, value: f32) {
        if !value.is_finite() || value < 0.0 {
            return;
        }
        if self.samples_ms.len() == self.capacity {
            self.samples_ms.pop_front();
        }
        self.samples_ms.push_back(value);
    }

    #[must_use]
    pub fn summary(&self) -> TimingSummary {
        let Some(&latest_ms) = self.samples_ms.back() else {
            return TimingSummary::default();
        };
        let mut ordered: Vec<_> = self.samples_ms.iter().copied().collect();
        ordered.sort_by(f32::total_cmp);
        TimingSummary {
            latest_ms,
            p50_ms: percentile_nearest_rank(&ordered, 50),
            p95_ms: percentile_nearest_rank(&ordered, 95),
            samples: ordered.len(),
        }
    }
}

fn percentile_nearest_rank(sorted: &[f32], percentile: usize) -> f32 {
    debug_assert!(!sorted.is_empty());
    debug_assert!((1..=100).contains(&percentile));
    let rank = (sorted.len() * percentile).div_ceil(100);
    sorted[rank.saturating_sub(1)]
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemoryBucketEstimates {
    pub project_semantic_bytes: u64,
    pub waveform_bytes: u64,
    pub renderer_working_gpu_bytes: u64,
}

fn animated_heap_bytes<T>(animated: &Animated<T>) -> usize {
    animated.keyframes().len() * size_of::<Keyframe<T>>()
}

fn transform_key_bytes(object: &rhythm_core::project::Object) -> usize {
    animated_heap_bytes(&object.transform.position)
        + animated_heap_bytes(&object.transform.scale)
        + animated_heap_bytes(&object.transform.rotation_degrees)
        + animated_heap_bytes(&object.transform.anchor)
        + animated_heap_bytes(&object.transform.opacity)
}

fn content_heap_bytes(content: &ObjectContent) -> usize {
    match content {
        ObjectContent::Rectangle(value) => {
            animated_heap_bytes(&value.size)
                + animated_heap_bytes(&value.fill)
                + animated_heap_bytes(&value.corner_radius)
        }
        ObjectContent::Ellipse(value) => {
            animated_heap_bytes(&value.size) + animated_heap_bytes(&value.fill)
        }
        ObjectContent::Image(_) => 0,
        ObjectContent::Text(value) => {
            value.text.capacity()
                + value.font.family.capacity()
                + animated_heap_bytes(&value.color)
        }
    }
}

fn effect_heap_bytes(kind: &EffectKind) -> usize {
    match kind {
        EffectKind::Blur(value) => animated_heap_bytes(&value.radius_px),
        EffectKind::Glow(value) => {
            animated_heap_bytes(&value.radius_px)
                + animated_heap_bytes(&value.intensity)
                + animated_heap_bytes(&value.threshold)
                + animated_heap_bytes(&value.color)
        }
        EffectKind::Tint(value) => {
            animated_heap_bytes(&value.color) + animated_heap_bytes(&value.amount)
        }
        EffectKind::Noise(value) => {
            animated_heap_bytes(&value.amount)
                + animated_heap_bytes(&value.size_px)
                + animated_heap_bytes(&value.evolution)
        }
        EffectKind::RgbSplit(value) => {
            animated_heap_bytes(&value.amount_px) + animated_heap_bytes(&value.angle_degrees)
        }
    }
}

/// Approximate owned semantic memory, intentionally excluding allocator
/// metadata/capacity slack. It is sampled periodically rather than per-frame.
#[must_use]
pub fn estimate_project_semantic_bytes(project: &Project) -> u64 {
    let mut bytes = size_of_val(project)
        + project.metadata.name.capacity()
        + project.assets.len() * size_of::<rhythm_core::project::AssetRecord>()
        + project.composition.objects.len() * size_of::<rhythm_core::project::Object>();

    for asset in &project.assets {
        let rhythm_core::project::AssetSource::File { path, .. } = &asset.source;
        bytes += path.capacity();
    }
    for object in &project.composition.objects {
        bytes += object.name.capacity();
        bytes += object.effects.len() * size_of::<rhythm_core::project::Effect>();
        bytes += transform_key_bytes(object);
        bytes += content_heap_bytes(&object.content);
        for effect in &object.effects {
            bytes += effect_heap_bytes(&effect.kind);
        }
    }
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

#[must_use]
pub fn estimate_waveform_bytes(waveform: Option<&WaveformData>) -> u64 {
    let Some(waveform) = waveform else {
        return 0;
    };
    let levels = &waveform.pyramid.levels;
    let bytes = size_of::<WaveformData>()
        + size_of_val(waveform.pyramid.as_ref())
        + levels.len() * size_of::<WaveformLevel>()
        + levels
            .iter()
            .map(|level| level.peaks.len() * size_of::<WavePeak>())
            .sum::<usize>();
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        TimingWindow, estimate_project_semantic_bytes, estimate_waveform_bytes,
    };
    use crate::benchmark_fixtures::{
        empty_benchmark_fixture, medium_motion_benchmark_fixture,
    };

    #[test]
    fn rolling_window_is_bounded_and_uses_nearest_rank_percentiles() {
        let mut window = TimingWindow::new(4);
        for value in [1.0, 2.0, 20.0, 4.0] {
            window.record_ms(value);
        }
        let summary = window.summary();
        assert_eq!(summary.latest_ms, 4.0);
        assert_eq!(summary.p50_ms, 2.0);
        assert_eq!(summary.p95_ms, 20.0);
        assert_eq!(summary.samples, 4);

        window.record_ms(3.0);
        let summary = window.summary();
        assert_eq!(summary.samples, 4);
        assert_eq!(summary.latest_ms, 3.0);
        assert_eq!(summary.p50_ms, 3.0);
        assert_eq!(summary.p95_ms, 20.0);
    }

    #[test]
    fn invalid_timing_sample_does_not_pollute_window() {
        let mut window = TimingWindow::new(3);
        window.record_ms(5.0);
        window.record_ms(f32::NAN);
        window.record_ms(-1.0);
        assert_eq!(window.summary().samples, 1);
    }

    #[test]
    fn semantic_memory_estimate_tracks_fixture_complexity() {
        let empty = empty_benchmark_fixture();
        let medium = medium_motion_benchmark_fixture();
        assert!(
            estimate_project_semantic_bytes(&medium.project)
                > estimate_project_semantic_bytes(&empty.project)
        );
        assert_eq!(estimate_waveform_bytes(None), 0);
    }
}
