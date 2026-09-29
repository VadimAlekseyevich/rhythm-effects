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

Effect order is semantic and deterministic. AI-258 provides the effect
execution boundary: an evaluated enabled effect list is visited strictly in
stack order, each pass receives the previous output and a distinct pooled
destination (ping-pong), and the last output goes to isolated compositing.
The previous source is released only once its pass is encoded. With an empty
list there is no intermediate allocation or shader call. The pass encoder is
a callback supplied by the actual effect implementation; AI-259 onward
supply individual shader algorithms. This scheduling boundary itself does
not pretend that the later filter shaders are already implemented.

## 3. Units

Spatial parameters are composition pixels.

Renderer compensates for preview-resolution scale.

All numeric values are finite and validated.

## 4. Blur

~~~text
radius_px: Animated<f32>, 0..128
~~~

Use separable horizontal/vertical Gaussian-like blur.

AI-259 supplies the WGSL implementation with cached Rgba16Float pipeline:
an isolated source is sampled horizontally into a distinct pooled temporary,
then vertically into the caller's output. The uniform kernel is symmetric,
Gaussian-like and normalized per pass, with up to 128 taps on each side and
a clamped radius of 0..128. RGB and alpha are convolved together in
linear-premultiplied representation, never sampled in-place. Radius zero
uses an identity sample in both passes. AI-260 converts the validated composition-pixel radius to the working
preview scale before GPU encoding: Full uses 1.0, Half 0.5, Quarter 0.25,
and Auto uses its resolved scale. Invalid/nonfinite scales or radii are
rejected without encoding. The project value is never rewritten; export
continues to use full-resolution radius. Unit tests cover identity,
Full/Half/Quarter equivalence and rejected invalid inputs.
This GPU blur encoder is now available to the effect-chain callback; later
render-loop wiring and visual reference tests remain separate from the
shader implementation.

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

AI-261 exposes `Renderer::encode_glow`: the threshold shader measures
straight, linear-light luminance (guarding zero alpha) and writes a
premultiplied alpha-weighted bright-region mask. The existing separable Blur
filters that mask in two axes using pooled intermediate Rgba16Float textures.
The final shader adds `color.rgb * blurred_coverage * color.a * intensity`
to the unmodified source's linear-premultiplied RGB, retaining HDR working
headroom; output alpha uses clamped coverage-over-original union. With zero
intensity or transparent mask, the blend is identity. Radius is 0..128,
threshold 0..1 and intensity 0..4, all validated before texture allocation.
No scene composition is sampled and intermediate checkouts are released only
after their final encoded uses. AI-262 provides `preview_glow_parameters`
and the renderer's `encode_preview_glow` entry point, scaling only Glow
radius by the resolved preview factor (Full 1, Half 0.5, Quarter 0.25)
after validating original bounds. Color, intensity and threshold are
unchanged; export uses composition-pixel radius at scale 1. Invalid input
is rejected before GPU encoding. GPU visual reference work is tracked later;
the current main window still uses placeholder composition.

## 6. Tint

~~~text
color: Animated<LinearRgba>
amount: Animated<f32>, 0..1
~~~

AI-263 implements the Rgba16Float Tint shader with a cached pipeline.
The effective tint factor is `amount * color.a`; it mixes source linear
premultiplied RGB with `color.rgb * source.a`, preserving source alpha.
Thus amount zero and transparent tint are identity, while fully transparent
input cannot acquire colored fringes. Amount is validated in 0..1 and color
channels are checked before any GPU pass is encoded. Unit tests cover
identity, opacity mixing, nonfinite/out-of-range input and alpha edge
behavior. GPU visual references are scheduled separately in AI-270.

Amount zero is identity.

## 7. Noise

~~~text
amount: Animated<f32>, 0..1
size_px: Animated<f32>, 1..256
evolution: Animated<f32>
seed: u32
~~~

Noise is deterministic and depends only on semantic inputs, never wall clock or mutable RNG state.

AI-264 adds one shared stateless integer avalanche hash with reference Rust
and embeddable WGSL implementations. Its only inputs are `seed`, the exact
`evolution` f32 bit pattern and integer pixel/grid x/y coordinates.
32-bit arithmetic wraps; only the low 24 bits map to [0,1]. Stable
reference vectors, repeatability across traversal orders and source code
checks protect this contract. Evolution changes are intentionally a
deterministic new pattern, not an FPS-dependent feedback process. AI-265
uses this function to implement amount and pixel-size rendering semantics.

AI-265 implements that WGSL pass: amount 0..1 adds centered monochrome
noise to RGB multiplied by original source alpha, never modifying alpha;
zero amount and transparent pixels are identity. size_px 1..256 groups
the working texture into repeatable rectangular blocks that sample one
seeded hash per block, with no frame dependence. The shader uses a single
cached pipeline and no GPU readback. A preview helper scales the creative
size by the resolved quality factor, with a one-working-pixel floor.
Amount, evolution and seed remain unchanged. Tests cover block boundaries,
intensity behavior, alpha retention, zero identity, preview sizing and
input validation. End-to-end effect pass integration and GPU references
remain separate from these renderer APIs.

## 8. RGB Split

~~~text
amount_px: Animated<f32>, 0..64
angle_degrees: Animated<f32>
~~~

Channels sample deterministic signed offsets along the angle.

AI-266 adds the Rgba16Float RGB Split shader: red samples along positive
`(cos(angle), sin(angle)) * amount_px`, blue along the negative displacement,
and green at the unshifted pixel. Top-left-origin composition coordinates
make positive angles clockwise. Its sampled colors are unpremultiplied
before reweighting with the original center alpha, ensuring neighboring
transparent pixels do not receive shifted RGB. Radius/amount is validated
in 0..64 and the angle must be finite; zero amount returns the center
sample exactly. The preview entry point scales amount by the resolved
quality factor without changing creative units. AI-267 adds targeted transparent-edge regression tests: displaced opaque
red/blue samples cannot color a zero-alpha center; low-alpha shifted
samples never introduce fringes; every output preserves original alpha
and channels remain premultiplied for ordinary nonnegative colors. Shader
contract checks assert displaced channels are reconstructed as straight
values then weighted by center alpha. GPU reference scenes remain AI-270.

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

AI-268 adds a single regression covering each evaluated Blur/Glow/Tint/Noise/
RGB Split parameter at an interpolated mid-beat time. Numeric and color
parameters use the shared interpolation path; RGB angle uses scalar rotation
rather than shortest-arc normalization. Noise's seed remains a static u32.
Static parameters can evaluate without an active tempo map; an animated
parameter requires musical-time resolution. No effect-specific animation
model exists.

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
