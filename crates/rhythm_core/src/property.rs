use crate::{
    animation::{
        Animated, AnimationInvariantError, Interpolation, Keyframe, evaluate_bezier_easing,
        interpolate_linear_f32, interpolate_linear_rgba, interpolate_linear_vec2,
    },
    domain::{LinearRgba, Vec2},
    ids::{EffectId, KeyframeId, ObjectId},
    project::{EffectKind, Object, ObjectContent, Project},
    time::MusicalTick,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectAnimatableProperty {
    BlurRadius,
    GlowRadius,
    GlowIntensity,
    GlowThreshold,
    GlowColor,
    TintColor,
    TintAmount,
    NoiseAmount,
    NoiseSize,
    NoiseEvolution,
    RgbSplitAmount,
    RgbSplitAngle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimatableProperty {
    Position,
    Scale,
    Rotation,
    Anchor,
    Opacity,
    RectangleSize,
    RectangleFill,
    RectangleCornerRadius,
    EllipseSize,
    EllipseFill,
    TextColor,
    Effect {
        effect_id: EffectId,
        property: EffectAnimatableProperty,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyValue {
    Scalar(f32),
    Vec2(Vec2),
    Color(LinearRgba),
}

impl PropertyValue {
    #[must_use]
    pub const fn is_compatible_with(self, property: AnimatableProperty) -> bool {
        match (self, property) {
            (
                Self::Vec2(_),
                AnimatableProperty::Position
                | AnimatableProperty::Scale
                | AnimatableProperty::Anchor
                | AnimatableProperty::RectangleSize
                | AnimatableProperty::EllipseSize,
            ) => true,
            (
                Self::Color(_),
                AnimatableProperty::RectangleFill
                | AnimatableProperty::EllipseFill
                | AnimatableProperty::TextColor,
            ) => true,
            (Self::Color(_), AnimatableProperty::Effect { property, .. }) => matches!(
                property,
                EffectAnimatableProperty::GlowColor | EffectAnimatableProperty::TintColor
            ),
            (
                Self::Scalar(_),
                AnimatableProperty::Rotation
                | AnimatableProperty::Opacity
                | AnimatableProperty::RectangleCornerRadius,
            ) => true,
            (Self::Scalar(_), AnimatableProperty::Effect { property, .. }) => !matches!(
                property,
                EffectAnimatableProperty::GlowColor | EffectAnimatableProperty::TintColor
            ),
            _ => false,
        }
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        match self {
            Self::Scalar(value) => value.is_finite(),
            Self::Vec2(value) => value.x().is_finite() && value.y().is_finite(),
            Self::Color(value) => {
                value.r().is_finite()
                    && value.g().is_finite()
                    && value.b().is_finite()
                    && value.a().is_finite()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyKeyframe {
    pub id: KeyframeId,
    pub tick: MusicalTick,
    pub value: PropertyValue,
    pub interpolation: Interpolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyAccessError {
    ObjectNotFound(ObjectId),
    EffectNotFound(EffectId),
    PropertyUnavailable,
    IncompatibleValue,
    NonFiniteValue,
    NonFiniteTick,
    DuplicateTick(MusicalTick),
}

enum AnimatedRef<'a> {
    Scalar(&'a Animated<f32>),
    Vec2(&'a Animated<Vec2>),
    Color(&'a Animated<LinearRgba>),
}

enum AnimatedMut<'a> {
    Scalar(&'a mut Animated<f32>),
    Vec2(&'a mut Animated<Vec2>),
    Color(&'a mut Animated<LinearRgba>),
}

#[must_use]
pub fn property_value_compatible(
    property: AnimatableProperty,
    value: PropertyValue,
) -> bool {
    value.is_compatible_with(property) && value.is_finite()
}

pub fn property_base_value(
    project: &Project,
    object_id: ObjectId,
    property: AnimatableProperty,
) -> Result<PropertyValue, PropertyAccessError> {
    match animated_ref(project, object_id, property)? {
        AnimatedRef::Scalar(animated) => Ok(PropertyValue::Scalar(*animated.base_value())),
        AnimatedRef::Vec2(animated) => Ok(PropertyValue::Vec2(*animated.base_value())),
        AnimatedRef::Color(animated) => Ok(PropertyValue::Color(*animated.base_value())),
    }
}

pub fn property_keyframe_count(
    project: &Project,
    object_id: ObjectId,
    property: AnimatableProperty,
) -> Result<usize, PropertyAccessError> {
    Ok(match animated_ref(project, object_id, property)? {
        AnimatedRef::Scalar(animated) => animated.keyframes().len(),
        AnimatedRef::Vec2(animated) => animated.keyframes().len(),
        AnimatedRef::Color(animated) => animated.keyframes().len(),
    })
}

pub fn property_keyframe_at_tick(
    project: &Project,
    object_id: ObjectId,
    property: AnimatableProperty,
    tick: MusicalTick,
) -> Result<Option<PropertyKeyframe>, PropertyAccessError> {
    Ok(match animated_ref(project, object_id, property)? {
        AnimatedRef::Scalar(animated) => animated
            .keyframe_at_tick(tick)
            .map(|keyframe| pack_scalar_keyframe(keyframe)),
        AnimatedRef::Vec2(animated) => animated
            .keyframe_at_tick(tick)
            .map(|keyframe| pack_vec2_keyframe(keyframe)),
        AnimatedRef::Color(animated) => animated
            .keyframe_at_tick(tick)
            .map(|keyframe| pack_color_keyframe(keyframe)),
    })
}

pub fn evaluate_property_at_tick(
    project: &Project,
    object_id: ObjectId,
    property: AnimatableProperty,
    continuous_tick: f64,
) -> Result<PropertyValue, PropertyAccessError> {
    if !continuous_tick.is_finite() {
        return Err(PropertyAccessError::NonFiniteTick);
    }

    match animated_ref(project, object_id, property)? {
        AnimatedRef::Scalar(animated) => Ok(PropertyValue::Scalar(evaluate_scalar(
            animated,
            continuous_tick,
        ))),
        AnimatedRef::Vec2(animated) => Ok(PropertyValue::Vec2(evaluate_vec2(
            animated,
            continuous_tick,
        ))),
        AnimatedRef::Color(animated) => Ok(PropertyValue::Color(evaluate_color(
            animated,
            continuous_tick,
        ))),
    }
}

pub(crate) fn set_property_base_value(
    project: &mut Project,
    object_id: ObjectId,
    property: AnimatableProperty,
    value: PropertyValue,
) -> Result<(), PropertyAccessError> {
    if !property_value_compatible(property, value) {
        return Err(if value.is_finite() {
            PropertyAccessError::IncompatibleValue
        } else {
            PropertyAccessError::NonFiniteValue
        });
    }

    match (animated_mut(project, object_id, property)?, value) {
        (AnimatedMut::Scalar(animated), PropertyValue::Scalar(value)) => {
            *animated.base_value_mut() = value;
        }
        (AnimatedMut::Vec2(animated), PropertyValue::Vec2(value)) => {
            *animated.base_value_mut() = value;
        }
        (AnimatedMut::Color(animated), PropertyValue::Color(value)) => {
            *animated.base_value_mut() = value;
        }
        _ => return Err(PropertyAccessError::IncompatibleValue),
    }
    Ok(())
}

pub(crate) fn insert_property_keyframe(
    project: &mut Project,
    object_id: ObjectId,
    property: AnimatableProperty,
    keyframe: PropertyKeyframe,
) -> Result<(), PropertyAccessError> {
    if !property_value_compatible(property, keyframe.value) {
        return Err(if keyframe.value.is_finite() {
            PropertyAccessError::IncompatibleValue
        } else {
            PropertyAccessError::NonFiniteValue
        });
    }

    let result = match (animated_mut(project, object_id, property)?, keyframe.value) {
        (AnimatedMut::Scalar(animated), PropertyValue::Scalar(value)) => animated.insert_keyframe(
            Keyframe::new(keyframe.id, keyframe.tick, value, keyframe.interpolation),
        ),
        (AnimatedMut::Vec2(animated), PropertyValue::Vec2(value)) => animated.insert_keyframe(
            Keyframe::new(keyframe.id, keyframe.tick, value, keyframe.interpolation),
        ),
        (AnimatedMut::Color(animated), PropertyValue::Color(value)) => animated.insert_keyframe(
            Keyframe::new(keyframe.id, keyframe.tick, value, keyframe.interpolation),
        ),
        _ => return Err(PropertyAccessError::IncompatibleValue),
    };

    match result {
        Ok(_) => Ok(()),
        Err(AnimationInvariantError::DuplicateTick(tick)) => {
            Err(PropertyAccessError::DuplicateTick(tick))
        }
    }
}

pub(crate) fn remove_property_keyframe_at_tick(
    project: &mut Project,
    object_id: ObjectId,
    property: AnimatableProperty,
    tick: MusicalTick,
) -> Result<Option<PropertyKeyframe>, PropertyAccessError> {
    Ok(match animated_mut(project, object_id, property)? {
        AnimatedMut::Scalar(animated) => animated
            .remove_keyframe_at_tick(tick)
            .as_ref()
            .map(pack_scalar_keyframe),
        AnimatedMut::Vec2(animated) => animated
            .remove_keyframe_at_tick(tick)
            .as_ref()
            .map(pack_vec2_keyframe),
        AnimatedMut::Color(animated) => animated
            .remove_keyframe_at_tick(tick)
            .as_ref()
            .map(pack_color_keyframe),
    })
}

pub(crate) fn remove_property_keyframe_by_id(
    project: &mut Project,
    object_id: ObjectId,
    property: AnimatableProperty,
    keyframe_id: KeyframeId,
) -> Result<Option<PropertyKeyframe>, PropertyAccessError> {
    Ok(match animated_mut(project, object_id, property)? {
        AnimatedMut::Scalar(animated) => animated
            .remove_keyframe_by_id(keyframe_id)
            .as_ref()
            .map(pack_scalar_keyframe),
        AnimatedMut::Vec2(animated) => animated
            .remove_keyframe_by_id(keyframe_id)
            .as_ref()
            .map(pack_vec2_keyframe),
        AnimatedMut::Color(animated) => animated
            .remove_keyframe_by_id(keyframe_id)
            .as_ref()
            .map(pack_color_keyframe),
    })
}

fn animated_ref(
    project: &Project,
    object_id: ObjectId,
    property: AnimatableProperty,
) -> Result<AnimatedRef<'_>, PropertyAccessError> {
    let object = project
        .composition
        .objects
        .iter()
        .find(|object| object.id == object_id)
        .ok_or(PropertyAccessError::ObjectNotFound(object_id))?;
    animated_ref_for_object(object, property)
}

fn animated_mut(
    project: &mut Project,
    object_id: ObjectId,
    property: AnimatableProperty,
) -> Result<AnimatedMut<'_>, PropertyAccessError> {
    let object = project
        .composition
        .objects
        .iter_mut()
        .find(|object| object.id == object_id)
        .ok_or(PropertyAccessError::ObjectNotFound(object_id))?;
    animated_mut_for_object(object, property)
}

fn animated_ref_for_object(
    object: &Object,
    property: AnimatableProperty,
) -> Result<AnimatedRef<'_>, PropertyAccessError> {
    match property {
        AnimatableProperty::Position => Ok(AnimatedRef::Vec2(&object.transform.position)),
        AnimatableProperty::Scale => Ok(AnimatedRef::Vec2(&object.transform.scale)),
        AnimatableProperty::Rotation => Ok(AnimatedRef::Scalar(&object.transform.rotation_degrees)),
        AnimatableProperty::Anchor => Ok(AnimatedRef::Vec2(&object.transform.anchor)),
        AnimatableProperty::Opacity => Ok(AnimatedRef::Scalar(&object.transform.opacity)),
        AnimatableProperty::RectangleSize => match &object.content {
            ObjectContent::Rectangle(rectangle) => Ok(AnimatedRef::Vec2(&rectangle.size)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::RectangleFill => match &object.content {
            ObjectContent::Rectangle(rectangle) => Ok(AnimatedRef::Color(&rectangle.fill)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::RectangleCornerRadius => match &object.content {
            ObjectContent::Rectangle(rectangle) => Ok(AnimatedRef::Scalar(&rectangle.corner_radius)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::EllipseSize => match &object.content {
            ObjectContent::Ellipse(ellipse) => Ok(AnimatedRef::Vec2(&ellipse.size)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::EllipseFill => match &object.content {
            ObjectContent::Ellipse(ellipse) => Ok(AnimatedRef::Color(&ellipse.fill)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::TextColor => match &object.content {
            ObjectContent::Text(text) => Ok(AnimatedRef::Color(&text.color)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::Effect {
            effect_id,
            property,
        } => {
            let effect = object
                .effects
                .iter()
                .find(|effect| effect.id == effect_id)
                .ok_or(PropertyAccessError::EffectNotFound(effect_id))?;
            effect_animated_ref(&effect.kind, property)
        }
    }
}

fn animated_mut_for_object(
    object: &mut Object,
    property: AnimatableProperty,
) -> Result<AnimatedMut<'_>, PropertyAccessError> {
    match property {
        AnimatableProperty::Position => Ok(AnimatedMut::Vec2(&mut object.transform.position)),
        AnimatableProperty::Scale => Ok(AnimatedMut::Vec2(&mut object.transform.scale)),
        AnimatableProperty::Rotation => {
            Ok(AnimatedMut::Scalar(&mut object.transform.rotation_degrees))
        }
        AnimatableProperty::Anchor => Ok(AnimatedMut::Vec2(&mut object.transform.anchor)),
        AnimatableProperty::Opacity => Ok(AnimatedMut::Scalar(&mut object.transform.opacity)),
        AnimatableProperty::RectangleSize => match &mut object.content {
            ObjectContent::Rectangle(rectangle) => Ok(AnimatedMut::Vec2(&mut rectangle.size)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::RectangleFill => match &mut object.content {
            ObjectContent::Rectangle(rectangle) => Ok(AnimatedMut::Color(&mut rectangle.fill)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::RectangleCornerRadius => match &mut object.content {
            ObjectContent::Rectangle(rectangle) => {
                Ok(AnimatedMut::Scalar(&mut rectangle.corner_radius))
            }
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::EllipseSize => match &mut object.content {
            ObjectContent::Ellipse(ellipse) => Ok(AnimatedMut::Vec2(&mut ellipse.size)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::EllipseFill => match &mut object.content {
            ObjectContent::Ellipse(ellipse) => Ok(AnimatedMut::Color(&mut ellipse.fill)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::TextColor => match &mut object.content {
            ObjectContent::Text(text) => Ok(AnimatedMut::Color(&mut text.color)),
            _ => Err(PropertyAccessError::PropertyUnavailable),
        },
        AnimatableProperty::Effect {
            effect_id,
            property,
        } => {
            let effect = object
                .effects
                .iter_mut()
                .find(|effect| effect.id == effect_id)
                .ok_or(PropertyAccessError::EffectNotFound(effect_id))?;
            effect_animated_mut(&mut effect.kind, property)
        }
    }
}

fn effect_animated_ref(
    effect: &EffectKind,
    property: EffectAnimatableProperty,
) -> Result<AnimatedRef<'_>, PropertyAccessError> {
    match (effect, property) {
        (EffectKind::Blur(blur), EffectAnimatableProperty::BlurRadius) => {
            Ok(AnimatedRef::Scalar(&blur.radius_px))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowRadius) => {
            Ok(AnimatedRef::Scalar(&glow.radius_px))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowIntensity) => {
            Ok(AnimatedRef::Scalar(&glow.intensity))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowThreshold) => {
            Ok(AnimatedRef::Scalar(&glow.threshold))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowColor) => {
            Ok(AnimatedRef::Color(&glow.color))
        }
        (EffectKind::Tint(tint), EffectAnimatableProperty::TintColor) => {
            Ok(AnimatedRef::Color(&tint.color))
        }
        (EffectKind::Tint(tint), EffectAnimatableProperty::TintAmount) => {
            Ok(AnimatedRef::Scalar(&tint.amount))
        }
        (EffectKind::Noise(noise), EffectAnimatableProperty::NoiseAmount) => {
            Ok(AnimatedRef::Scalar(&noise.amount))
        }
        (EffectKind::Noise(noise), EffectAnimatableProperty::NoiseSize) => {
            Ok(AnimatedRef::Scalar(&noise.size_px))
        }
        (EffectKind::Noise(noise), EffectAnimatableProperty::NoiseEvolution) => {
            Ok(AnimatedRef::Scalar(&noise.evolution))
        }
        (EffectKind::RgbSplit(split), EffectAnimatableProperty::RgbSplitAmount) => {
            Ok(AnimatedRef::Scalar(&split.amount_px))
        }
        (EffectKind::RgbSplit(split), EffectAnimatableProperty::RgbSplitAngle) => {
            Ok(AnimatedRef::Scalar(&split.angle_degrees))
        }
        _ => Err(PropertyAccessError::PropertyUnavailable),
    }
}

fn effect_animated_mut(
    effect: &mut EffectKind,
    property: EffectAnimatableProperty,
) -> Result<AnimatedMut<'_>, PropertyAccessError> {
    match (effect, property) {
        (EffectKind::Blur(blur), EffectAnimatableProperty::BlurRadius) => {
            Ok(AnimatedMut::Scalar(&mut blur.radius_px))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowRadius) => {
            Ok(AnimatedMut::Scalar(&mut glow.radius_px))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowIntensity) => {
            Ok(AnimatedMut::Scalar(&mut glow.intensity))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowThreshold) => {
            Ok(AnimatedMut::Scalar(&mut glow.threshold))
        }
        (EffectKind::Glow(glow), EffectAnimatableProperty::GlowColor) => {
            Ok(AnimatedMut::Color(&mut glow.color))
        }
        (EffectKind::Tint(tint), EffectAnimatableProperty::TintColor) => {
            Ok(AnimatedMut::Color(&mut tint.color))
        }
        (EffectKind::Tint(tint), EffectAnimatableProperty::TintAmount) => {
            Ok(AnimatedMut::Scalar(&mut tint.amount))
        }
        (EffectKind::Noise(noise), EffectAnimatableProperty::NoiseAmount) => {
            Ok(AnimatedMut::Scalar(&mut noise.amount))
        }
        (EffectKind::Noise(noise), EffectAnimatableProperty::NoiseSize) => {
            Ok(AnimatedMut::Scalar(&mut noise.size_px))
        }
        (EffectKind::Noise(noise), EffectAnimatableProperty::NoiseEvolution) => {
            Ok(AnimatedMut::Scalar(&mut noise.evolution))
        }
        (EffectKind::RgbSplit(split), EffectAnimatableProperty::RgbSplitAmount) => {
            Ok(AnimatedMut::Scalar(&mut split.amount_px))
        }
        (EffectKind::RgbSplit(split), EffectAnimatableProperty::RgbSplitAngle) => {
            Ok(AnimatedMut::Scalar(&mut split.angle_degrees))
        }
        _ => Err(PropertyAccessError::PropertyUnavailable),
    }
}

fn pack_scalar_keyframe(keyframe: &Keyframe<f32>) -> PropertyKeyframe {
    PropertyKeyframe {
        id: keyframe.id,
        tick: keyframe.tick,
        value: PropertyValue::Scalar(keyframe.value),
        interpolation: keyframe.interpolation,
    }
}

fn pack_vec2_keyframe(keyframe: &Keyframe<Vec2>) -> PropertyKeyframe {
    PropertyKeyframe {
        id: keyframe.id,
        tick: keyframe.tick,
        value: PropertyValue::Vec2(keyframe.value),
        interpolation: keyframe.interpolation,
    }
}

fn pack_color_keyframe(keyframe: &Keyframe<LinearRgba>) -> PropertyKeyframe {
    PropertyKeyframe {
        id: keyframe.id,
        tick: keyframe.tick,
        value: PropertyValue::Color(keyframe.value),
        interpolation: keyframe.interpolation,
    }
}

fn eased_progress(interpolation: Interpolation, progress: f64) -> f64 {
    match interpolation {
        Interpolation::Hold => 0.0,
        Interpolation::Linear => progress,
        Interpolation::CubicBezier(easing) => evaluate_bezier_easing(easing, progress),
    }
}

fn evaluate_scalar(animated: &Animated<f32>, tick: f64) -> f32 {
    if animated.keyframes().is_empty() {
        return *animated.base_value();
    }
    if let Some(value) = animated.range_value(tick) {
        return *value;
    }

    let segment = animated
        .segment_at(tick)
        .expect("interior property tick resolves to segment");
    interpolate_linear_f32(
        segment.from.value,
        segment.to.value,
        eased_progress(segment.from.interpolation, segment.progress),
    )
}

fn evaluate_vec2(animated: &Animated<Vec2>, tick: f64) -> Vec2 {
    if animated.keyframes().is_empty() {
        return *animated.base_value();
    }
    if let Some(value) = animated.range_value(tick) {
        return *value;
    }

    let segment = animated
        .segment_at(tick)
        .expect("interior property tick resolves to segment");
    interpolate_linear_vec2(
        segment.from.value,
        segment.to.value,
        eased_progress(segment.from.interpolation, segment.progress),
    )
}

fn evaluate_color(animated: &Animated<LinearRgba>, tick: f64) -> LinearRgba {
    if animated.keyframes().is_empty() {
        return *animated.base_value();
    }
    if let Some(value) = animated.range_value(tick) {
        return *value;
    }

    let segment = animated
        .segment_at(tick)
        .expect("interior property tick resolves to segment");
    interpolate_linear_rgba(
        segment.from.value,
        segment.to.value,
        eased_progress(segment.from.interpolation, segment.progress),
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        animation::{Animated, Interpolation, Keyframe},
        domain::{LinearRgba, Vec2},
        ids::{KeyframeId, ObjectId},
        project::{
            Object, ObjectContent, Project, ProjectSettings, RectangleObject, TransformAnimation,
        },
        property::{AnimatableProperty, PropertyValue, evaluate_property_at_tick},
        time::{GridOffsetNs, MusicalTick, TempoMap},
    };

    #[test]
    fn property_evaluation_uses_existing_animation_semantics() {
        let object_id = ObjectId::new(1).expect("object id");
        let mut project = Project::new(
            "Property",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id: object_id,
            name: "Rect".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::with_keyframes(
                    1.0,
                    vec![
                        Keyframe::new(
                            KeyframeId::new(2).expect("key id"),
                            MusicalTick::new(0),
                            0.0,
                            Interpolation::Linear,
                        ),
                        Keyframe::new(
                            KeyframeId::new(3).expect("key id"),
                            MusicalTick::new(960),
                            1.0,
                            Interpolation::Linear,
                        ),
                    ],
                )
                .expect("keys"),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 50.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });

        assert_eq!(
            evaluate_property_at_tick(&project, object_id, AnimatableProperty::Opacity, 480.0),
            Ok(PropertyValue::Scalar(0.5))
        );
    }
}
