# Architecture Decision Records

ADRs capture decisions with meaningful alternatives or long-term consequences.

## Status

- Proposed
- Accepted
- Accepted for MVP/prototype
- Superseded

## Existing ADRs

- [0001](0001-direct-egui-winit-wgpu.md) — direct egui + winit + wgpu integration.
- [0002](0002-musical-ticks-as-keyframe-time.md) — integer musical ticks are canonical keyframe time.
- [0003](0003-audio-clock-drives-playback.md) — audio clock drives active playback.
- [0004](0004-three-crate-initial-workspace.md) — initial three-crate workspace.
- [0005](0005-no-general-async-runtime-for-mvp.md) — no general async runtime by default.
- [0006](0006-text-stack-cosmic-text-glyphon.md) — prototype composition text with cosmic-text + glyphon.
- [0007](0007-no-fft-for-waveform.md) — waveform uses time-domain min/max aggregation.
- [0008](0008-project-local-typed-u64-ids.md) — typed project-local u64 IDs.
- [0009](0009-json-project-schema-v1.md) — versioned JSON project schema V1.
- [0010](0010-ffmpeg-process-boundary.md) — FFmpeg child-process boundary for MVP export.
- [0011](0011-single-writer-project-state.md) — active Project has one canonical writer.
- [0012](0012-linear-light-premultiplied-alpha-semantics.md) — renderer color/alpha semantic contract.
- [0013](0013-fixed-point-bpm-and-nanosecond-project-time.md) — deterministic BPM/project-time representation.
- [0014](0014-mvp-beat-division-set-and-snap-ties.md) — grid divisions and snap rounding.
- [0015](0015-transform-coordinate-conventions.md) — composition and transform conventions.
- [0016](0016-animation-segment-semantics.md) — musical interpolation and outgoing-key segment ownership.

If an ADR is superseded, keep it and link to the replacement. Do not rewrite history.
