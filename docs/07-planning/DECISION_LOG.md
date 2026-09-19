# Decision Log

> **Status: Accepted — living index**
>
> Detailed rationale belongs in canonical specs or ADRs.

| ID | Date | Decision | Status | Canonical source |
|---|---|---|---|---|
| D-001 | 2026-09-19 | Rhythm Effects is rhythm-first motion design | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md |
| D-002 | 2026-09-19 | Keyframe identity is integer MusicalTick | Accepted | ../adr/0002-musical-ticks-as-keyframe-time.md |
| D-003 | 2026-09-19 | UI comfort/shallow navigation are first-class | Accepted | ../00-overview/PROJECT_PHILOSOPHY.md |
| D-004 | 2026-09-19 | Runtime and build iteration performance are first-class | Accepted | ../06-quality/PERFORMANCE.md |
| D-005 | 2026-09-19 | Direct egui + winit + wgpu integration | Accepted for MVP | ../adr/0001-direct-egui-winit-wgpu.md |
| D-006 | 2026-09-19 | Initial three-crate workspace | Accepted | ../adr/0004-three-crate-initial-workspace.md |
| D-007 | 2026-09-19 | Audio clock drives active playback | Accepted | ../adr/0003-audio-clock-drives-playback.md |
| D-008 | 2026-09-19 | No general async runtime by default | Accepted for MVP | ../adr/0005-no-general-async-runtime-for-mvp.md |
| D-009 | 2026-09-19 | cosmic-text + glyphon for composition text | Accepted for MVP | ../03-engine/TEXT_RENDERING.md |
| D-010 | 2026-09-19 | Waveform is time-domain min/max, no FFT | Accepted | ../adr/0007-no-fft-for-waveform.md |
| D-011 | 2026-09-19 | Typed project-local u64 IDs | Accepted | ../adr/0008-project-local-typed-u64-ids.md |
| D-012 | 2026-09-19 | .rhfx V1 uses versioned JSON | Accepted | ../adr/0021-rhfx-versioned-json-project-files.md |
| D-013 | 2026-09-19 | FFmpeg remains child-process export boundary | Accepted | ../adr/0010-ffmpeg-process-boundary.md |
| D-014 | 2026-09-19 | Active Project has a single writer | Accepted | ../adr/0011-single-writer-project-state.md |
| D-015 | 2026-09-19 | Linear-light/premultiplied creative renderer semantics | Accepted | ../adr/0012-linear-light-premultiplied-alpha-semantics.md |
| D-016 | 2026-09-19 | Fixed-point BPM + integer nanosecond project time | Accepted | ../adr/0013-fixed-point-bpm-and-nanosecond-project-time.md |
| D-017 | 2026-09-19 | MVP beat divisions + rightward half-tie snap | Accepted | ../adr/0014-mvp-beat-division-set-and-snap-ties.md |
| D-018 | 2026-09-19 | Top-left Y-down coordinates + normalized anchor | Accepted | ../adr/0015-transform-coordinate-conventions.md |
| D-019 | 2026-09-19 | Outgoing key owns musical interpolation segment | Accepted | ../adr/0016-animation-segment-semantics.md |
| D-020 | 2026-09-19 | Undo stores affected semantic before/after state; 500 entries | Accepted | ../02-architecture/COMMANDS_UNDO.md |
| D-021 | 2026-09-19 | Prepared PCM + CPAL playback timestamps | Accepted | ../adr/0017-prepared-audio-buffer-and-timestamp-clock.md |
| D-022 | 2026-09-19 | Rgba16Float working targets + scaled preview | Accepted | ../adr/0018-rgba16f-working-target-and-preview-scale.md |
| D-023 | 2026-09-19 | System fonts + bundled Inter fallback | Accepted | ../adr/0019-system-fonts-with-inter-fallback.md |
| D-024 | 2026-09-19 | Assets remain external; relative paths when practical | Accepted | ../adr/0020-external-assets-relative-when-possible.md |
| D-025 | 2026-09-19 | Recovery is 30-second current/previous rolling state | Accepted | ../adr/0022-two-generation-30s-recovery.md |
| D-026 | 2026-09-19 | Export is raw-video pipe to FFmpeg H.264 MP4 | Accepted | ../adr/0023-ffmpeg-rawvideo-h264-mp4-export.md |
| D-027 | 2026-09-19 | No global Auto-Key; animation is property-scoped | Accepted | ../01-product/INTERACTION_MODEL.md |
| D-028 | 2026-09-19 | Space is exclusively Play/Pause in editor context | Accepted | ../01-product/KEYBOARD_SHORTCUTS.md |
| D-029 | 2026-09-19 | MVP effects: Blur/Glow/Tint/Noise/RGB Split | Accepted | ../03-engine/EFFECTS.md |
| D-030 | 2026-09-19 | MVP ships one dark theme using Inter UI family | Accepted | ../01-product/DESIGN_SYSTEM.md |
| D-031 | 2026-09-19 | MVP distribution is Windows x86_64 portable ZIP | Accepted | ../06-quality/PACKAGING.md |
| D-032 | 2026-09-19 | MVP is local-first with no telemetry/account/cloud | Accepted | ../06-quality/SECURITY_PRIVACY.md |

## Maintenance

When a decision changes:

1. update/supersede ADR if material;
2. update canonical spec;
3. add/modify this index entry;
4. update dependent docs/backlog/tests;
5. retain historical rationale.
