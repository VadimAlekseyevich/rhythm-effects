# Performance

> **Status: Draft**
>
> Performance includes runtime responsiveness, audio reliability, memory behavior, export throughput, startup time, and developer build/iteration speed.

## 1. Philosophy

Performance is not a post-MVP cleanup phase.

The product promise depends on the editor feeling immediate while manipulating musical timing and visual state.

At the same time, development speed matters because a UI-heavy product requires many small iterations.

We optimize two loops:

~~~text
user loop:
input → visual/audio feedback

developer loop:
edit code → compile → run → evaluate UX
~~~

Both loops need explicit budgets.

---

## 2. Performance domains

Track separately:

- application startup;
- project open;
- UI input latency;
- preview frame time;
- timeline frame time;
- animation evaluation;
- waveform interaction;
- audio callback stability;
- memory;
- asset loading;
- export throughput;
- incremental compile time;
- clean compile time;
- link time;
- test feedback time.

Do not hide all of these behind a single "FPS" metric.

---

## 3. Preview frame budget

Target baseline:

- 60 FPS minimum for normal MVP projects on supported hardware;
- 120 FPS desirable for simple scenes where display refresh allows.

Frame budget at 60 FPS:

~~~text
16.67 ms total
~~~

A healthy editor should not consume the whole budget on UI chrome.

Suggested provisional budget breakdown for a normal project:

- input/editor logic: < 2 ms typical;
- timeline/UI: < 4 ms typical;
- animation evaluation: < 1 ms typical;
- composition rendering: < 8 ms typical;
- margin/present/driver variability: remaining budget.

These are diagnostic targets, not hard architectural contracts until measured.

---

## 4. Input latency

User actions that should feel immediate:

- selecting keyframe;
- dragging keyframe;
- moving object;
- changing property;
- play/pause;
- moving playhead;
- zooming timeline.

Avoid any routine input path that waits for:

- file I/O;
- audio decode;
- GPU readback;
- full project serialization;
- full waveform rebuild;
- expensive global layout.

---

## 5. Timeline scalability

Timeline cost should depend primarily on visible content, not total project size.

Required strategy:

- visible row culling;
- visible time-range filtering;
- binary/range lookup for keyframes;
- no full-project scan per frame unless benchmark proves harmless;
- cached text/layout where sensible.

### Benchmark fixtures

Small:
- 10 objects;
- 100 keyframes.

Medium:
- 100 objects;
- 1,000 keyframes.

Stress:
- 500 objects;
- 10,000+ keyframes.

MVP performance acceptance is based mainly on Small/Medium.

Stress reveals algorithmic problems.

---

## 6. Animation evaluation

Evaluation should avoid:

- allocation per property;
- linear search through all keyframes;
- unnecessary cloning;
- repeated time conversion work that can be shared.

Use sorted keyframes and binary search/range cursor strategies.

Measure actual scene evaluation cost independently from rendering.

---

## 7. Audio callback

The audio callback has stricter requirements than UI.

Callback rules:

- no heap allocation on normal path;
- no file I/O;
- no blocking locks;
- no logging per callback;
- no project mutation;
- bounded predictable work.

Track:

- underruns;
- callback duration if observable;
- buffer starvation;
- device reconfiguration failures.

Any audible glitch during normal editing is a serious bug.

---

## 8. Waveform

Waveform rendering must never process raw PCM proportional to whole song length every frame.

Use:

- precomputed min/max levels;
- multiple resolutions;
- visible-range slicing;
- batched drawing.

Zoom/pan should stay smooth on multi-minute tracks.

---

## 9. Renderer

Track:

- draw calls;
- bind-group changes;
- intermediate textures;
- temporary allocations;
- effect pass count;
- GPU readback;
- texture upload frequency.

Avoid optimizing draw calls blindly before measurement.

Correct batching/caching should naturally prevent pathological behavior.

---

## 10. Effects

Effects can multiply rendering cost.

MVP effect stack should remain bounded and observable.

For each effect measure:

- added GPU passes;
- intermediate texture allocation;
- resolution dependence;
- per-object vs shared costs.

Potentially expensive effects should expose quality limits or be excluded from MVP rather than making normal preview unreliable.

---

## 11. Text rendering

Measure:

- shaping/layout;
- glyph cache misses;
- atlas updates;
- per-frame text rebuild;
- large text object counts.

Text content that has not changed should not be reshaped every frame unnecessarily.

---

## 12. Asset loading

