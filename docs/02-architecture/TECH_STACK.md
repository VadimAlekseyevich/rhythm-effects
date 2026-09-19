# Technology Stack

> **Status: Draft**
>
> Evaluated: 2026-09-19.
>
> Versions listed below describe the ecosystem state at evaluation time. Exact Cargo versions will be pinned when the repository is bootstrapped.

## 1. Decision summary

Recommended MVP stack:

| Area | Choice |
|---|---|
| Language | Rust |
| Window/event loop | winit |
| Editor UI | egui |
| egui platform integration | egui-winit |
| egui GPU integration | egui-wgpu |
| Graphics | wgpu |
| Shaders | WGSL |
| Audio output | CPAL |
| Audio decode/demux | Symphonia |
| Serialization | serde |
| Project format | serde-based, exact syntax TBD |
| Video encode/mux | FFmpeg |
| Logging | tracing or lightweight equivalent, final choice TBD |
| Image decode | image crate or narrower alternative, benchmark compile/runtime cost |
| Text | dedicated shaping/raster stack TBD |
| Concurrency | std threads/channels first; add specialized crates only when needed |

---

## 2. Language: Rust

### Why

Fits project priorities:

- native performance;
- predictable memory ownership;
- safe concurrency for complex editor state;
- strong enum/newtype modeling;
- good wgpu ecosystem;
- no managed runtime/GC pauses;
- useful compiler assistance for large refactors.

### Cost

- slower compile times than C in many cases;
- borrow/lifetime complexity can slow early iteration;
- GUI ecosystem less mature than web frontend stacks.

### Project response

Treat build speed as a monitored metric, keep crate/dependency graph restrained, and avoid unnecessary generic/macro-heavy architecture.

---

## 3. Windowing: winit

Current evaluated release: **0.30.13**.

Official docs describe winit as a cross-platform window creation and event-loop library and include DPI/scale-factor handling.

Why:

- direct native event loop;
- established wgpu integration path;
- keyboard/mouse/window/DPI events;
- avoids embedding a browser runtime.

MVP target is Windows, but using winit avoids needless Windows-only window boilerplate.

### Decision

Use winit directly in rhythm_app.

---

## 4. UI: egui

Current evaluated release: **0.36.2**.

egui is immediate-mode and specifically suited to highly interactive applications where UI is rebuilt/painted each frame.

Why it fits:

- custom timeline widgets;
- custom hit-testing;
- custom keyframe painting;
- responsive direct-manipulation UI;
- easy contextual disclosure;
- pure Rust;
- integrates with wgpu.

### Risks

- frequent upstream API changes;
- immediate-mode cost can become significant for huge unvirtualized UIs;
- visual polish requires deliberate custom component work;
- not a ready-made professional motion editor toolkit.

### Mitigation

- pin versions;
- upgrade intentionally, not automatically;
- virtualize timeline rows/keyframes;
- build our own small design-system layer;
- measure UI frame time.

### Decision

Use egui for MVP.

---

## 5. egui integration: direct, not eframe-first

Preferred stack:

- egui;
- egui-winit;
- egui-wgpu;
- winit;
- shared wgpu device/queue.

Why not make eframe the architectural foundation:

- we need explicit control over the native event/render lifecycle;
- composition rendering is already a first-class custom wgpu pipeline;
- sharing rendering resources is central;
- minimizing wrapper layers makes performance/debug boundaries clearer.

This does not mean eframe is bad; it means direct integration better matches this editor's architecture.

Create ADR-0001 for this choice.

---

## 6. Graphics: wgpu

Current evaluated release: **30.0.1**.

Official wgpu docs describe a safe cross-platform Rust graphics API running on native Vulkan, Metal, D3D12, and OpenGL backends. WGSL is supported by default.

Why:

- Windows-native D3D12/Vulkan paths through one API;
- future macOS path via Metal without redesigning renderer;
- modern explicit GPU model;
- strong Rust ecosystem;
- WGSL shader support;
- good fit for offscreen composition rendering and multi-pass effects.

### MVP backend policy

On Windows:

- prefer default wgpu adapter selection;
- log adapter/backend for diagnostics;
- do not expose backend selection in normal UI;
- keep a fallback strategy for unsupported/problematic adapters.

### Decision

Use wgpu + WGSL.

---

## 7. Audio output: CPAL

Current evaluated release: **0.18.2**.

CPAL is a low-level cross-platform audio I/O library.

Why:

- direct access to output streams;
- appropriate for a custom playback clock;
- no large media framework required;
- cross-platform enough for future expansion.

### Important architectural consequence

CPAL callback is treated as real-time-sensitive.

No project locks or heavyweight work in callback.

### Decision

Use CPAL.

---

## 8. Audio decode: Symphonia

Current evaluated release: **0.6.1**.

Symphonia is a pure-Rust audio decoding/demuxing framework. Its current documented feature set includes WAV/OGG/MP4 containers and PCM/FLAC/Vorbis/AAC/MP3 codecs behind feature flags as applicable.

Why:

- Rust-native;
- avoids shelling out to FFmpeg for interactive playback;
- feature flags let us avoid enabling unnecessary codecs;
- appropriate for decode/preprocess pipeline.

### Feature policy

Do not enable a giant all-features set by default without checking compile/binary cost.

Enable only formats needed by MVP.

Likely:

- wav;
- mp3;
- aac/isomp4;
- flac;
- ogg/vorbis.

Exact Cargo features are decided during audio spike.

### Decision

