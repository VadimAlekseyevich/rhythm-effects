# Effects

> **Status: Accepted for MVP**

## 1. Final effect set

Schema V1 contains exactly:

1. Blur
2. Glow
3. Tint
4. Noise
5. RGB Split

Additional effects are post-MVP.

## 2. Model

~~~rust
Effect {
    id: EffectId,
    enabled: bool,
    kind: EffectKind,
}
~~~

Typed enum variants only.

Effect order is semantic and deterministic.

## 3. Units

Spatial parameters are composition pixels.

Renderer compensates for preview-resolution scale.

All numeric values are finite and validated.

## 4. Blur

~~~text
radius_px: Animated<f32>, 0..128
~~~

Use separable horizontal/vertical Gaussian-like blur.

Radius zero is identity.

Implementation may optimize kernel/downsample behavior without changing radius semantics.

## 5. Glow

~~~text
radius_px: Animated<f32>, 0..128
intensity: Animated<f32>, 0..4
threshold: Animated<f32>, 0..1
color: Animated<LinearRgba>
~~~

Concept:

~~~text
source
-> threshold mask
-> blur
-> color/intensity
-> additive composite over source
~~~

## 6. Tint

~~~text
color: Animated<LinearRgba>
amount: Animated<f32>, 0..1
~~~

Amount zero is identity.

Shader behavior is covered by visual reference tests.

## 7. Noise

~~~text
amount: Animated<f32>, 0..1
size_px: Animated<f32>, 1..256
evolution: Animated<f32>
seed: u32
~~~

Noise is deterministic and depends only on semantic inputs, never wall clock or mutable RNG state.

## 8. RGB Split

~~~text
amount_px: Animated<f32>, 0..64
angle_degrees: Animated<f32>
~~~

Channels sample deterministic signed offsets along the angle.

Alpha behavior is stable and tested to avoid colored transparent fringes.

## 9. Isolation

Blur/Glow require isolated targets.

Tint/Noise/RGB Split may be single pass but still obey stack order.

Compatible-pass fusion is only a later optimization.

## 10. Temporary resources

Use renderer texture pool.

No per-frame texture create/destroy on normal path.

## 11. Animation

All animated effect parameters use the same Animated<T> and timeline system as transforms.

No effect-specific animation model exists.

## 12. Inspector

Each effect is one shallow group:

- title;
- enabled;
- reorder;
- remove;
- parameters.

No nested tabs.

## 13. Determinism

Effects do not depend on editor FPS, wall clock, prior frame, or global random state.

Feedback/temporal effects are post-MVP.

## 14. Tests

- identity at zero where defined;
- bounds;
- order;
- preview-scale parity;
- deterministic Noise;
- alpha edges;
- linear-light blur/glow;
- preview/export parity.

## 15. Definition of Done

All five effects render, animate, preserve order, reuse intermediates, remain deterministic, and meet representative performance gates.
