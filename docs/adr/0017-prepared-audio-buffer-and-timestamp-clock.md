# ADR 0017 — Prepared Playback PCM and CPAL Playback-Timestamp Clock

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Music synchronization is the product promise. Streaming decode/resampling in the realtime path increases complexity and makes seek/clock behavior harder.

CPAL exposes predicted playback timestamps for output callback data and a stream-local monotonic now() clock.

## Decision

For MVP:

1. decode full music source in background;
2. build waveform peaks;
3. resample in background to active output rate;
4. keep one immutable prepared f32 playback buffer;
5. realtime callback only copies/converts/applies gain;
6. publish clock anchors mapping CPAL playback StreamInstant to ProjectTimeNs;
7. editor samples the same stream clock to estimate audible project time.

Seek/source replacement increments playback generation.

## Consequences

Positive:

- simple realtime callback;
- exact random seek;
- no decoder/resampler jitter in callback;
- timing independent of UI FPS.

Negative:

- higher memory than streaming;
- output-rate change requires re-preparation;
- hour-long media is outside optimized MVP workload.

## Revisit condition

Introduce streaming only after real project measurements show prepared-buffer memory is unacceptable.
