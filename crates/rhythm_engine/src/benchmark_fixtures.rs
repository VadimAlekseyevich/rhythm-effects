//! Generated, redistributable performance benchmark fixtures.
//!
//! No copyrighted media is checked in. Projects, checker image and PCM audio
//! are deterministic from code and can be materialized beside a benchmark
//! project when a filesystem-backed run is required.

use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use rhythm_core::{
    animation::{Animated, Interpolation, Keyframe},
    domain::{LinearRgba, Vec2},
    ids::{AssetId, EffectId, KeyframeId, ObjectId},
    project::{
        AssetKind, AssetRecord, AssetSource, AudioTrack, BlurEffect, Effect, EffectKind,
        EllipseObject, GlowEffect, ImageObject, NoiseEffect, Object, ObjectContent, Project,
        ProjectSettings, RectangleObject, RgbSplitEffect, TextAlignment, TextObject, TintEffect,
        TransformAnimation, FontReference, FontStyle, FontWeight,
    },
    time::{
        BpmMicros, DurationNs, GridOffsetNs, MusicalTick, SampleRate, TempoMap, TimeSignature,
    },
};

pub const BENCHMARK_IMAGE_FILE: &str = "benchmark-checker.png";
pub const BASIC_AUDIO_FILE: &str = "benchmark-basic-audio.wav";
pub const LONG_AUDIO_FILE: &str = "benchmark-long-audio.wav";
pub const BENCHMARK_AUDIO_SAMPLE_RATE: u32 = 48_000;
pub const LONG_AUDIO_DURATION_NS: u64 = 10 * 60 * 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkKind {
    Empty,
    BasicRhythm,
    MediumMotion,
    TextHeavy,
    EffectsHeavy,
    TimelineStress,
}

#[derive(Debug, Clone)]
pub struct GeneratedImageFixture {
    pub filename: &'static str,
    pub width: u32,
    pub height: u32,
}

impl GeneratedImageFixture {
    pub fn png_bytes(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            for x in 0..self.width {
                let light = ((x / 8) + (y / 8)).is_multiple_of(2);
                let (r, g, b, a) = if light {
                    (240, 120, 32, 230)
                } else {
                    (24, 84, 220, 180)
                };
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(
                &rgba,
                self.width,
                self.height,
                ExtendedColorType::Rgba8,
            )
            .expect("generated benchmark image is valid RGBA8");
        encoded
    }

    pub fn materialize(&self, directory: &Path) -> io::Result<PathBuf> {
        let path = directory.join(self.filename);
        std::fs::write(&path, self.png_bytes())?;
        Ok(path)
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedAudioFixture {
    pub filename: &'static str,
    pub duration_ns: u64,
    pub sample_rate: SampleRate,
}

impl GeneratedAudioFixture {
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.duration_ns * u64::from(self.sample_rate.get()) / 1_000_000_000
    }

    #[must_use]
    pub fn data_bytes(&self) -> u64 {
        self.frame_count() * 2
    }

    #[must_use]
    pub fn sample_at(&self, frame: u64) -> i16 {
        // Sparse one-sample click each second plus a tiny deterministic bed.
        let sample_rate = u64::from(self.sample_rate.get());
        if frame.is_multiple_of(sample_rate) {
            20_000
        } else {
            let phase = frame % 256;
            i16::try_from(phase).expect("0..255") - 128
        }
    }

    fn wav_header(&self) -> [u8; 44] {
        let data_bytes = u32::try_from(self.data_bytes()).expect("benchmark WAV below RIFF 4 GiB");
        let riff_size = 36_u32.checked_add(data_bytes).expect("RIFF size");
        let sample_rate = self.sample_rate.get();
        let mut header = [0_u8; 44];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&riff_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16_u32.to_le_bytes());
        header[20..22].copy_from_slice(&1_u16.to_le_bytes());
        header[22..24].copy_from_slice(&1_u16.to_le_bytes());
        header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&(sample_rate * 2).to_le_bytes());
        header[32..34].copy_from_slice(&2_u16.to_le_bytes());
        header[34..36].copy_from_slice(&16_u16.to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&data_bytes.to_le_bytes());
        header
    }

