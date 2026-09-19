# Animation Engine

> **Status: Draft**
>
> The animation engine evaluates user-authored musical keyframes into continuous visual values. It is independent from editor widgets, GPU state, and audio backend details.

## 1. Responsibilities

The animation engine owns:

- animated property representation;
- keyframe ordering and lookup;
- interpolation/easing;
- evaluation at arbitrary continuous time;
- deterministic behavior used by preview and export.

It does not own:

- playback;
- timeline UI;
- project mutation history;
- GPU rendering.

---

## 2. Canonical keyframe time

Keyframes are stored at MusicalTick positions.

~~~rust
struct Keyframe<T> {
    id: KeyframeId,
    tick: MusicalTick,
    value: T,
    interpolation: Interpolation,
}
~~~

Invariant:

- keyframes sorted by tick;
- no duplicate tick per property;
- tick is integer musical time;
- no raw seconds stored as keyframe identity.

---

## 3. Animated property

~~~rust
struct Animated<T> {
    base_value: T,
    keyframes: Vec<Keyframe<T>>,
}
~~~

MVP property types:

- scalar f32;
- Vec2;
- Color;
- possibly discrete enum/bool values with Hold semantics.

Do not create one animation system per object type.

---

## 4. Evaluation input

Evaluation accepts continuous project time.

Flow:

~~~text
ProjectTime
→ TempoMap
→ continuous musical tick position
→ surrounding authored keyframes
→ interpolation
→ value
~~~

The evaluator may represent the continuous tick coordinate as f64 internally, but persisted keyframe positions remain integers.

---

## 5. Lookup

For sorted keyframes:

- zero keys: return base_value;
- one key: return key value according to before/after policy;
- multiple keys: binary-search surrounding pair.

Use binary search rather than scanning from start every frame.

Future optimization may cache last segment during forward playback, but correctness must not depend on monotonic time because scrubbing seeks arbitrarily.

---

## 6. Before-first and after-last behavior

MVP proposal:

- before first keyframe: hold first keyframe value;
- after last keyframe: hold last keyframe value;
- no keyframes: use base_value.

This is predictable and matches common animation expectations.

Changing this later would affect project semantics, so finalize before Accepted status.

---

## 7. Segment progress

For keyframes A and B:

~~~text
u = (current_musical_tick - A.tick) / (B.tick - A.tick)
~~~

Clamp u to 0..1 for segment evaluation.

Important: progress is defined in **musical space**.

This preserves the meaning of "halfway between beat 1 and beat 2" even if future tempo changes alter real-time duration.

At constant BPM, musical-space and absolute-time progress are equivalent.

---

## 8. Interpolation enum

Concept:

~~~rust
enum Interpolation {
    Hold,
    Linear,
    CubicBezier(BezierEasing),
}
~~~

Preset easing is stored as canonical Bezier parameters or mapped to them.

UI presets:

- Linear;
- Ease In;
- Ease Out;
- Ease In-Out.

Do not store preset names if they can be represented by stable curve parameters unless product semantics require preserving preset identity.

---

## 9. Hold

For A→B with Hold:

~~~text
value(t) = A.value for t < B.tick
value(B.tick) = B.value
~~~

This is useful for beat-synchronized cuts/flashes.

---

## 10. Linear

Scalar:

~~~text
lerp(a, b, u)
~~~

Vec2:

~~~text
lerp each component
~~~

Color interpolation must use the project's defined working color representation, not UI color types.

---

## 11. Cubic Bezier easing

Bezier easing maps linear segment progress u to eased progress e(u).

The curve is a timing curve, not an arbitrary spatial value curve in MVP.

Conceptual control points:

~~~text
(0,0), (x1,y1), (x2,y2), (1,1)
~~~

Need a robust solver for x→y mapping.

Requirements:

- deterministic;
- bounded input;
- no NaN;
- handles degenerate but valid curves;
- tested against known presets.

---

## 12. Rotation interpolation

MVP simplest rule:

- rotation stored as scalar degrees;
- interpolate numeric value directly;
- do not automatically choose shortest angular path.

