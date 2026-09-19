# Performance

> **Status: Accepted for MVP**
>
> Performance includes runtime responsiveness, audio reliability, memory behavior, export throughput, startup time, and developer iteration speed.

## 1. Performance philosophy

Rhythm Effects has two latency loops:

~~~text
creator:
input -> visual/audio feedback

developer:
edit -> compile -> run -> judge UX
~~~

Both matter.

Performance work is architectural, measured, and regression-controlled rather than postponed to a final optimization phase.

## 2. Reference project classes

### Small

- 10 objects;
- 100 keyframes;
- 1080p composition;
- 0–2 effects per object;
- one 3-minute audio track.

### Medium — primary MVP performance gate

- 100 objects;
- 1,000 keyframes;
- 1080p composition;
- mixed shapes/images/text;
- representative effects;
- one 10-minute audio track.

### Stress

- 500 objects;
- 10,000+ keyframes;
- 4K composition;
- heavy effect/text density.

Stress exists to expose algorithmic failure. It is not the normal release smoothness promise.

## 3. Preview frame budget

Primary release target on representative supported hardware:

~~~text
60 FPS
16.67 ms/frame
~~~

For the Medium fixture at 1080p/Auto preview:

- p95 frame time <= 16.67 ms;
- p99 frame time <= 25 ms during non-loading steady editing;
- no repeated >50 ms hitch during ordinary keyframe/viewport manipulation.

Simple scenes may run at 120+ FPS, but 120 FPS is not an MVP release requirement.

## 4. Subsystem diagnostic budgets

For Medium fixture, typical steady-state CPU targets:

- input/editor command logic: <= 2 ms/frame;
- timeline + UI layout/draw CPU: <= 4 ms/frame;
- animation evaluation: <= 1 ms/frame;
- renderer CPU command encoding: <= 3 ms/frame.

These are diagnostic budgets, not individually user-visible release gates. GPU time consumes the remainder of the 16.67 ms target.

## 5. Interaction latency

No routine interaction may synchronously wait on:

- disk I/O;
- media decode;
- waveform build;
- project serialization;
- GPU readback;
- FFmpeg;
- full-project traversal not required by the operation.

Target:

- pointer/keyboard edit should become visible by the next rendered frame under normal load.

## 6. Timeline complexity

Frame work depends primarily on visible content.

Required:

- visible-row virtualization;
- visible MusicalTick range query;
- sorted keyframe range lookup;
- no O(total keyframes) drawing/hit-test each frame;
- no one-widget-per-invisible-keyframe architecture.

10,000-key stress fixture must remain navigable even if it does not sustain the Medium 60 FPS gate.

## 7. Animation evaluation

Avoid per-frame:

- heap allocation per property;
- full keyframe scans;
- cloning keyframe vectors;
- full Project validation.

Use binary/range lookup and rebuildable caches.

## 8. Audio realtime gate

Normal playback callback:

- zero heap allocation on normal path;
- zero file I/O;
- zero resampling/decode;
- zero blocking project locks;
- bounded work.

Release gate on reference wired/local output:

- no known underruns in 30-minute stress playback under Medium editor interaction;
- no cumulative A/V drift;
- systematic steady sync error <= 25 ms, target <= 10 ms where timestamp backend is reliable.

Bluetooth/wireless latency is best-effort and documented.

## 9. Waveform

A 10-minute track:

- preprocessing runs outside UI/audio callback;
- timeline pan/zoom does not scan raw PCM;
- visible waveform rendering remains within timeline budget.

## 10. Renderer

Track:

- render passes;
- draw calls;
- isolated effect objects;
- temporary target count;
- texture uploads;
- working-target memory;
- CPU encode time;
- GPU time where timestamps are available.

No ordinary preview GPU readback.

## 11. Effects

Each effect benchmark records cost at:

- 1080p Full;
- 4K Auto preview;
- full-resolution export.

If an effect makes representative Medium editing miss the frame gate, optimize/isolate its quality cost before release rather than hiding the regression.

