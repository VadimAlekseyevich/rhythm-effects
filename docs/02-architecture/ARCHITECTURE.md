# Architecture

> **Status: Accepted for MVP**
>
> This document defines the top-level runtime architecture and dependency direction for the MVP.

## 1. Architecture goals

The architecture must optimize for the product philosophy rather than abstract purity.

Primary goals:

1. low-latency editor interaction;
2. audio-synchronized animation;
3. deterministic preview/export evaluation;
4. clear ownership of project state;
5. fast incremental builds;
6. minimal cross-thread coordination in hot paths;
7. subsystem testability without UI;
8. enough modularity to grow without early framework-building.

Non-goal: creating a generic game engine, DAW framework, or plugin architecture before the MVP exists.

---

## 2. Top-level model

Rhythm Effects is a native desktop application with one authoritative project model.

~~~text
                       ┌────────────────────┐
                       │      Editor UI     │
                       │ egui + interaction │
                       └─────────┬──────────┘
                                 │ intents / commands
                                 ▼
                       ┌────────────────────┐
                       │   Editor Session   │
                       │ selection/history  │
                       │ transient editing  │
                       └─────────┬──────────┘
                                 │
                                 ▼
                       ┌────────────────────┐
                       │      Project       │
                       │ authoritative data │
                       └──────┬──────┬──────┘
                              │      │
                 evaluate     │      │ assets/config
                              ▼      ▼
                    ┌────────────┐  ┌─────────────┐
                    │ Animation  │  │ Asset layer │
                    │ + Time     │  │ caches      │
                    └─────┬──────┘  └──────┬──────┘
                          │ evaluated scene │
                          ▼                 │
                    ┌───────────────────────▼┐
                    │       Renderer         │
                    │      wgpu / WGSL       │
                    └────────────────────────┘

 audio file → decoder/cache → audio output
                         │
                         └──── playback clock ────► time evaluation
~~~

The UI never becomes the source of truth for project content.

---

## 3. Initial crate strategy

Do not start with a large crate graph.

Accepted initial workspace:

~~~text
crates/
├─ rhythm_core/
├─ rhythm_engine/
└─ rhythm_app/
~~~

### rhythm_core

Must stay lightweight and UI/GPU/audio-backend independent.

Owns:

- project data model;
- IDs;
- musical time;
- tempo map;
- animation/keyframes;
- interpolation math;
- edit commands/history data;
- serialization schema;
- validation;
- pure utility math where broadly shared.

Should not depend on:

- egui;
- winit;
- wgpu;
- CPAL;
- FFmpeg.

This crate should compile and test quickly.

### rhythm_engine

Owns heavyweight runtime facilities:

- renderer;
- GPU resources;
- audio decode/playback;
- waveform preprocessing/cache;
- text rendering integration;
- runtime asset loading;
- effects;
- export rendering support.

Depends on rhythm_core.

Should not contain editor widgets or product interaction logic.

### rhythm_app

Executable/editor shell.

Owns:

- winit event loop;
- egui integration;
- editor workspace;
- panel state;
- selection;
- focus;
- interaction state;
- shortcut dispatch;
- command invocation;
- project lifecycle;
- orchestration of engine services.

Depends on rhythm_core and rhythm_engine.

### Why only three crates initially

More crates may look cleaner but increase:

- API ceremony;
- compile/link boundaries;
- refactor friction;
- dependency-management overhead.

Split a crate only when there is a measured build-time, testing, reuse, or ownership benefit.

---

## 4. Dependency direction

Allowed:

~~~text
rhythm_app
  ├─► rhythm_engine
  └─► rhythm_core

rhythm_engine
  └─► rhythm_core

rhythm_core
  └─► small general-purpose dependencies only
~~~

Forbidden:

- rhythm_core importing editor/UI types;
- rhythm_core importing GPU handles;
- renderer mutating editor selection;
- audio callback mutating project data;
- UI widgets owning canonical animation data.

---

## 5. Authoritative state categories

State must be classified to prevent accidental serialization or cross-system coupling.

### 5.1. Project state

Persisted and user-authored.

Examples:

- composition settings;
- BPM/offset;
- objects;
- assets;
- keyframes;
- effects.

### 5.2. Editor session state

Not part of the creative document unless explicitly promoted later.

Examples:

- selected objects;
- selected keyframes;
- focused panel;
- timeline zoom/scroll;
- viewport camera;
- expanded property rows;
- active tool/transient interaction.

Some workspace preferences may eventually be persisted separately as app preferences.

