# Implementation Readiness

> **Status: Accepted**
>
> This document is the gate between architecture documentation and production code.

## 1. Current decision state

The MVP has accepted contracts for:

- product philosophy/scope;
- interaction model/design tokens/hotkeys;
- crate/dependency architecture;
- domain/time/coordinate types;
- Project schema V1;
- command/undo semantics;
- ownership/threading/error model;
- animation semantics;
- audio clock and playback preparation;
- renderer working-space/preview semantics;
- waveform;
- effects;
- text/font behavior;
- editor timeline/viewport/inspector/assets/curves;
- .rhfx save/load/migration;
- recovery;
- export;
- performance/testing/usability;
- Windows packaging;
- security/privacy.

There is no known architecture question that blocks starting M1 implementation.

## 2. What "Accepted" means

Accepted documentation is the default implementation contract.

A developer should not locally invent a contradictory behavior because another implementation is easier.

If implementation proves an accepted contract impossible or materially harmful:

1. stop at the affected boundary;
2. record evidence/measurement;
3. update or supersede the ADR/spec;
4. synchronize dependent documents;
5. only then implement the changed contract.

## 3. Verification spikes are not unresolved product decisions

Several early implementation tasks intentionally validate external-library/hardware assumptions.

They do not mean the architecture is undefined.

### V1 — wgpu/egui offscreen presentation

Verify:

- one Device/Queue;
- Rgba16Float offscreen target;
- composition texture shown through egui;
- DPI/resize lifecycle.

Failure action:
- preserve renderer color/alpha semantics;
- document any format fallback with ADR.

### V2 — CPAL playback timestamps

Verify on Windows:

- OutputCallbackInfo timestamps usable;
- stream.now() correlation;
- seek-generation clock behavior.

Failure action:
- implement documented cursor/latency fallback;
- do not fall back to UI delta time.

### V3 — cosmic-text + glyphon

Verify:

- shared device;
- Cyrillic;
- system font selection;
- Inter fallback;
- deterministic bounds.

Failure action:
- replace text library behind the same project/text semantics, via ADR.

### V4 — FFmpeg raw-video pipe

Verify:

- bundled build accepts raw video;
- H.264 encoder/AAC capabilities;
- bounded readback/pipe;
- cancellation.

Failure action:
- alter transport/encoder integration behind EXPORT contract, not project timing/render semantics.

### V5 — Rgba16Float target compatibility

Test representative NVIDIA/AMD/Intel where accessible.

Failure action:
- explicit tested renderer fallback; do not silently switch color math.

## 4. Schema freeze

Project schema V1 semantics are frozen.

The first implementation fixture should be committed before M8.

After the first V1 fixture is treated as public/stable:

- persisted semantic changes require migration review;
- IDs/time/coordinate semantics must not change silently.

## 5. Performance baseline gate

Absolute compile-time numbers cannot be truthfully fixed before code exists.

As soon as M1 scaffold builds successfully:

1. record benchmark machine/toolchain;
2. measure clean debug/release;
3. measure one-line app/core incremental builds;
4. commit baseline data under docs/06-quality/baselines/;
5. apply PERFORMANCE regression thresholds afterward.

Runtime performance gates are already defined.

## 6. Dependency pin gate

Before M1 implementation is considered complete:

- pin Rust toolchain policy;
- commit Cargo.lock;
- record major dependency versions;
- disable unnecessary default features;
- verify license compatibility for redistributed components.

Exact dependency versions are implementation metadata, not architecture semantics.

## 7. Required first fixtures

Create/generated during implementation:

- minimal V1 .rhfx;
- Basic Rhythm;
- generated click + visual flash sync asset;
- Cyrillic text sample;
- sRGB transparency image;
- Long Audio;
- Timeline Stress.

Fixtures should be generated or redistributable.

## 8. Documentation update rule during code

For every meaningful PR/change:

- behavior/schema/architecture change -> update canonical spec;
- tradeoff change -> ADR;
- new risk -> RISKS.md;
- implementation state -> document status may move Accepted -> Implemented.

Do not copy implementation details into many documents; link to the canonical owner.

## 9. Definition of Ready for Coding

Ready means:

- no unresolved scope blocker;
- no unspecified canonical time unit;
- no unspecified state owner;
- no unspecified save/export safety model;
- no unspecified audio clock;
- no unspecified render color/alpha model;
- no unspecified keyframe collision/easing model;
- no unspecified project schema boundary;
- quality gates exist.

These conditions are met.

## 10. Next action

The first code task is Epic 0 / M1 foundation, not further product-scope expansion.

Documentation remains a living contract, but implementation may now begin without architecture-by-accident.
