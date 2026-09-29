//! Deterministic semantic reference scene for the five MVP effects.
//!
//! Six separately identified swatches: each of Blur/Glow/Tint/Noise/RGB Split,
//! followed by the same five kinds in one ordered stack. This is a fixture
//! for future GPU screenshot comparisons; it does not claim to rasterize
//! pixels in the current placeholder preview.

use rhythm_core::{
    animation::Animated,
    domain::{LinearRgba, Vec2},
    ids::{EffectId, ObjectId},
    project::{
        BlurEffect, Effect, EffectKind, GlowEffect, NoiseEffect, Object, ObjectContent, Project,
        ProjectSettings, RectangleObject, RgbSplitEffect, TintEffect, TransformAnimation,
    },
    time::{GridOffsetNs, ProjectTimeNs, TempoMap},
};

pub struct ReferenceScene {
    pub project: Project,
    pub evaluation_time: ProjectTimeNs,
}

fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> LinearRgba {
    LinearRgba::new(red, green, blue, alpha).expect("valid reference swatch color")
}

fn transform(x: f32, y: f32) -> TransformAnimation {
    TransformAnimation::new(
        Animated::new_static(Vec2::new(x, y).expect("reference position")),
        Animated::new_static(Vec2::new(1.0, 1.0).expect("unit scale")),
        Animated::new_static(0.0),
        Animated::new_static(Vec2::new(0.5, 0.5).expect("center anchor")),
        Animated::new_static(1.0),
    )
}

fn reference_effects() -> [EffectKind; 5] {
    [
        EffectKind::Blur(BlurEffect {
            radius_px: Animated::new_static(24.0),
        }),
        EffectKind::Glow(GlowEffect {
            radius_px: Animated::new_static(32.0),
            intensity: Animated::new_static(1.5),
            threshold: Animated::new_static(0.6),
            color: Animated::new_static(rgba(1.0, 0.5, 0.1, 0.9)),
        }),
        EffectKind::Tint(TintEffect {
            color: Animated::new_static(rgba(0.1, 0.8, 1.0, 0.75)),
            amount: Animated::new_static(0.4),
        }),
        EffectKind::Noise(NoiseEffect {
            amount: Animated::new_static(0.25),
            size_px: Animated::new_static(8.0),
            evolution: Animated::new_static(0.375),
            seed: 0x51A7_2026,
        }),
        EffectKind::RgbSplit(RgbSplitEffect {
            amount_px: Animated::new_static(12.0),
            angle_degrees: Animated::new_static(30.0),
        }),
    ]
}

/// Each swatch uses a deliberately semitransparent fill, exposing edges and
/// compositing behavior. The final swatch validates effect ordering, not only
/// the isolated rendering of five unrelated kinds.
#[must_use]
pub fn five_effects_reference_scene() -> ReferenceScene {
    let mut project = Project::new(
        "Five Effects Reference",
        ProjectSettings::default(),
        TempoMap::unset(GridOffsetNs::new(0)),
    );
    let effects = reference_effects();
    let names = ["Blur", "Glow", "Tint", "Noise", "RGB Split"];
    let positions = [
        (190.0, 320.0),
        (485.0, 320.0),
        (780.0, 320.0),
        (1075.0, 320.0),
        (1370.0, 320.0),
    ];
    let fills = [
        rgba(1.0, 0.35, 0.12, 0.8),
        rgba(0.9, 0.8, 0.3, 0.65),
        rgba(0.45, 0.2, 0.9, 0.75),
        rgba(0.5, 0.7, 0.8, 0.7),
        rgba(0.3, 0.9, 0.55, 0.6),
    ];

    for (index, effect) in effects.iter().enumerate() {
        let index_id = u64::try_from(index).expect("five swatches");
        project.composition.objects.push(Object {
            id: ObjectId::new(index_id + 1).expect("nonzero object ID"),
            name: names[index].into(),
            visible: true,
            locked: false,
            transform: transform(positions[index].0, positions[index].1),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(250.0, 240.0).expect("positive size")),
                fill: Animated::new_static(fills[index]),
                corner_radius: Animated::new_static(18.0),
            }),
            effects: vec![Effect {
                id: EffectId::new(index_id + 101).expect("nonzero effect ID"),
                enabled: true,
                kind: effect.clone(),
            }],
        });
    }

    project.composition.objects.push(Object {
        id: ObjectId::new(6).expect("nonzero object ID"),
        name: "Five Effects Ordered Stack".into(),
        visible: true,
        locked: false,
        transform: transform(960.0, 790.0),
        content: ObjectContent::Rectangle(RectangleObject {
            size: Animated::new_static(Vec2::new(560.0, 250.0).expect("positive size")),
            fill: Animated::new_static(rgba(0.8, 0.65, 0.4, 0.7)),
            corner_radius: Animated::new_static(20.0),
        }),
        effects: effects
            .into_iter()
            .enumerate()
            .map(|(index, kind)| Effect {
                id: EffectId::new(u64::try_from(index).expect("five effects") + 106)
                    .expect("nonzero effect ID"),
                enabled: true,
                kind,
            })
            .collect(),
    });

    project.next_entity_id = 111;
    ReferenceScene {
        project,
        evaluation_time: ProjectTimeNs::new(0),
    }
}

