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

#[derive(Debug, Clone, PartialEq)]
pub struct AssetRecord {
    pub id: AssetId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrack {
    pub asset_id: AssetId,
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

use crate::{
    animation::Animated,
    domain::{LinearRgba, Vec2},
    ids::{AssetId, EffectId, ObjectId},
    time::{DurationNs, FrameRate, TempoMap},
};

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
    use super::TransformAnimation;
    use crate::{animation::Animated, domain::Vec2};

    #[test]
    fn effect_variants_use_typed_mvp_parameter_structs() {
        let black = crate::domain::LinearRgba::black_opaque();

        let blur = super::EffectKind::Blur(super::BlurEffect {
            radius_px: Animated::new_static(12.0),
        });
        let glow = super::EffectKind::Glow(super::GlowEffect {
            radius_px: Animated::new_static(16.0),
            intensity: Animated::new_static(1.5),
            threshold: Animated::new_static(0.25),
            color: Animated::new_static(black),
        });
        let tint = super::EffectKind::Tint(super::TintEffect {
            color: Animated::new_static(black),
            amount: Animated::new_static(0.5),
        });
        let noise = super::EffectKind::Noise(super::NoiseEffect {
            amount: Animated::new_static(0.2),
            size_px: Animated::new_static(4.0),
            evolution: Animated::new_static(2.0),
            seed: 42,
        });
        let split = super::EffectKind::RgbSplit(super::RgbSplitEffect {
            amount_px: Animated::new_static(8.0),
            angle_degrees: Animated::new_static(45.0),
        });

        assert!(matches!(blur, super::EffectKind::Blur(_)));
        assert!(matches!(glow, super::EffectKind::Glow(_)));
        assert!(matches!(tint, super::EffectKind::Tint(_)));
        assert!(matches!(noise, super::EffectKind::Noise(_)));
        assert!(matches!(split, super::EffectKind::RgbSplit(_)));
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
            color: Animated::new_static(crate::domain::LinearRgba::black_opaque()),
            alignment: super::TextAlignment::Center,
        };

        assert_eq!(text.text, "Привет, rhythm");
        assert_eq!(text.font.family, "Inter");
        assert_eq!(text.font.weight, super::FontWeight::SemiBold);
        assert_eq!(text.font.style, super::FontStyle::Italic);
        assert_eq!(text.font_size, 48.0);
        assert_eq!(text.alignment, super::TextAlignment::Center);
        assert_eq!(*text.color.base_value(), crate::domain::LinearRgba::black_opaque());
    }

    #[test]
    fn image_object_stores_only_asset_identity() {
        let asset = crate::ids::AssetId::new(7).expect("nonzero asset id");
        let image = super::ImageObject { asset };

        assert_eq!(image.asset, asset);
    }

    #[test]
    fn rectangle_and_ellipse_store_only_accepted_mvp_fields() {
        let size = Animated::new_static(Vec2::new(100.0, 50.0).expect("finite size"));
        let fill = Animated::new_static(
            crate::domain::LinearRgba::new(1.0, 0.0, 0.0, 1.0).expect("finite color"),
        );

        let rectangle = super::RectangleObject {
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
        let project = super::Project::new(
            "Untitled",
            super::ProjectSettings::default(),
            crate::time::TempoMap::unset(crate::time::GridOffsetNs::new(0)),
        );

        assert_eq!(project.metadata.name, "Untitled");
        assert!(project.audio_track.is_none());
        assert!(project.assets.is_empty());
        assert!(project.composition.objects.is_empty());
        assert_eq!(project.next_entity_id, 1);
    }

    #[test]
    fn project_settings_match_mvp_defaults() {
        let settings = super::ProjectSettings::default();

        assert_eq!(settings.composition_width, 1920);
        assert_eq!(settings.composition_height, 1080);
        assert_eq!(settings.frame_rate.numerator(), 60);
        assert_eq!(settings.frame_rate.denominator(), 1);
        assert_eq!(settings.duration.get(), 10_000_000_000);
        assert_eq!(settings.background, crate::domain::LinearRgba::black_opaque());
    }

    #[test]
    fn transform_animation_exposes_the_accepted_semantic_fields() {
        let transform = TransformAnimation::new(
            Animated::new_static(Vec2::new(960.0, 540.0).expect("finite position")),
            Animated::new_static(Vec2::new(1.0, 1.0).expect("finite scale")),
            Animated::new_static(0.0),
            Animated::new_static(Vec2::new(0.5, 0.5).expect("finite anchor")),
            Animated::new_static(1.0),
        );

        assert_eq!(transform.position.base_value().x(), 960.0);
        assert_eq!(transform.scale.base_value().x(), 1.0);
        assert_eq!(*transform.rotation_degrees.base_value(), 0.0);
        assert_eq!(transform.anchor.base_value().x(), 0.5);
        assert_eq!(*transform.opacity.base_value(), 1.0);
    }
}
