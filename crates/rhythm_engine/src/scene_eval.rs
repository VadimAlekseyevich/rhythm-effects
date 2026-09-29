use rhythm_core::{
    animation::{
        Animated, Interpolation, evaluate_bezier_easing, interpolate_linear_f32,
        interpolate_linear_rgba, interpolate_linear_vec2, interpolate_rotation_degrees,
    },
    domain::{LinearRgba, Vec2},
    ids::{AssetId, EffectId, ObjectId},
    project::{
        BlurEffect, EffectKind, FontReference, GlowEffect, NoiseEffect, ObjectContent, Project,
        RgbSplitEffect, TextAlignment, TintEffect,
    },
    time::{ProjectTimeNs, TimeConversionError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneEvaluationError {
    TempoUnavailable,
    TimeOverflow,
}

impl From<TimeConversionError> for SceneEvaluationError {
    fn from(value: TimeConversionError) -> Self {
        match value {
            TimeConversionError::TempoUnavailable => Self::TempoUnavailable,
            TimeConversionError::Overflow | TimeConversionError::NonFiniteTickPosition => {
                Self::TimeOverflow
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedScene {
    pub objects: Vec<EvaluatedObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedObject {
    pub id: ObjectId,
    pub transform: EvaluatedTransform,
    pub content: EvaluatedObjectContent,
    pub effects: Vec<EvaluatedEffect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvaluatedTransform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation_degrees: f32,
    pub anchor: Vec2,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvaluatedObjectContent {
    Rectangle {
        size: Vec2,
        fill: LinearRgba,
        corner_radius: f32,
    },
    Ellipse {
        size: Vec2,
        fill: LinearRgba,
    },
    Image {
        asset: AssetId,
    },
    Text {
        text: String,
        font: FontReference,
        font_size: f32,
        color: LinearRgba,
        alignment: TextAlignment,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedEffect {
    pub id: EffectId,
    pub kind: EvaluatedEffectKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvaluatedEffectKind {
    Blur {
        radius_px: f32,
    },
    Glow {
        radius_px: f32,
        intensity: f32,
        threshold: f32,
        color: LinearRgba,
    },
    Tint {
        color: LinearRgba,
        amount: f32,
    },
    Noise {
        amount: f32,
        size_px: f32,
        evolution: f32,
        seed: u32,
    },
    RgbSplit {
        amount_px: f32,
        angle_degrees: f32,
    },
}

pub fn evaluate_scene(
    project: &Project,
    project_time: ProjectTimeNs,
) -> Result<EvaluatedScene, SceneEvaluationError> {
    let continuous_tick = project
        .tempo_map
        .continuous_tick_position(project_time)
        .ok();

    let mut objects = Vec::with_capacity(project.composition.objects.len());

    for object in &project.composition.objects {
        if !object.visible {
            continue;
        }

        let transform = EvaluatedTransform {
            position: evaluate_vec2(&object.transform.position, continuous_tick)?,
            scale: evaluate_vec2(&object.transform.scale, continuous_tick)?,
            rotation_degrees: evaluate_rotation(
                &object.transform.rotation_degrees,
                continuous_tick,
            )?,
            anchor: evaluate_vec2(&object.transform.anchor, continuous_tick)?,
            opacity: evaluate_f32(&object.transform.opacity, continuous_tick)?,
        };

        let content = match &object.content {
            ObjectContent::Rectangle(rectangle) => EvaluatedObjectContent::Rectangle {
                size: evaluate_vec2(&rectangle.size, continuous_tick)?,
                fill: evaluate_rgba(&rectangle.fill, continuous_tick)?,
                corner_radius: evaluate_f32(&rectangle.corner_radius, continuous_tick)?,
            },
            ObjectContent::Ellipse(ellipse) => EvaluatedObjectContent::Ellipse {
                size: evaluate_vec2(&ellipse.size, continuous_tick)?,
                fill: evaluate_rgba(&ellipse.fill, continuous_tick)?,
            },
            ObjectContent::Image(image) => EvaluatedObjectContent::Image { asset: image.asset },
            ObjectContent::Text(text) => EvaluatedObjectContent::Text {
                text: text.text.clone(),
                font: text.font.clone(),
                font_size: text.font_size,
                color: evaluate_rgba(&text.color, continuous_tick)?,
                alignment: text.alignment,
            },
        };

        let mut effects = Vec::with_capacity(object.effects.len());
        for effect in &object.effects {
            if !effect.enabled {
                continue;
            }

            effects.push(EvaluatedEffect {
                id: effect.id,
                kind: evaluate_effect(&effect.kind, continuous_tick)?,
            });
        }

        objects.push(EvaluatedObject {
            id: object.id,
            transform,
            content,
            effects,
        });
    }

    Ok(EvaluatedScene { objects })
}

fn evaluate_effect(
    effect: &EffectKind,
    continuous_tick: Option<f64>,
) -> Result<EvaluatedEffectKind, SceneEvaluationError> {
    match effect {
        EffectKind::Blur(BlurEffect { radius_px }) => Ok(EvaluatedEffectKind::Blur {
            radius_px: evaluate_f32(radius_px, continuous_tick)?,
        }),
        EffectKind::Glow(GlowEffect {
            radius_px,
            intensity,
            threshold,
            color,
        }) => Ok(EvaluatedEffectKind::Glow {
            radius_px: evaluate_f32(radius_px, continuous_tick)?,
            intensity: evaluate_f32(intensity, continuous_tick)?,
            threshold: evaluate_f32(threshold, continuous_tick)?,
            color: evaluate_rgba(color, continuous_tick)?,
        }),
        EffectKind::Tint(TintEffect { color, amount }) => Ok(EvaluatedEffectKind::Tint {
            color: evaluate_rgba(color, continuous_tick)?,
            amount: evaluate_f32(amount, continuous_tick)?,
        }),
        EffectKind::Noise(NoiseEffect {
            amount,
            size_px,
            evolution,
            seed,
        }) => Ok(EvaluatedEffectKind::Noise {
            amount: evaluate_f32(amount, continuous_tick)?,
            size_px: evaluate_f32(size_px, continuous_tick)?,
            evolution: evaluate_f32(evolution, continuous_tick)?,
            seed: *seed,
        }),
        EffectKind::RgbSplit(RgbSplitEffect {
            amount_px,
            angle_degrees,
        }) => Ok(EvaluatedEffectKind::RgbSplit {
            amount_px: evaluate_f32(amount_px, continuous_tick)?,
            angle_degrees: evaluate_rotation(angle_degrees, continuous_tick)?,
        }),
    }
}

fn require_tick<T>(
    animated: &Animated<T>,
    continuous_tick: Option<f64>,
) -> Result<Option<f64>, SceneEvaluationError> {
    if animated.keyframes().is_empty() {
        Ok(None)
    } else {
        continuous_tick
            .map(Some)
            .ok_or(SceneEvaluationError::TempoUnavailable)
    }
}

fn eased_progress(interpolation: Interpolation, progress: f64) -> f64 {
    match interpolation {
        Interpolation::Hold => 0.0,
        Interpolation::Linear => progress,
        Interpolation::CubicBezier(easing) => evaluate_bezier_easing(easing, progress),
    }
}

fn evaluate_f32(
    animated: &Animated<f32>,
    continuous_tick: Option<f64>,
) -> Result<f32, SceneEvaluationError> {
    let Some(tick) = require_tick(animated, continuous_tick)? else {
        return Ok(*animated.base_value());
    };

    if let Some(value) = animated.range_value(tick) {
        return Ok(*value);
    }

    let segment = animated
        .segment_at(tick)
        .expect("interior non-key tick must resolve to a segment");
    let progress = eased_progress(segment.from.interpolation, segment.progress);

    Ok(interpolate_linear_f32(
        segment.from.value,
        segment.to.value,
        progress,
    ))
}

fn evaluate_rotation(
    animated: &Animated<f32>,
    continuous_tick: Option<f64>,
) -> Result<f32, SceneEvaluationError> {
    let Some(tick) = require_tick(animated, continuous_tick)? else {
        return Ok(*animated.base_value());
    };

    if let Some(value) = animated.range_value(tick) {
        return Ok(*value);
    }

    let segment = animated
        .segment_at(tick)
        .expect("interior non-key tick must resolve to a segment");
    let progress = eased_progress(segment.from.interpolation, segment.progress);

    Ok(interpolate_rotation_degrees(
        segment.from.value,
        segment.to.value,
        progress,
    ))
}

fn evaluate_vec2(
    animated: &Animated<Vec2>,
    continuous_tick: Option<f64>,
) -> Result<Vec2, SceneEvaluationError> {
    let Some(tick) = require_tick(animated, continuous_tick)? else {
        return Ok(*animated.base_value());
    };

    if let Some(value) = animated.range_value(tick) {
        return Ok(*value);
    }

    let segment = animated
        .segment_at(tick)
        .expect("interior non-key tick must resolve to a segment");
    let progress = eased_progress(segment.from.interpolation, segment.progress);

    Ok(interpolate_linear_vec2(
        segment.from.value,
        segment.to.value,
        progress,
    ))
}

fn evaluate_rgba(
    animated: &Animated<LinearRgba>,
    continuous_tick: Option<f64>,
) -> Result<LinearRgba, SceneEvaluationError> {
    let Some(tick) = require_tick(animated, continuous_tick)? else {
        return Ok(*animated.base_value());
    };

    if let Some(value) = animated.range_value(tick) {
        return Ok(*value);
    }

    let segment = animated
        .segment_at(tick)
        .expect("interior non-key tick must resolve to a segment");
    let progress = eased_progress(segment.from.interpolation, segment.progress);

    Ok(interpolate_linear_rgba(
        segment.from.value,
        segment.to.value,
        progress,
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        EvaluatedEffectKind, EvaluatedObjectContent, SceneEvaluationError, evaluate_effect,
        evaluate_scene,
    };
    use rhythm_core::{
        animation::{Animated, Interpolation, Keyframe},
        domain::{LinearRgba, Vec2},
        ids::{KeyframeId, ObjectId},
        project::{
            BlurEffect, EffectKind, GlowEffect, NoiseEffect, Object, ObjectContent, Project,
            ProjectSettings, RectangleObject, RgbSplitEffect, TintEffect, TransformAnimation,
        },
        time::{BpmMicros, GridOffsetNs, MusicalTick, ProjectTimeNs, TempoMap, TimeSignature},
    };

    fn transform(position: Animated<Vec2>) -> TransformAnimation {
        TransformAnimation::new(
            position,
            Animated::new_static(Vec2::new(1.0, 1.0).expect("finite scale")),
            Animated::new_static(0.0),
            Animated::new_static(Vec2::new(0.5, 0.5).expect("finite anchor")),
            Animated::new_static(1.0),
        )
    }

    #[test]
    fn evaluated_scene_preserves_painter_order_and_skips_hidden_objects() {
        let tempo = TempoMap::with_initial_tempo(
            GridOffsetNs::new(0),
            BpmMicros::new(120_000_000).expect("valid BPM"),
            TimeSignature::default(),
        );
        let mut project = Project::new("test", ProjectSettings::default(), tempo);

        let animated_position = Animated::with_keyframes(
            Vec2::new(0.0, 0.0).expect("finite position"),
            vec![
                Keyframe::new(
                    KeyframeId::new(2).expect("keyframe id"),
                    MusicalTick::new(0),
                    Vec2::new(0.0, 0.0).expect("finite position"),
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(3).expect("keyframe id"),
                    MusicalTick::new(960),
                    Vec2::new(100.0, 50.0).expect("finite position"),
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("unique keyframes");

        for (id, visible, position) in [
            (1, true, animated_position),
            (
                4,
                false,
                Animated::new_static(Vec2::new(10.0, 10.0).expect("finite position")),
            ),
            (
                5,
                true,
                Animated::new_static(Vec2::new(20.0, 20.0).expect("finite position")),
            ),
        ] {
            project.composition.objects.push(Object {
                id: ObjectId::new(id).expect("object id"),
                name: format!("object {id}"),
                visible,
                locked: false,
                transform: transform(position),
                content: ObjectContent::Rectangle(RectangleObject {
                    size: Animated::new_static(Vec2::new(100.0, 50.0).expect("finite size")),
                    fill: Animated::new_static(LinearRgba::black_opaque()),
                    corner_radius: Animated::new_static(0.0),
                }),
                effects: Vec::new(),
            });
        }

        let scene =
            evaluate_scene(&project, ProjectTimeNs::new(250_000_000)).expect("scene evaluation");
        assert_eq!(scene.objects.len(), 2);
        assert_eq!(scene.objects[0].id.get(), 1);
        assert_eq!(scene.objects[1].id.get(), 5);
        assert_eq!(scene.objects[0].transform.position.x(), 50.0);
        assert_eq!(scene.objects[0].transform.position.y(), 25.0);
        assert!(matches!(
            scene.objects[0].content,
            EvaluatedObjectContent::Rectangle { .. }
        ));
    }

    #[test]
    fn static_scene_evaluates_without_tempo_but_keyframes_require_tempo() {
        let mut project = Project::new(
            "test",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id: ObjectId::new(1).expect("object id"),
            name: "static".to_owned(),
            visible: true,
            locked: false,
            transform: transform(Animated::new_static(
                Vec2::new(10.0, 20.0).expect("finite position"),
            )),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 50.0).expect("finite size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });

        let static_scene =
            evaluate_scene(&project, ProjectTimeNs::new(0)).expect("static scene without BPM");
        assert_eq!(static_scene.objects[0].transform.position.x(), 10.0);

        project.composition.objects[0].transform.position = Animated::with_keyframes(
            Vec2::new(0.0, 0.0).expect("finite position"),
            vec![Keyframe::new(
                KeyframeId::new(2).expect("keyframe id"),
                MusicalTick::new(0),
                Vec2::new(1.0, 2.0).expect("finite position"),
                Interpolation::Linear,
            )],
        )
        .expect("unique keyframe");

        assert_eq!(
            evaluate_scene(&project, ProjectTimeNs::new(0)),
            Err(SceneEvaluationError::TempoUnavailable)
        );
    }
    fn animated_scalar(id: u64, start: f32, end: f32) -> Animated<f32> {
        Animated::with_keyframes(
            start,
            vec![
                Keyframe::new(
                    KeyframeId::new(id).expect("start key id"),
                    MusicalTick::new(0),
                    start,
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(id + 1).expect("end key id"),
                    MusicalTick::new(960),
                    end,
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("valid scalar keys")
    }

    fn color(r: f32, g: f32, b: f32, a: f32) -> LinearRgba {
        LinearRgba::new(r, g, b, a).expect("finite color")
    }

    fn animated_color(id: u64, start: LinearRgba, end: LinearRgba) -> Animated<LinearRgba> {
        Animated::with_keyframes(
            start,
            vec![
                Keyframe::new(
                    KeyframeId::new(id).expect("start color id"),
                    MusicalTick::new(0),
                    start,
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(id + 1).expect("end color id"),
                    MusicalTick::new(960),
                    end,
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("valid color keys")
    }

    fn near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn every_documented_effect_parameter_uses_shared_animated_evaluation() {
        let mid = Some(480.0);
        assert_eq!(
            evaluate_effect(
                &EffectKind::Blur(BlurEffect {
                    radius_px: animated_scalar(100, 0.0, 128.0),
                }),
                mid
            ),
            Ok(EvaluatedEffectKind::Blur { radius_px: 64.0 })
        );

        let first_color = color(0.2, 0.0, 0.0, 0.5);
        let second_color = color(0.6, 1.0, 0.8, 1.0);
        let glow = evaluate_effect(
            &EffectKind::Glow(GlowEffect {
                radius_px: animated_scalar(110, 16.0, 32.0),
                intensity: animated_scalar(112, 0.0, 4.0),
                threshold: animated_scalar(114, 0.2, 0.8),
                color: animated_color(116, first_color, second_color),
            }),
            mid,
        )
        .expect("animated glow");
        let EvaluatedEffectKind::Glow {
            radius_px,
            intensity,
            threshold,
            color: glow_color,
        } = glow
        else {
            panic!("expected Glow");
        };
        near(radius_px, 24.0);
        near(intensity, 2.0);
        near(threshold, 0.5);
        near(glow_color.r(), 0.4);
        near(glow_color.g(), 0.5);
        near(glow_color.b(), 0.4);
        near(glow_color.a(), 0.75);

        let tint = evaluate_effect(
            &EffectKind::Tint(TintEffect {
                color: animated_color(120, first_color, second_color),
                amount: animated_scalar(122, 0.0, 1.0),
            }),
            mid,
        )
        .expect("animated tint");
        let EvaluatedEffectKind::Tint { color, amount } = tint else {
            panic!("expected Tint");
        };
        near(amount, 0.5);
        near(color.r(), 0.4);
        near(color.g(), 0.5);
        near(color.b(), 0.4);
        near(color.a(), 0.75);

        let noise = evaluate_effect(
            &EffectKind::Noise(NoiseEffect {
                amount: animated_scalar(130, 0.0, 1.0),
                size_px: animated_scalar(132, 1.0, 255.0),
                evolution: animated_scalar(134, -2.0, 2.0),
                seed: 47,
            }),
            mid,
        )
        .expect("animated noise");
        let EvaluatedEffectKind::Noise {
            amount,
            size_px,
            evolution,
            seed,
        } = noise
        else {
            panic!("expected Noise");
        };
        near(amount, 0.5);
        near(size_px, 128.0);
        near(evolution, 0.0);
        assert_eq!(seed, 47, "seed remains a nonanimated semantic parameter");

        let rgb = evaluate_effect(
            &EffectKind::RgbSplit(RgbSplitEffect {
                amount_px: animated_scalar(140, 0.0, 64.0),
                angle_degrees: animated_scalar(142, 0.0, 270.0),
            }),
            mid,
        )
        .expect("animated RGB Split");
        let EvaluatedEffectKind::RgbSplit {
            amount_px,
            angle_degrees,
        } = rgb
        else {
            panic!("expected RGB Split");
        };
        near(amount_px, 32.0);
        near(angle_degrees, 135.0);
    }

    #[test]
    fn static_effects_evaluate_without_tempo_but_animated_effects_require_it() {
        let static_tint = EffectKind::Tint(TintEffect {
            color: Animated::new_static(color(0.5, 0.3, 0.1, 1.0)),
            amount: Animated::new_static(0.75),
        });
        assert_eq!(
            evaluate_effect(&static_tint, None),
            Ok(EvaluatedEffectKind::Tint {
                color: color(0.5, 0.3, 0.1, 1.0),
                amount: 0.75,
            })
        );
        assert_eq!(
            evaluate_effect(
                &EffectKind::Noise(NoiseEffect {
                    amount: Animated::new_static(0.1),
                    size_px: animated_scalar(150, 4.0, 12.0),
                    evolution: Animated::new_static(0.0),
                    seed: 5,
                }),
                None,
            ),
            Err(SceneEvaluationError::TempoUnavailable)
        );
    
}