#[cfg(test)]
mod tests {
    use super::five_effects_reference_scene;
    use crate::scene_eval::{EvaluatedEffectKind, EvaluatedObjectContent, evaluate_scene};
    use rhythm_core::serialization::{parse_project_file_v1, serialize_project_file_v1};

    #[test]
    fn five_effects_fixture_is_valid_and_serialization_stable() {
        let scene = five_effects_reference_scene();
        scene.project.validate().expect("valid semantic fixture");
        assert_eq!(scene.project.settings.composition_width, 1920);
        assert_eq!(scene.project.settings.composition_height, 1080);
        assert_eq!(scene.project.composition.objects.len(), 6);

        let bytes = serialize_project_file_v1(&scene.project).expect("serialize fixture");
        let parsed = parse_project_file_v1(&bytes).expect("round trip");
        assert_eq!(parsed.project, scene.project);
        assert_eq!(
            five_effects_reference_scene().project,
            scene.project,
            "reference project must be deterministic"
        );
    }

    #[test]
    fn reference_scene_has_exact_golden_effect_order_and_parameters() {
        let scene = five_effects_reference_scene();
        let evaluated = evaluate_scene(&scene.project, scene.evaluation_time).expect("evaluate");
        assert_eq!(evaluated.objects.len(), 6);
        assert_eq!(
            evaluated
                .objects
                .iter()
                .map(|object| object.id.get())
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6]
        );
        assert_eq!(
            evaluated
                .objects
                .iter()
                .map(|object| object.effects.len())
                .collect::<Vec<_>>(),
            [1, 1, 1, 1, 1, 5]
        );
        assert_eq!(
            evaluated.objects[5]
                .effects
                .iter()
                .map(|effect| effect.id.get())
                .collect::<Vec<_>>(),
            [106, 107, 108, 109, 110]
        );
        assert!(matches!(
            &evaluated.objects[0].effects[0].kind,
            EvaluatedEffectKind::Blur { radius_px: 24.0 }
        ));
        assert!(matches!(
            &evaluated.objects[1].effects[0].kind,
            EvaluatedEffectKind::Glow {
                radius_px: 32.0,
                intensity: 1.5,
                threshold: 0.6,
                ..
            }
        ));
        assert!(matches!(
            &evaluated.objects[2].effects[0].kind,
            EvaluatedEffectKind::Tint { amount: 0.4, .. }
        ));
        assert!(matches!(
            &evaluated.objects[3].effects[0].kind,
            EvaluatedEffectKind::Noise {
                amount: 0.25,
                size_px: 8.0,
                evolution: 0.375,
                seed: 0x51A7_2026
            }
        ));
        assert!(matches!(
            &evaluated.objects[4].effects[0].kind,
            EvaluatedEffectKind::RgbSplit {
                amount_px: 12.0,
                angle_degrees: 30.0
            }
        ));

        for (isolated, stacked) in evaluated.objects[..5]
            .iter()
            .zip(evaluated.objects[5].effects.iter())
        {
            assert_eq!(isolated.effects[0].kind, stacked.kind);
        }
        for object in &evaluated.objects {
            let EvaluatedObjectContent::Rectangle { fill, .. } = &object.content else {
                panic!("fixture object must be a rectangle");
            };
            assert!(fill.a() > 0.0 && fill.a() < 1.0);
        }
    }

    #[test]
    fn static_reference_values_do_not_depend_on_editor_playback_clock() {
        let scene = five_effects_reference_scene();
        let at_zero = evaluate_scene(&scene.project, scene.evaluation_time).expect("initial");
        let at_second = evaluate_scene(
            &scene.project,
            rhythm_core::time::ProjectTimeNs::new(1_000_000_000),
        )
        .expect("later");
        assert_eq!(at_zero, at_second);
    }
}
