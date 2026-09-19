use std::{
    fs::File,
    path::{Path, PathBuf},
};

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

    use super::{AudioChannelLayout, DecodedAudio, decode_audio_file};

    fn write_pcm16_mono_wav(path: &Path, sample_rate: u32, samples: &[i16]) {
        let data_len = u32::try_from(samples.len() * 2).expect("small fixture");
        let riff_size = 36_u32 + data_len;
        let byte_rate = sample_rate * 2;

        let mut bytes = Vec::with_capacity(44 + data_len as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        fs::write(path, bytes).expect("write WAV fixture");
    }

    fn temp_fixture_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rhythm-effects-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn wav_fixture_decodes_to_mono_interleaved_f32() {
        let path = temp_fixture_path("decode-fixture.wav");
        write_pcm16_mono_wav(&path, 8_000, &[0, 16_384, -16_384, 32_767]);

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
