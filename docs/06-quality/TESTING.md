# Testing

> **Status: Draft**
>
> This document defines how Rhythm Effects proves correctness across time, animation, persistence, rendering, audio synchronization, interaction, and release behavior.

## 1. Principles

Testing should focus strongest automation on deterministic core logic.

UI feel still requires human review.

Testing layers:

1. unit tests;
2. property/invariant tests;
3. integration tests;
4. renderer/reference tests where practical;
5. end-to-end project fixtures;
6. manual UX/release QA.

---

## 2. Highest-risk correctness areas

Prioritize:

- time conversion;
- grid snapping;
- keyframe ordering/collisions;
- animation interpolation;
- undo/redo;
- save/load/migration;
- audio playback position;
- A/V export sync;
- missing/corrupt assets;
- renderer device/resize lifecycle.

---

## 3. Core unit tests

rhythm_core should have fast tests for:

- ID allocation;
- time conversion;
- PPQ/grid resolution;
- BPM validation;
- tempo map;
- project validation;
- keyframe insert/delete/move;
- interpolation;
- command/history operations;
- serialization semantic helpers.

These tests should run in seconds, not minutes.

---

## 4. Time-model tests

Required examples:

- tick zero equals grid offset;
- 120 BPM quarter note = 0.5 s;
- decimal BPM;
- negative/pre-roll tick behavior;
- exact common subdivision tick counts;
- repeated step forward/back returns exact original tick;
- frame timestamp derived from index does not accumulate error;
- nearest snap boundary policy deterministic.

Use table-driven tests.

---

## 5. Animation tests

For each supported animated type:

- before first keyframe;
- exactly first keyframe;
- between keyframes;
- exactly second keyframe;
- after last keyframe;
- Hold;
- Linear;
- easing presets;
- cubic Bezier edge cases;
- duplicate/collision prevention.

Preview and export evaluator call same tested implementation.

---

## 6. Command/history tests

Test semantic sequences:

~~~text
edit → undo → original
edit → undo → redo → edited
~~~

Include:

- property edit;
- object create/delete;
- keyframe move;
- multi-keyframe move;
- effect change;
- BPM/offset change;
- compound paste;
- redo truncation after new edit;
- drag transaction cancellation.

---

## 7. Serialization tests

Keep fixture files.

Tests:

- current schema round trip;
- migration from every historical schema fixture;
- unknown future schema;
- malformed syntax;
- invalid semantic references;
- duplicate IDs;
- missing assets;
- safe-save behavior.

Never delete old migration fixtures casually.

---

## 8. Waveform tests

Given known PCM:

- min/max aggregation correct;
- multilevel reduction correct;
- final partial bucket correct;
- stereo combine policy correct;
- visible-range query boundaries correct.

Waveform visual tests may use generated deterministic PCM.

---

## 9. Audio engine tests

Pure audio logic can be automated:

- decode known short fixtures;
- seek mapping;
- resampling buffer behavior;
- playback-position math.

Device callback behavior requires integration/manual testing across devices.

Do not make CI depend on a physical audio output device.

---

## 10. Renderer tests

GPU tests can be fragile across drivers.

Prioritize deterministic logic separately from pixel output.

Useful renderer tests:

- offscreen target creation;
- coordinate conversion;
- primitive scene render;
- reference screenshots with tolerant comparison on controlled CI only if stable;
- shader compilation;
- resize/recreate paths.

Do not require exact per-pixel equality across unrelated GPUs unless proven stable.

---

## 11. Golden/reference scenes

Maintain small deterministic scene descriptions:

- one rectangle transform;
- opacity;
- image;
- text;
- easing;
- blur/effect;
- overlapping alpha.

For selected timestamps, store expected semantic evaluated values.

Pixel references can supplement but semantic references remain portable.

---

## 12. Export tests

Automated:

- export short test project;
- FFmpeg returns success;
- probe output dimensions/FPS/duration/audio stream;
- frame count reasonable;
- cancellation cleanup.

Timing fixture:

- generated click audio;
- visual flash aligned to known beat.

Validate sync numerically using controlled media analysis where practical.

---

## 13. End-to-end project fixtures

Create representative fixtures:

### Basic Rhythm
Small shape animation.

### Mixed Assets
Image + text + shape.

### Easing
Different interpolation types.

### Effects
Supported effect stack.

### Long Timeline
Many keyframes.

### Missing Asset
Intentional unresolved file.

Fixtures should use redistributable/generated assets.

---

## 14. UI logic tests

Where UI logic can be separated from rendering, test:

- selection semantics;
- focus shortcut routing;
- keyframe drag resolution;
- viewport/timeline coordinate math;
- box selection;
- command enable/disable rules.

Do not over-invest in brittle screenshot tests for every control.

---

## 15. Manual interaction QA

Required checks:

- small target comfort;
- high-DPI scaling;
- keyboard focus;
- Escape/cancel;
- text-entry shortcut conflicts;
- key repeat;
- drag cancellation;
- timeline zoom orientation;
- selection coherence between panels.

See USABILITY.md.

---

## 16. Audio/manual device matrix

Before MVP, test representative:

- built-in motherboard/headphone output;
- USB audio device if available;
- Bluetooth output if supported/usable;
- sample rates 44.1 kHz and 48 kHz;
- device switching/error behavior.

Document known limitations.

---

## 17. GPU/manual matrix

At least test:

- NVIDIA;
- AMD;
- Intel integrated graphics where accessible.

Not every vendor needs perfect stress performance, but normal editor rendering must function.

---

## 18. Windows matrix

MVP target:

- Windows 10;
- Windows 11;
- common scaling such as 100%, 125%, 150%.

Architecture may support other OSes later, but they are not MVP release gates.

---

## 19. Fuzz/property testing

Potential high-value targets:

- serialized project parser/migrations;
- time/grid conversions;
- random command sequences preserving invariants.

Introduce only where it adds value without slowing normal iteration.

---

## 20. CI

Minimum CI:

- formatting;
- clippy;
- unit tests;
- build.

Later:

- Windows release compile;
- selected integration tests;
- project migration fixtures;
- shader validation.

Heavy GPU/audio tests can remain scheduled/manual if CI environment is unreliable.

---

## 21. Bug regression rule

Every fixed deterministic core bug should receive a regression test when practical.

Examples:

- wrong snap at negative tick;
- undo collision corruption;
- migration drops effect;
- export frame off by one.

---

## 22. Release smoke test

On clean installed/portable build:

1. launch;
2. new project;
3. import test audio;
4. set BPM;
5. create shape;
6. create two keyframes;
7. preview;
8. save;
9. reopen;
10. undo/redo;
11. add text/image;
12. export;
13. play result externally.

---

## 23. Failure testing

Explicitly test:

- invalid project file;
- disk write denied;
- missing audio/image;
- FFmpeg unavailable/fails;
- audio device fails;
- window minimize/restore;
- resize during playback;
- export cancellation;
- recovery restoration.

---

## 24. Performance testing

Testing includes performance regression checks from PERFORMANCE.md.

Keep correctness and benchmark tests conceptually separate so timing noise does not fail ordinary unit CI unnecessarily.

---

## 25. Definition of Done

Testing strategy is MVP-ready when:

- core deterministic systems have automated coverage;
- schema migration fixtures exist;
- representative E2E projects exist;
- export smoke/timing tests exist;
- release QA checklist is repeatable;
- supported Windows/GPU/audio combinations have been manually exercised;
- major bugs can be reproduced through tests or fixtures where practical.
