# ADR 0007 — Waveform Generation Does Not Use FFT

> **Status: Accepted**
>
> **Date: 2026-09-19**

## Context

Waveform display needs time-domain amplitude envelopes. FFT solves frequency-domain problems.

## Decision

Build waveform mip levels from time-domain min/max aggregation.

Do not add rustfft for MVP waveform rendering.

## Consequences

- simpler preprocessing;
- smaller dependency/build cost;
- deterministic zoomable waveform cache;
- FFT remains available later for spectrum or BPM-analysis features when explicitly scoped.
