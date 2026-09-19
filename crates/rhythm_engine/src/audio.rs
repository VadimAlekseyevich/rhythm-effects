use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU32, AtomicU64, Ordering},
    },
};

use cpal::{
    FromSample, I24, SizedSample, U24,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rhythm_core::time::{DurationNs, ProjectTimeNs, SampleRate};
use rubato::{Fft, FixedSync, Resampler, audioadapter_buffers::owned::InterleavedOwned};
use symphonia::core::{
    codecs::{CodecParameters, audio::AudioDecoderOptions},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PlaybackGeneration(u64);

impl PlaybackGeneration {
    pub const INITIAL: Self = Self(0);

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackClockAnchor {
    pub playback_stream_instant: cpal::StreamInstant,
    pub project_time_of_first_frame: ProjectTimeNs,
    pub generation: PlaybackGeneration,
}

struct AtomicClockAnchor {
    sequence: AtomicU64,
    playback_stream_nanos: AtomicU64,
    project_time_ns: AtomicI64,
    generation: AtomicU64,
}

impl Default for AtomicClockAnchor {
    fn default() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            playback_stream_nanos: AtomicU64::new(0),
            project_time_ns: AtomicI64::new(0),
            generation: AtomicU64::new(PlaybackGeneration::INITIAL.get()),
        }
    }
}

impl AtomicClockAnchor {
    fn publish(&self, anchor: PlaybackClockAnchor) -> bool {
        let Ok(playback_stream_nanos) = u64::try_from(anchor.playback_stream_instant.as_nanos())
        else {
            return false;
        };

        self.sequence.fetch_add(1, Ordering::AcqRel);
        self.playback_stream_nanos
            .store(playback_stream_nanos, Ordering::Relaxed);
        self.project_time_ns
            .store(anchor.project_time_of_first_frame.get(), Ordering::Relaxed);
        self.generation
            .store(anchor.generation.get(), Ordering::Relaxed);
        self.sequence.fetch_add(1, Ordering::Release);
        true
    }

    fn read(&self) -> Option<PlaybackClockAnchor> {
        for _ in 0..8 {
            let before = self.sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                std::hint::spin_loop();
                continue;
            }

            let playback_stream_nanos = self.playback_stream_nanos.load(Ordering::Relaxed);
            let project_time_ns = self.project_time_ns.load(Ordering::Relaxed);
            let generation = PlaybackGeneration(self.generation.load(Ordering::Relaxed));

            let after = self.sequence.load(Ordering::Acquire);
            if before == after && after & 1 == 0 {
                if generation == PlaybackGeneration::INITIAL {
                    return None;
                }

                return Some(PlaybackClockAnchor {
                    playback_stream_instant: cpal::StreamInstant::from_nanos(playback_stream_nanos),
                    project_time_of_first_frame: ProjectTimeNs::new(project_time_ns),
                    generation,
                });
            }
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlaybackState {
    Unavailable = 0,
    Ready = 1,
    Playing = 2,
    Paused = 3,
    Ended = 4,
    Error = 5,
}

impl PlaybackState {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Ready,
            2 => Self::Playing,
            3 => Self::Paused,
            4 => Self::Ended,
            5 => Self::Error,
            _ => Self::Unavailable,
        }
    }
}

#[derive(Debug)]
pub enum AudioStreamControlError {
    Cpal(String),
    InvalidGain,
    InvalidState {
        expected: PlaybackState,
        actual: PlaybackState,
    },
}

#[derive(Debug)]
pub enum AudioStreamBuildError {
    Build(String),
    UnsupportedSampleFormat(cpal::SampleFormat),
}

struct PlaybackControl {
    frame_cursor: AtomicU64,
    ended: AtomicBool,
    state: AtomicU8,
    stream_error_count: AtomicU64,
    gain_bits: AtomicU32,
    next_generation: AtomicU64,
    active_generation: AtomicU64,
    pending_generation: AtomicU64,
    pending_seek_frame: AtomicU64,
    clock_anchor: AtomicClockAnchor,
}

impl Default for PlaybackControl {
    fn default() -> Self {
        Self {
            frame_cursor: AtomicU64::new(0),
            ended: AtomicBool::new(false),
            state: AtomicU8::new(PlaybackState::Ready as u8),
            stream_error_count: AtomicU64::new(0),
            gain_bits: AtomicU32::new(1.0_f32.to_bits()),
            next_generation: AtomicU64::new(1),
            active_generation: AtomicU64::new(PlaybackGeneration::INITIAL.get()),
            pending_generation: AtomicU64::new(PlaybackGeneration::INITIAL.get()),
            pending_seek_frame: AtomicU64::new(0),
            clock_anchor: AtomicClockAnchor::default(),
        }
    }
}

impl PlaybackControl {
    fn state(&self) -> PlaybackState {
        PlaybackState::from_u8(self.state.load(Ordering::Acquire))
    }

    fn set_state(&self, state: PlaybackState) {
        self.state.store(state as u8, Ordering::Release);
    }

    fn request_seek(&self, frame: u64) -> PlaybackGeneration {
        let generation = PlaybackGeneration(self.next_generation.fetch_add(1, Ordering::AcqRel));
        self.pending_seek_frame.store(frame, Ordering::Relaxed);
        self.pending_generation
            .store(generation.get(), Ordering::Release);
        generation
    }

    fn ensure_initial_generation(&self) -> PlaybackGeneration {
        let active = self.active_generation();
        let pending = self.pending_generation();

        if active != PlaybackGeneration::INITIAL {
            return active;
        }
        if pending != PlaybackGeneration::INITIAL {
            return pending;
        }

        self.request_seek(self.frame_cursor.load(Ordering::Acquire))
    }

    fn apply_pending_seek_at_callback_boundary(&self) -> Option<PlaybackGeneration> {
        let pending = self.pending_generation();
        if pending == PlaybackGeneration::INITIAL || pending == self.active_generation() {
            return None;
        }

        let frame = self.pending_seek_frame.load(Ordering::Relaxed);
        self.frame_cursor.store(frame, Ordering::Release);
        self.active_generation
            .store(pending.get(), Ordering::Release);
        self.ended.store(false, Ordering::Release);
        Some(pending)
    }

    fn pending_generation(&self) -> PlaybackGeneration {
        PlaybackGeneration(self.pending_generation.load(Ordering::Acquire))
    }

    fn active_generation(&self) -> PlaybackGeneration {
        PlaybackGeneration(self.active_generation.load(Ordering::Acquire))
    }

    fn accepts_generation(&self, generation: PlaybackGeneration) -> bool {
        generation != PlaybackGeneration::INITIAL && generation == self.active_generation()
    }

    fn handle_stream_error(&self) {
        self.stream_error_count.fetch_add(1, Ordering::Relaxed);
        self.active_generation
            .store(PlaybackGeneration::INITIAL.get(), Ordering::Release);
        self.pending_generation
            .store(PlaybackGeneration::INITIAL.get(), Ordering::Release);
        self.set_state(PlaybackState::Error);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioClockSource {
    Timestamp,
    FrameCursorFallback,
}

#[derive(Debug, Clone)]
pub struct AudioDiagnostics {
    pub device_label: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: cpal::SampleFormat,
    pub callback_buffer_frames: u64,
    pub playback_generation: PlaybackGeneration,
    pub audible_project_time: Option<ProjectTimeNs>,
    pub clock_source: AudioClockSource,
    pub stream_error_count: u64,
    pub playback_buffer_memory_bytes: usize,
    pub playback_state: PlaybackState,
}

pub struct CpalPlaybackStream {
    stream: cpal::Stream,
    control: Arc<PlaybackControl>,
    frame_count: u64,
    sample_rate: SampleRate,
    duration: DurationNs,
    fallback_latency_frames: u64,
}

impl CpalPlaybackStream {
    #[must_use]
    pub fn playback_state(&self) -> PlaybackState {
        self.control.state()
    }

    #[must_use]
    pub fn active_generation(&self) -> PlaybackGeneration {
        self.control.active_generation()
    }

    #[must_use]
    pub fn clock_anchor(&self) -> Option<PlaybackClockAnchor> {
        let anchor = self.control.clock_anchor.read()?;
        self.control
            .accepts_generation(anchor.generation)
            .then_some(anchor)
    }

    #[must_use]
    pub fn estimated_audible_project_time(&self) -> Option<ProjectTimeNs> {
        if let Some(anchor) = self.clock_anchor() {
            let now = self.stream.now();
            if let Some(project_time) =
                estimate_project_time_from_anchor(anchor, now, self.duration)
            {
                return Some(project_time);
            }
        }

        estimate_project_time_from_frame_cursor(
            self.frame_cursor(),
            self.fallback_latency_frames,
            self.sample_rate,
            self.duration,
        )
    }

    #[must_use]
    pub const fn fallback_latency_frames(&self) -> u64 {
        self.fallback_latency_frames
    }

    #[must_use]
    pub fn diagnostics(&self) -> AudioDiagnostics {
        let clock_source = if self.clock_anchor().is_some() {
            AudioClockSource::Timestamp
        } else {
            AudioClockSource::FrameCursorFallback
        };

        AudioDiagnostics {
            device_label: self.device_label.clone(),
            sample_rate: self.sample_rate.get(),
            channels: self.output_channels,
            sample_format: self.sample_format,
            callback_buffer_frames: self.fallback_latency_frames,
            playback_generation: self.active_generation(),
            audible_project_time: self.estimated_audible_project_time(),
            clock_source,
            stream_error_count: self.stream_error_count(),
            playback_buffer_memory_bytes: self.playback_buffer_memory_bytes,
            playback_state: self.playback_state(),
        }
    }

    #[must_use]
    pub fn accepts_clock_anchor_generation(&self, generation: PlaybackGeneration) -> bool {
        self.control.accepts_generation(generation)
    }

    pub fn request_playing_seek_frame(
        &self,
        frame: u64,
    ) -> Result<PlaybackGeneration, AudioStreamControlError> {
        let actual = self.playback_state();
        if actual != PlaybackState::Playing {
            return Err(AudioStreamControlError::InvalidState {
                expected: PlaybackState::Playing,
                actual,
            });
        }

        let frame = frame.min(self.frame_count);
        Ok(self.control.request_seek(frame))
    }

    pub fn play(&self) -> Result<(), AudioStreamControlError> {
        self.control.ensure_initial_generation();
        self.stream
            .play()
            .map_err(|error| AudioStreamControlError::Cpal(error.to_string()))?;
        self.control.ended.store(false, Ordering::Release);
        self.control.set_state(PlaybackState::Playing);
        Ok(())
    }

    pub fn pause(&self) -> Result<(), AudioStreamControlError> {
        self.stream
            .pause()
            .map_err(|error| AudioStreamControlError::Cpal(error.to_string()))?;
        self.control.set_state(PlaybackState::Paused);
        Ok(())
    }

    #[must_use]
    pub fn frame_cursor(&self) -> u64 {
        self.control.frame_cursor.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn ended(&self) -> bool {
        self.control.ended.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn stream_error_count(&self) -> u64 {
        self.control.stream_error_count.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn gain(&self) -> f32 {
        f32::from_bits(self.control.gain_bits.load(Ordering::Relaxed))
    }

    pub fn set_gain(&self, gain: f32) -> Result<(), AudioStreamControlError> {
        if !gain.is_finite() {
            return Err(AudioStreamControlError::InvalidGain);
        }

        self.control
            .gain_bits
            .store(gain.to_bits(), Ordering::Release);
        Ok(())
    }

    pub fn stream(&self) -> &cpal::Stream {
        &self.stream
    }
}

#[derive(Debug)]
pub enum AudioOutputInitError {
    NoDefaultOutputDevice,
    Device(String),
}

#[derive(Debug)]
pub struct CpalOutputEndpoint {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    device_label: String,
}

impl CpalOutputEndpoint {
    #[must_use]
    pub fn device_label(&self) -> &str {
        &self.device_label
    }

    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate()
    }

    #[must_use]
    pub fn channels(&self) -> u16 {
        self.config.channels()
    }

    #[must_use]
    pub fn sample_format(&self) -> cpal::SampleFormat {
        self.config.sample_format()
    }

    #[must_use]
    pub fn stream_config(&self) -> cpal::StreamConfig {
        self.config.config()
    }
}

pub fn build_playback_stream(
    endpoint: &CpalOutputEndpoint,
    buffer: PlaybackBuffer,
) -> Result<CpalPlaybackStream, AudioStreamBuildError> {
    let output_channels = usize::from(endpoint.channels());
    let config = endpoint.stream_config();
    let frame_count = u64::try_from(buffer.frame_count()).unwrap_or(u64::MAX);
    let sample_rate = buffer.sample_rate();
    let duration = buffer.duration();
    let playback_buffer_memory_bytes = buffer.memory_bytes();
    let device_label = endpoint.device_label().to_owned();
    let output_channels_u16 = endpoint.channels();
    let sample_format = endpoint.sample_format();
    let control = Arc::new(PlaybackControl::default());

    let stream = match endpoint.sample_format() {
        cpal::SampleFormat::I8 => {
            build_typed_playback_stream::<i8>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::I16 => {
            build_typed_playback_stream::<i16>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::I24 => {
            build_typed_playback_stream::<I24>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::I32 => {
            build_typed_playback_stream::<i32>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::I64 => {
            build_typed_playback_stream::<i64>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::U8 => {
            build_typed_playback_stream::<u8>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::U16 => {
            build_typed_playback_stream::<u16>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::U24 => {
            build_typed_playback_stream::<U24>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::U32 => {
            build_typed_playback_stream::<u32>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::U64 => {
            build_typed_playback_stream::<u64>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::F32 => {
            build_typed_playback_stream::<f32>(endpoint, config, buffer, output_channels, &control)
        }
        cpal::SampleFormat::F64 => {
            build_typed_playback_stream::<f64>(endpoint, config, buffer, output_channels, &control)
        }
        format => return Err(AudioStreamBuildError::UnsupportedSampleFormat(format)),
    }?;

    let fallback_latency_frames = stream.buffer_size().map(u64::from).unwrap_or(0);

    Ok(CpalPlaybackStream {
        stream,
        control,
        frame_count,
        sample_rate,
        duration,
        fallback_latency_frames,
        device_label,
        output_channels: output_channels_u16,
        sample_format,
        playback_buffer_memory_bytes,
    })
}

fn build_typed_playback_stream<T>(
    endpoint: &CpalOutputEndpoint,
    config: cpal::StreamConfig,
    buffer: PlaybackBuffer,
    output_channels: usize,
    control: &Arc<PlaybackControl>,
) -> Result<cpal::Stream, AudioStreamBuildError>
where
    T: SizedSample + FromSample<f32>,
{
    let callback_control = Arc::clone(control);
    let error_control = Arc::clone(control);

    endpoint
        .device
        .build_output_stream(
            config,
            move |output: &mut [T], info| {
                if callback_control.state() == PlaybackState::Error {
                    output.fill(T::from_sample(0.0_f32));
                    return;
                }

                callback_control.apply_pending_seek_at_callback_boundary();
                let mut cursor =
                    usize::try_from(callback_control.frame_cursor.load(Ordering::Acquire))
                        .unwrap_or(usize::MAX);
                let generation = callback_control.active_generation();

                if generation != PlaybackGeneration::INITIAL {
                    let first_frame = u64::try_from(cursor).unwrap_or(u64::MAX);
                    if let Some(project_time) =
                        project_time_for_frame(first_frame, buffer.sample_rate())
                    {
                        callback_control.clock_anchor.publish(PlaybackClockAnchor {
                            playback_stream_instant: info.timestamp().playback,
                            project_time_of_first_frame: project_time,
                            generation,
                        });
                    }
                }

                let gain =
                    f32::from_bits(callback_control.gain_bits.load(Ordering::Relaxed));
                write_playback_samples(&buffer, &mut cursor, output_channels, gain, output);

                callback_control
                    .frame_cursor
                    .store(cursor as u64, Ordering::Release);
                let ended = cursor >= buffer.frame_count();
                callback_control.ended.store(ended, Ordering::Release);
                if ended {
                    callback_control.set_state(PlaybackState::Ended);
                }
            },
            move |_| {
                error_control.handle_stream_error();
            },
            None,
        )
        .map_err(|error| AudioStreamBuildError::Build(error.to_string()))
}

fn project_time_for_frame(frame: u64, sample_rate: SampleRate) -> Option<ProjectTimeNs> {
    if sample_rate.get() == 0 {
        return None;
    }

    let nanos = (u128::from(frame))
        .checked_mul(1_000_000_000)?
        .checked_div(u128::from(sample_rate.get()))?;
    let nanos = i64::try_from(nanos).ok()?;
    Some(ProjectTimeNs::new(nanos))
}

fn estimate_project_time_from_anchor(
    anchor: PlaybackClockAnchor,
    now: cpal::StreamInstant,
    duration: DurationNs,
) -> Option<ProjectTimeNs> {
    let delta = now.saturating_duration_since(anchor.playback_stream_instant);
    let delta_ns = i128::try_from(delta.as_nanos()).ok()?;
    let base_ns = i128::from(anchor.project_time_of_first_frame.get());
    let estimated_ns = base_ns.checked_add(delta_ns)?;
    let duration_ns = i128::from(duration.get());
    let clamped_ns = estimated_ns.clamp(0, duration_ns);
    Some(ProjectTimeNs::new(i64::try_from(clamped_ns).ok()?))
}

fn estimate_project_time_from_frame_cursor(
    frame_cursor: u64,
    latency_frames: u64,
    sample_rate: SampleRate,
    duration: DurationNs,
) -> Option<ProjectTimeNs> {
    let audible_frame = frame_cursor.saturating_sub(latency_frames);
    let project_time = project_time_for_frame(audible_frame, sample_rate)?;
    let clamped = project_time
        .get()
        .clamp(0, i64::try_from(duration.get()).unwrap_or(i64::MAX));
    Some(ProjectTimeNs::new(clamped))
}

fn write_playback_samples<T>(
    buffer: &PlaybackBuffer,
    frame_cursor: &mut usize,
    output_channels: usize,
    gain: f32,
    output: &mut [T],
) where
    T: SizedSample + FromSample<f32>,
{
    if output_channels == 0 {
        return;
    }

    let silence = T::from_sample(0.0_f32);
    output.fill(silence);

    for output_frame in output.chunks_mut(output_channels) {
        if *frame_cursor >= buffer.frame_count() {
            break;
        }

        let source_index = *frame_cursor * 2;
        let left = buffer.interleaved_stereo_f32[source_index] * gain;
        let right = buffer.interleaved_stereo_f32[source_index + 1] * gain;

        match output_channels {
            1 => {
                output_frame[0] = T::from_sample((left + right) * 0.5);
            }
            _ => {
                output_frame[0] = T::from_sample(left);
                output_frame[1] = T::from_sample(right);
            }
        }

        *frame_cursor += 1;
    }
}

pub fn initialize_default_output() -> Result<CpalOutputEndpoint, AudioOutputInitError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(AudioOutputInitError::NoDefaultOutputDevice)?;
    let default_config = device
        .default_output_config()
        .map_err(|error| AudioOutputInitError::Device(error.to_string()))?;

    let supported = device.supported_output_configs().ok();
    let config = select_preferred_output_config(default_config, supported);
    let device_label = device.to_string();

    Ok(CpalOutputEndpoint {
        device,
        config,
        device_label,
    })
}

fn select_preferred_output_config(
    default_config: cpal::SupportedStreamConfig,
    supported: Option<impl Iterator<Item = cpal::SupportedStreamConfigRange>>,
) -> cpal::SupportedStreamConfig {
    if default_config.channels() == 2 {
        return default_config;
    }

    let Some(supported) = supported else {
        return default_config;
    };

    let preferred_range = supported
        .filter(|range| range.channels() == 2)
        .max_by(cpal::SupportedStreamConfigRange::cmp_default_heuristics);

    let Some(range) = preferred_range else {
        return default_config;
    };

    if range.contains_rate(default_config.sample_rate()) {
        return range.with_sample_rate(default_config.sample_rate());
    }

    range
        .try_with_standard_sample_rate()
        .unwrap_or_else(|| range.with_max_sample_rate())
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
    if !audio.interleaved_f32.len().is_multiple_of(channels) {
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
    fn clock_anchor_seqlock_round_trips_coherent_generation() {
        let clock = super::AtomicClockAnchor::default();
        let anchor = super::PlaybackClockAnchor {
            playback_stream_instant: cpal::StreamInstant::from_nanos(5_000_000),
            project_time_of_first_frame: ProjectTimeNs::new(1_000_000_000),
            generation: super::PlaybackGeneration(7),
        };

        assert!(clock.publish(anchor));
        assert_eq!(clock.read(), Some(anchor));
    }

    #[test]
    fn audible_time_uses_stream_clock_and_clamps_to_duration() {
        let anchor = super::PlaybackClockAnchor {
            playback_stream_instant: cpal::StreamInstant::from_nanos(2_000_000_000),
            project_time_of_first_frame: ProjectTimeNs::new(500_000_000),
            generation: super::PlaybackGeneration(1),
        };

        let before_playback = super::estimate_project_time_from_anchor(
            anchor,
            cpal::StreamInstant::from_nanos(1_900_000_000),
            rhythm_core::time::DurationNs::new(2_000_000_000),
        )
        .expect("time");
        assert_eq!(before_playback.get(), 500_000_000);

        let advanced = super::estimate_project_time_from_anchor(
            anchor,
            cpal::StreamInstant::from_nanos(2_250_000_000),
            rhythm_core::time::DurationNs::new(2_000_000_000),
        )
        .expect("time");
        assert_eq!(advanced.get(), 750_000_000);

        let clamped = super::estimate_project_time_from_anchor(
            anchor,
            cpal::StreamInstant::from_nanos(5_000_000_000),
            rhythm_core::time::DurationNs::new(2_000_000_000),
        )
        .expect("time");
        assert_eq!(clamped.get(), 2_000_000_000);
    }

    #[test]
    fn frame_cursor_fallback_subtracts_latency_without_drift() {
        let sample_rate = rhythm_core::time::SampleRate::new(48_000);
        let duration = rhythm_core::time::DurationNs::new(10_000_000_000);

        assert_eq!(
            super::estimate_project_time_from_frame_cursor(48_480, 480, sample_rate, duration,),
            Some(ProjectTimeNs::new(1_000_000_000))
        );
        assert_eq!(
            super::estimate_project_time_from_frame_cursor(240, 480, sample_rate, duration,),
            Some(ProjectTimeNs::new(0))
        );
    }

    #[test]
    fn frame_to_project_time_is_direct_and_non_accumulating() {
        assert_eq!(
            super::project_time_for_frame(48_000, rhythm_core::time::SampleRate::new(48_000),)
                .expect("time")
                .get(),
            1_000_000_000
        );
        assert_eq!(
            super::project_time_for_frame(44_100, rhythm_core::time::SampleRate::new(44_100),)
                .expect("time")
                .get(),
            1_000_000_000
        );
    }

    #[test]
    fn playing_seek_switches_generation_only_at_callback_boundary() {
        let control = super::PlaybackControl::default();

        let first = control.request_seek(120);
        assert_eq!(first.get(), 1);
        assert_eq!(
            control.active_generation(),
            super::PlaybackGeneration::INITIAL
        );
        assert!(!control.accepts_generation(first));

        assert_eq!(
            control.apply_pending_seek_at_callback_boundary(),
            Some(first)
        );
        assert_eq!(
            control
                .frame_cursor
                .load(std::sync::atomic::Ordering::Acquire),
            120
        );
        assert!(control.accepts_generation(first));

        let second = control.request_seek(480);
        assert_eq!(second.get(), 2);
        assert!(control.accepts_generation(first));
        assert!(!control.accepts_generation(second));

        assert_eq!(
            control.apply_pending_seek_at_callback_boundary(),
            Some(second)
        );
        assert_eq!(
            control
                .frame_cursor
                .load(std::sync::atomic::Ordering::Acquire),
            480
        );
        assert!(!control.accepts_generation(first));
        assert!(control.accepts_generation(second));
    }

    #[test]
    fn repeated_callback_boundary_without_new_seek_keeps_generation() {
        let control = super::PlaybackControl::default();
        let generation = control.request_seek(32);

        assert_eq!(
            control.apply_pending_seek_at_callback_boundary(),
            Some(generation)
        );
        assert_eq!(control.apply_pending_seek_at_callback_boundary(), None);
        assert_eq!(control.active_generation(), generation);
    }

    #[test]
    fn stream_error_preserves_cursor_and_invalidates_generation() {
        let control = super::PlaybackControl::default();
        control
            .frame_cursor
            .store(321, std::sync::atomic::Ordering::Release);
        let generation = control.request_seek(321);
        control.apply_pending_seek_at_callback_boundary();
        assert!(control.accepts_generation(generation));

        control.handle_stream_error();

        assert_eq!(control.state(), super::PlaybackState::Error);
        assert_eq!(
            control
                .frame_cursor
                .load(std::sync::atomic::Ordering::Acquire),
            321
        );
        assert_eq!(
            control.active_generation(),
            super::PlaybackGeneration::INITIAL
        );
        assert_eq!(control.stream_error_count.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn playback_control_state_transitions_are_explicit() {
        let control = super::PlaybackControl::default();
        assert_eq!(control.state(), super::PlaybackState::Ready);

        control.set_state(super::PlaybackState::Playing);
        assert_eq!(control.state(), super::PlaybackState::Playing);

        control.set_state(super::PlaybackState::Paused);
        assert_eq!(control.state(), super::PlaybackState::Paused);

        control.set_state(super::PlaybackState::Ended);
        assert_eq!(control.state(), super::PlaybackState::Ended);

        control.set_state(super::PlaybackState::Error);
        assert_eq!(control.state(), super::PlaybackState::Error);
    }

    #[test]
    fn callback_writer_applies_gain_without_allocating_state() {
        let playback = super::prepare_playback_buffer(
            DecodedAudio {
                sample_rate: 48_000,
                channel_layout: AudioChannelLayout::Stereo,
                interleaved_f32: vec![0.5, -0.5],
            },
            48_000,
        )
        .expect("playback");

        let mut cursor = 0;
        let mut output = [0.0_f32; 2];
        super::write_playback_samples(&playback, &mut cursor, 2, 0.25, &mut output);

        assert_eq!(output, [0.125, -0.125]);
    }

    #[test]
    fn gain_atomic_defaults_to_one_and_rejects_non_finite_values() {
        let control = super::PlaybackControl::default();
        assert_eq!(
            f32::from_bits(control.gain_bits.load(std::sync::atomic::Ordering::Relaxed)),
            1.0
        );

        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(!invalid.is_finite());
        }
    }

    #[test]
    fn callback_writer_copies_stereo_and_emits_silence_at_end() {
        let playback = super::prepare_playback_buffer(
            DecodedAudio {
                sample_rate: 48_000,
                channel_layout: AudioChannelLayout::Stereo,
                interleaved_f32: vec![0.25, -0.25, 0.5, -0.5],
            },
            48_000,
        )
        .expect("playback");

        let mut cursor = 0;
        let mut output = [9.0_f32; 6];
        super::write_playback_samples(&playback, &mut cursor, 2, 1.0, &mut output);

        assert_eq!(cursor, 2);
        assert_eq!(output, [0.25, -0.25, 0.5, -0.5, 0.0, 0.0]);
    }

    #[test]
    fn callback_writer_adapts_stereo_to_mono_without_allocation() {
        let playback = super::prepare_playback_buffer(
            DecodedAudio {
                sample_rate: 48_000,
                channel_layout: AudioChannelLayout::Stereo,
                interleaved_f32: vec![1.0, -1.0, 0.5, 0.25],
            },
            48_000,
        )
        .expect("playback");

        let mut cursor = 0;
        let mut output = [9.0_f32; 2];
        super::write_playback_samples(&playback, &mut cursor, 1, 1.0, &mut output);

        assert_eq!(cursor, 2);
        assert_eq!(output, [0.0, 0.375]);
    }

    #[test]
    fn callback_writer_uses_first_two_channels_and_silences_extras() {
        let playback = super::prepare_playback_buffer(
            DecodedAudio {
                sample_rate: 48_000,
                channel_layout: AudioChannelLayout::Stereo,
                interleaved_f32: vec![0.25, -0.25],
            },
            48_000,
        )
        .expect("playback");

        let mut cursor = 0;
        let mut output = [9.0_f32; 4];
        super::write_playback_samples(&playback, &mut cursor, 4, 1.0, &mut output);

        assert_eq!(cursor, 1);
        assert_eq!(output, [0.25, -0.25, 0.0, 0.0]);
    }

    #[test]
    fn preferred_output_config_selects_stereo_when_default_is_mono() {
        let default = cpal::SupportedStreamConfig::new(
            1,
            44_100,
            cpal::SupportedBufferSize::Range {
                min: 128,
                max: 1_024,
            },
            cpal::SampleFormat::F32,
        );
        let supported = vec![cpal::SupportedStreamConfigRange::new(
            2,
            44_100,
            48_000,
            cpal::SupportedBufferSize::Range {
                min: 128,
                max: 1_024,
            },
            cpal::SampleFormat::F32,
        )];

        let selected = super::select_preferred_output_config(default, Some(supported.into_iter()));

        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 44_100);
        assert_eq!(selected.sample_format(), cpal::SampleFormat::F32);
    }

    #[test]
    fn preferred_output_config_keeps_stereo_default() {
        let default = cpal::SupportedStreamConfig::new(
            2,
            48_000,
            cpal::SupportedBufferSize::Range {
                min: 128,
                max: 1_024,
            },
            cpal::SampleFormat::I16,
        );

        let selected = super::select_preferred_output_config(
            default,
            Some(std::iter::empty::<cpal::SupportedStreamConfigRange>()),
        );

        assert_eq!(selected, default);
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
