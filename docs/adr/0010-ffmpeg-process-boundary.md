# ADR 0010 — Use FFmpeg Through a Process Boundary for MVP Export

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Rhythm Effects needs H.264/MP4 export with audio muxing.

Native FFmpeg bindings introduce C ABI/build complexity, platform packaging complexity, unsafe boundary, and substantial compile/dependency cost.

MVP does not require zero-copy hardware-encoder integration.

## Decision

Use a controlled FFmpeg executable as a child process for MVP.

The app owns:

- exact bundled/resolved executable path;
- command construction;
- raw video/frame transport;
- audio input/mux configuration;
- stderr/progress capture;
- cancellation;
- temp-output publication.

## Consequences

Positive:

- isolates native media stack;
- Rust build remains simpler;
- easy manual diagnostics;
- known executable version can be bundled.

Negative:

- process startup/pipe overhead;
- frame transport may require CPU readback;
- packaging/licensing must include FFmpeg;
- progress/error parsing needs care.

## Constraints

- do not rely on user's PATH in normal bundled distribution;
- export writes temporary output then publishes on success;
- raw FFmpeg stderr is diagnostic, not primary user-facing text.

## Revisit condition

Use native/hardware integration only if measured export throughput or required codec functionality makes the process boundary insufficient.