    /// Stream PCM directly to disk in bounded chunks. A 10-minute fixture
    /// therefore does not require retaining a second ~55 MiB sample buffer.
    pub fn materialize(&self, directory: &Path) -> io::Result<PathBuf> {
        let path = directory.join(self.filename);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&self.wav_header())?;
        const CHUNK_FRAMES: u64 = 8_192;
        let mut start = 0_u64;
        while start < self.frame_count() {
            let end = (start + CHUNK_FRAMES).min(self.frame_count());
            let mut bytes = Vec::with_capacity(usize::try_from((end - start) * 2).expect("chunk"));
            for frame in start..end {
                bytes.extend_from_slice(&self.sample_at(frame).to_le_bytes());
            }
            writer.write_all(&bytes)?;
            start = end;
        }
        writer.flush()?;
        Ok(path)
    }
}

#[must_use]
pub fn long_audio_fixture() -> GeneratedAudioFixture {
    GeneratedAudioFixture {
        filename: LONG_AUDIO_FILE,
        duration_ns: LONG_AUDIO_DURATION_NS,
        sample_rate: SampleRate::new(BENCHMARK_AUDIO_SAMPLE_RATE),
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkFixture {
    pub kind: BenchmarkKind,
    pub project: Project,
    pub image: Option<GeneratedImageFixture>,
    pub audio: Option<GeneratedAudioFixture>,
}

impl BenchmarkFixture {
    pub fn materialize_assets(&self, canonical_project_path: &Path) -> io::Result<Vec<PathBuf>> {
        let directory = canonical_project_path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "project path has no parent")
        })?;
        let mut paths = Vec::new();
        if let Some(image) = &self.image {
            paths.push(image.materialize(directory)?);
        }
        if let Some(audio) = &self.audio {
            paths.push(audio.materialize(directory)?);
        }
        Ok(paths)
    }

    #[must_use]
    pub fn object_count(&self) -> usize {
        self.project.composition.objects.len()
    }

    #[must_use]
    pub fn keyframe_count(&self) -> usize {
        project_keyframe_count(&self.project)
    }
}

#[derive(Debug)]
struct FixtureIds {
    next: u64,
}

impl FixtureIds {
    const fn new() -> Self {
        Self { next: 1 }
    }

    fn raw(&mut self) -> u64 {
        let current = self.next;
        self.next += 1;
        current
    }

    fn object(&mut self) -> ObjectId {
        ObjectId::new(self.raw()).expect("nonzero object ID")
    }

    fn asset(&mut self) -> AssetId {
        AssetId::new(self.raw()).expect("nonzero asset ID")
    }

    fn effect(&mut self) -> EffectId {
        EffectId::new(self.raw()).expect("nonzero effect ID")
    }

    fn keyframe(&mut self) -> KeyframeId {
        KeyframeId::new(self.raw()).expect("nonzero keyframe ID")
    }
}

fn benchmark_tempo() -> TempoMap {
    TempoMap::with_initial_tempo(
        GridOffsetNs::new(0),
        BpmMicros::new(120_000_000).expect("120 BPM"),
        TimeSignature::default(),
    )
}

fn settings(duration_ns: u64, size: [u32; 2]) -> ProjectSettings {
    ProjectSettings {
        composition_width: size[0],
        composition_height: size[1],
        duration: DurationNs::new(duration_ns),
        ..ProjectSettings::default()
    }
}

fn color(r: f32, g: f32, b: f32, a: f32) -> LinearRgba {
    LinearRgba::new(r, g, b, a).expect("benchmark color")
}

