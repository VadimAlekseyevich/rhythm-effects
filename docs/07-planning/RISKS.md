# Risk Register

> **Status: Draft**
>
> This register tracks risks that can materially affect product quality, architecture, or MVP schedule.

## Scoring

Use qualitative:

- Probability: Low / Medium / High
- Impact: Low / Medium / High / Critical

Update as prototypes provide evidence.

---

## R1 — Timeline UI complexity

**Probability:** High  
**Impact:** Critical

### Risk
Timeline selection, hit testing, virtualization, zoom/pan, waveform, grid, property rows, drag semantics, and shortcuts become the largest editor subsystem.

### Mitigation
- prototype early;
- vertical implementation;
- visible-range rendering;
- centralized coordinate conversions;
- strict interaction model;
- avoid advanced AE-style graph/timeline complexity.

### Trigger
Routine timeline interaction exceeds frame budget or usability becomes inconsistent.

---

## R2 — Audio/visual synchronization

**Probability:** Medium  
**Impact:** Critical

### Risk
Preview drifts or feels late relative to audio.

### Mitigation
- audio-derived playback clock;
- no frame-delta clock;
- deterministic time conversions;
- generated sync fixtures;
- seek/playback stress tests.

### Trigger
Beat-aligned flash visibly/measureably drifts over long playback.

---

## R3 — Audio device variability

**Probability:** Medium  
**Impact:** High

### Risk
Different sample rates/buffer sizes/devices expose glitches or clock inaccuracies.

### Mitigation
- narrow CPAL boundary;
- explicit resampling strategy;
- no allocations/locks callback;
- manual device matrix.

---

## R4 — Text rendering complexity

**Probability:** High  
**Impact:** High

### Risk
Font discovery, shaping, Cyrillic, fallback, glyph caching, and export parity consume disproportionate effort.

### Mitigation
- prototype cosmic-text/glyphon early;
- limit typography scope;
- use explicit bundled fallback policy;
- test Cyrillic immediately.

---

## R5 — UI framework limitations

**Probability:** Medium  
**Impact:** High

### Risk
egui immediate-mode cost/polish constraints make complex editor UI difficult.

### Mitigation
- custom design-system layer;
- timeline virtualization;
- profile;
- direct winit/wgpu integration;
- avoid dependency on framework-specific project state.

### Escape path
UI layer can evolve because core/engine remain independent.

---

## R6 — Build-time growth

**Probability:** High  
**Impact:** High

### Risk
Rust + wgpu + UI + media dependencies make iteration slow.

### Mitigation
- three-crate start;
- dependency feature discipline;
- no unnecessary async/FFT/plugin stacks;
- measure incremental builds;
- isolate heavyweight dependencies.

---

## R7 — GPU driver/backend variability

**Probability:** Medium  
**Impact:** High

### Risk
Shaders/effects behave differently on NVIDIA/AMD/Intel or device-loss paths.

### Mitigation
- standard WGSL;
- simple pipelines;
- vendor testing;
- avoid exotic features for MVP;
- clear fallback/errors.

---

## R8 — Effects explode render cost

**Probability:** Medium  
**Impact:** High

### Risk
Per-object multipass effects destroy realtime preview.

### Mitigation
- small effect set;
- benchmark each effect;
- texture pooling;
- quality constraints;
- exclude expensive effects rather than hiding bad performance.

---

## R9 — Export readback/FFmpeg throughput

**Probability:** Medium  
**Impact:** High

### Risk
GPU readback or pipe strategy is slow/unstable.

### Mitigation
- export spike before polish phase;
- bounded pipeline;
- process isolation;
- benchmark 1080p60;
- pipeline buffers later only if necessary.

---

## R10 — Preview/export mismatch

**Probability:** Medium  
**Impact:** Critical

### Risk
Different code paths cause final output to differ from editor.

### Mitigation
- same evaluator;
- same renderer/shaders;
- offscreen preview architecture;
- parity tests at known timestamps.

---

## R11 — Project corruption/data loss

**Probability:** Low-Medium  
**Impact:** Critical

### Risk
Interrupted save/autosave damages work.

### Mitigation
- temp + replace;
- validation before swapping active project;
- recovery files;
- migration fixtures;
- never overwrite newer unknown schema.

---

## R12 — Overbuilding architecture

**Probability:** High  
**Impact:** High

### Risk
Time spent on generic engine/plugin/ECS abstractions delays first rhythm animation.

### Mitigation
- milestone gates;
- three crates;
- explicit non-goals;
- no generic event bus/async runtime unless needed.

---

## R13 — Feature creep

**Probability:** High  
**Impact:** Critical

### Risk
Particles, masks, video editing, nodes, AI, plugins, etc. expand MVP indefinitely.

### Mitigation
- MVP scope contract;
- feature filter against philosophy;
- backlog "post-MVP" bucket;
- M6 product proof before expansion.

---

## R14 — UI complexity creep

**Probability:** High  
**Impact:** Critical

### Risk
Every new feature adds controls/panels until editor becomes dense.

### Mitigation
- design-system progressive disclosure order;
- usability regression checklist;
- command search;
- contextual inspector;
- panel-count discipline.

---

## R15 — Keyframe grid restriction feels too rigid

**Probability:** Medium  
**Impact:** High

### Risk
Strict musical-grid authoring blocks legitimate motion timing.

### Mitigation
- sufficiently fine PPQ/subdivisions;
- triplets;
- keep evaluation continuous;
- prototype real projects;
- consider explicit advanced microtiming post-MVP only if evidence demands it.

Do not weaken the core product thesis preemptively.

---

## R16 — BPM setup friction

**Probability:** Medium  
**Impact:** High

### Risk
Manual BPM/offset alignment is too slow.

### Mitigation
- excellent offset nudge UX;
- waveform;
- tap tempo candidate;
- future BPM detection after MVP;
- clear beat hierarchy.

---

## R17 — Asset path portability

**Probability:** Medium  
**Impact:** Medium

### Risk
Projects break when moved between folders/machines.

### Mitigation
- relative path preference;
- unresolved asset state;
- relink flow;
- packed project later.

---

## R18 — Undo memory growth

**Probability:** Medium  
**Impact:** Medium

### Risk
Large deleted objects/keyframe sets consume memory.

### Mitigation
- semantic minimal history payloads;
- measure;
- history cap later if needed;
- no whole-project snapshot per edit.

---

## R19 — Windows packaging/antivirus friction

**Probability:** Medium  
**Impact:** Medium-High

### Risk
Unsigned binaries/bundled FFmpeg trigger warnings/quarantine.

### Mitigation
- known package structure;
- signing evaluation;
- clean-machine tests;
- public distribution planning.

---

## R20 — Documentation divergence

**Probability:** Medium  
**Impact:** Medium

### Risk
Architecture changes but docs retain obsolete contracts.

### Mitigation
- docs updated with implementation change;
- ADRs;
- document statuses;
- canonical-file rules.

---

## Review cadence

Review risks at:

- each milestone boundary;
- major tech-stack decision;
- scope change;
- significant performance regression.

Add/remove risks based on evidence.
