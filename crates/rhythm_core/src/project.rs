use std::collections::{HashMap, HashSet};

use crate::{
    animation::Animated,
    domain::{LinearRgba, Vec2},
    ids::{AssetId, EffectId, ObjectId},
    time::{DurationNs, FrameRate, TempoMap},
};

pub const MIN_COMPOSITION_DIMENSION: u32 = 16;
pub const MAX_COMPOSITION_DIMENSION: u32 = 8192;
pub const MAX_PROJECT_DURATION_NS: u64 = 24 * 60 * 60 * 1_000_000_000;
pub const MAX_PROJECT_OBJECTS: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectValidationError {
    InvalidCompositionDimensions,
    DurationTooLong,
    TooManyObjects,
    DuplicateEntityId(u64),
    InvalidNextEntityId,
    MissingAssetReference {
        asset_id: u64,
        expected_kind: AssetKind,
    },
    AssetKindMismatch {
        asset_id: u64,
        expected_kind: AssetKind,
        actual_kind: AssetKind,
    },
    NonFiniteValue(&'static str),
    ValueOutOfRange(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMetadata {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub metadata: ProjectMetadata,
    pub settings: ProjectSettings,
    pub tempo_map: TempoMap,
    pub audio_track: Option<AudioTrack>,
    pub assets: Vec<AssetRecord>,
    pub composition: Composition,
    pub next_entity_id: u64,
}

impl Project {
    #[must_use]
    pub fn new(name: impl Into<String>, settings: ProjectSettings, tempo_map: TempoMap) -> Self {
        Self {
            metadata: ProjectMetadata { name: name.into() },
            settings,
            tempo_map,
            audio_track: None,
            assets: Vec::new(),
            composition: Composition::default(),
            next_entity_id: 1,
        }
    }

    pub fn validate(&self) -> Result<(), ProjectValidationError> {
        validate_settings(&self.settings)?;

        if self.composition.objects.len() > MAX_PROJECT_OBJECTS {
            return Err(ProjectValidationError::TooManyObjects);
        }

        let mut seen_ids = HashSet::new();
        let mut max_entity_id = 0_u64;
        let mut asset_kinds = HashMap::new();

        for asset in &self.assets {
            register_entity_id(asset.id.get(), &mut seen_ids, &mut max_entity_id)?;
            asset_kinds.insert(asset.id, asset.kind);
        }

        for object in &self.composition.objects {
            register_entity_id(object.id.get(), &mut seen_ids, &mut max_entity_id)?;
            validate_transform(&object.transform)?;

            match &object.content {
                ObjectContent::Rectangle(rectangle) => validate_rectangle(rectangle)?,
                ObjectContent::Ellipse(ellipse) => validate_ellipse(ellipse)?,
                ObjectContent::Image(image) => {
                    validate_asset_reference(image.asset, AssetKind::Image, &asset_kinds)?;
                }
                ObjectContent::Text(text) => validate_text(text)?,
            }

            for effect in &object.effects {
                register_entity_id(effect.id.get(), &mut seen_ids, &mut max_entity_id)?;
                validate_effect(&effect.kind)?;
            }
        }

        if let Some(audio_track) = &self.audio_track {
            if !audio_track.gain.is_finite() {
                return Err(ProjectValidationError::NonFiniteValue("audio_track.gain"));
            }
            validate_asset_reference(
                audio_track.asset_id,
                AssetKind::Audio,
                &asset_kinds,
            )?;
        }

        if self.next_entity_id == 0 || self.next_entity_id <= max_entity_id {
            return Err(ProjectValidationError::InvalidNextEntityId);
        }

        Ok(())
    }
}

fn validate_settings(settings: &ProjectSettings) -> Result<(), ProjectValidationError> {
    let width_valid =
        (MIN_COMPOSITION_DIMENSION..=MAX_COMPOSITION_DIMENSION).contains(&settings.composition_width);
    let height_valid =
        (MIN_COMPOSITION_DIMENSION..=MAX_COMPOSITION_DIMENSION).contains(&settings.composition_height);

    if !width_valid || !height_valid {
        return Err(ProjectValidationError::InvalidCompositionDimensions);
    }

    if settings.duration.get() > MAX_PROJECT_DURATION_NS {
        return Err(ProjectValidationError::DurationTooLong);
    }

    Ok(())
}

fn register_entity_id(
    id: u64,
    seen_ids: &mut HashSet<u64>,
    max_entity_id: &mut u64,
) -> Result<(), ProjectValidationError> {
    if !seen_ids.insert(id) {
        return Err(ProjectValidationError::DuplicateEntityId(id));
    }

    *max_entity_id = (*max_entity_id).max(id);
    Ok(())
}

fn validate_asset_reference(
    asset_id: AssetId,
    expected_kind: AssetKind,
    asset_kinds: &HashMap<AssetId, AssetKind>,
) -> Result<(), ProjectValidationError> {
    let actual_kind =
        asset_kinds
            .get(&asset_id)
            .copied()
            .ok_or(ProjectValidationError::MissingAssetReference {
                asset_id: asset_id.get(),
                expected_kind,
            })?;

    if actual_kind != expected_kind {
        return Err(ProjectValidationError::AssetKindMismatch {
            asset_id: asset_id.get(),
            expected_kind,
            actual_kind,
        });
    }

    Ok(())
}

fn validate_transform(transform: &TransformAnimation) -> Result<(), ProjectValidationError> {
    validate_finite(
        *transform.rotation_degrees.base_value(),
        "transform.rotation_degrees",
    )?;
    validate_finite(*transform.opacity.base_value(), "transform.opacity")?;
    validate_range(
        *transform.opacity.base_value(),
        0.0,
        1.0,
        "transform.opacity",
    )
}

fn validate_rectangle(rectangle: &RectangleObject) -> Result<(), ProjectValidationError> {
    validate_finite(
        *rectangle.corner_radius.base_value(),
        "rectangle.corner_radius",
    )
}

fn validate_ellipse(_ellipse: &EllipseObject) -> Result<(), ProjectValidationError> {
    Ok(())
}

fn validate_text(text: &TextObject) -> Result<(), ProjectValidationError> {
    validate_finite(text.font_size, "text.font_size")?;
    if text.font_size <= 0.0 {
        return Err(ProjectValidationError::ValueOutOfRange("text.font_size"));
    }
    Ok(())
}

fn validate_effect(effect: &EffectKind) -> Result<(), ProjectValidationError> {
    match effect {
        EffectKind::Blur(blur) => {
            validate_range(*blur.radius_px.base_value(), 0.0, 128.0, "blur.radius_px")
        }
        EffectKind::Glow(glow) => {
            validate_range(*glow.radius_px.base_value(), 0.0, 128.0, "glow.radius_px")?;
            validate_range(*glow.intensity.base_value(), 0.0, 4.0, "glow.intensity")?;
            validate_range(*glow.threshold.base_value(), 0.0, 1.0, "glow.threshold")
        }
        EffectKind::Tint(tint) => {
            validate_range(*tint.amount.base_value(), 0.0, 1.0, "tint.amount")
        }
        EffectKind::Noise(noise) => {
            validate_range(*noise.amount.base_value(), 0.0, 1.0, "noise.amount")?;
            validate_range(*noise.size_px.base_value(), 1.0, 256.0, "noise.size_px")?;
            validate_finite(*noise.evolution.base_value(), "noise.evolution")
        }
        EffectKind::RgbSplit(split) => {
            validate_range(*split.amount_px.base_value(), 0.0, 64.0, "rgb_split.amount_px")?;
            validate_finite(*split.angle_degrees.base_value(), "rgb_split.angle_degrees")
        }
    }
}

fn validate_finite(value: f32, field: &'static str) -> Result<(), ProjectValidationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ProjectValidationError::NonFiniteValue(field))
    }
}

fn validate_range(
    value: f32,
    min: f32,
    max: f32,
    field: &'static str,
) -> Result<(), ProjectValidationError> {
    validate_finite(value, field)?;
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(ProjectValidationError::ValueOutOfRange(field))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Composition {
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub id: ObjectId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub transform: TransformAnimation,
    pub content: ObjectContent,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectContent {
    Rectangle(RectangleObject),
    Ellipse(EllipseObject),
    Image(ImageObject),
    Text(TextObject),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectangleObject {
    pub size: Animated<Vec2>,
    pub fill: Animated<LinearRgba>,
    pub corner_radius: Animated<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EllipseObject {
    pub size: Animated<Vec2>,
    pub fill: Animated<LinearRgba>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageObject {
    pub asset: AssetId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontReference {
    pub family: String,
    pub weight: FontWeight,
    pub style: FontStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextObject {
    pub text: String,
    pub font: FontReference,
    pub font_size: f32,
    pub color: Animated<LinearRgba>,
    pub alignment: TextAlignment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub id: EffectId,
    pub enabled: bool,
    pub kind: EffectKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectKind {
    Blur(BlurEffect),
    Glow(GlowEffect),
    Tint(TintEffect),
    Noise(NoiseEffect),
    RgbSplit(RgbSplitEffect),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlurEffect {
    pub radius_px: Animated<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlowEffect {
    pub radius_px: Animated<f32>,
    pub intensity: Animated<f32>,
    pub threshold: Animated<f32>,
    pub color: Animated<LinearRgba>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TintEffect {
    pub color: Animated<LinearRgba>,
    pub amount: Animated<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseEffect {
    pub amount: Animated<f32>,
    pub size_px: Animated<f32>,
    pub evolution: Animated<f32>,
    pub seed: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RgbSplitEffect {
    pub amount_px: Animated<f32>,
    pub angle_degrees: Animated<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Audio,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetSource {
    File {
        path: String,
        relative_to_project: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRecord {
    pub id: AssetId,
    pub kind: AssetKind,
    pub source: AssetSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrack {
    pub asset_id: AssetId,
    pub gain: f32,
}

impl AudioTrack {
    #[must_use]
    pub const fn new(asset_id: AssetId) -> Self {
        Self {
            asset_id,
            gain: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSettings {
    pub composition_width: u32,
    pub composition_height: u32,
    pub frame_rate: FrameRate,
    pub duration: DurationNs,
    pub background: LinearRgba,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            composition_width: 1920,
            composition_height: 1080,
            frame_rate: FrameRate::new(60, 1).expect("60/1 is a valid frame rate"),
            duration: DurationNs::new(10_000_000_000),
            background: LinearRgba::black_opaque(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformAnimation {
    pub position: Animated<Vec2>,
    pub scale: Animated<Vec2>,
    pub rotation_degrees: Animated<f32>,
    pub anchor: Animated<Vec2>,
    pub opacity: Animated<f32>,
}

impl TransformAnimation {
    #[must_use]
    pub const fn new(
        position: Animated<Vec2>,
        scale: Animated<Vec2>,
        rotation_degrees: Animated<f32>,
        anchor: Animated<Vec2>,
        opacity: Animated<f32>,
    ) -> Self {
        Self {
            position,
            scale,
            rotation_degrees,
            anchor,
            opacity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AssetKind, AssetRecord, AssetSource, Effect, EffectKind, ImageObject, Object, ObjectContent,
        Project, ProjectSettings, ProjectValidationError, RectangleObject, TransformAnimation,
        MAX_PROJECT_DURATION_NS,
    };
    use crate::{
        animation::Animated,
        domain::{LinearRgba, Vec2},
        ids::{AssetId, EffectId, ObjectId},
        time::{DurationNs, GridOffsetNs, TempoMap},
    };

    fn transform() -> TransformAnimation {
        TransformAnimation::new(
            Animated::new_static(Vec2::new(960.0, 540.0).expect("finite position")),
            Animated::new_static(Vec2::new(1.0, 1.0).expect("finite scale")),
            Animated::new_static(0.0),
            Animated::new_static(Vec2::new(0.5, 0.5).expect("finite anchor")),
            Animated::new_static(1.0),
        )
    }

    fn rectangle_object(id: u64) -> Object {
        Object {
            id: ObjectId::new(id).expect("nonzero object id"),
            name: "Rectangle".to_owned(),
            visible: true,
            locked: false,
            transform: transform(),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 50.0).expect("finite size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(8.0),
            }),
            effects: Vec::new(),
        }
    }

    #[test]
    fn default_project_validates() {
        let project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );

        assert_eq!(project.validate(), Ok(()));
    }

    #[test]
    fn validation_rejects_invalid_dimensions_and_duration() {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.settings.composition_width = 15;
        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::InvalidCompositionDimensions)
        );

        project.settings.composition_width = 1920;
        project.settings.duration = DurationNs::new(MAX_PROJECT_DURATION_NS + 1);
        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::DurationTooLong)
        );
    }

    #[test]
    fn validation_rejects_duplicate_shared_ids_and_stale_allocator() {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.assets.push(AssetRecord {
            id: AssetId::new(5).expect("asset id"),
            kind: AssetKind::Image,
            source: AssetSource::File {
                path: "image.png".to_owned(),
                relative_to_project: true,
            },
        });
        project.composition.objects.push(rectangle_object(5));
        project.next_entity_id = 6;

        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::DuplicateEntityId(5))
        );

        project.composition.objects.clear();
        project.next_entity_id = 5;
        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::InvalidNextEntityId)
        );
    }

    #[test]
    fn validation_rejects_missing_and_mismatched_asset_references() {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let missing = AssetId::new(7).expect("asset id");
        let mut image = rectangle_object(1);
        image.content = ObjectContent::Image(ImageObject { asset: missing });
        project.composition.objects.push(image);
        project.next_entity_id = 8;

        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::MissingAssetReference {
                asset_id: 7,
                expected_kind: AssetKind::Image,
            })
        );

        project.assets.push(AssetRecord {
            id: missing,
            kind: AssetKind::Audio,
            source: AssetSource::File {
                path: "audio.wav".to_owned(),
                relative_to_project: true,
            },
        });
        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::AssetKindMismatch {
                asset_id: 7,
                expected_kind: AssetKind::Image,
                actual_kind: AssetKind::Audio,
            })
        );
    }

    #[test]
    fn validation_rejects_invalid_persisted_floats() {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let mut object = rectangle_object(1);
        object.transform.rotation_degrees = Animated::new_static(f32::NAN);
        project.composition.objects.push(object);
        project.next_entity_id = 2;

        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::NonFiniteValue(
                "transform.rotation_degrees"
            ))
        );

        project.composition.objects[0].transform.rotation_degrees = Animated::new_static(0.0);
        project.composition.objects[0].transform.opacity = Animated::new_static(1.5);
        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::ValueOutOfRange(
                "transform.opacity"
            ))
        );
    }

    #[test]
    fn validation_rejects_invalid_effect_ranges() {
        let mut project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let mut object = rectangle_object(1);
        object.effects.push(Effect {
            id: EffectId::new(2).expect("effect id"),
            enabled: true,
            kind: EffectKind::Blur(super::BlurEffect {
                radius_px: Animated::new_static(129.0),
            }),
        });
        project.composition.objects.push(object);
        project.next_entity_id = 3;

        assert_eq!(
            project.validate(),
            Err(ProjectValidationError::ValueOutOfRange("blur.radius_px"))
        );
    }

    #[test]
    fn asset_and_audio_track_schema_match_mvp_contract() {
        let asset_id = AssetId::new(11).expect("nonzero asset id");
        let asset = AssetRecord {
            id: asset_id,
            kind: AssetKind::Audio,
            source: AssetSource::File {
                path: "audio/song.flac".to_owned(),
                relative_to_project: true,
            },
        };
        let track = super::AudioTrack::new(asset_id);

        assert_eq!(asset.kind, AssetKind::Audio);
        assert_eq!(track.asset_id, asset_id);
        assert_eq!(track.gain, 1.0);
        assert!(matches!(
            asset.source,
            AssetSource::File {
                relative_to_project: true,
                ..
            }
        ));
    }

    #[test]
    fn effect_variants_use_typed_mvp_parameter_structs() {
        let black = LinearRgba::black_opaque();

        let blur = EffectKind::Blur(super::BlurEffect {
            radius_px: Animated::new_static(12.0),
        });
        let glow = EffectKind::Glow(super::GlowEffect {
            radius_px: Animated::new_static(16.0),
            intensity: Animated::new_static(1.5),
            threshold: Animated::new_static(0.25),
            color: Animated::new_static(black),
        });
        let tint = EffectKind::Tint(super::TintEffect {
            color: Animated::new_static(black),
            amount: Animated::new_static(0.5),
        });
        let noise = EffectKind::Noise(super::NoiseEffect {
            amount: Animated::new_static(0.2),
            size_px: Animated::new_static(4.0),
            evolution: Animated::new_static(2.0),
            seed: 42,
        });
        let split = EffectKind::RgbSplit(super::RgbSplitEffect {
            amount_px: Animated::new_static(8.0),
            angle_degrees: Animated::new_static(45.0),
        });

        assert!(matches!(blur, EffectKind::Blur(_)));
        assert!(matches!(glow, EffectKind::Glow(_)));
        assert!(matches!(tint, EffectKind::Tint(_)));
        assert!(matches!(noise, EffectKind::Noise(_)));
        assert!(matches!(split, EffectKind::RgbSplit(_)));
    }

    #[test]
    fn font_reference_defaults_to_normal_upright_style() {
        assert_eq!(super::FontWeight::default(), super::FontWeight::Normal);
        assert_eq!(super::FontStyle::default(), super::FontStyle::Normal);
    }

    #[test]
    fn text_object_separates_static_layout_from_animated_color() {
        let text = super::TextObject {
            text: "Привет, rhythm".to_owned(),
            font: super::FontReference {
                family: "Inter".to_owned(),
                weight: super::FontWeight::SemiBold,
                style: super::FontStyle::Italic,
            },
            font_size: 48.0,
            color: Animated::new_static(LinearRgba::black_opaque()),
            alignment: super::TextAlignment::Center,
        };

        assert_eq!(text.text, "Привет, rhythm");
        assert_eq!(text.font.family, "Inter");
        assert_eq!(text.font.weight, super::FontWeight::SemiBold);
        assert_eq!(text.font.style, super::FontStyle::Italic);
        assert_eq!(text.font_size, 48.0);
        assert_eq!(text.alignment, super::TextAlignment::Center);
        assert_eq!(*text.color.base_value(), LinearRgba::black_opaque());
    }

    #[test]
    fn image_object_stores_only_asset_identity() {
        let asset = AssetId::new(7).expect("nonzero asset id");
        let image = ImageObject { asset };

        assert_eq!(image.asset, asset);
    }

    #[test]
    fn rectangle_and_ellipse_store_only_accepted_mvp_fields() {
        let size = Animated::new_static(Vec2::new(100.0, 50.0).expect("finite size"));
        let fill = Animated::new_static(
            LinearRgba::new(1.0, 0.0, 0.0, 1.0).expect("finite color"),
        );

        let rectangle = RectangleObject {
            size: size.clone(),
            fill: fill.clone(),
            corner_radius: Animated::new_static(8.0),
        };
        let ellipse = super::EllipseObject { size, fill };

        assert_eq!(rectangle.size.base_value().x(), 100.0);
        assert_eq!(*rectangle.corner_radius.base_value(), 8.0);
        assert_eq!(ellipse.size.base_value().y(), 50.0);
    }

    #[test]
    fn new_project_root_starts_with_empty_creative_collections() {
        let project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );

        assert_eq!(project.metadata.name, "Untitled");
        assert!(project.audio_track.is_none());
        assert!(project.assets.is_empty());
        assert!(project.composition.objects.is_empty());
        assert_eq!(project.next_entity_id, 1);
    }

    #[test]
    fn project_settings_match_mvp_defaults() {
        let settings = ProjectSettings::default();

        assert_eq!(settings.composition_width, 1920);
        assert_eq!(settings.composition_height, 1080);
        assert_eq!(settings.frame_rate.numerator(), 60);
        assert_eq!(settings.frame_rate.denominator(), 1);
        assert_eq!(settings.duration.get(), 10_000_000_000);
        assert_eq!(settings.background, LinearRgba::black_opaque());
    }

    #[test]
    fn transform_animation_exposes_the_accepted_semantic_fields() {
        let transform = transform();

        assert_eq!(transform.position.base_value().x(), 960.0);
        assert_eq!(transform.scale.base_value().x(), 1.0);
        assert_eq!(*transform.rotation_degrees.base_value(), 0.0);
        assert_eq!(transform.anchor.base_value().x(), 0.5);
        assert_eq!(*transform.opacity.base_value(), 1.0);
    }
}
