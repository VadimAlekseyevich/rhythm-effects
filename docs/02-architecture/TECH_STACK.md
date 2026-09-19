# Technology Stack

> **Status: Draft**
>
> Evaluated: 2026-09-19.
>
> Exact Cargo versions will be pinned when implementation begins.

## 1. Recommended MVP stack

| Area | Choice |
|---|---|
| Language | Rust |
| Window/event loop | winit |
| Editor UI | egui |
| UI platform/GPU integration | egui-winit + egui-wgpu |
| Graphics | wgpu |
| Shaders | WGSL |
| Audio output | CPAL |
| Audio decode | Symphonia |
| Audio resampling | Rubato candidate |
| Serialization | serde |
| Composition text | cosmic-text + glyphon prototype |
| Video encode/mux | FFmpeg |
| Concurrency | std threads/channels first |

---

## 2. Evaluated ecosystem state

At the evaluation date:

- wgpu 30.0.1;
- egui 0.36.2;
- winit 0.30.13;
- CPAL 0.18.2;
- Symphonia 0.6.1;
- Rubato 5.0.0;
- cosmic-text 0.19.0;
- glyphon 0.12.0.

These are evaluation references, not a promise to blindly upgrade to latest versions.

Pin a compatible set in Cargo.lock.

---

## 3. Rust

Chosen for:

- native performance;
- memory/concurrency safety;
- strong modeling;
- wgpu ecosystem;
- no managed runtime;
- useful refactoring guarantees.

Cost: compile times and implementation complexity.

Response: dependency discipline and measured build budgets.

---

## 4. winit

Use directly in rhythm_app.

Responsibilities:

- native window;
- event loop;
- input;
- DPI/scale events.

MVP targets Windows, but winit keeps future portability cheap.

---

## 5. egui

Use for editor chrome and custom editing widgets.

Why:

- immediate-mode interaction suits timeline/gizmos;
- easy custom painting/hit testing;
- pure Rust;
- wgpu integration.

Risks:

- upstream breaking changes;
- large unvirtualized UI cost;
- polish requires custom component layer.

Mitigation:

- pin versions;
- virtualize timeline;
- measure frame cost;
- build project design system.

---

## 6. Direct integration

Use:

- egui;
- egui-winit;
- egui-wgpu;
- winit;
- wgpu.

Do not make eframe the foundation for MVP.

See ADR 0001.

---

## 7. wgpu

Use for:

- offscreen composition;
- shape/image rendering;
- effects;
- text integration;
- export frame rendering.

WGSL is default shader language.

Windows backend selection remains wgpu-managed unless diagnostics require override.

---

## 8. CPAL

Use as low-level audio output.

Audio callback is real-time-sensitive.

No project mutex or decoding inside callback.

---

## 9. Symphonia

Use for decode/demux.

Enable only needed formats/codecs.

Target:

- WAV;
- MP3;
- AAC/M4A;
- FLAC;
- OGG/Vorbis.

---

## 10. Rubato

Candidate for sample-rate conversion.

Reason:

- source and device sample rates can differ;
- current API supports preallocated processing patterns suitable for realtime use.

Do not finalize resampler algorithm before audio clock prototype.

---

## 11. No FFT dependency for waveform

Waveform is min/max time-domain aggregation.

Do not add rustfft until a frequency-domain feature exists.

See ADR 0007.

---

## 12. serde

Use for semantic project data.

Exact text format remains open.

Runtime UI/GPU/audio objects are never serialized.

---

## 13. Text

Prototype:

- cosmic-text shaping/layout/font fallback;
- glyphon wgpu rendering.

Do not use egui text as composition text engine.

See ADR 0006.

---

## 14. FFmpeg

Use for H.264/MP4 encoding and audio mux.

Initial preference is process boundary rather than native library bindings.

Reasons:

- isolate native dependency;
- reduce Rust build complexity;
- simpler MVP packaging/debugging boundary.

Finalize in EXPORT/PACKAGING ADR.

---

## 15. Images

Choose a Rust decoder during asset spike.

Required:

- PNG;
- JPEG;
- WebP desirable.

Evaluate compile cost and disable unnecessary format features.

---

## 16. Math

Keep project math types independent from egui/wgpu.

Use small custom types or glam only if renderer math justifies it.

Do not leak graphics-library vector types into serialized core.

---

## 17. IDs

Use typed project-local u64 IDs initially.

No UUID dependency by default.

Cross-context duplication allocates fresh IDs.

---

## 18. Concurrency

No general async runtime by default.

Use explicit workers and narrow communication.

See ADR 0005.

---

## 19. Logging

Need structured diagnostics.

tracing is candidate.

Measure dependency/build cost before accepting broad subscriber stack.

---

## 20. Dependency policy

For each major dependency evaluate:

- runtime value;
- clean build cost;
- incremental impact;
- transitive graph;
- binary size;
- duplicate functionality.

Avoid broad all-feature flags.

Keep rhythm_core free of GPU/UI/audio backend dependencies.

---

## 21. Mandatory technical prototype

Before deep implementation, validate:

1. winit + egui + wgpu;
2. offscreen composition texture in editor;
3. audio decode/playback;
4. audio-driven animation clock;
5. waveform preprocessing;
6. Cyrillic text with candidate stack;
7. one multi-pass effect;
8. offscreen frame readback for export;
9. acceptable incremental build loop.

Any failure reopens the relevant ADR instead of being hidden behind workaround layers.
