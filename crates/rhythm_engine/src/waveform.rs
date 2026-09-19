use crate::audio::{AudioChannelLayout, DecodedAudio};

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
    pub peaks: Vec<WavePeak>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformPyramid {
    pub source_sample_rate: u32,
    pub source_frame_count: u64,
    pub levels: Vec<WaveformLevel>,
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
        peaks: base,
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
            peaks: next,
        });
    }

    Ok(WaveformPyramid {
        source_sample_rate: audio.sample_rate,
        source_frame_count: u64::try_from(source_frame_count).unwrap_or(u64::MAX),
        levels,
    })
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
    use crate::audio::{AudioChannelLayout, DecodedAudio};

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
