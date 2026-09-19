# ADR 0002 — Musical Ticks Are the Canonical Keyframe Time

> **Status: Accepted**
>
> **Date: 2026-09-19**

## Context

The product promise is that motion is authored on a BPM grid.

Storing keyframes as arbitrary seconds and merely snapping the UI would make musical time secondary and complicate BPM changes.

## Decision

Persist keyframe position as integer MusicalTick.

MVP PPQ proposal is 960 ticks per quarter note.

Seconds, audio samples, and video frames are derived time domains.

The current authoring subdivision is a subset of the underlying PPQ lattice.

## Consequences

Positive:

- exact rhythmic spacing;
- keyframe patterns remain musical when BPM changes;
- no accumulated float identity error;
- keyboard movement is exact integer arithmetic;
- changing visible subdivision never needs to move old keys.

Negative:

- every playback/export path needs centralized tick↔time conversion;
- future tempo maps require carefully tested conversion;
- arbitrary off-grid keyframe workflows are intentionally unsupported.

## Product consequence

The playhead may be continuous, but authored keyframes remain musical-grid entities.
