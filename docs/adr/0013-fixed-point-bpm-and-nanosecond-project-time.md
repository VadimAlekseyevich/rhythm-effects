# ADR 0013 — Fixed-Point BPM and Integer Nanosecond Project Time

> **Status: Accepted**
>
> **Date: 2026-09-19**

## Context

Musical keyframes use integer ticks, but BPM and grid offset also participate in identity-sensitive time conversion.

Persisting BPM and project timing only as floating-point values would make conversion and serialization less deterministic.

## Decision

Use:

- BpmMicros(u64): BPM multiplied by 1,000,000;
- ProjectTimeNs(i64): signed project nanoseconds;
- GridOffsetNs(i64): project time of MusicalTick(0);
- DurationNs(u64) where negative values have no meaning;
- i128/u128 intermediates for fixed-point and rational conversion.

PPQ remains 960.

Floating-point values may still be used for transient interpolation/render math, but not as persisted time identity.

## Consequences

Positive:

- deterministic serialization;
- no NaN/Infinity BPM;
- repeatable snapping/conversion;
- explicit rounding behavior;
- precision far beyond normal BPM entry requirements.

Negative:

- conversion helpers require deliberate integer/rational code;
- BPM parse/format helpers are needed;
- overflow/rounding tests become important.

## Revisit condition

Only revisit if future tempo-map requirements show that another exact representation materially improves correctness or simplicity.