fn animated_position(ids: &mut FixtureIds, object_index: usize, count: usize) -> Animated<Vec2> {
    let base_x = 80.0 + (object_index % 16) as f32 * 110.0;
    let base_y = 80.0 + (object_index / 16) as f32 * 95.0;
    if count == 0 {
        return Animated::new_static(Vec2::new(base_x, base_y).expect("position"));
    }
    let keyframes = (0..count)
        .map(|index| {
            let offset = ((index * 17 + object_index * 13) % 120) as f32;
            Keyframe::new(
                ids.keyframe(),
                MusicalTick::new(i64::try_from(index).expect("index") * 240),
                Vec2::new(base_x + offset, base_y + offset * 0.35).expect("position"),
                Interpolation::Linear,
            )
        })
        .collect();
    Animated::with_keyframes(
        Vec2::new(base_x, base_y).expect("base position"),
        keyframes,
    )
    .expect("unique benchmark position ticks")
}

fn transform(ids: &mut FixtureIds, object_index: usize, position_keys: usize) -> TransformAnimation {
    TransformAnimation::new(
        animated_position(ids, object_index, position_keys),
        Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
        Animated::new_static(0.0),
        Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
        Animated::new_static(1.0),
    )
}

fn rectangle(ids: &mut FixtureIds, index: usize, position_keys: usize) -> Object {
    Object {
        id: ids.object(),
        name: format!("Rectangle {index:03}"),
        visible: true,
        locked: false,
        transform: transform(ids, index, position_keys),
        content: ObjectContent::Rectangle(RectangleObject {
            size: Animated::new_static(Vec2::new(96.0, 72.0).expect("size")),
            fill: Animated::new_static(color(0.2, 0.55, 0.9, 0.8)),
            corner_radius: Animated::new_static(10.0),
        }),
        effects: Vec::new(),
    }
}

fn ellipse(ids: &mut FixtureIds, index: usize, position_keys: usize) -> Object {
    Object {
        id: ids.object(),
        name: format!("Ellipse {index:03}"),
        visible: true,
        locked: false,
        transform: transform(ids, index, position_keys),
        content: ObjectContent::Ellipse(EllipseObject {
            size: Animated::new_static(Vec2::new(88.0, 88.0).expect("size")),
            fill: Animated::new_static(color(0.9, 0.35, 0.2, 0.75)),
        }),
        effects: Vec::new(),
    }
}

fn text_object(
    ids: &mut FixtureIds,
    index: usize,
    position_keys: usize,
    color_keys: usize,
) -> Object {
    let families = ["Inter", "Segoe UI", "Arial"];
    let base_color = color(0.85, 0.9, 1.0, 0.9);
    let animated_color = if color_keys == 0 {
        Animated::new_static(base_color)
    } else {
        let keys = (0..color_keys)
            .map(|key| {
                let blend = (key % 5) as f32 / 4.0;
                Keyframe::new(
                    ids.keyframe(),
                    MusicalTick::new(i64::try_from(key).expect("key") * 480),
                    color(0.2 + 0.6 * blend, 0.9 - 0.5 * blend, 0.5, 0.9),
                    Interpolation::Linear,
                )
            })
            .collect();
        Animated::with_keyframes(base_color, keys).expect("unique color ticks")
    };
    Object {
        id: ids.object(),
        name: format!("Text {index:03}"),
        visible: true,
        locked: false,
        transform: transform(ids, index, position_keys),
        content: ObjectContent::Text(TextObject {
            text: format!("Rhythm {index:02} — Привет мир"),
            font: FontReference {
                family: families[index % families.len()].into(),
                weight: match index % 3 {
                    0 => FontWeight::Normal,
                    1 => FontWeight::SemiBold,
                    _ => FontWeight::Bold,
                },
                style: if index.is_multiple_of(7) {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                },
            },
            font_size: 28.0 + (index % 5) as f32 * 2.0,
            color: animated_color,
            alignment: match index % 3 {
                0 => TextAlignment::Left,
                1 => TextAlignment::Center,
                _ => TextAlignment::Right,
            },
        }),
        effects: Vec::new(),
    }
}

