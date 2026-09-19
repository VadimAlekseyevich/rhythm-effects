# ADR 0023 — MVP Export Uses Raw-Video Pipe to FFmpeg H.264 MP4

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Decision

- offline deterministic render;
- bounded GPU readback;
- raw RGBA/BGRA video piped to FFmpeg;
- H.264 MP4, yuv420p;
- AAC audio when present;
- source audio file is mux input;
- temporary output is published only after successful finalization.

The bundled release FFmpeg build must expose a validated H.264 encoder. Encoder implementation name is packaging-specific.

## Revisit condition

Native/zero-copy hardware encoder integration requires measured export need after MVP.
