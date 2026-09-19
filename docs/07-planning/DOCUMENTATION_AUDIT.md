# Documentation Audit — Pre-Code MVP Architecture

> **Status: Accepted**
>
> **Audit date:** 2026-09-19
>
> **Audited HEAD before this record:** 38399210d9815705f1ebe14caa23651cc5a4c3ab

## 1. Result

The pre-code MVP documentation phase is complete.

Canonical product/technical documents in docs/00–07 were reviewed for:

- document status;
- unresolved TBD/TODO;
- Open decisions/Open questions markers;
- major cross-document contradictions;
- stale candidate/proposal language where it could conflict with an accepted decision.

Result:

- no canonical subsystem spec remains Draft;
- no canonical subsystem spec contains a known blocking TBD/Open decision;
- Project/time/animation/audio/renderer/editor/persistence/export semantics have accepted owners;
- the implementation backlog reflects the accepted architecture;
- remaining uncertainty is explicitly classified as implementation verification, benchmark measurement, hardware/library validation, or future post-MVP scope.

## 2. Accepted documentation layers

### 00 — Overview

Accepted:

- Project Philosophy;
- MVP Scope;
- Glossary;
- Documentation Rules.

### 01 — Product

Accepted:

- Product Specification;
- User Flows;
- Interaction Model;
- Design System;
- Keyboard Shortcuts.

### 02 — Architecture

Accepted:

- Architecture;
- Technology Stack;
- Project Model;
- Domain Types;
- Time Model;
- Coordinate Systems;
- Commands/Undo;
- Threading Model;
- State Ownership;
- Error Model.

### 03 — Engine

Accepted:

- Animation Engine;
- Renderer;
- Audio Engine;
- Waveform;
- Effects;
- Text Rendering.

### 04 — Editor

Accepted:

- Editor UI;
- Timeline;
- Viewport;
- Inspector;
- Assets;
- Curve Editor.

### 05 — Persistence / Export

Accepted:

- Serialization;
- Project Recovery;
- Export.

### 06 — Quality

Accepted:

- Performance;
- Testing;
- Usability;
- Packaging;
- Security/Privacy.

### 07 — Planning

Accepted/living:

- Implementation Readiness;
- Milestones;
- MVP Backlog;
- Risks;
- Decision Log;
- this audit.

## 3. ADR state

ADR index contains accepted decisions for:

- editor/renderer stack;
- time/keyframe identity;
- playback clock;
- crate boundaries;
- async-runtime policy;
- text stack;
- waveform strategy;
- entity IDs;
- JSON/.rhfx;
- FFmpeg boundary/export;
- single-writer state;
- color/alpha;
- BPM/project time;
- grid/snap;
- coordinates;
- animation segments;
- prepared PCM;
- preview working targets;
- font fallback;
- external assets;
- recovery.

Older ADRs remain historical records rather than being rewritten away.

## 4. Historical documents

The following are intentionally not canonical implementation contracts:

- /MVP_PLAN.md;
- docs/ru/MVP_PLAN_RU.md.

They are retained as the original planning history and explicitly labelled as such.

The current Russian overview is:

- docs/ru/FINAL_MVP_ARCHITECTURE_RU.md.

## 5. Remaining implementation validations

These are not architecture TODOs.

They are evidence-producing implementation tasks described in IMPLEMENTATION_READINESS.md:

- wgpu/egui offscreen composition integration;
- Rgba16Float target compatibility;
- CPAL playback timestamp quality/fallback;
- cosmic-text/glyphon system-font/Cyrillic behavior;
- FFmpeg raw-video H.264/AAC pipe;
- measured build-time baseline;
- representative GPU/audio hardware testing.

A failed validation may require an ADR revision, but implementation should first attempt the accepted contract.

## 6. What is intentionally not fixed before code

Only values that cannot be truthfully known without implementation/hardware measurement remain empirical rather than preselected.

Examples:

- exact Cargo dependency versions;
- actual clean/incremental compile times;
- actual GPU timings on specific adapters;
- exact audio backend latency on specific devices;
- exact usability completion times.

The methodology, acceptable behavior, and regression gates for those values are already documented.

## 7. Definition of Documentation Complete

The documentation phase is considered complete because a developer can answer, from canonical docs, before implementing a feature:

1. what belongs in MVP;
2. what the user sees/does;
3. what state owns the behavior;
4. what units/IDs/time domain it uses;
5. what thread may modify it;
6. how it is saved/recovered;
7. how it is evaluated/rendered/exported;
8. what errors/failures mean;
9. what performance/test/usability gate proves it works;
10. what milestone/backlog task owns implementation.

## 8. Next phase

Proceed to M1 implementation.

From this point onward:

- Accepted specs move to Implemented only when code/tests match;
- architectural changes update docs/ADR before or with code;
- documentation refinement continues, but further speculative design should not delay the first vertical slices.
