use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

use rhythm_core::time::{DurationNs, SampleRate};
use rubato::{Fft, FixedSync, Resampler, audioadapter_buffers::owned::InterleavedOwned};
use symphonia::core::{
    codecs::{AudioDecoderOptions, CodecParameters},
    errors::Error as SymphoniaError,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannelLayout {
    Mono,
    Stereo,
}

impl AudioChannelLayout {
    #[must_use]
    pub const fn channels(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProbe {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub channel_layout: AudioChannelLayout,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedAudio {
    pub sample_rate: u32,
    pub channel_layout: AudioChannelLayout,
    pub interleaved_f32: Vec<f32>,
}

impl DecodedAudio {
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.interleaved_f32.len() / usize::from(self.channel_layout.channels())
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackBuffer {
    sample_rate: SampleRate,
    interleaved_stereo_f32: Arc<[f32]>,
    duration: DurationNs,
}

impl PlaybackBuffer {
    #[must_use]
    pub const fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    #[must_use]
    pub const fn duration(&self) -> DurationNs {
        self.duration
    }

    #[must_use]
    pub fn interleaved_stereo_f32(&self) -> &[f32] {
        &self.interleaved_stereo_f32
    }

    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.interleaved_stereo_f32.len() / 2
    }

    #[must_use]
    pub fn memory_bytes(&self) -> usize {
        self.interleaved_stereo_f32.len() * std::mem::size_of::<f32>()
    }
}

#[derive(Debug)]
pub enum AudioDecodeError {
    Io(std::io::Error),
    Probe(String),
    Decode(String),
    Resample(String),
    InvalidSampleRate,
    InvalidPcmLength,
    NonFinitePcm,
    DurationOverflow,
    NoAudioTrack,
    MissingSampleRate,
    MissingChannelLayout,
    UnsupportedChannelCount(usize),
}

impl From<std::io::Error> for AudioDecodeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn probe_audio_file(path: &Path) -> Result<AudioProbe, AudioDecodeError> {
    let file = File::open(path)?;
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }

    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|error| AudioDecodeError::Probe(error.to_string()))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or(AudioDecodeError::NoAudioTrack)?;
    let codec_params = match track.codec_params.as_ref() {
        Some(CodecParameters::Audio(audio)) => audio,
        _ => return Err(AudioDecodeError::NoAudioTrack),
    };

    let sample_rate = codec_params
        .sample_rate
        .ok_or(AudioDecodeError::MissingSampleRate)?;
    let channel_count = codec_params
        .channels
        .as_ref()
        .ok_or(AudioDecodeError::MissingChannelLayout)?
        .count();
    let channel_layout = channel_layout_from_count(channel_count)?;

    Ok(AudioProbe {
        path: path.to_path_buf(),
        sample_rate,
        channel_layout,
    })
}

pub fn decode_audio_file(path: &Path) -> Result<DecodedAudio, AudioDecodeError> {
    let file = File::open(path)?;
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }

    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|error| AudioDecodeError::Probe(error.to_string()))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or(AudioDecodeError::NoAudioTrack)?;
    let track_id = track.id;
    let codec_params = match track.codec_params.as_ref() {
        Some(CodecParameters::Audio(audio)) => audio,
        _ => return Err(AudioDecodeError::NoAudioTrack),
    };
    let expected_sample_rate = codec_params
        .sample_rate
        .ok_or(AudioDecodeError::MissingSampleRate)?;
    let expected_channel_count = codec_params
        .channels
        .as_ref()
        .ok_or(AudioDecodeError::MissingChannelLayout)?
        .count();
    let expected_layout = channel_layout_from_count(expected_channel_count)?;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(codec_params, &AudioDecoderOptions::default())
        .map_err(|error| AudioDecodeError::Decode(error.to_string()))?;

    let mut interleaved_f32 = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(error) => return Err(AudioDecodeError::Decode(error.to_string())),
        };

        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(error) => return Err(AudioDecodeError::Decode(error.to_string())),
        };

        let spec = decoded.spec();
        if spec.rate() != expected_sample_rate {
            return Err(AudioDecodeError::Decode(
                "audio sample rate changed during decode".to_owned(),
            ));
        }

        let layout = channel_layout_from_count(spec.channels().count())?;
        if layout != expected_layout {
            return Err(AudioDecodeError::Decode(
                "audio channel layout changed during decode".to_owned(),
            ));
        }

        let mut packet_samples = Vec::with_capacity(decoded.samples_interleaved());
        decoded.copy_to_vec_interleaved::<f32>(&mut packet_samples);
        interleaved_f32.extend_from_slice(&packet_samples);
    }

    Ok(DecodedAudio {
        sample_rate: expected_sample_rate,
        channel_layout: expected_layout,
        interleaved_f32,
    })
}

