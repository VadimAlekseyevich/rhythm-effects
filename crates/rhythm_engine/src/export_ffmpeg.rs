//! Structured FFmpeg H.264 export process. No shell, command-string
//! interpolation, screen capture or realtime audio playback recording.

use std::{
    io::{self, Read, Write},
    path::{Component, PathBuf},
    process::{Child, ChildStderr, ChildStdin, Command, ExitStatus, Stdio},
    thread::{self, JoinHandle},
};

use rhythm_core::time::FrameRate;

use crate::{export_progress::ExportCancellationToken, export_renderer::ExportRendererPlan};

pub const MAX_FFMPEG_DIAGNOSTIC_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegFailureStage {
    EncoderUnavailable,
    PermissionDenied,
    DiskFull,
    InvalidAudioSource,
    ProcessFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegDiagnostic {
    pub stage: FfmpegFailureStage,
    pub stderr_tail: String,
}

fn classify_diagnostic(bytes: &[u8]) -> FfmpegDiagnostic {
    let stderr_tail = String::from_utf8_lossy(bytes).into_owned();
    let lower = stderr_tail.to_ascii_lowercase();
    let stage = if lower.contains("no space left on device") || lower.contains("disk full") {
        FfmpegFailureStage::DiskFull
    } else if lower.contains("unknown encoder")
        || lower.contains("encoder not found")
        || lower.contains("unknown encoder 'libx264'")
    {
        FfmpegFailureStage::EncoderUnavailable
    } else if lower.contains("permission denied") || lower.contains("access is denied") {
        FfmpegFailureStage::PermissionDenied
    } else if lower.contains("invalid data found when processing input")
        || lower.contains("could not find codec parameters")
    {
        FfmpegFailureStage::InvalidAudioSource
    } else {
        FfmpegFailureStage::ProcessFailed
    };
    FfmpegDiagnostic { stage, stderr_tail }
}

/// Always drain the entire child pipe to prevent encoder deadlock. Retain only
/// the latest 64 KiB even if FFmpeg logs gigabytes during a long export.
fn drain_stderr(mut stderr: impl Read) -> io::Result<Vec<u8>> {
    let mut tail = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let count = stderr.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        let remove = (tail.len() + count).saturating_sub(MAX_FFMPEG_DIAGNOSTIC_BYTES);
        tail.drain(..remove);
        tail.extend_from_slice(&chunk[..count]);
    }
    Ok(tail)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportQuality {
    Fast,
    Balanced,
    High,
}

impl ExportQuality {
    /// Encoder implementation selection belongs to release packaging.
    /// These concrete libx264 controls are tested independently of CLI UI.
    #[must_use]
    pub const fn libx264_options(self) -> (&'static str, &'static str) {
        match self {
            Self::Fast => ("veryfast", "23"),
            Self::Balanced => ("medium", "20"),
            Self::High => ("slow", "17"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfmpegConfigurationError {
    MissingAbsoluteExecutable,
    MissingAbsoluteStagingPath,
    UnsafeStagingPath,
    InvalidDimensions,
    InvalidFrameCount,
    FrameSizeOverflow,
    AudioDestinationConflict,
    InvalidAudioPath,
    DurationOverflow,
}

#[derive(Debug, Clone)]
pub struct FfmpegExportOptions {
    /// Resolved/bundled FFmpeg executable, not a shell command line.
    pub executable: PathBuf,
    /// Caller-owned unique .partial.mp4 path, distinct from the final file.
    pub staging_destination: PathBuf,
    pub output_size: [u32; 2],
    pub frame_rate: FrameRate,
    pub frame_count: u64,
    pub duration_ns: u64,
    /// The ORIGINAL primary audio asset: never captured playback output.
    pub primary_audio: Option<PathBuf>,
    pub quality: ExportQuality,
}

impl FfmpegExportOptions {
    /// Build options from an immutable export plan, not the live editor.
    #[must_use]
    pub fn from_plan(
        plan: &ExportRendererPlan,
        executable: PathBuf,
        staging_destination: PathBuf,
        quality: ExportQuality,
    ) -> Self {
        Self {
            executable,
            staging_destination,
            output_size: plan.resolution().output_size(),
            frame_rate: plan.timeline().output_fps(),
            frame_count: plan.timeline().frame_count(),
            duration_ns: plan.snapshot().project().settings.duration.get(),
            primary_audio: plan
                .resources()
                .audio
                .as_ref()
                .map(|probe| probe.path.clone()),
            quality,
        }
    }

    pub fn validate(&self) -> Result<usize, FfmpegConfigurationError> {
        if !self.executable.is_absolute() {
            return Err(FfmpegConfigurationError::MissingAbsoluteExecutable);
        }
        if !self.staging_destination.is_absolute() {
            return Err(FfmpegConfigurationError::MissingAbsoluteStagingPath);
        }
        if self
            .staging_destination
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        {
            return Err(FfmpegConfigurationError::UnsafeStagingPath);
        }
        let [width, height] = self.output_size;
        if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
            return Err(FfmpegConfigurationError::InvalidDimensions);
        }
        if self.frame_count == 0 {
            return Err(FfmpegConfigurationError::InvalidFrameCount);
        }
        if self.duration_ns == 0 {
            return Err(FfmpegConfigurationError::DurationOverflow);
        }
        if let Some(path) = &self.primary_audio {
            if !path.is_absolute() {
                return Err(FfmpegConfigurationError::InvalidAudioPath);
            }
            if *path == self.staging_destination {
                return Err(FfmpegConfigurationError::AudioDestinationConflict);
            }
        }
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(FfmpegConfigurationError::FrameSizeOverflow)?;
        usize::try_from(bytes).map_err(|_| FfmpegConfigurationError::FrameSizeOverflow)
    }

    fn duration_seconds(&self) -> Result<String, FfmpegConfigurationError> {
        if self.duration_ns == 0 {
            return Err(FfmpegConfigurationError::DurationOverflow);
        }
        let secs = self.duration_ns / 1_000_000_000;
        let nanos = self.duration_ns % 1_000_000_000;
        Ok(format!("{secs}.{nanos:09}"))
    }

    /// Single lexical argument per option/path: quotes, Unicode, ampersands
    /// and spaces are passed through unchanged, never interpreted by cmd.exe.
    pub fn command(&self) -> Result<Command, FfmpegConfigurationError> {
        self.validate()?;
        let mut command = Command::new(&self.executable);
        command
            .arg("-hide_banner")
            .arg("-nostdin")
            .arg("-n")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("-video_size")
            .arg(format!("{}x{}", self.output_size[0], self.output_size[1]))
            .arg("-framerate")
            .arg(format!(
                "{}/{}",
                self.frame_rate.numerator(),
                self.frame_rate.denominator()
            ))
            .arg("-i")
            .arg("pipe:0");

        if let Some(path) = &self.primary_audio {
            // FFmpeg decodes original audio at t=0. Trim only audio, with
            // no -shortest: a video tail after a shorter audio file remains.
            command.arg("-i").arg(path);
        }

        command
            .arg("-map")
            .arg("0:v:0")
            .arg("-frames:v")
            .arg(self.frame_count.to_string())
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg(self.quality.libx264_options().0)
            .arg("-crf")
            .arg(self.quality.libx264_options().1)
            .arg("-pix_fmt")
            .arg("yuv420p");

        if self.primary_audio.is_some() {
            command
                .arg("-map")
                .arg("1:a:0")
                .arg("-c:a")
                .arg("aac")
                .arg("-b:a")
                .arg("192k")
                .arg("-af")
                .arg(format!(
                    "atrim=duration={},asetpts=PTS-STARTPTS",
                    self.duration_seconds()?
                ));
        } else {
            command.arg("-an");
        }

        command
            .arg("-movflags")
            .arg("+faststart")
            .arg("-f")
            .arg("mp4")
            .arg(&self.staging_destination);
        Ok(command)
    }

    /// Starts a direct child process. A dedicated reader immediately drains
    /// piped stderr for its entire lifetime while keeping only 64 KiB; FFmpeg
    /// cannot block indefinitely because its diagnostics pipe is full.
    pub fn spawn(&self) -> Result<FfmpegRawVideoProcess, FfmpegProcessError> {
        let frame_size = self.validate().map_err(FfmpegProcessError::Configuration)?;
        let mut command = self.command().map_err(FfmpegProcessError::Configuration)?;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(FfmpegProcessError::Spawn)?;
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FfmpegProcessError::MissingStdin);
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FfmpegProcessError::MissingStderr);
            }
        };
        let stderr_reader = match thread::Builder::new()
            .name("rhythm-ffmpeg-diagnostics".into())
            .spawn(move || drain_stderr(stderr))
        {
            Ok(reader) => reader,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(FfmpegProcessError::StderrThread(error));
            }
        };
        Ok(FfmpegRawVideoProcess {
            child: Some(child),
            stdin: Some(stdin),
            stderr_reader: Some(stderr_reader),
            frame_bytes: frame_size,
            next_frame: 0,
            expected_frames: self.frame_count,
        })
    }
}

