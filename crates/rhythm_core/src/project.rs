use crate::{
    animation::Animated,
    domain::Vec2,
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
    use crate::{
        animation::Animated,
        domain::Vec2,
    };

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
