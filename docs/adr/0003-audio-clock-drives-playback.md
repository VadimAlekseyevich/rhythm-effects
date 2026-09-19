# ADR 0003 — Audio Clock Drives Active Playback

> **Status: Accepted**
>
> **Date: 2026-09-19**

## Context

A motion editor synchronized to music cannot derive playback time from UI redraw frequency without risking drift and jitter.

## Decision

When audio playback is active, animation evaluation time is derived from audio playback position.

The editor frame loop observes the clock; it does not advance the clock.

When paused, editor playhead state becomes the authority until playback resumes.

## Consequences

Positive:

- animation remains synchronized when UI FPS varies;
- dropped visual frames do not shift musical timing;
- timing architecture matches the product's rhythm-first philosophy.

Negative:

- audio subsystem must expose a robust playback position;
- seek/pause/resume transitions need explicit synchronization;
- audio callback/threading constraints become architecture-critical.

## Constraint

The real-time audio callback may not acquire a project-wide lock or perform heavyweight editor/render work.