#[derive(Debug)]
pub enum FfmpegProcessError {
    Configuration(FfmpegConfigurationError),
    Spawn(io::Error),
    MissingStdin,
    MissingStderr,
    StderrThread(io::Error),
    StderrRead(io::Error),
    StderrJoin,
    Cancelled,
    OutOfOrderFrame { expected: u64, actual: u64 },
    FrameSizeMismatch { expected: usize, actual: usize },
    TooManyFrames,
    MissingFrames { expected: u64, actual: u64 },
    Write(io::Error),
    Wait(io::Error),
    EncoderFailed {
        status: ExitStatus,
        diagnostic: FfmpegDiagnostic,
    },
}

#[derive(Debug)]
pub struct FfmpegRawVideoProcess {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stderr_reader: Option<JoinHandle<io::Result<Vec<u8>>>>,
    frame_bytes: usize,
    next_frame: u64,
    expected_frames: u64,
}

impl FfmpegRawVideoProcess {
    /// Before sending each video frame, honor cooperative cancellation. The
    /// owning export worker drops or explicitly cancels this child on error.
    pub fn write_frame_checked(
        &mut self,
        cancellation: &ExportCancellationToken,
        frame_index: u64,
        rgba: &[u8],
    ) -> Result<(), FfmpegProcessError> {
        if cancellation.is_cancelled() {
            return Err(FfmpegProcessError::Cancelled);
        }
        self.write_frame(frame_index, rgba)
    }

