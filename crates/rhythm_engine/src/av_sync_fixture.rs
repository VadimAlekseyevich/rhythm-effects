//! Generated click/flash A/V synchronization fixture.
//!
//! The fixture has no checked-in binary media. One deterministic schedule
//! drives both a PCM WAV click onset and an opacity flash keyframe.

use std::{fs, io, path::{Path, PathBuf}};

use rhythm_core::{
    animation::{Animated, Interpolation, Keyframe},
    domain::{LinearRgba, Vec2},
    ids::{AssetId, KeyframeId, ObjectId},
    project::{
        AssetKind, AssetRecord, AssetSource, AudioTrack, Object, ObjectContent, Project,
        ProjectSettings, RectangleObject, TransformAnimation,
    },
    time::{
        BpmMicros, DurationNs, GridOffsetNs, MusicalTick, ProjectTimeNs, TempoMap, TimeSignature,
        PPQ,
    },
};

pub const AV_SYNC_AUDIO_FILE: &str = "av-sync-clicks.wav";
pub const AV_SYNC_SAMPLE_RATE: u32 = 48_000;
pub const AV_SYNC_DURATION_NS: u64 = 5_000_000_000;
pub const AV_SYNC_FLASH_TICKS: i64 = 16;
pub const AV_SYNC_EVENT_SECONDS: [u64; 4] = [1, 2, 3, 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvSyncEvent {
    pub project_time: ProjectTimeNs,
    pub musical_tick: MusicalTick,
    pub audio_frame: u64,
}

#[derive(Debug, Clone)]
pub struct GeneratedAvSyncFixture {
    pub project: Project,
    pub events: Vec<AvSyncEvent>,
    pub audio_wav: Vec<u8>,
}

impl GeneratedAvSyncFixture {
    /// Write only the generated media beside a would-be canonical project.
    /// The Project stores the WAV as a relative AssetSource, matching normal
    /// Save/Save As semantics and export preflight resolution.
    pub fn materialize_audio(&self, canonical_project_path: &Path) -> io::Result<PathBuf> {
        let directory = canonical_project_path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "project path has no parent")
        })?;
        let path = directory.join(AV_SYNC_AUDIO_FILE);
        fs::write(&path, &self.audio_wav)?;
        Ok(path)
    }
}

fn transform(opacity: Animated<f32>) -> TransformAnimation {
    TransformAnimation::new(
        Animated::new_static(Vec2::new(960.0, 540.0).expect("center")),
        Animated::new_static(Vec2::new(1.0, 1.0).expect("unit scale")),
        Animated::new_static(0.0),
        Animated::new_static(Vec2::new(0.5, 0.5).expect("center anchor")),
        opacity,
    )
}

