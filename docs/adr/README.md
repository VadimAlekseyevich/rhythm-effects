# Architecture Decision Records

ADRs capture decisions with meaningful alternatives or long-term consequences.

## Status

- Proposed
- Accepted
- Accepted for MVP/prototype
- Superseded

## Index

- [0001](0001-direct-egui-winit-wgpu.md) — direct egui + winit + wgpu.
- [0002](0002-musical-ticks-as-keyframe-time.md) — MusicalTick keyframe identity.
- [0003](0003-audio-clock-drives-playback.md) — audio clock drives playback.
- [0004](0004-three-crate-initial-workspace.md) — three-crate workspace.
- [0005](0005-no-general-async-runtime-for-mvp.md) — no general async runtime.
- [0006](0006-text-stack-cosmic-text-glyphon.md) — cosmic-text + glyphon prototype decision.
- [0007](0007-no-fft-for-waveform.md) — no FFT for waveform.
- [0008](0008-project-local-typed-u64-ids.md) — typed local u64 IDs.
- [0009](0009-json-project-schema-v1.md) — JSON schema V1 basis.
- [0010](0010-ffmpeg-process-boundary.md) — FFmpeg process boundary.
- [0011](0011-single-writer-project-state.md) — single-writer Project.
- [0012](0012-linear-light-premultiplied-alpha-semantics.md) — linear/premultiplied semantics.
- [0013](0013-fixed-point-bpm-and-nanosecond-project-time.md) — fixed BPM/time units.
- [0014](0014-mvp-beat-division-set-and-snap-ties.md) — divisions/snap ties.
- [0015](0015-transform-coordinate-conventions.md) — transform/coordinate convention.
- [0016](0016-animation-segment-semantics.md) — segment interpolation semantics.
- [0017](0017-prepared-audio-buffer-and-timestamp-clock.md) — prepared PCM + CPAL timestamp clock.
- [0018](0018-rgba16f-working-target-and-preview-scale.md) — Rgba16Float + preview scale.
- [0019](0019-system-fonts-with-inter-fallback.md) — system fonts + Inter fallback.
- [0020](0020-external-assets-relative-when-possible.md) — external/relative asset paths.
- [0021](0021-rhfx-versioned-json-project-files.md) — .rhfx JSON + transactional save.
- [0022](0022-two-generation-30s-recovery.md) — rolling recovery policy.
- [0023](0023-ffmpeg-rawvideo-h264-mp4-export.md) — raw-video H.264 MP4 export.

If an ADR is superseded, keep it, mark it, and link the replacement. Do not rewrite architectural history.
