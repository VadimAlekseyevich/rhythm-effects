# Milestones

> **Status: Accepted for MVP**

Milestones are end-to-end capability gates, not module-completion labels.

## M0 — Architecture Contract — COMPLETE

Exit criteria:

- philosophy/scope accepted;
- product/interaction contracts accepted;
- time/domain/project semantics accepted;
- engine/editor/persistence/export contracts accepted;
- quality/release gates accepted;
- ADR/decision indexes established;
- implementation backlog established;
- IMPLEMENTATION_READINESS says no architecture blocker.

M0 completion does not mean the assumptions are proven on hardware; those are explicit M1–M4 verification tasks.

## M1 — Window / Foundation

Goal: native app shell and measurable development loop.

Exit:

- Cargo workspace: rhythm_core/rhythm_engine/rhythm_app;
- pinned toolchain/dependencies/Cargo.lock;
- winit window;
- wgpu Device/Queue;
- egui integration;
- Rgba16Float offscreen spike;
- base editor regions;
- tracing/logs;
- debug timing overlay shell;
- CI fmt/clippy/test/build;
- initial build-performance baseline recorded.

## M2 — First Pixel

Goal: creative renderer visible through editor.

Exit:

- offscreen composition target;
- rectangle;
- accepted coordinate/transform convention;
- viewport pan/zoom;
- sRGB/premultiplied reference checks;
- resize/DPI path;
- Full/Half preview spike.

## M3 — First Motion

Goal: deterministic Project-driven animation.

Exit:

- domain/time types;
- Project core model/IDs;
- Animated<T>;
- Hold/Linear/Bezier evaluator;
- EvaluatedScene;
- rectangle Position/Scale/Rotation/Opacity;
- ProjectEditor/history basic transactions;
- deterministic tests.

## M4 — First Sound

Goal: authoritative music playback clock.

Exit:

- required decode path baseline;
- prepared PCM;
- Rubato mismatch conversion;
- CPAL output;
- play/pause/seek;
- timestamp-clock spike passes or documented fallback implemented;
- generation semantics;
- no callback allocations/file I/O/decode;
- initial sync tests.

## M5 — First Beat

Goal: waveform + BPM grid usable.

Exit:

- 64-frame waveform peaks/mips;
- timeline ruler;
- BPM/offset;
- accepted divisions;
- playhead;
- rhythm keyboard navigation;
- smooth visible-range waveform/grid rendering.

## M6 — First Rhythm Animation

Goal: prove the product thesis end to end.

Exit:

- property keyframe affordance;
- two-key animation created on BPM grid;
- audio-driven playback;
- grid-constrained key drag;
- multi/key selection basics;
- undo/redo;
- key collision policy;
- no UI-delta clock.

## M7 — Editor Core

Exit:

- Rectangle/Ellipse/Image/Text;
- object list;
- inspector;
- viewport move/scale/rotate;
- property-scoped animation;
- multi-keyframe select/drag/copy/paste/duplicate;
- shortcuts/focus;
- easing presets + custom timing curve;
- external asset relink.

## M8 — First Durable Project

Exit:

- .rhfx V1 fixture committed;
- versioned JSON;
- safe Save/Save As/Open;
- migration framework;
- relative/absolute assets;
- missing-asset handling;
- 30-second two-generation recovery;
- forced-crash restore test.

## M9 — Visual Depth

Exit:

- all five effects;
- effect animation/order;
- cosmic-text/glyphon;
- Cyrillic;
- system fonts + Inter fallback;
- preview scale semantic parity;
- representative performance still acceptable.

## M10 — First Export

Exit:

- exact frame-index time;
- export snapshot;
- full-resolution offline render;
- bounded readback;
- FFmpeg H.264 MP4/yuv420p;
- AAC source audio;
- progress/cancel/safe publication;
- A/V sync fixture passes.

## M11 — MVP Candidate

Feature freeze except blockers.

Exit:

- PRODUCT_SPEC acceptance scenario passes;
- Medium performance gate passes;
- 30-minute audio stress passes;
- schema/recovery/export tests pass;
- 3-person formative usability observation completed;
- 60-minute editing session reviewed;
- clean Windows portable package;
- known limitations recorded;
- no critical data-loss/corruption/drift/export blockers.

## M12 — MVP Release

Exit:

- clean Windows 10/11 smoke;
- package contains FFmpeg/licenses/Inter/runtime assets;
- non-ASCII path smoke;
- recovery force-test;
- export external playback/sync;
- GitHub Release ZIP + checksum;
- all deferred work explicitly outside MVP backlog.

## Rule

A milestone is complete only when its user-visible/invariant exit criteria work together.
