use std::sync::Arc;

use crate::audio::{AudioChannelLayout, DecodedAudio};
use rhythm_core::ids::AssetId;

pub const BASE_BUCKET_FRAMES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WavePeak {
    pub min: f32,
    pub max: f32,
}

impl WavePeak {
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformLevel {
    pub frames_per_peak: u64,
    pub peaks: Arc<[WavePeak]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformPyramid {
    pub source_sample_rate: u32,
    pub source_frame_count: u64,
    pub levels: Arc<[WaveformLevel]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformData {
    pub asset_id: AssetId,
    pub generation: u64,
    pub pyramid: Arc<WaveformPyramid>,
}

#[derive(Debug)]
pub struct WaveformBuildRequest {
    pub asset_id: AssetId,
    pub generation: u64,
    pub audio: DecodedAudio,
}

#[derive(Debug)]
pub struct WaveformBuildResult {
    pub asset_id: AssetId,
    pub generation: u64,
    pub waveform: Result<WaveformData, WaveformError>,
}

#[derive(Debug, Clone, Copy)]
pub struct WaveformSlice<'a> {
    pub level_index: usize,
    pub first_peak_index: usize,
    pub frames_per_peak: u64,
    pub peaks: &'a [WavePeak],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformError {
    InvalidPcmLength,
    NonFiniteSample,
}

pub fn build_waveform_pyramid(audio: &DecodedAudio) -> Result<WaveformPyramid, WaveformError> {
    let channels = usize::from(audio.channel_layout.channels());
    if channels == 0 || !audio.interleaved_f32.len().is_multiple_of(channels) {
        return Err(WaveformError::InvalidPcmLength);
    }
    if !audio.interleaved_f32.iter().all(|sample| sample.is_finite()) {
        return Err(WaveformError::NonFiniteSample);
    }

    let source_frame_count = audio.frame_count();
    let base = build_base_level(audio, channels);
    let mut levels = vec![WaveformLevel {
        frames_per_peak: BASE_BUCKET_FRAMES as u64,
        peaks: Arc::from(base),
    }];

    while levels
        .last()
        .is_some_and(|level| level.peaks.len() > 1)
    {
        let previous = levels.last().expect("level exists");
        let mut next = Vec::with_capacity(previous.peaks.len().div_ceil(2));

        for pair in previous.peaks.chunks(2) {
            let combined = match pair {
                [a, b] => a.combine(*b),
                [a] => *a,
                _ => unreachable!("chunks(2) produces one or two items"),
            };
            next.push(combined);
        }

        levels.push(WaveformLevel {
            frames_per_peak: previous.frames_per_peak.saturating_mul(2),
            peaks: Arc::from(next),
        });
    }

    Ok(WaveformPyramid {
        source_sample_rate: audio.sample_rate,
        source_frame_count: u64::try_from(source_frame_count).unwrap_or(u64::MAX),
        levels: Arc::from(levels),
    })
}

pub fn build_waveform_data(
    asset_id: AssetId,
    generation: u64,
    audio: &DecodedAudio,
) -> Result<WaveformData, WaveformError> {
    let pyramid = build_waveform_pyramid(audio)?;
    Ok(WaveformData {
        asset_id,
        generation,
        pyramid: Arc::new(pyramid),
    })
}

pub fn spawn_waveform_worker(
    request: WaveformBuildRequest,
) -> std::thread::JoinHandle<WaveformBuildResult> {
    std::thread::spawn(move || {
        let waveform = build_waveform_data(request.asset_id, request.generation, &request.audio);
        WaveformBuildResult {
            asset_id: request.asset_id,
            generation: request.generation,
            waveform,
        }
    })
}

impl WaveformData {
    #[must_use]
    pub fn visible_slice(
        &self,
        level_index: usize,
        start_frame: u64,
        end_frame: u64,
    ) -> Option<WaveformSlice<'_>> {
        let level = self.pyramid.levels.get(level_index)?;
        let peak_count = level.peaks.len();

        if peak_count == 0 || end_frame <= start_frame {
            return Some(WaveformSlice {
                level_index,
                first_peak_index: 0,
                frames_per_peak: level.frames_per_peak,
                peaks: &[],
            });
        }

        let first = usize::try_from(start_frame / level.frames_per_peak)
            .unwrap_or(usize::MAX)
            .min(peak_count);
        let end_peak = end_frame
            .saturating_add(level.frames_per_peak.saturating_sub(1))
            / level.frames_per_peak;
        let end = usize::try_from(end_peak)
            .unwrap_or(usize::MAX)
            .min(peak_count);

        Some(WaveformSlice {
            level_index,
            first_peak_index: first,
            frames_per_peak: level.frames_per_peak,
            peaks: &level.peaks[first..end],
        })
    }

    #[must_use]
    pub fn choose_level_for_view(
        &self,
        start_frame: u64,
        end_frame: u64,
        pixel_width: f32,
    ) -> usize {
        if self.pyramid.levels.len() <= 1 || pixel_width <= 1.0 || end_frame <= start_frame {
            return 0;
        }

        let visible_frames = end_frame.saturating_sub(start_frame) as f64;
        let frames_per_pixel = visible_frames / f64::from(pixel_width);

        let mut best_index = 0;
        let mut best_error = f64::INFINITY;

        for (index, level) in self.pyramid.levels.iter().enumerate() {
            let ratio = level.frames_per_peak as f64 / frames_per_pixel.max(f64::MIN_POSITIVE);
            let error = ratio.log2().abs();
            if error < best_error {
                best_error = error;
                best_index = index;
            }
        }

        best_index
    }
}

fn build_base_level(audio: &DecodedAudio, channels: usize) -> Vec<WavePeak> {
    let frame_count = audio.frame_count();
    if frame_count == 0 {
        return Vec::new();
    }

    let mut peaks = Vec::with_capacity(frame_count.div_ceil(BASE_BUCKET_FRAMES));

    for first_frame in (0..frame_count).step_by(BASE_BUCKET_FRAMES) {
        let last_frame = (first_frame + BASE_BUCKET_FRAMES).min(frame_count);
        let first_sample = first_frame * channels;
        let last_sample = last_frame * channels;
        let samples = &audio.interleaved_f32[first_sample..last_sample];

        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for &sample in samples {
            min = min.min(sample);
            max = max.max(sample);
        }

        peaks.push(WavePeak { min, max });
    }

    peaks
}

#[cfg(test)]
mod tests {
    use super::{BASE_BUCKET_FRAMES, WavePeak, build_waveform_pyramid};
    use rhythm_core::ids::AssetId;
    use crate::audio::{AudioChannelLayout, DecodedAudio};

