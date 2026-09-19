# ADR 0011 — Canonical Project State Has a Single Writer

> **Status: Accepted**
>
> **Date: 2026-09-19**

## Context

The application has UI interaction, audio realtime work, media decode, waveform generation, autosave, and export.

Allowing every subsystem to mutate shared project state through locks would create difficult ordering, undo, deadlock, and consistency problems.

## Decision

The active mutable Project has one canonical writer in the editor/application context.

Background workers receive owned or immutable job inputs and return results.

Audio callback never mutates Project.

Completed worker results are validated by ID/generation before the owning editor/runtime subsystem applies them.

## Consequences

Positive:

- deterministic command/undo semantics;
- no project-wide lock in audio callback;
- simpler serialization snapshots;
- easier debugging;
- stale async results can be rejected explicitly.

Negative:

- some worker results require a handoff back to owner;
- long mutations must be designed to avoid blocking editor;
- export may need an immutable snapshot.

## Revisit condition

Collaboration or large-scale concurrent editing requirements may require a more advanced state model. That is outside MVP.