pub fn spawn_output_resample(
    audio: DecodedAudio,
    output_sample_rate: u32,
) -> std::thread::JoinHandle<Result<DecodedAudio, AudioDecodeError>> {
    std::thread::spawn(move || resample_decoded_audio(audio, output_sample_rate))
}

fn resample_decoded_audio(
    audio: DecodedAudio,
    output_sample_rate: u32,
) -> Result<DecodedAudio, AudioDecodeError> {
    if audio.sample_rate == 0 || output_sample_rate == 0 {
        return Err(AudioDecodeError::InvalidSampleRate);
    }

    let channels = usize::from(audio.channel_layout.channels());
    if audio.interleaved_f32.len() % channels != 0 {
        return Err(AudioDecodeError::InvalidPcmLength);
    }
    if !audio
        .interleaved_f32
        .iter()
        .all(|sample| sample.is_finite())
    {
        return Err(AudioDecodeError::NonFinitePcm);
    }

    if audio.sample_rate == output_sample_rate {
        return Ok(audio);
    }

    let frame_count = audio.interleaved_f32.len() / channels;
    if frame_count == 0 {
        return Ok(DecodedAudio {
            sample_rate: output_sample_rate,
            channel_layout: audio.channel_layout,
            interleaved_f32: Vec::new(),
        });
    }

    let input = InterleavedOwned::new_from(audio.interleaved_f32, channels, frame_count)
        .map_err(|error| AudioDecodeError::Resample(error.to_string()))?;

    let mut resampler = Fft::<f32>::new(
        audio.sample_rate as usize,
        output_sample_rate as usize,
        1024,
        channels,
        FixedSync::Both,
    )
    .map_err(|error| AudioDecodeError::Resample(error.to_string()))?;

    let output = resampler
        .process_all(&input, frame_count, None)
        .map_err(|error| AudioDecodeError::Resample(error.to_string()))?;

    Ok(DecodedAudio {
        sample_rate: output_sample_rate,
        channel_layout: audio.channel_layout,
        interleaved_f32: output.take_data(),
    })
}

pub fn spawn_playback_buffer_prepare(
    audio: DecodedAudio,
    output_sample_rate: u32,
) -> std::thread::JoinHandle<Result<PlaybackBuffer, AudioDecodeError>> {
    std::thread::spawn(move || prepare_playback_buffer(audio, output_sample_rate))
}

/// Consumes temporary decoded source PCM and returns the only full PCM buffer
/// retained for steady-state playback. Callers should derive waveform peaks
/// before handing ownership of `audio` to this function.
pub fn prepare_playback_buffer(
    audio: DecodedAudio,
    output_sample_rate: u32,
) -> Result<PlaybackBuffer, AudioDecodeError> {
    let prepared = resample_decoded_audio(audio, output_sample_rate)?;
    let source_frames = prepared.frame_count();

    let interleaved_stereo_f32 = match prepared.channel_layout {
        AudioChannelLayout::Stereo => prepared.interleaved_f32,
        AudioChannelLayout::Mono => {
            let mut stereo = Vec::with_capacity(source_frames.saturating_mul(2));
            for sample in prepared.interleaved_f32 {
                stereo.push(sample);
                stereo.push(sample);
            }
            stereo
        }
    };

    let frame_count = interleaved_stereo_f32.len() / 2;
    let duration_ns = (frame_count as u128)
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_div(u128::from(output_sample_rate)))
        .ok_or(AudioDecodeError::DurationOverflow)?;
    let duration_ns = u64::try_from(duration_ns).map_err(|_| AudioDecodeError::DurationOverflow)?;

    Ok(PlaybackBuffer {
        sample_rate: SampleRate::new(output_sample_rate),
        interleaved_stereo_f32: Arc::from(interleaved_stereo_f32),
        duration: DurationNs::new(duration_ns),
    })
}

