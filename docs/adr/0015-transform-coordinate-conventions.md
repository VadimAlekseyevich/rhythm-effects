# ADR 0015 — Composition Uses Top-Left Y-Down Space and Normalized Anchor

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Renderer, viewport, gizmos, hit testing, and export need one transform convention.

Leaving coordinate and pivot semantics implicit causes sign, anchor, and DPI bugs.

## Decision

Project transform semantics are:

- composition origin at top-left;
- positive X points right;
- positive Y points down;
- one spatial unit equals one composition pixel;
- Position is anchor location in composition space;
- Anchor is normalized relative to resolved object bounds;
- default Anchor is (0.5, 0.5);
- transform order is subtract anchor, scale, rotate, translate;
- positive Rotation appears clockwise on screen;
- Scale 1.0 equals 100 percent;
- negative scale is valid for mirroring.

Viewport, DPI, and GPU clip-space transforms remain editor/runtime details and are not persisted in Project.

## Consequences

Positive:

- familiar 2D editor coordinates;
- stable relative center pivot when object bounds change;
- one transform convention for renderer and hit testing;
- export stays independent from UI/DPI.

Negative:

- math-library conventions may require explicit sign conversion;
- normalized anchor differs from some pixel-anchor editors.

## Revisit condition

Do not change persisted transform semantics after public project files exist without an explicit schema migration.