fn image_object(ids: &mut FixtureIds, index: usize, position_keys: usize, asset: AssetId) -> Object {
    Object {
        id: ids.object(),
        name: format!("Image {index:03}"),
        visible: true,
        locked: false,
        transform: transform(ids, index, position_keys),
        content: ObjectContent::Image(ImageObject { asset }),
        effects: Vec::new(),
    }
}

fn one_representative_effect(ids: &mut FixtureIds, index: usize) -> Effect {
    let kind = match index % 5 {
        0 => EffectKind::Blur(BlurEffect {
            radius_px: Animated::new_static(6.0),
        }),
        1 => EffectKind::Glow(GlowEffect {
            radius_px: Animated::new_static(8.0),
            intensity: Animated::new_static(0.8),
            threshold: Animated::new_static(0.55),
            color: Animated::new_static(color(1.0, 0.5, 0.1, 0.8)),
        }),
        2 => EffectKind::Tint(TintEffect {
            color: Animated::new_static(color(0.15, 0.75, 1.0, 0.8)),
            amount: Animated::new_static(0.2),
        }),
        3 => EffectKind::Noise(NoiseEffect {
            amount: Animated::new_static(0.12),
            size_px: Animated::new_static(6.0),
            evolution: Animated::new_static(index as f32 * 0.1),
            seed: u32::try_from(index).expect("benchmark index") + 1,
        }),
        _ => EffectKind::RgbSplit(RgbSplitEffect {
            amount_px: Animated::new_static(3.0),
            angle_degrees: Animated::new_static(30.0),
        }),
    };
    Effect {
        id: ids.effect(),
        enabled: true,
        kind,
    }
}

fn full_effect_stack(ids: &mut FixtureIds, seed: u32) -> Vec<Effect> {
    [
        EffectKind::Blur(BlurEffect {
            radius_px: Animated::new_static(16.0),
        }),
        EffectKind::Glow(GlowEffect {
            radius_px: Animated::new_static(20.0),
            intensity: Animated::new_static(1.25),
            threshold: Animated::new_static(0.6),
            color: Animated::new_static(color(1.0, 0.45, 0.15, 0.9)),
        }),
        EffectKind::Tint(TintEffect {
            color: Animated::new_static(color(0.2, 0.7, 1.0, 0.8)),
            amount: Animated::new_static(0.35),
        }),
        EffectKind::Noise(NoiseEffect {
            amount: Animated::new_static(0.2),
            size_px: Animated::new_static(8.0),
            evolution: Animated::new_static(0.5),
            seed,
        }),
        EffectKind::RgbSplit(RgbSplitEffect {
            amount_px: Animated::new_static(8.0),
            angle_degrees: Animated::new_static(35.0),
        }),
    ]
    .into_iter()
    .map(|kind| Effect {
        id: ids.effect(),
        enabled: true,
        kind,
    })
    .collect()
}

fn add_audio(
    project: &mut Project,
    ids: &mut FixtureIds,
    fixture: GeneratedAudioFixture,
) -> GeneratedAudioFixture {
    let asset = ids.asset();
    project.assets.push(AssetRecord {
        id: asset,
        kind: AssetKind::Audio,
        source: AssetSource::File {
            path: fixture.filename.into(),
            relative_to_project: true,
        },
    });
    project.audio_track = Some(AudioTrack::new(asset));
    fixture
}

fn add_checker_asset(project: &mut Project, ids: &mut FixtureIds) -> (AssetId, GeneratedImageFixture) {
    let asset = ids.asset();
    project.assets.push(AssetRecord {
        id: asset,
        kind: AssetKind::Image,
        source: AssetSource::File {
            path: BENCHMARK_IMAGE_FILE.into(),
            relative_to_project: true,
        },
    });
    (
        asset,
        GeneratedImageFixture {
            filename: BENCHMARK_IMAGE_FILE,
            width: 64,
            height: 64,
        },
    )
}

