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
    time::{DurationNs, FrameRate},
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
