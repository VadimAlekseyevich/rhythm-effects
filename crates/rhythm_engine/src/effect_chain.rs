//! Order-preserving GPU effect-chain orchestration.
//!
//! Effect shaders are separate implementations (AI-259 onward). This runner
//! owns intermediate checkouts and guarantees every enabled evaluated effect
//! receives the previous pass output rather than the original object source.

use crate::{
    scene_eval::EvaluatedEffect,
    temporary_textures::{TemporaryTexture, TemporaryTexturePool},
};

fn visit_effects_in_order<T>(
    effects: &[EvaluatedEffect],
    initial: T,
    mut step: impl FnMut(&EvaluatedEffect, T) -> T,
) -> T {
    effects
        .iter()
        .fold(initial, |previous, effect| step(effect, previous))
}

/// Encode a strict left-to-right effect chain over an isolated object.
///
/// The callback must encode one pass that reads `source` and writes `output`,
/// without mutating the semantic effect list. The callback receives a
/// distinct output checkout and can acquire additional intermediates for
/// two-pass filters. Only after it has encoded the pass is the
/// previous input returned to the pool. The final checkout is returned to
/// the caller for composition, and an empty effect list returns `source`
/// unchanged without allocating intermediates.
pub fn encode_ordered_effect_chain(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    pool: &mut TemporaryTexturePool,
    source: TemporaryTexture,
    effects: &[EvaluatedEffect],
    mut encode_effect: impl FnMut(
        &mut wgpu::CommandEncoder,
        &EvaluatedEffect,
        &TemporaryTexture,
        &TemporaryTexture,
        &mut TemporaryTexturePool,
    ),
) -> TemporaryTexture {
    visit_effects_in_order(effects, source, |effect, input| {
        let output = pool.acquire(device, input.key());
        encode_effect(encoder, effect, &input, &output, pool);
        pool.release(input);
        output
    })
}

#[cfg(test)]
mod tests {
    use super::visit_effects_in_order;
    use crate::scene_eval::{EvaluatedEffect, EvaluatedEffectKind};
    use rhythm_core::{domain::LinearRgba, ids::EffectId};

    #[test]
    fn effect_chain_preserves_enabled_scene_order_and_previous_output() {
        let effects = [
            EvaluatedEffect {
                id: EffectId::new(5).expect("id"),
                kind: EvaluatedEffectKind::Tint {
                    color: LinearRgba::black_opaque(),
                    amount: 0.5,
                },
            },
            EvaluatedEffect {
                id: EffectId::new(3).expect("id"),
                kind: EvaluatedEffectKind::Blur { radius_px: 8.0 },
            },
            EvaluatedEffect {
                id: EffectId::new(9).expect("id"),
                kind: EvaluatedEffectKind::Noise {
                    amount: 0.3,
                    size_px: 2.0,
                    evolution: 0.0,
                    seed: 5,
                },
            },
        ];
        let mut calls = Vec::new();
        let result = visit_effects_in_order(&effects, String::from("object"), |effect, input| {
            calls.push((effect.id.get(), input.clone()));
            format!("{input}/{}", effect.id.get())
        });
        assert_eq!(
            calls,
            vec![
                (5, "object".into()),
                (3, "object/5".into()),
                (9, "object/5/3".into())
            ]
        );
        assert_eq!(result, "object/5/3/9");
    }

    #[test]
    fn zero_effects_keep_original_checkout_and_encode_nothing() {
        let mut calls = 0;
        let value = visit_effects_in_order(&[], 42_u32, |_, _| {
            calls += 1;
            0
        });
        assert_eq!(value, 42);
        assert_eq!(calls, 0);
    }
}