Use Symphonia for imported audio decode unless a blocker appears in prototype testing.

---

## 9. Audio analysis

Do not commit to a large DSP framework for MVP.

Waveform peak extraction requires no FFT.

FFT is only needed if/when implementing:

- spectrum views;
- beat detection;
- frequency-reactive features.

Therefore:

- do not add rustfft solely for waveform generation;
- introduce it only for a specified feature.

Automatic BPM detection is not required for MVP.

---

## 10. Serialization: serde

Use serde-derived project structures where practical.

Why:

- mature;
- supports multiple formats;
- keeps model decoupled from a single text/binary encoding;
- straightforward versioned project schema.

Exact project syntax (RON vs JSON or other serde format) is a separate decision.

Avoid serializing runtime UI/GPU/audio objects.

---

## 11. Project file format

Not yet finalized.

Candidates:

### JSON

Pros:

- ubiquitous;
- easy external inspection;
- tooling everywhere.

Cons:

- verbose;
- comments unsupported;
- large float-heavy documents can be noisy.

### RON

Pros:

- pleasant Rust-centric representation;
- human-readable;
- expressive.

Cons:

- less universal tooling;
- stronger ecosystem coupling to Rust.

MVP should favor reliability and migration simplicity over compactness.

An ADR will finalize the format after PROJECT_MODEL is concrete.

---

## 12. Video export: FFmpeg

FFmpeg is the preferred encoding/muxing backend.

Use it for:

- H.264 encoding;
- MP4 muxing;
- audio muxing.

Do not use FFmpeg as the interactive renderer.

### Integration choice TBD

Options:

1. spawn a packaged FFmpeg executable;
2. link native FFmpeg libraries through bindings.

Initial preference: **process boundary** for MVP because it isolates a large native dependency and can simplify Rust build complexity.

Tradeoffs, licensing/distribution, progress parsing, and cancellation must be documented in EXPORT.md / PACKAGING.md.

---

## 13. Images

Candidate: the Rust image ecosystem.

Requirements:

- PNG;
- JPEG;
- WebP desirable;
- decode off main interaction path;
- predictable RGBA conversion;
- controlled feature set.

Before accepting a specific crate configuration, measure:

- compile time;
- binary size;
- format coverage.

---

## 14. Text stack

Not decided yet because text is a known risk area.

Requirements:

- Cyrillic;
- Unicode shaping;
- font fallback strategy;
- glyph caching/atlas;
- wgpu rendering;
- acceptable compile cost.

Candidates will be evaluated in TEXT_RENDERING.md.

Do not let egui's own UI text renderer automatically become the composition text renderer; composition typography has different requirements.

---

## 15. Math

Prefer small, explicit math types.

Options:

- own small Vec2/Transform types in core;
- glam if renderer math complexity justifies it.

Avoid leaking wgpu/egui vector types into project model.

A math crate should be accepted only if it improves code clarity enough to justify the dependency.

---

## 16. IDs

Do not add UUID by default.

MVP can use project-local typed monotonically allocated integer IDs.

Benefits:

- tiny representation;
- fast;
- deterministic;
- no random generator dependency;
- simple serialization.

Clipboard/import duplicates receive newly allocated IDs.

If future collaboration requires globally unique IDs, migration can be introduced deliberately.

---

## 17. Async/concurrency

Do not adopt Tokio as a default application runtime.

The application is primarily:

- event-loop driven;
- GPU driven;
- audio callback driven;
- worker-job driven.

Start with:

- std::thread;
- std::sync primitives;
- bounded channels or a small specialized channel crate if needed.

Add an async runtime only when concrete APIs make it beneficial.

---

## 18. Logging and diagnostics

Need:

- structured levels;
- file logging in release/dev as appropriate;
- subsystem context;
- GPU adapter/backend;
- audio device/config;
- project load/save failures;
- export diagnostics.

Candidate: tracing ecosystem.

Before adopting broad tracing subscriber stacks, measure compile cost.

---

## 19. Error handling

Core library errors should use typed enums / explicit result types.

Application boundary may use an ergonomic error wrapper for contextual propagation if needed.

Avoid adding a heavy error framework to hot-path or public core APIs merely for convenience.

---

## 20. Testing stack

Prefer built-in Rust test framework first.

Add:

- property tests only where they materially help time conversion/math;
- snapshot/golden tests selectively;
- integration harness for project roundtrip/export scenes later.

Do not create a giant testing framework before code exists.

---

## 21. Build-time policy

Every substantial dependency should be evaluated for:

- clean compile cost;
- incremental compile impact;
- transitive dependency count;
- binary size contribution;
- whether the same capability already exists in the stack.

Rules:

- disable unused default features where practical;
- avoid duplicated graphics/audio stacks;
- avoid broad "all" feature flags without reason;
- keep UI-only dependencies out of rhythm_core;
- isolate expensive code generation/macros.

---

## 22. Current technology decisions

Accepted direction, pending implementation spike:

- Rust native app;
- direct winit event loop;
- egui editor UI;
- shared wgpu composition/UI graphics stack;
- WGSL shaders;
- CPAL output;
- Symphonia decode;
- serde project model;
- FFmpeg for final encode.

The first technical prototype must validate:

1. app window + egui + wgpu coexist cleanly;
2. offscreen composition texture can be displayed in egui;
3. audio playback clock can drive animation without visible drift;
4. incremental developer loop is acceptable.

If one of these fails materially, reopen the relevant ADR rather than layering hacks on top.