### 5.3. Runtime cache state

Derived/rebuildable.

Examples:

- decoded image textures;
- waveform mip levels;
- glyph atlas;
- render pipelines;
- asset thumbnails;
- ID lookup indexes.

Never make a project file depend on the existence of a runtime cache.

### 5.4. Device/runtime state

Examples:

- GPU device;
- audio device;
- stream;
- window;
- swapchain/surface configuration.

Never serialize.

---

## 6. Main loop

The main thread owns the window event loop and editor frame.

Conceptual frame:

~~~text
receive OS/input events
→ update editor interaction state
→ dispatch/continue edits
→ query playback clock
→ resolve evaluation time
→ evaluate project
→ build editor UI
→ render composition + editor
→ present
~~~

The exact egui/wgpu order may differ for implementation reasons, but project mutation and time evaluation boundaries should remain explicit.

---

## 7. Audio timing boundary

Audio playback is not driven by editor FPS.

The audio subsystem exposes a monotonic playback position in its own clock domain.

The editor converts that position through the central time model and evaluates animation from it.

~~~text
audio callback / stream
        │
        ▼
atomic or lock-free playback position
        │
        ▼
ProjectTime
        │
        ▼
MusicalTime / scene evaluation
~~~

The UI may miss frames. Audio time must continue correctly.

---

## 8. Audio real-time thread rules

Audio callback code must be treated as real-time-sensitive.

Avoid in callback:

- heap allocation;
- file I/O;
- logging on normal path;
- blocking mutexes;
- project mutation;
- shader/GPU work;
- long decoding work.

Use predecoded/buffered data and lock-free or bounded communication where practical.

Exact implementation belongs in AUDIO_ENGINE.md.

---

## 9. Background work

MVP needs background work for tasks such as:

- audio decode;
- waveform generation;
- image decode/thumbnail generation;
- autosave preparation;
- potentially export.

Do not introduce a general async runtime unless a concrete need appears.

Accepted starting point:

- explicit worker thread(s);
- bounded job queue;
- result channel;
- cancellation token/atomic flag where necessary.

This keeps runtime behavior and build dependencies understandable.

---

## 10. Project mutation path

Canonical path:

~~~text
input
→ editor action
→ edit transaction / command
→ Project mutation
→ history update
→ dirty state
→ renderer observes updated project
~~~

Widgets do not independently write arbitrary project fields.

This makes:

- undo/redo;
- validation;
- autosave;
- testing;
- future collaboration/event logging

far easier to reason about.

---

## 11. Transient editing

Not every pointer movement should create permanent project history.

For drag-like operations:

~~~text
BeginInteraction
  capture before-state

UpdateInteraction
  apply transient current value for live preview

CommitInteraction
  record one semantic history entry

CancelInteraction
  restore before-state
~~~

Examples:

- object transform;
- keyframe drag;
- curve handle drag;
- numeric scrubbing;
- BPM offset drag.

---

## 12. Animation evaluation boundary

Animation evaluation is a pure conceptual operation:

~~~text
(project, evaluation_time) -> evaluated scene values
~~~

It must not depend on:

- egui frame number;
- current mouse position;
- audio backend object;
- GPU state.

The same evaluator is used by preview and export.

Caching is allowed as an optimization as long as results remain equivalent.

---

## 13. Render boundary

Renderer receives evaluated visual state plus runtime asset handles.

It should not know why a value changed.

Examples:

~~~text
EvaluatedObject {
    transform,
    opacity,
    appearance,
    resolved_asset,
    evaluated_effect_parameters,
}
~~~

Renderer is responsible for pixels, not authoring semantics.

---

## 14. Preview vs export

Preview and export share:

- project model;
- time conversion;
- animation evaluation;
- scene representation;
- rendering logic/shaders as far as practical.

They differ in their clock:

### Preview

Clock comes from playback/editor time.

### Export

Clock comes from exact frame index.

~~~text
frame_index + fps
→ exact export timestamp
→ same evaluator
→ same scene renderer
~~~

This is required for trustworthy preview/export parity.

---

## 15. UI rendering and composition rendering

The composition renderer and egui renderer may share a wgpu device/queue.

Accepted conceptual flow:

1. render composition to an offscreen texture;
2. expose that texture to the editor viewport;
3. render editor chrome through egui;
4. present final application surface.

Benefits:

- viewport scaling independent from composition resolution;
- reusable composition target for effects;
- clearer path to offline rendering;
- no screen-capture-based export.

