# Architecture Decision Records

ADRs capture decisions with meaningful alternatives or long-term consequences.

## Naming

~~~text
0001-short-decision-name.md
0002-another-decision.md
~~~

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

## Template

~~~markdown
# ADR NNNN — Title

> Status: Proposed | Accepted | Superseded
> Date: YYYY-MM-DD

## Context
## Decision
## Alternatives considered
## Consequences
## Constraints
## Revisit condition
~~~

If an ADR is superseded, keep it and link to the replacement. Do not rewrite history.
