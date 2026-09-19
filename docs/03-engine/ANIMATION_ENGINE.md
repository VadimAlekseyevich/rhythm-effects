# Animation Engine

> **Status: Accepted for MVP**
>
> The animation engine evaluates musical keyframes into continuous values. It is independent from editor widgets, GPU state, and audio backend details.

## 1. Responsibilities

The engine owns:

- Animated value representation;
- keyframe ordering and lookup;
- interpolation and easing;
- deterministic evaluation at arbitrary project time;
- scene-value evaluation shared by preview and export.

It does not own playback, timeline UI, project mutation history, or GPU rendering.

## 2. Canonical property model

~~~rust
struct Animated<T> {
    base_value: T,
    keyframes: Vec<Keyframe<T>>,
}

struct Keyframe<T> {
    id: KeyframeId,
    tick: MusicalTick,
    value: T,
    interpolation: Interpolation,
}
~~~

Invariants:

- keyframes sorted by MusicalTick;
- no duplicate tick within one property;
- every keyframe has stable KeyframeId;
- keyframe identity is integer musical time;
- persisted values are finite and valid.

## 3. Supported MVP value classes

Core interpolation support:

- scalar f32;
- Vec2;
- LinearRgba;
- discrete values through Hold semantics.

Object and effect properties compose these value classes.

## 4. Evaluation input

Input is ProjectTimeNs.

~~~text
ProjectTimeNs
-> TempoMap
-> continuous musical position
-> surrounding keyframes
-> segment interpolation
-> value
~~~

Persistent keyframe positions remain integer ticks.

## 5. Lookup

Evaluation behavior:

- zero keyframes: base_value;
- one keyframe: that keyframe value;
- multiple keyframes: binary search surrounding segment.

Do not scan from the beginning every frame.

Playback-only segment caches are allowed later, but arbitrary seek order must always remain correct.

## 6. Range behavior

Accepted behavior:

~~~text
before first keyframe -> first keyframe value
exactly on keyframe  -> exact stored value
after last keyframe  -> last keyframe value
~~~

Once keyframes exist, base_value no longer defines out-of-range curve output.

## 7. Segment ownership

For adjacent keyframes A and B:

~~~text
A.interpolation
~~~

defines the transition from A to B.

B.interpolation defines B to the next keyframe.

The final keyframe may still store interpolation metadata even though it has no outgoing visible segment.

## 8. Musical progress

For A.tick < B.tick:

~~~text
u =
    (continuous_tick - A.tick)
    / (B.tick - A.tick)
~~~

Clamp u to [0,1].

Progress is measured in musical space.

This is intentional: future tempo changes preserve animation timing relative to beats rather than silently switching to absolute-time semantics.

## 9. Interpolation types

~~~rust
enum Interpolation {
    Hold,
    Linear,
    CubicBezier(BezierEasing),
}
~~~

UI presets such as Ease In, Ease Out, and Ease In-Out resolve to canonical Bezier parameters.

## 10. Hold

For A to B:

~~~text
time before B -> A.value
time at B     -> B.value
~~~

Hold is a first-class rhythm effect for cuts, flashes, and state changes.

## 11. Linear

Scalar:

~~~text
a + (b - a) * u
~~~

Vec2:

- interpolate each component.

LinearRgba:

- interpolate RGB components in linear-light representation;
- interpolate alpha linearly.

Do not interpolate user-facing sRGB bytes directly.

## 12. Cubic Bezier timing

Bezier changes timing progress, not spatial geometry.

Control points:

~~~text
P0 = (0,0)
P1 = (x1,y1)
P2 = (x2,y2)
P3 = (1,1)
~~~

Input is linear progress u.

The solver maps x progress to eased y progress e, then property interpolation uses e.

Requirements:

- deterministic;
- finite;
- robust near 0 and 1;
- tested against known presets;
- no NaN output.

For normal timing curves, x handles stay in [0,1].

## 13. Rotation

Rotation is scalar degrees.

No normalization and no automatic shortest path.

Examples:

~~~text
0 -> 360
one complete turn

0 -> 720
two complete turns
~~~

This preserves explicit motion-design intent.

## 14. Scale

Scale follows DOMAIN_TYPES.md:

- 1.0 = 100 percent;
- negative values mirror;
- interpolate component-wise.

Crossing zero is valid.

## 15. Opacity

Opacity remains in [0,1].

Project mutation and load validation reject invalid persisted values.

## 16. Color

Color is LinearRgba.

Animation engine does not know about texture formats or UI color widgets.

Boundary:

~~~text
user-facing sRGB
-> LinearRgba project value
-> animation
-> renderer
~~~

## 17. Discrete values

Enum-like or boolean properties use Hold semantics.

Do not force discrete data through scalar interpolation.

## 18. Identity vs position

KeyframeId is entity identity.

MusicalTick is temporal position.

Moving a keyframe changes its tick, not its ID.

## 19. Collision invariant

One Animated property cannot contain two keyframes at the same MusicalTick.

Editor/command layer resolves collisions before a committed state reaches the evaluator.

## 20. Scene evaluation

Renderer does not query keyframes directly.

~~~text
Project + ProjectTimeNs
-> evaluate transforms
-> evaluate content properties
-> evaluate effect parameters
-> EvaluatedScene
-> Renderer
~~~

EvaluatedScene is derived runtime state and is never serialized.

## 21. Determinism

Given the same Project, ProjectTimeNs, and engine semantics, output must be independent from:

- editor FPS;
- previous evaluation time;
- playback history;
- mouse state;
- whether playback is active.

This is required for scrubbing and export.

## 22. Caching

Allowed optimizations:

- last segment cursor;
- precomputed Bezier coefficients;
- static-property flags.

A cache must be rebuildable and must never change semantic output.

## 23. Precision

- keyframe/time identity uses integer/fixed-point types;
- interpolation progress may use f64;
- visual values normally use f32;
- never downcast project time to f32 before segment lookup.

## 24. Performance

Normal evaluation should avoid:

- heap allocation per property;
- cloning keyframe arrays;
- linear scans through long tracks;
- full project validation every frame.

Evaluation assumes the project has already passed validation.

## 25. Required tests

Lookup:
- zero, one, multiple keyframes;
- exact keyframe hit;
- arbitrary seek order.

Range:
- before first;
- after last.

Interpolation:
- Hold;
- Linear scalar;
- Vec2;
- LinearRgba;
- easing presets;
- Bezier boundaries.

Semantics:
- outgoing key owns following segment;
- 0 to 360 rotation spans full numeric range;
- negative scale;
- color midpoint in linear space;
- duplicate tick is rejected before evaluation.

Determinism:
- repeated same-time evaluation;
- out-of-order evaluation;
- preview/export callers receive identical scene values.

## 26. Definition of Done

The animation engine is implementation-ready when:

- Animated and Keyframe types live in core;
- sorting and uniqueness invariants are enforced;
- range behavior is tested;
- Hold, Linear, and Bezier timing work;
- all MVP value classes work;
- EvaluatedScene is produced without UI/GPU dependencies;
- arbitrary seek evaluation is deterministic.