    pub fn finish_checked(
        self,
        cancellation: &ExportCancellationToken,
    ) -> Result<(), FfmpegProcessError> {
        if cancellation.is_cancelled() {
            return Err(FfmpegProcessError::Cancelled);
        }
        self.finish()
    }

    /// Explicitly release resources on a canceled job; Drop kills and reaps
    /// any encoder still running without touching the user's final output.
    pub fn cancel(self) {
        drop(self);
    }

    /// FFmpeg stdin is the rawvideo pipe; an export worker feeds completed,
    /// tightly packed RGBA8 frames after GPU readback and row unpadding.
    pub fn write_frame(&mut self, frame_index: u64, rgba: &[u8]) -> Result<(), FfmpegProcessError> {
        if frame_index != self.next_frame {
            return Err(FfmpegProcessError::OutOfOrderFrame {
                expected: self.next_frame,
                actual: frame_index,
            });
        }
        if self.next_frame >= self.expected_frames {
            return Err(FfmpegProcessError::TooManyFrames);
        }
        if rgba.len() != self.frame_bytes {
            return Err(FfmpegProcessError::FrameSizeMismatch {
                expected: self.frame_bytes,
                actual: rgba.len(),
            });
        }
        self.stdin
            .as_mut()
            .ok_or(FfmpegProcessError::MissingStdin)?
            .write_all(rgba)
            .map_err(FfmpegProcessError::Write)?;
        self.next_frame += 1;
        Ok(())
    }