#[must_use]
pub fn empty_benchmark_fixture() -> BenchmarkFixture {
    BenchmarkFixture {
        kind: BenchmarkKind::Empty,
        project: Project::new(
            "Benchmark — Empty",
            settings(10_000_000_000, [1920, 1080]),
            benchmark_tempo(),
        ),
        image: None,
        audio: None,
    }
}

#[must_use]
pub fn basic_rhythm_benchmark_fixture() -> BenchmarkFixture {
    let mut ids = FixtureIds::new();
    let mut project = Project::new(
        "Benchmark — Basic Rhythm",
        settings(3 * 60 * 1_000_000_000, [1920, 1080]),
        benchmark_tempo(),
    );
    let audio = add_audio(
        &mut project,
        &mut ids,
        GeneratedAudioFixture {
            filename: BASIC_AUDIO_FILE,
            duration_ns: 3 * 60 * 1_000_000_000,
            sample_rate: SampleRate::new(BENCHMARK_AUDIO_SAMPLE_RATE),
        },
    );
    for index in 0..10 {
        let mut object = rectangle(&mut ids, index, 10);
        if index.is_multiple_of(2) {
            object.effects.push(one_representative_effect(&mut ids, index));
        }
        project.composition.objects.push(object);
    }
    project.next_entity_id = ids.next;
    BenchmarkFixture {
        kind: BenchmarkKind::BasicRhythm,
        project,
        image: None,
        audio: Some(audio),
    }
}

#[must_use]
pub fn medium_motion_benchmark_fixture() -> BenchmarkFixture {
    let mut ids = FixtureIds::new();
    let mut project = Project::new(
        "Benchmark — Medium Motion",
        settings(LONG_AUDIO_DURATION_NS, [1920, 1080]),
        benchmark_tempo(),
    );
    let audio = add_audio(&mut project, &mut ids, long_audio_fixture());
    let (image_asset, image) = add_checker_asset(&mut project, &mut ids);
    for index in 0..100 {
        let mut object = match index % 4 {
            0 => rectangle(&mut ids, index, 10),
            1 => ellipse(&mut ids, index, 10),
            2 => text_object(&mut ids, index, 10, 0),
            _ => image_object(&mut ids, index, 10, image_asset),
        };
        object.effects.push(one_representative_effect(&mut ids, index));
        project.composition.objects.push(object);
    }
    project.next_entity_id = ids.next;
    BenchmarkFixture {
        kind: BenchmarkKind::MediumMotion,
        project,
        image: Some(image),
        audio: Some(audio),
    }
}

#[must_use]
pub fn text_heavy_benchmark_fixture() -> BenchmarkFixture {
    let mut ids = FixtureIds::new();
    let mut project = Project::new(
        "Benchmark — Text Heavy",
        settings(60_000_000_000, [1920, 1080]),
        benchmark_tempo(),
    );
    for index in 0..50 {
        project
            .composition
            .objects
            .push(text_object(&mut ids, index, 2, 2));
    }
    project.next_entity_id = ids.next;
    BenchmarkFixture {
        kind: BenchmarkKind::TextHeavy,
        project,
        image: None,
        audio: None,
    }
}

#[must_use]
pub fn effects_heavy_benchmark_fixture() -> BenchmarkFixture {
    let mut ids = FixtureIds::new();
    let mut project = Project::new(
        "Benchmark — Effects Heavy",
        settings(60_000_000_000, [1920, 1080]),
        benchmark_tempo(),
    );
    for index in 0..30 {
        let mut object = rectangle(&mut ids, index, 4);
        object.effects = full_effect_stack(
            &mut ids,
            u32::try_from(index).expect("benchmark index") + 1,
        );
        project.composition.objects.push(object);
    }
    project.next_entity_id = ids.next;
    BenchmarkFixture {
        kind: BenchmarkKind::EffectsHeavy,
        project,
        image: None,
        audio: None,
    }
}