fn click_wav(events: &[AvSyncEvent]) -> Vec<u8> {
    const CLICK_FRAMES: u64 = 480; // 10 ms at 48 kHz.
    const PCM_AMPLITUDE: i16 = 24_000;
    let total_frames = AV_SYNC_DURATION_NS * u64::from(AV_SYNC_SAMPLE_RATE) / 1_000_000_000;
    let data_bytes = total_frames
        .checked_mul(2)
        .and_then(|bytes| u32::try_from(bytes).ok())
        .expect("short fixture PCM fits RIFF");
    let riff_size = 36_u32.checked_add(data_bytes).expect("RIFF size");
    let mut wav = Vec::with_capacity(44 + data_bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1_u16.to_le_bytes()); // mono
    wav.extend_from_slice(&AV_SYNC_SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(AV_SYNC_SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());

    let mut samples = vec![0_i16; total_frames as usize];
    for event in events {
        let end = (event.audio_frame + CLICK_FRAMES).min(total_frames);
        for frame in event.audio_frame..end {
            // Alternating sign gives an unambiguous sharp onset without DC.
            samples[frame as usize] = if (frame - event.audio_frame).is_multiple_of(2) {
                PCM_AMPLITUDE
            } else {
                -PCM_AMPLITUDE
            };
        }
    }
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

/// Five-second fixture with flashes/clicks at exact 1 s boundaries.
/// At 60 BPM and PPQ=960, one second is exactly 960 musical ticks.
#[must_use]
pub fn generated_av_sync_fixture() -> GeneratedAvSyncFixture {
    let tempo = TempoMap::with_initial_tempo(
        GridOffsetNs::new(0),
        BpmMicros::new(60_000_000).expect("60 BPM"),
        TimeSignature::default(),
    );
    let settings = ProjectSettings {
        duration: DurationNs::new(AV_SYNC_DURATION_NS),
        ..ProjectSettings::default()
    };
    let mut project = Project::new("Generated A/V Sync", settings, tempo);

    let audio_id = AssetId::new(1).expect("asset ID");
    project.assets.push(AssetRecord {
        id: audio_id,
        kind: AssetKind::Audio,
        source: AssetSource::File {
            path: AV_SYNC_AUDIO_FILE.into(),
            relative_to_project: true,
        },
    });
    project.audio_track = Some(AudioTrack::new(audio_id));

    let events: Vec<_> = AV_SYNC_EVENT_SECONDS
        .into_iter()
        .map(|second| AvSyncEvent {
            project_time: ProjectTimeNs::new(
                i64::try_from(second * 1_000_000_000).expect("fixture time"),
            ),
            musical_tick: MusicalTick::new(
                i64::try_from(second).expect("fixture second") * PPQ,
            ),
            audio_frame: second * u64::from(AV_SYNC_SAMPLE_RATE),
        })
        .collect();

    let mut keyframes = vec![Keyframe::new(
        KeyframeId::new(3).expect("keyframe"),
        MusicalTick::new(0),
        0.0,
        Interpolation::Hold,
    )];
    let mut next_id = 4_u64;
    for event in &events {
        keyframes.push(Keyframe::new(
            KeyframeId::new(next_id).expect("flash on keyframe"),
            event.musical_tick,
            1.0,
            Interpolation::Hold,
        ));
        next_id += 1;
        keyframes.push(Keyframe::new(
            KeyframeId::new(next_id).expect("flash off keyframe"),
            MusicalTick::new(event.musical_tick.get() + AV_SYNC_FLASH_TICKS),
            0.0,
            Interpolation::Hold,
        ));
        next_id += 1;
    }
    let opacity = Animated::with_keyframes(0.0, keyframes).expect("unique flash ticks");

    project.composition.objects.push(Object {
        id: ObjectId::new(2).expect("object ID"),
        name: "A/V Sync Flash".into(),
        visible: true,
        locked: false,
        transform: transform(opacity),
        content: ObjectContent::Rectangle(RectangleObject {
            size: Animated::new_static(Vec2::new(400.0, 400.0).expect("size")),
            fill: Animated::new_static(
                LinearRgba::new(1.0, 1.0, 1.0, 1.0).expect("opaque white"),
            ),
            corner_radius: Animated::new_static(0.0),
        }),
        effects: Vec::new(),
    });
    project.next_entity_id = next_id;

    let audio_wav = click_wav(&events);
    GeneratedAvSyncFixture {
        project,
        events,
        audio_wav,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AV_SYNC_AUDIO_FILE, AV_SYNC_DURATION_NS, AV_SYNC_EVENT_SECONDS, AV_SYNC_SAMPLE_RATE,
        generated_av_sync_fixture,
    };
    use crate::{audio::probe_audio_file, scene_eval::evaluate_scene};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rhythm-av-sync-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).expect("temp dir");
            Self(root)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample_at(wav: &[u8], frame: u64) -> i16 {
        let offset = 44 + usize::try_from(frame).expect("frame") * 2;
        i16::from_le_bytes([wav[offset], wav[offset + 1]])
    }

    #[test]
    fn one_schedule_drives_exact_click_frames_and_flash_ticks() {
        let fixture = generated_av_sync_fixture();
        fixture.project.validate().expect("valid generated Project");
        assert_eq!(fixture.events.len(), AV_SYNC_EVENT_SECONDS.len());
        assert_eq!(fixture.project.settings.duration.get(), AV_SYNC_DURATION_NS);
        for (event, second) in fixture.events.iter().zip(AV_SYNC_EVENT_SECONDS) {
            assert_eq!(
                event.project_time.get(),
                i64::try_from(second * 1_000_000_000).expect("event time")
            );
            assert_eq!(event.musical_tick.get(), i64::try_from(second).expect("second") * 960);
            assert_eq!(event.audio_frame, second * u64::from(AV_SYNC_SAMPLE_RATE));
            assert_ne!(sample_at(&fixture.audio_wav, event.audio_frame), 0);
            assert_eq!(sample_at(&fixture.audio_wav, event.audio_frame - 1), 0);

            let on = evaluate_scene(&fixture.project, event.project_time).expect("flash on");
            assert_eq!(on.objects[0].transform.opacity, 1.0);
            let before = evaluate_scene(
                &fixture.project,
                rhythm_core::time::ProjectTimeNs::new(event.project_time.get() - 1),
            )
            .expect("just before");
            assert_eq!(before.objects[0].transform.opacity, 0.0);
        }
    }

    #[test]
    fn generated_wav_is_probeable_and_materialized_beside_project() {
        let fixture = generated_av_sync_fixture();
        let root = TestDir::new();
        let project_path = root.0.join("sync.rhfx");
        let audio = fixture.materialize_audio(&project_path).expect("write generated WAV");
        assert_eq!(audio, root.0.join(AV_SYNC_AUDIO_FILE));
        let probe = probe_audio_file(&audio).expect("probe generated PCM WAV");
        assert_eq!(probe.sample_rate, AV_SYNC_SAMPLE_RATE);
        assert_eq!(probe.channel_layout.channels(), 1);
        assert_eq!(
            fs::metadata(audio).expect("metadata").len(),
            44 + (AV_SYNC_DURATION_NS * u64::from(AV_SYNC_SAMPLE_RATE) / 1_000_000_000) * 2
        );
    }

    #[test]
    fn fixture_bytes_and_project_are_deterministic() {
        let first = generated_av_sync_fixture();
        let second = generated_av_sync_fixture();
        assert_eq!(first.project, second.project);
        assert_eq!(first.events, second.events);
        assert_eq!(first.audio_wav, second.audio_wav);
    }
}