Reason: motion designers may intentionally animate multiple rotations (e.g. 0°→720°).

---

## 13. Scale

Recommended internal convention:

~~~text
1.0 = 100%
~~~

Inspector may display percent.

Negative scale behavior should be either explicitly supported or clamped; decide before implementation.

---

## 14. Opacity

Internal convention:

~~~text
0.0 = transparent
1.0 = opaque
~~~

Project mutation validates/clamps according to final UX policy.

Renderer receives evaluated opacity.

---

## 15. Colors

Color animation should interpolate in one documented working space.

For MVP, avoid complex perceptual interpolation modes.

Renderer/color pipeline document determines exact space.

The animation engine must not depend on wgpu texture formats.

---

## 16. Discrete values

Properties that should not interpolate use Hold/discrete semantics.

Examples:

- visibility if ever animated;
- enum-like mode values.

Do not force every property through scalar interpolation.

---

## 17. Evaluation result

Renderer should not independently query keyframes.

A scene evaluation stage resolves animation and produces render-facing values.

Concept:

~~~rust
EvaluatedObject {
    id,
    transform,
    content,
    effects,
}
~~~

This separates authoring model from render model.

---

## 18. Scene evaluation

Conceptual flow:

~~~text
for visible object:
    evaluate transform
    evaluate content properties
    evaluate effect properties
    produce EvaluatedObject
~~~

At MVP scale, straightforward iteration is preferred.

Optimize only after profiling.

---

## 19. Caching

Potential safe caches:

- last binary-search segment per property during monotonic playback;
- precomputed Bezier curve coefficients;
- static property detection.

Rules:

- cache is rebuildable;
- scrubbing invalidates assumptions as needed;
- cache never changes semantic output.

---

## 20. Static properties

An Animated<T> with zero keyframes is static.

Renderer/evaluator may avoid repeated expensive processing for static content, especially:

- text layout;
- image fit geometry;
- effect pipeline decisions.

Do not prematurely create a generic reactive dependency graph.

---

## 21. Keyframe IDs vs ticks

KeyframeId is entity identity for selection/history.

MusicalTick is temporal position.

Do not use tick as unique entity ID because moving a keyframe would change its identity.

---

## 22. Editing collisions

Animation model enforces one keyframe per tick per property.

Collision policy belongs to editor/commands, but after any committed command the engine must see a valid sorted unique sequence.

---

## 23. Determinism

Given:

- same Project;
- same evaluation ProjectTime;
- same engine version/semantics;

evaluation should return the same values independent of:

- editor FPS;
- playback history;
- mouse position;
- previous evaluation time.

This is essential for export parity.

---

## 24. Precision

Use f64 for time-domain calculations.

Visual values can generally use f32.

Reason:

- long-duration time conversion benefits from f64;
- GPU/render data naturally uses f32;
- keyframe tick identity remains exact integer.

Do not convert project time to f32 early.

---

## 25. Error policy

Invalid project state should ideally be rejected before evaluation.

Evaluator should not panic on user-editable data.

Examples:

- empty keys: valid;
- duplicate ticks: validation error;
- NaN property values: invalid project/edit state;
- unknown future effect/object version: load/migration concern.

---

## 26. Required tests

- zero/one/multiple keyframes;
- exact keyframe hit returns exact value;
- before/after hold behavior;
- linear interpolation midpoint;
- musical-space interpolation;
- Hold transition;
- all easing presets;
- Bezier edge cases;
- rotation 0→720 preserves full rotation;
- arbitrary seek order produces same results;
- long tick ranges;
- negative ticks if supported;
- preview/export time inputs return identical evaluated scene.

---

## 27. Definition of Done

MVP animation engine is ready when:

- core types exist without UI/GPU dependencies;
- keyframe lookup is tested;
- Hold/Linear/preset easing work;
- custom Bezier works if included;
- scene evaluation handles all MVP object properties;
- evaluation is deterministic;
- renderer can consume evaluated values without reading editor state.
