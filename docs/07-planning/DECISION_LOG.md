# Decision Log

> **Status: Draft**
>
> Compact index of project decisions. Detailed rationale belongs in canonical specs or ADRs.

## Decisions

| ID | Date | Decision | Status | Canonical source |
|---|---|---|---|---|
| D-001 | 2026-09-19 | Rhythm Effects is rhythm-first motion design | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md |
| D-002 | 2026-09-19 | Authored keyframes use integer MusicalTick identity | Accepted | ../adr/0002-musical-ticks-as-keyframe-time.md |
| D-003 | 2026-09-19 | UI comfort and shallow navigation are first-class | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md |
| D-004 | 2026-09-19 | Runtime performance and build speed are first-class | Accepted | ../06-quality/PERFORMANCE.md |
| D-005 | 2026-09-19 | Direct egui + winit + wgpu editor integration | Accepted for MVP | ../adr/0001-direct-egui-winit-wgpu.md |
| D-006 | 2026-09-19 | Initial workspace has rhythm_core, rhythm_engine, rhythm_app | Accepted for initial implementation | ../adr/0004-three-crate-initial-workspace.md |
| D-007 | 2026-09-19 | Audio clock drives active playback | Accepted | ../adr/0003-audio-clock-drives-playback.md |
| D-008 | 2026-09-19 | No general async runtime by default | Accepted for MVP | ../adr/0005-no-general-async-runtime-for-mvp.md |
| D-009 | 2026-09-19 | Composition text prototype uses cosmic-text + glyphon | Accepted for prototype | ../adr/0006-text-stack-cosmic-text-glyphon.md |
| D-010 | 2026-09-19 | Waveform uses time-domain min/max aggregation, not FFT | Accepted | ../adr/0007-no-fft-for-waveform.md |
| D-011 | 2026-09-19 | Typed project-local u64 entity IDs | Accepted for MVP | ../adr/0008-project-local-typed-u64-ids.md |
| D-012 | 2026-09-19 | Project schema V1 uses versioned JSON | Accepted for MVP | ../adr/0009-json-project-schema-v1.md |
| D-013 | 2026-09-19 | FFmpeg runs through a child-process boundary for MVP | Accepted for MVP | ../adr/0010-ffmpeg-process-boundary.md |
| D-014 | 2026-09-19 | Active Project has a single canonical writer | Accepted | ../adr/0011-single-writer-project-state.md |
| D-015 | 2026-09-19 | Renderer uses linear-light effect/blend semantics with premultiplied alpha | Accepted | ../adr/0012-linear-light-premultiplied-alpha-semantics.md |
| D-016 | 2026-09-19 | BPM is fixed-point and project time uses integer nanoseconds | Accepted | ../adr/0013-fixed-point-bpm-and-nanosecond-project-time.md |
| D-017 | 2026-09-19 | MVP grid divisions and snap tie behavior are fixed | Accepted for MVP | ../adr/0014-mvp-beat-division-set-and-snap-ties.md |
| D-018 | 2026-09-19 | Composition uses top-left Y-down coordinates and normalized anchor | Accepted for MVP | ../adr/0015-transform-coordinate-conventions.md |
| D-019 | 2026-09-19 | Animation segments are musical and owned by outgoing keyframe | Accepted for MVP | ../adr/0016-animation-segment-semantics.md |
| D-020 | 2026-09-19 | Undo stores semantic affected state, not whole-project snapshots per edit | Draft | ../02-architecture/COMMANDS_UNDO.md |
| D-021 | 2026-09-19 | Save uses versioning and safe temporary-file replacement | Draft | ../05-persistence-export/SERIALIZATION.md |
| D-022 | 2026-09-19 | Autosave is recovery state, not implicit canonical save | Draft | ../05-persistence-export/PROJECT_RECOVERY.md |

## Maintenance rule

When a decision changes:

1. add or update an ADR when rationale is material;
2. mark the previous decision Superseded;
3. link the replacement;
4. update affected canonical specs;
5. keep historical reasoning instead of rewriting it.