    #[test]
    fn immutable_waveform_result_keeps_asset_generation_identity() {
        let asset_id = AssetId::new(7).expect("asset id");
        let audio = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: vec![0.0; 128],
        };

        let waveform = super::build_waveform_data(asset_id, 3, &audio).expect("waveform");

        assert_eq!(waveform.asset_id, asset_id);
        assert_eq!(waveform.generation, 3);
        assert_eq!(waveform.pyramid.levels[0].peaks.len(), 2);
    }

    #[test]
    fn background_worker_round_trips_request_identity() {
        let asset_id = AssetId::new(11).expect("asset id");
        let request = super::WaveformBuildRequest {
            asset_id,
            generation: 9,
            audio: DecodedAudio {
                sample_rate: 48_000,
                channel_layout: AudioChannelLayout::Mono,
                interleaved_f32: vec![0.0; 64],
            },
        };

        let result = super::spawn_waveform_worker(request)
            .join()
            .expect("waveform worker must not panic");

        assert_eq!(result.asset_id, asset_id);
        assert_eq!(result.generation, 9);
        assert!(result.waveform.is_ok());
    }

    #[test]
    fn visible_slice_returns_only_overlapping_peaks() {
        let asset_id = AssetId::new(13).expect("asset id");
        let audio = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: vec![0.0; BASE_BUCKET_FRAMES * 10],
        };
        let waveform = super::build_waveform_data(asset_id, 1, &audio).expect("waveform");

        let slice = waveform
            .visible_slice(0, 2 * BASE_BUCKET_FRAMES as u64, 5 * BASE_BUCKET_FRAMES as u64)
            .expect("level exists");