fn channel_layout_from_count(channel_count: usize) -> Result<AudioChannelLayout, AudioDecodeError> {
    match channel_count {
        1 => Ok(AudioChannelLayout::Mono),
        2 => Ok(AudioChannelLayout::Stereo),
        other => Err(AudioDecodeError::UnsupportedChannelCount(other)),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::{
        AudioChannelLayout, AudioDecodeError, DecodedAudio, decode_audio_file, probe_audio_file,
    };

    fn write_pcm16_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[i16]) {
        let data_len = u32::try_from(samples.len() * 2).expect("small fixture");
        let riff_size = 36_u32 + data_len;
        let block_align = channels * 2;
        let byte_rate = sample_rate * u32::from(block_align);

        let mut bytes = Vec::with_capacity(44 + data_len as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        fs::write(path, bytes).expect("write WAV fixture");
    }

    fn temp_fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rhythm-effects-{}-{name}", std::process::id()))
    }

    #[test]
    fn playback_buffer_duplicates_mono_to_stereo_and_consumes_source_pcm() {
        let decoded = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: vec![0.25, -0.5, 1.0],
        };

        let playback =
            super::prepare_playback_buffer(decoded, 48_000).expect("prepare playback buffer");

        assert_eq!(playback.sample_rate().get(), 48_000);
        assert_eq!(playback.frame_count(), 3);
        assert_eq!(
            playback.interleaved_stereo_f32(),
            &[0.25, 0.25, -0.5, -0.5, 1.0, 1.0]
        );
        assert_eq!(playback.duration().get(), 62_500);
        assert_eq!(playback.memory_bytes(), 6 * std::mem::size_of::<f32>());
    }

    #[test]
    fn playback_buffer_preserves_stereo_samples() {
        let decoded = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Stereo,
            interleaved_f32: vec![0.1, 0.2, 0.3, 0.4],
        };

        let playback =
            super::prepare_playback_buffer(decoded, 48_000).expect("prepare playback buffer");

        assert_eq!(playback.frame_count(), 2);
        assert_eq!(playback.interleaved_stereo_f32(), &[0.1, 0.2, 0.3, 0.4]);
        assert_eq!(playback.duration().get(), 41_666);
    }

    #[test]
    fn output_resample_runs_on_worker_and_changes_sample_rate() {
        let input_frames = 4_410_usize;
        let mut samples = Vec::with_capacity(input_frames);
        for frame in 0..input_frames {
            samples.push(((frame as f32) * 0.01).sin() * 0.5);
        }

        let audio = DecodedAudio {
            sample_rate: 44_100,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: samples,
        };

        let worker = super::spawn_output_resample(audio, 48_000);
        let output = worker
            .join()
            .expect("resample worker must not panic")
            .expect("resample succeeds");

        assert_eq!(output.sample_rate, 48_000);
        assert_eq!(output.channel_layout, AudioChannelLayout::Mono);
        assert_eq!(output.frame_count(), 4_800);
        assert!(
            output
                .interleaved_f32
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn multichannel_source_is_rejected_without_downmix() {
        let path = temp_fixture_path("three-channel.wav");
        write_pcm16_wav(&path, 8_000, 3, &[0, 0, 0, 1_000, 2_000, 3_000]);

        let probe_error = probe_audio_file(&path).expect_err("3-channel probe must fail");
        assert!(matches!(
            probe_error,
            AudioDecodeError::UnsupportedChannelCount(3)
        ));

        let decode_error = decode_audio_file(&path).expect_err("3-channel decode must fail");
        let _ = fs::remove_file(&path);
        assert!(matches!(
            decode_error,
            AudioDecodeError::UnsupportedChannelCount(3)
        ));
    }

    #[test]
    fn wav_fixture_decodes_to_mono_interleaved_f32() {
        let path = temp_fixture_path("decode-fixture.wav");
        write_pcm16_wav(&path, 8_000, 1, &[0, 16_384, -16_384, 32_767]);

        let decoded = decode_audio_file(&path).expect("decode generated WAV");
        let _ = fs::remove_file(&path);

        assert_eq!(decoded.sample_rate, 8_000);
        assert_eq!(decoded.channel_layout, AudioChannelLayout::Mono);
        assert_eq!(decoded.frame_count(), 4);
        assert!((decoded.interleaved_f32[0] - 0.0).abs() < 0.000_01);
        assert!((decoded.interleaved_f32[1] - 0.5).abs() < 0.000_1);
        assert!((decoded.interleaved_f32[2] + 0.5).abs() < 0.000_1);
        assert!(decoded.interleaved_f32[3] > 0.99);
    }

    #[test]
    fn decoded_audio_frame_count_uses_semantic_channel_layout() {
        let mono = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Mono,
            interleaved_f32: vec![0.0, 0.5, -0.5],
        };
        assert_eq!(mono.frame_count(), 3);

        let stereo = DecodedAudio {
            sample_rate: 48_000,
            channel_layout: AudioChannelLayout::Stereo,
            interleaved_f32: vec![0.0, 0.0, 0.5, -0.5],
        };
        assert_eq!(stereo.frame_count(), 2);
    }
}
