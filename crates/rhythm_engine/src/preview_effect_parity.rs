//! Pure semantic parity assertions shared by Preview Full/Half/Quarter.
//! GPU screenshot parity remains a separate hardware-backed reference check.

use crate::{
    blur::preview_blur_radius,
    glow::{GlowParameters, preview_glow_parameters},
    noise::{NoiseParameters, noise_cell, preview_noise_parameters},
    reference_scenes::five_effects_reference_scene,
    rgb_split::{RgbSplitParameters, channel_offsets, preview_rgb_split_parameters},
    scene_eval::{EvaluatedEffectKind, evaluate_scene},
};

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.0001,
        "{actual} not within tolerance of {expected}"
    );
}

#[test]
fn all_five_effects_preserve_composition_semantics_at_three_preview_qualities() {
    let fixture = five_effects_reference_scene();
    let before = fixture.project.clone();
    let scene = evaluate_scene(&fixture.project, fixture.evaluation_time).expect("evaluated");
    let stack = &scene.objects[5].effects;
    assert_eq!(stack.len(), 5);

    let EvaluatedEffectKind::Blur { radius_px: blur } = &stack[0].kind else {
        panic!("first is Blur");
    };
    let EvaluatedEffectKind::Glow {
        radius_px: glow_radius,
        intensity,
        threshold,
        color: glow_color,
    } = &stack[1].kind
    else {
        panic!("second is Glow");
    };
    let glow = GlowParameters {
        radius_px: *glow_radius,
        intensity: *intensity,
        threshold: *threshold,
        color: *glow_color,
    };
    let EvaluatedEffectKind::Tint {
        color: tint,
        amount,
    } = &stack[2].kind
    else {
        panic!("third is Tint");
    };
    let original_tint = (*tint, *amount);
    let EvaluatedEffectKind::Noise {
        amount: noise_amount,
        size_px,
        evolution,
        seed,
    } = &stack[3].kind
    else {
        panic!("fourth is Noise");
    };
    let noise = NoiseParameters {
        amount: *noise_amount,
        size_px: *size_px,
        evolution: *evolution,
        seed: *seed,
    };
    let EvaluatedEffectKind::RgbSplit {
        amount_px,
        angle_degrees,
    } = &stack[4].kind
    else {
        panic!("fifth is RGB Split");
    };
    let split = RgbSplitParameters {
        amount_px: *amount_px,
        angle_degrees: *angle_degrees,
    };
    let full_offsets = channel_offsets(split).expect("full offset");

    for scale in [1.0_f32, 0.5, 0.25] {
        close(
            preview_blur_radius(*blur, scale).expect("valid Blur"),
            24.0 * scale,
        );

        let preview_glow = preview_glow_parameters(glow, scale).expect("valid Glow");
        close(preview_glow.radius_px, 32.0 * scale);
        assert_eq!(preview_glow.intensity, glow.intensity);
        assert_eq!(preview_glow.threshold, glow.threshold);
        assert_eq!(preview_glow.color, glow.color);

        // Tint is non-spatial: preview scale must not alter its linear color
        // or blend amount, nor change the semantic EvaluatedScene.
        assert_eq!((*tint, *amount), original_tint);

        let preview_noise = preview_noise_parameters(noise, scale).expect("valid Noise");
        close(preview_noise.size_px, noise.size_px * scale);
        assert_eq!(preview_noise.amount, noise.amount);
        assert_eq!(preview_noise.evolution, noise.evolution);
        assert_eq!(preview_noise.seed, noise.seed);
        // A composition coordinate mapped to the working preview must stay
        // in the same deterministic noise cell at exact power-of-two scales.
        let mapped_x = (48.0 * scale) as u32;
        let mapped_y = (72.0 * scale) as u32;
        assert_eq!(
            noise_cell(mapped_x, mapped_y, preview_noise.size_px).expect("working cell"),
            noise_cell(48, 72, noise.size_px).expect("composition cell")
        );

        let preview_split = preview_rgb_split_parameters(split, scale).expect("valid split");
        close(preview_split.amount_px, split.amount_px * scale);
        assert_eq!(preview_split.angle_degrees, split.angle_degrees);
        let working_offsets = channel_offsets(preview_split).expect("working offsets");
        for channel in 0..2 {
            for axis in 0..2 {
                close(working_offsets.0[axis] / scale, full_offsets.0[axis]);
                close(working_offsets.1[axis] / scale, full_offsets.1[axis]);
            }
            assert_eq!(working_offsets.0[channel], -working_offsets.1[channel]);
        }
    }
    assert_eq!(
        fixture.project, before,
        "preview never changes creative project"
    );
}

#[test]
fn zero_spatial_effects_remain_identity_at_every_preview_scale() {
    for scale in [1.0_f32, 0.5, 0.25] {
        assert_eq!(preview_blur_radius(0.0, scale), Ok(0.0));
        assert_eq!(
            preview_glow_parameters(
                GlowParameters {
                    radius_px: 0.0,
                    intensity: 0.0,
                    threshold: 0.6,
                    color: rhythm_core::domain::LinearRgba::black_opaque(),
                },
                scale
            )
            .expect("valid glow")
            .radius_px,
            0.0
        );
        assert_eq!(
            preview_rgb_split_parameters(
                RgbSplitParameters {
                    amount_px: 0.0,
                    angle_degrees: 30.0,
                },
                scale
            )
            .expect("valid split")
            .amount_px,
            0.0
        );
    }
}

#[test]
fn invalid_preview_scale_is_rejected_by_each_spatial_parameter_path() {
    use crate::{
        blur::BlurPreviewScaleError, glow::GlowPreviewScaleError, noise::NoiseError,
        rgb_split::RgbSplitError,
    };

    for scale in [0.0_f32, -0.25, 1.25, f32::NAN, f32::INFINITY] {
        assert_eq!(
            preview_blur_radius(24.0, scale),
            Err(BlurPreviewScaleError::InvalidPreviewScale)
        );
        assert_eq!(
            preview_glow_parameters(
                GlowParameters {
                    radius_px: 32.0,
                    intensity: 1.5,
                    threshold: 0.6,
                    color: rhythm_core::domain::LinearRgba::black_opaque(),
                },
                scale
            ),
            Err(GlowPreviewScaleError::InvalidPreviewScale)
        );
        assert_eq!(
            preview_noise_parameters(
                NoiseParameters {
                    amount: 0.25,
                    size_px: 8.0,
                    evolution: 0.375,
                    seed: 8,
                },
                scale
            ),
            Err(NoiseError::InvalidPreviewScale)
        );
        assert_eq!(
            preview_rgb_split_parameters(
                RgbSplitParameters {
                    amount_px: 12.0,
                    angle_degrees: 30.0,
                },
                scale
            ),
            Err(RgbSplitError::InvalidPreviewScale)
        );
    }
}