---

## 16. Asset architecture

Persisted asset records contain identity and project-facing metadata.

Runtime asset manager contains loaded/decoded state.

~~~text
AssetId
  │
  ├─ persisted AssetRecord
  │     path/type/metadata
  │
  └─ RuntimeAsset
        decoded image / GPU texture / waveform / etc.
~~~

Missing runtime asset does not invalidate the project object graph.

---

## 17. Error boundaries

Subsystems return structured errors to orchestration code.

Avoid:

- renderer directly opening dialogs;
- decoder directly producing toasts;
- core project model depending on UI error strings.

Pattern:

~~~text
subsystem error
→ typed/contextual error
→ app policy
→ log + user-facing message if needed
~~~

---

## 18. Performance boundaries

Hot paths:

- audio callback;
- per-frame time conversion;
- animation lookup/evaluation;
- timeline visible-range processing;
- renderer command construction;
- input/drag feedback.

Cold paths:

- project migration;
- import setup;
- full waveform rebuild;
- export initialization.

Do not optimize cold paths at the expense of architectural complexity unless measurements require it.

---

## 19. Data ownership

Prefer clear ownership over pervasive shared mutable state.

Avoid a global application mutex containing everything.

Suggested ownership:

- app/editor owns Project;
- renderer owns GPU resources;
- audio engine owns stream/buffers;
- asset runtime owns decode/GPU cache mapping;
- background workers receive immutable job inputs and return results.

Cross-thread shared values should be narrow:

- playback position;
- task progress;
- cancellation flags;
- bounded queues.

---

## 20. Event model

Do not create a generic event bus for everything.

Use direct calls where ownership is clear.

Use channels only for actual asynchronous boundaries.

This reduces hidden control flow and makes editor behavior easier to debug.

---

## 21. Startup sequence

Target sequence:

1. initialize logging/crash diagnostics;
2. create event loop/window;
3. initialize wgpu;
4. initialize egui integration;
5. initialize editor session;
6. initialize audio host lazily or early depending measured startup cost;
7. show usable shell as early as practical;
8. load project/assets asynchronously where possible.

Fast perceived startup is more important than eagerly initializing every subsystem.

---

## 22. Shutdown sequence

On normal close:

1. resolve dirty-project policy;
2. stop playback;
3. stop/cancel background tasks;
4. flush required save state;
5. release runtime services.

Do not rely on destructors alone for user-data safety.

---

## 23. MVP architectural invariants

1. Project data is independent from UI widgets.
2. Keyframe authored time is musical integer time, not arbitrary float seconds.
3. Audio clock is independent from editor FPS.
4. Preview and export use the same animation evaluation semantics.
5. Core does not depend on wgpu/egui/CPAL.
6. UI changes flow through explicit edit operations.
7. Audio callback never takes a project lock.
8. Runtime caches are rebuildable.
9. A feature may not add a permanent architecture layer without concrete benefit.
10. Build-time impact is part of dependency evaluation.

---

## 24. Deferred architecture

Do not design these before required:

- plugin ABI;
- scripting VM;
- networking/collaboration;
- ECS;
- node graph;
- distributed render;
- generic media graph;
- custom async runtime;
- cross-platform abstraction beyond what chosen libraries already provide.

---

## 25. Definition of Done

Architecture is ready for implementation when:

- dependency direction is accepted;
- initial crate boundaries are accepted;
- TECH_STACK.md is accepted;
- TIME_MODEL.md is accepted;
- PROJECT_MODEL.md is sufficient to serialize a basic project;
- command/history mutation path is accepted;
- renderer/audio integration boundaries are clear;
- unresolved implementation choices are explicitly listed rather than hidden.


---

## Accepted architecture baseline

The MVP starts with exactly three application crates:

~~~text
rhythm_core
rhythm_engine
rhythm_app
~~~

Dependency direction:

~~~text
rhythm_app -> rhythm_engine -> rhythm_core
rhythm_app -----------------> rhythm_core
~~~

rhythm_core never depends on egui, wgpu, CPAL, FFmpeg, or OS UI libraries.

The active Project has one writer in the app/editor ownership path.

Engine subsystems receive semantic snapshots/commands and return runtime results; they do not acquire a project-wide mutable lock.

The event loop and normal preview rendering remain on the application/main orchestration path.

Background work is explicit and bounded rather than organized around a generic async runtime.

These boundaries are considered stable for MVP. Crate extraction requires measured build-time, ownership, or reuse justification.
