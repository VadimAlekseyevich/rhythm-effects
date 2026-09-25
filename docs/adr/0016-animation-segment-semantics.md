# ADR 0016 — Animation Segment Semantics Are Musical and Outgoing-Key Owned

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

The animation engine needs one unambiguous rule for segment ownership, progress, out-of-range behavior, rotation, and color interpolation.

## Decision

For adjacent keyframes A and B:

- A owns interpolation/easing for the segment from A to B;
- progress is measured in musical position between A.tick and B.tick;
- exactly on a keyframe returns its exact stored value;
- before the first keyframe, evaluation holds the first keyframe value;
- after the last keyframe, evaluation holds the last keyframe value;
- no keyframes means base_value;
- Rotation interpolates numeric degrees directly, without shortest-path normalization;
- Color interpolates component-wise in the canonical linear-light representation;
- discrete properties use Hold semantics.

Preset easing resolves to canonical Bezier parameters: Ease In `(0.42, 0.0, 1.0, 1.0)`, Ease Out `(0.0, 0.0, 0.58, 1.0)`, and Ease In-Out `(0.42, 0.0, 0.58, 1.0)`.

## Consequences

Positive:

- each segment has one clear owner;
- rhythm remains the semantic interpolation domain;
- multi-turn rotations are possible;
- preview and export can share identical behavior.

Negative:

- changing interpolation on a key affects its following segment;
- future tempo changes inside a segment preserve musical rather than absolute-time progress.

## Revisit condition

Alternative interpolation domains or spatial curves must be introduced as explicit new animation modes rather than silently changing this semantic.