        assert_eq!(slice.first_peak_index, 2);
        assert_eq!(slice.frames_per_peak, BASE_BUCKET_FRAMES as u64);
        assert_eq!(slice.peaks.len(), 3);
    }

    #[test]
    fn mip_level_selection_tracks_frames_per_pixel() {
        let asset_id = AssetId::new(17).expect("asset id");
        let audio = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: vec![0.0; BASE_BUCKET_FRAMES * 64],
        };
        let waveform = super::build_waveform_data(asset_id, 1, &audio).expect("waveform");

        let zoomed_in = waveform.choose_level_for_view(0, 640, 640.0);
        let zoomed_out = waveform.choose_level_for_view(
            0,
            u64::try_from(audio.frame_count()).expect("frame count"),
            8.0,
        );

        assert_eq!(zoomed_in, 0);
        assert!(zoomed_out > zoomed_in);
    }

    #[test]
    fn base_level_aggregates_exactly_64_source_frames() {
        let mut samples = vec![0.0_f32; BASE_BUCKET_FRAMES * 2];
        samples[5] = -0.75;
        samples[42] = 0.9;

        let audio = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: samples,
        };

        let waveform = build_waveform_pyramid(&audio).expect("waveform");
        assert_eq!(waveform.levels[0].peaks.len(), 2);
        assert_eq!(
            waveform.levels[0].peaks[0],
            WavePeak {
                min: -0.75,
                max: 0.9,
            }
        );
        assert_eq!(
            waveform.levels[0].peaks[1],
            WavePeak { min: 0.0, max: 0.0 }
        );
    }

    #[test]
    fn stereo_channels_are_combined_into_one_envelope() {
        let audio = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Stereo,
            interleaved_f32: vec![
                -0.2, 0.8,
                -0.9, 0.1,
                0.3, 0.4,
            ],
        };

        let waveform = build_waveform_pyramid(&audio).expect("waveform");
        assert_eq!(waveform.levels[0].peaks.len(), 1);
        assert_eq!(
            waveform.levels[0].peaks[0],
            WavePeak {
                min: -0.9,
                max: 0.8,
            }
        );
    }

    #[test]
    fn mip_levels_reduce_pairs_and_keep_unpaired_tail() {
        let frames = BASE_BUCKET_FRAMES * 5;
        let mut samples = vec![0.0_f32; frames];

        for bucket in 0..5 {
            let index = bucket * BASE_BUCKET_FRAMES;
            samples[index] = -(bucket as f32);
            samples[index + 1] = bucket as f32 + 0.5;
        }

        let audio = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: samples,
        };
        let waveform = build_waveform_pyramid(&audio).expect("waveform");

        assert_eq!(waveform.levels[0].peaks.len(), 5);
        assert_eq!(waveform.levels[1].peaks.len(), 3);
        assert_eq!(waveform.levels[2].peaks.len(), 2);
        assert_eq!(waveform.levels[3].peaks.len(), 1);
        assert_eq!(
            waveform.levels[1].peaks[2],
            waveform.levels[0].peaks[4]
        );
    }

    #[test]
    fn final_partial_bucket_is_included() {
        let mut samples = vec![0.0_f32; BASE_BUCKET_FRAMES + 3];
        samples[BASE_BUCKET_FRAMES + 1] = -1.0;
        samples[BASE_BUCKET_FRAMES + 2] = 0.75;

        let audio = DecodedAudio {
            sample_rate: 44_100,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: samples,
        };
        let waveform = build_waveform_pyramid(&audio).expect("waveform");

        assert_eq!(waveform.levels[0].peaks.len(), 2);
        assert_eq!(
            waveform.levels[0].peaks[1],
            WavePeak {
                min: -1.0,
                max: 0.75,
            }
        );
    }

    #[test]
    fn empty_audio_is_safe() {
        let audio = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: Vec::new(),
        };
        let waveform = build_waveform_pyramid(&audio).expect("empty waveform");

        assert_eq!(waveform.source_frame_count, 0);
        assert_eq!(waveform.levels.len(), 1);
        assert!(waveform.levels[0].peaks.is_empty());
    }
}
