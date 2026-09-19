use std::{
    fs::File,
    path::{Path, PathBuf},
};

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

#[derive(Debug)]
pub enum AudioDecodeError {
    Io(std::io::Error),
    Probe(String),
    Decode(String),
    Resample(String),
    InvalidSampleRate,
    InvalidPcmLength,
    NonFinitePcm,
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
