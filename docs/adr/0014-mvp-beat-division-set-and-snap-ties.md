# ADR 0014 — Define MVP Beat Divisions and Deterministic Snap Ties

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

The product depends on strict BPM-grid keyframe placement.

Grid divisions and nearest-point rounding must be deterministic, including negative ticks and exact half-step pointer positions.

## Decision

MVP authoring grid supports:

~~~text
1/1
1/2
1/3
1/4
1/6
1/8
1/12
1/16
1/24
1/32
~~~

The denominator means equal parts per quarter-note beat.

With PPQ 960, every division maps to integer ticks.

Keyframe create/drag uses nearest grid point.

An exact half-step tie resolves toward the later/right grid point on timeline.

Keyboard movement never rounds; it adds/subtracts exact integer grid-step ticks.

Changing the current division never quantizes existing keyframes.

## Consequences

- straight and triplet-oriented timing are first-class;
- no ambiguous float snapping;
- repeated keyboard moves are exactly reversible;
- fine-grid keyframes survive coarser editing grids.

## Revisit condition

Additional exact divisors may be added later without changing existing project semantics.
