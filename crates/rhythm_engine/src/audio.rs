use std::{
    fs::File,
    path::{Path, PathBuf},
};

use symphonia::core::{
    codecs::CodecParameters,
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
    let channel_layout = match channel_count {
        1 => AudioChannelLayout::Mono,
        2 => AudioChannelLayout::Stereo,
        other => return Err(AudioDecodeError::UnsupportedChannelCount(other)),
    };

    Ok(AudioProbe {
        path: path.to_path_buf(),
        sample_rate,
        channel_layout,
    })
}

#[cfg(test)]
mod tests {
    use super::{AudioChannelLayout, DecodedAudio};

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
