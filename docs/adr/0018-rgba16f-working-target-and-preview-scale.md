# ADR 0018 — Rgba16Float Working Targets and Scaled Preview

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Blur, glow, alpha compositing, preview/export parity, and 4K performance require an explicit target policy.

## Decision

- creative working target: Rgba16Float;
- RGB semantics: linear-light;
- alpha: premultiplied;
- preview quality: Auto/Full/Half/Quarter;
- Auto chooses 1, 1/2, or 1/4 so large compositions stay around a 1920×1080 working target;
- spatial effect parameters remain composition-pixel units and are internally scaled for reduced preview;
- export uses full requested resolution.

## Consequences

Positive:

- predictable effect math;
- glow headroom;
- fewer intermediate banding artifacts;
- clear performance path for 4K.

Negative:

- higher GPU memory/bandwidth;
- preview compensation requires tests.

## Revisit condition

Only add a lower-bandwidth working format after profiling demonstrates a real supported-hardware blocker.