## 12. Text

Unchanged text must not reshape every frame.

Text-heavy fixture:

- 50 TextObjects;
- Latin/Cyrillic mix;
- several fonts;
- animated transforms/colors.

## 13. Asset loading

Decode/upload is progressive/background.

Large image import may take time, but existing timeline/transport remain responsive.

## 14. Memory

Track separately:

- semantic Project/history;
- prepared audio buffer;
- waveform peaks;
- image CPU staging;
- GPU textures;
- Rgba16Float working/effect targets;
- text atlas;
- export readback buffers.

Normal export memory is bounded with duration.

After audio preparation, do not retain both full source PCM and output-rate PCM indefinitely.

## 15. Startup

Release target on the project's recorded reference machine:

- warm launch to interactive shell: <= 1.5 s;
- cold launch target: <= 3 s.

Nonessential font/media caches may initialize lazily after shell appears.

## 16. Project open

For a Medium .rhfx with local assets available:

- semantic JSON parse/migrate/validate target <= 500 ms on reference machine;
- editor may become interactive before every image thumbnail/runtime asset is ready.

Missing media must not stall indefinitely.

## 17. Save/recovery

Medium explicit Save:

- target semantic serialization/snapshot <= 100 ms;
- disk publication may continue through a safe short operation;
- no audible playback interruption.

Autosave/recovery must not create a visible repeated hitch.

## 18. Export

Correctness precedes realtime speed.

Track:

- animation eval;
- render;
- readback;
- FFmpeg throughput;
- memory.

No release requirement that export be realtime.

A 1080p60 Basic Rhythm fixture should not exhibit pathological per-frame startup/allocation cost.

## 19. Build-time performance

Absolute Rust compile time depends heavily on CPU, linker, cache state, and filesystem.

Therefore the first successful scaffold build establishes a version-controlled baseline file containing:

- machine CPU/RAM/OS;
- Rust toolchain;
- clean debug build;
- clean release build;
- one-line rhythm_app incremental edit;
- one-line rhythm_core incremental edit;
- test-suite time.

Before that first measurement, architecture uses these guardrails:

- no new heavyweight dependency without justification;
- avoid default feature bloat;
- only three main crates initially;
- no general async ecosystem;
- shaders should not require Rust recompilation during normal shader-only iteration if hot/reload workflow is later added.

## 20. Build regression gate

After baseline exists:

- routine incremental build regression >20% requires investigation;
- clean-build regression >15% from one dependency/architecture change requires explicit acceptance;
- unit/core test loop should remain fast enough for frequent local execution.

Regression percentages are compared on the same benchmark machine with warm/cold conditions recorded consistently.

## 21. Instrumentation

Debug build includes an opt-in diagnostics overlay showing at least:

- FPS/frame ms rolling p50/p95;
- UI/timeline CPU ms;
- animation eval ms;
- renderer CPU ms;
- visible key count;
- draw/pass count;
- preview scale;
- background queue/jobs;
- audio errors/clock state;
- approximate major memory buckets.

Use tracing for subsystem spans/events.

## 22. Benchmark fixtures

Version-controlled generated/redistributable fixtures:

- Empty;
- Basic Rhythm;
- Medium Motion;
- Text Heavy;
- Effects Heavy;
- Timeline Stress;
- Long Audio.

No copyrighted test media is required.

## 23. Regression policy

A significant core-workflow performance regression is documented with:

- fixture;
- before;
- after;
- machine;
- reason;
- accepted mitigation/tradeoff.

Do not silently move performance targets to accommodate a regression.

## 24. Definition of Done

Performance is MVP-ready when:

- benchmark environment/baselines are recorded;
- Medium fixture passes frame gates;
- 30-minute audio stress has no known underruns/drift growth;
- startup/open/save targets are measured;
- timeline stress proves visible-range architecture;
- export memory is duration-bounded;
- build-time regression tracking exists.
