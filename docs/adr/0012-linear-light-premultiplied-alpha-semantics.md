# ADR 0012 — Use Linear-Light Effect/Blend Semantics with Premultiplied Alpha

> **Status: Accepted as renderer semantic contract**
>
> **Date: 2026-09-19**

## Context

Incorrect color-space or alpha conventions cause visible artifacts in image compositing, blur, glow, transparency edges, and effect chains.

If conventions are left implicit, different shaders and asset paths will disagree.

## Decision

Renderer semantics for MVP are:

1. color values used for blending/effect math are treated in a documented linear-light working space;
2. sRGB source images are sampled with correct sRGB conversion;
3. compositing uses premultiplied-alpha semantics at renderer boundaries;
4. final SDR output is converted to expected sRGB presentation/export representation.

The exact physical intermediate texture format is not fixed by this ADR and may be selected based on quality/performance measurements.

## Consequences

Positive:

- blur/glow behave more correctly;
- alpha edge behavior is consistent;
- preview/export can share one color contract.

Negative:

- asset/shader boundaries must convert correctly;
- some UI-provided color values require explicit conversion;
- testing needs reference images.

## Constraints

Do not silently mix straight and premultiplied alpha in different pipelines.

Do not perform effect math in gamma-encoded sRGB merely because a texture is 8-bit.

## Revisit condition

HDR/wide-gamut support will require a broader color-management ADR post-MVP.