    /// Close the video pipe before waiting for MP4 mux finalization. Keep the
    /// staging file unpublished until the exit status is successful.
    pub fn finish(mut self) -> Result<(), FfmpegProcessError> {
        if self.next_frame != self.expected_frames {
            return Err(FfmpegProcessError::MissingFrames {
                expected: self.expected_frames,
                actual: self.next_frame,
            });
        }
        self.stdin.take();
        let status = self
            .child
            .as_mut()
            .ok_or(FfmpegProcessError::MissingStdin)?
            .wait()
            .map_err(FfmpegProcessError::Wait)?;
        self.child.take();
        let stderr = self
            .stderr_reader
            .take()
            .ok_or(FfmpegProcessError::MissingStderr)?
            .join()
            .map_err(|_| FfmpegProcessError::StderrJoin)?
            .map_err(FfmpegProcessError::StderrRead)?;
        if !status.success() {
            return Err(FfmpegProcessError::EncoderFailed {
                status,
                diagnostic: classify_diagnostic(&stderr),
            });
        }
        Ok(())
    }

    #[must_use]
    pub const fn frames_written(&self) -> u64 {
        self.next_frame
    }
}

impl Drop for FfmpegRawVideoProcess {
    fn drop(&mut self) {
        self.stdin.take();
        if let Some(mut child) = self.child.take() {
            // No orphan encoder when a write fails or an export job aborts.
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportQuality, FfmpegConfigurationError, FfmpegExportOptions};
    use rhythm_core::time::FrameRate;
    use std::{
        ffi::{OsStr, OsString},
        path::PathBuf,
    };

    fn options(audio: bool) -> FfmpegExportOptions {
        let directory = std::env::temp_dir().join("Rhythm Export & Test");
        FfmpegExportOptions {
            executable: directory.join("bundled ffmpeg.exe"),
            staging_destination: directory.join("вывод & alpha.partial.mp4"),
            output_size: [1280, 720],
            frame_rate: FrameRate::new(30_000, 1_001).expect("valid rational rate"),
            frame_count: 300,
            duration_ns: 10_000_000_001,
            primary_audio: audio.then(|| directory.join("audio & source; (1).wav")),
            quality: ExportQuality::Balanced,
        }
    }

    fn args(options: &FfmpegExportOptions) -> Vec<OsString> {
        options
            .command()
            .expect("valid options")
            .get_args()
            .map(OsString::from)
            .collect()
    }

    fn contains_pair(args: &[OsString], flag: &str, value: &str) -> bool {
        args.windows(2)
            .any(|pair| pair[0] == OsString::from(flag) && pair[1] == OsString::from(value))
    }

    #[test]
    fn stderr_drain_is_bounded_but_reads_the_entire_stream() {
        let input = vec![b'x'; super::MAX_FFMPEG_DIAGNOSTIC_BYTES + 37];
        let tail = super::drain_stderr(std::io::Cursor::new(&input)).expect("drain");
        assert_eq!(tail.len(), super::MAX_FFMPEG_DIAGNOSTIC_BYTES);
        assert_eq!(&tail[..], &input[37..]);
    }

    #[test]
    fn stderr_diagnostics_map_common_encoder_and_io_failures() {
        use super::{FfmpegFailureStage, classify_diagnostic};
        let cases = [
            (b"Unknown encoder 'libx264'".as_slice(), FfmpegFailureStage::EncoderUnavailable),
            (b"Permission denied".as_slice(), FfmpegFailureStage::PermissionDenied),
            (b"No space left on device".as_slice(), FfmpegFailureStage::DiskFull),
            (b"Invalid data found when processing input".as_slice(), FfmpegFailureStage::InvalidAudioSource),
            (b"the encoder exited unexpectedly".as_slice(), FfmpegFailureStage::ProcessFailed),
        ];
        for (stderr, expected) in cases {
            let diagnostic = classify_diagnostic(stderr);
            assert_eq!(diagnostic.stage, expected);
            assert_eq!(diagnostic.stderr_tail.as_bytes(), stderr);
        }
    }

    #[test]
    fn direct_command_has_rational_raw_rgba_and_h264_yuv420p_mp4_without_shell() {
        let options = options(false);
        let args = args(&options);
        assert_eq!(options.validate(), Ok(1280 * 720 * 4));
        assert!(contains_pair(&args, "-f", "rawvideo"));
        assert!(contains_pair(&args, "-pix_fmt", "rgba"));
        assert!(contains_pair(&args, "-video_size", "1280x720"));
        assert!(contains_pair(&args, "-framerate", "30000/1001"));
        assert!(contains_pair(&args, "-i", "pipe:0"));
        assert!(contains_pair(&args, "-frames:v", "300"));
        assert!(contains_pair(&args, "-c:v", "libx264"));
        assert!(contains_pair(&args, "-pix_fmt", "yuv420p"));
        assert!(contains_pair(&args, "-f", "mp4"));
        assert!(args.contains(&OsString::from("-an")));
        assert!(!args.contains(&OsString::from("cmd.exe")));
        assert!(!args.contains(&OsString::from("-shortest")));
        assert_eq!(
            args.last(),
            Some(&options.staging_destination.as_os_str().to_os_string())
        );
    }

    #[test]
    fn audio_is_original_single_literal_path_and_trimmed_without_shortest() {
        let options = options(true);
        let args = args(&options);
        let audio = options.primary_audio.as_ref().expect("audio");
        assert!(args.windows(2).any(|pair| {
            pair[0].as_os_str() == OsStr::new("-i") && pair[1].as_os_str() == audio.as_os_str()
        }));
        assert!(contains_pair(&args, "-map", "0:v:0"));
        assert!(contains_pair(&args, "-map", "1:a:0"));
        assert!(contains_pair(&args, "-c:a", "aac"));
        assert!(contains_pair(
            &args,
            "-af",
            "atrim=duration=10.000000001,asetpts=PTS-STARTPTS"
        ));
        assert!(!args.contains(&OsString::from("-shortest")));
        assert!(!args.contains(&OsString::from("-an")));
        assert_eq!(
            args.last(),
            Some(&options.staging_destination.as_os_str().to_os_string())
        );
    }

    #[test]
    fn quality_presets_are_explicit_and_distinct_for_validated_libx264_backend() {
        assert_eq!(ExportQuality::Fast.libx264_options(), ("veryfast", "23"));
        assert_eq!(ExportQuality::Balanced.libx264_options(), ("medium", "20"));
        assert_eq!(ExportQuality::High.libx264_options(), ("slow", "17"));
    }

    #[test]
    fn rejects_odd_yuv420_size_relative_executable_and_unsafe_audio_replacement() {
        let mut options = options(true);
        options.output_size = [1281, 720];
        assert_eq!(
            options.validate(),
            Err(FfmpegConfigurationError::InvalidDimensions)
        );
        options.output_size = [1280, 720];
        options.executable = PathBuf::from("ffmpeg");
        assert_eq!(
            options.validate(),
            Err(FfmpegConfigurationError::MissingAbsoluteExecutable)
        );
        options.executable = std::env::temp_dir().join("ffmpeg.exe");
        options.primary_audio = Some(options.staging_destination.clone());
        assert_eq!(
            options.validate(),
            Err(FfmpegConfigurationError::AudioDestinationConflict)
        );
        options.primary_audio = Some(PathBuf::from("relative.wav"));
        assert_eq!(
            options.validate(),
            Err(FfmpegConfigurationError::InvalidAudioPath)
        );
    }
}