Import/decode may run in background.

Do not block the main UI while:

- decoding large image;
- generating thumbnail;
- building waveform.

The editor should show progressive readiness where practical.

---

## 13. Memory

Track at minimum:

- project semantic data;
- decoded PCM;
- waveform caches;
- decoded images;
- GPU textures;
- intermediate render targets;
- text atlases;
- history/undo;
- export buffers.

Avoid hidden duplicate full-size images/PCM where one representation can be shared.

---

## 14. Export

Export throughput is measured separately from realtime preview.

Correctness first.

Track:

- evaluation time/frame;
- GPU render time;
- GPU readback;
- pixel conversion;
- encoder throughput.

Memory must remain bounded with project duration.

---

## 15. Startup

Product expectation:

the editor shell should appear quickly.

Provisional target on a typical development machine:

- warm startup to usable shell: ideally < 1 second;
- cold startup: keep comfortably low, exact target measured later.

Do not eagerly initialize every codec/cache before the window appears if it can be lazy.

---

## 16. Project open

Separate:

- parse/migrate;
- semantic validation;
- editor availability;
- asset readiness.

The project shell should become interactable before every nonessential thumbnail/cache is ready.

---

## 17. Build-time performance

Build speed is a first-class metric.

Track on a known development machine:

### Incremental UI edit
A small change in rhythm_app followed by debug build.

### Core edit
A small rhythm_core change.

### Shader change
If runtime shader loading allows avoiding Rust rebuild, measure that workflow.

### Clean debug build

### Clean release build

Targets should be established after initial scaffold, then regression-tracked.

---

## 18. Dependency budget

Before adding a significant dependency, evaluate:

- value;
- transitive crate count;
- compile time;
- feature flags;
- native dependencies;
- duplicate functionality.

Prefer disabling default features when unused.

Do not add an async ecosystem, image formats, codec families, or serialization stacks merely "for later."

---

## 19. Crate boundaries and compile speed

Initial small crate graph is intentional.

A core change that rebuilds wgpu/egui-heavy code too often may justify boundary refinement.

Do not split crates based only on conceptual purity.

Use build measurements.

---

## 20. Profiling

Provide easy opt-in instrumentation.

Candidate tools/approaches:

- tracing spans;
- custom frame timing overlay;
- per-subsystem rolling averages;
- GPU timestamp queries later if needed;
- external profilers for deep investigations.

A hidden debug overlay may show:

- FPS/frame ms;
- UI ms;
- scene eval ms;
- visible keyframes;
- draw calls;
- GPU texture memory estimate;
- audio underruns;
- background jobs.

---

## 21. Regression policy

A feature PR/commit that causes a clear performance regression on core interaction should not be accepted silently.

Document:

- before;
- after;
- reason;
- whether regression is acceptable.

Large regressions need explicit tradeoff decision.

---

## 22. Performance test projects

Keep version-controlled generated/fixture descriptions where practical.

Fixtures:

- Empty;
- Basic Rhythm;
- Medium Motion;
- Text Heavy;
- Effects Heavy;
- Timeline Stress;
- Long Audio.

Do not require committing large copyrighted audio/video assets.

Use generated or redistributable test media.

---

## 23. Debug vs release

Debug Rust/wgpu performance may differ substantially.

Use:

- debug for iteration correctness;
- optimized/dev profile tuning if needed;
- release for meaningful runtime acceptance.

Do not dismiss catastrophic debug iteration performance either; it affects development UX.

---

## 24. MVP performance gates

Before MVP candidate:

- no routine UI operation causes visible multi-second stalls;
- medium timeline remains comfortably interactive;
- audio playback has no known normal-workflow underruns;
- 60 FPS preview achievable on representative scenes/hardware;
- project save/autosave does not cause disruptive hitching;
- export memory bounded;
- startup acceptable;
- build-time baseline documented.

---

## 25. Open decisions

- representative Windows hardware baseline;
- exact startup target;
- exact medium-scene frame budget;
- target maximum project duration;
- decoded PCM strategy/memory limits;
- acceptable build-time budgets;
- debug profile optimization settings.

---

## 26. Definition of Done

Performance documentation is actionable when:

- benchmark fixtures exist;
- debug timing overlay exists;
- startup/frame/audio/build metrics can be recorded;
- baseline numbers are checked into docs or CI artifacts;
- no obvious O(total project size) timeline rendering path remains;
- build-time regression is considered during dependency decisions.