#[must_use]
pub fn timeline_stress_benchmark_fixture() -> BenchmarkFixture {
    let mut ids = FixtureIds::new();
    let mut project = Project::new(
        "Benchmark — Timeline Stress",
        settings(120_000_000_000, [3840, 2160]),
        benchmark_tempo(),
    );
    for index in 0..500 {
        let mut object = if index.is_multiple_of(5) {
            text_object(&mut ids, index, 21, 0)
        } else if index.is_multiple_of(2) {
            ellipse(&mut ids, index, 21)
        } else {
            rectangle(&mut ids, index, 21)
        };
        if index.is_multiple_of(4) {
            object.effects.push(one_representative_effect(&mut ids, index));
        }
        project.composition.objects.push(object);
    }
    project.next_entity_id = ids.next;
    BenchmarkFixture {
        kind: BenchmarkKind::TimelineStress,
        project,
        image: None,
        audio: None,
    }
}

fn animated_count<T>(value: &Animated<T>) -> usize {
    value.keyframes().len()
}

#[must_use]
pub fn project_keyframe_count(project: &Project) -> usize {
    project
        .composition
        .objects
        .iter()
        .map(|object| {
            let transform = animated_count(&object.transform.position)
                + animated_count(&object.transform.scale)
                + animated_count(&object.transform.rotation_degrees)
                + animated_count(&object.transform.anchor)
                + animated_count(&object.transform.opacity);
            let content = match &object.content {
                ObjectContent::Rectangle(value) => {
                    animated_count(&value.size)
                        + animated_count(&value.fill)
                        + animated_count(&value.corner_radius)
                }
                ObjectContent::Ellipse(value) => {
                    animated_count(&value.size) + animated_count(&value.fill)
                }
                ObjectContent::Image(_) => 0,
                ObjectContent::Text(value) => animated_count(&value.color),
            };
            let effects: usize = object
                .effects
                .iter()
                .map(|effect| match &effect.kind {
                    EffectKind::Blur(value) => animated_count(&value.radius_px),
                    EffectKind::Glow(value) => {
                        animated_count(&value.radius_px)
                            + animated_count(&value.intensity)
                            + animated_count(&value.threshold)
                            + animated_count(&value.color)
                    }
                    EffectKind::Tint(value) => {
                        animated_count(&value.color) + animated_count(&value.amount)
                    }
                    EffectKind::Noise(value) => {
                        animated_count(&value.amount)
                            + animated_count(&value.size_px)
                            + animated_count(&value.evolution)
                    }
                    EffectKind::RgbSplit(value) => {
                        animated_count(&value.amount_px)
                            + animated_count(&value.angle_degrees)
                    }
                })
                .sum();
            transform + content + effects
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{
        BENCHMARK_AUDIO_SAMPLE_RATE, LONG_AUDIO_DURATION_NS, BenchmarkKind, GeneratedAudioFixture,
        basic_rhythm_benchmark_fixture, effects_heavy_benchmark_fixture,
        empty_benchmark_fixture, long_audio_fixture, medium_motion_benchmark_fixture,
        text_heavy_benchmark_fixture, timeline_stress_benchmark_fixture,
    };
    use rhythm_core::time::SampleRate;

    #[test]
    fn empty_and_basic_fixtures_match_declared_small_counts() {
        let empty = empty_benchmark_fixture();
        empty.project.validate().expect("empty valid");
        assert_eq!(empty.kind, BenchmarkKind::Empty);
        assert_eq!(empty.object_count(), 0);
        assert_eq!(empty.keyframe_count(), 0);

        let basic = basic_rhythm_benchmark_fixture();
        basic.project.validate().expect("basic valid");
        assert_eq!(basic.object_count(), 10);
        assert_eq!(basic.keyframe_count(), 100);
        assert_eq!(basic.project.settings.composition_width, 1920);
        assert_eq!(basic.project.settings.composition_height, 1080);
        assert_eq!(basic.audio.as_ref().expect("audio").duration_ns, 180_000_000_000);
    }

    #[test]
    fn medium_fixture_is_exact_primary_performance_class() {
        let medium = medium_motion_benchmark_fixture();
        medium.project.validate().expect("medium valid");
        assert_eq!(medium.object_count(), 100);
        assert_eq!(medium.keyframe_count(), 1_000);
        assert!(medium.image.is_some());
        assert_eq!(
            medium.audio.as_ref().expect("10 minute audio").duration_ns,
            LONG_AUDIO_DURATION_NS
        );
        let text = medium
            .project
            .composition
            .objects
            .iter()
            .filter(|object| matches!(&object.content, rhythm_core::project::ObjectContent::Text(_)))
            .count();
        let images = medium
            .project
            .composition
            .objects
            .iter()
            .filter(|object| matches!(&object.content, rhythm_core::project::ObjectContent::Image(_)))
            .count();
        assert!(text > 0 && images > 0);
        assert!(medium.project.composition.objects.iter().all(|object| !object.effects.is_empty()));
    }

    #[test]
    fn text_effect_and_timeline_stress_fixtures_have_intended_pressure() {
        let text = text_heavy_benchmark_fixture();
        text.project.validate().expect("text valid");
        assert_eq!(text.object_count(), 50);
        assert_eq!(text.keyframe_count(), 200);
        assert!(text.project.composition.objects.iter().all(|object| {
            matches!(&object.content, rhythm_core::project::ObjectContent::Text(_))
        }));

        let effects = effects_heavy_benchmark_fixture();
        effects.project.validate().expect("effects valid");
        assert_eq!(effects.object_count(), 30);
        assert_eq!(
            effects
                .project
                .composition
                .objects
                .iter()
                .map(|object| object.effects.len())
                .sum::<usize>(),
            150
        );

        let stress = timeline_stress_benchmark_fixture();
        stress.project.validate().expect("stress valid");
        assert_eq!(stress.object_count(), 500);
        assert_eq!(stress.keyframe_count(), 10_500);
        assert_eq!(stress.project.settings.composition_width, 3840);
        assert_eq!(stress.project.settings.composition_height, 2160);
    }

    #[test]
    fn long_audio_is_ten_minutes_and_stream_generator_is_deterministic() {
        let audio = long_audio_fixture();
        assert_eq!(audio.duration_ns, LONG_AUDIO_DURATION_NS);
        assert_eq!(audio.sample_rate.get(), BENCHMARK_AUDIO_SAMPLE_RATE);
        assert_eq!(audio.frame_count(), 28_800_000);
        assert_eq!(audio.data_bytes(), 57_600_000);
        assert_eq!(audio.sample_at(0), 20_000);
        assert_eq!(audio.sample_at(48_000), 20_000);
        assert_eq!(audio.sample_at(1), audio.sample_at(257));

        let tiny = GeneratedAudioFixture {
            filename: "tiny.wav",
            duration_ns: 1_000_000_000,
            sample_rate: SampleRate::new(BENCHMARK_AUDIO_SAMPLE_RATE),
        };
        let header = tiny.wav_header();
        assert_eq!(&header[0..4], b"RIFF");
        assert_eq!(&header[8..12], b"WAVE");
        assert_eq!(&header[36..40], b"data");
        assert_eq!(u32::from_le_bytes(header[40..44].try_into().expect("data size")), 96_000);
    }

    #[test]
    fn every_fixture_is_deterministic() {
        assert_eq!(
            basic_rhythm_benchmark_fixture().project,
            basic_rhythm_benchmark_fixture().project
        );
        assert_eq!(
            medium_motion_benchmark_fixture().project,
            medium_motion_benchmark_fixture().project
        );
        assert_eq!(
            text_heavy_benchmark_fixture().project,
            text_heavy_benchmark_fixture().project
        );
        assert_eq!(
            effects_heavy_benchmark_fixture().project,
            effects_heavy_benchmark_fixture().project
        );
        assert_eq!(
            timeline_stress_benchmark_fixture().project,
            timeline_stress_benchmark_fixture().project
        );
    }
}
