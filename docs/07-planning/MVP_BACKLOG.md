# MVP Backlog

> **Status: Draft**
>
> This is an implementation-oriented index. Canonical behavior remains in subsystem specs.

## Backlog principles

- Work by vertical slice.
- Every task references a spec.
- Avoid giant "implement subsystem" tasks.
- Prefer demonstrable capability at the end of each epic.
- Performance/test work is included inside epics, not deferred completely to the end.
- Anything outside MVP scope goes to Post-MVP rather than quietly entering active work.

---

# Epic 0 — Repository / Tooling

## Goal
Create a fast development foundation.

### Tasks
- [ ] Create Cargo workspace.
- [ ] Create rhythm_core.
- [ ] Create rhythm_engine.
- [ ] Create rhythm_app.
- [ ] Pin initial dependency versions.
- [ ] Configure rustfmt.
- [ ] Configure clippy.
- [ ] Add basic CI.
- [ ] Add logging bootstrap.
- [ ] Add debug/release profiles.
- [ ] Add basic benchmark/timing utility.

### Exit
M1 Window prerequisites ready.

---

# Epic 1 — Native App Shell

### Tasks
- [ ] winit event loop.
- [ ] application window.
- [ ] wgpu adapter/device/queue.
- [ ] egui-winit integration.
- [ ] egui-wgpu rendering.
- [ ] base workspace panels.
- [ ] DPI/resize handling.
- [ ] frame timing overlay.
- [ ] clean shutdown path.

### Specs
- ARCHITECTURE.md
- TECH_STACK.md
- EDITOR_UI.md

---

# Epic 2 — Core Time

### Tasks
- [ ] MusicalTick newtype.
- [ ] BPM validated type.
- [ ] GridOffset type.
- [ ] TimeSignature.
- [ ] PPQ constant/contract.
- [ ] GridResolution.
- [ ] tick ↔ project time.
- [ ] frame index ↔ project time.
- [ ] snap nearest/floor/ceil.
- [ ] negative/pre-roll tests.
- [ ] decimal BPM tests.

### Exit
No UI yet required; deterministic time unit tests pass.

---

# Epic 3 — Project Model

### Tasks
- [ ] typed IDs.
- [ ] Project root.
- [ ] Composition settings.
- [ ] Object enum/model.
- [ ] Transform.
- [ ] Asset records.
- [ ] effect placeholder/model.
- [ ] validation.
- [ ] ID indexes/helpers.

---

# Epic 4 — Animation Core

### Tasks
- [ ] Animated<T>.
- [ ] Keyframe<T>.
- [ ] sorted insert/delete.
- [ ] Hold.
- [ ] Linear.
- [ ] easing presets.
- [ ] cubic Bezier representation.
- [ ] evaluate at arbitrary continuous time.
- [ ] keyframe collision policy implementation.
- [ ] unit tests.

### Exit
M3 First Motion core available.

---

# Epic 5 — Minimal Renderer

### Tasks
- [ ] composition offscreen target.
- [ ] viewport texture presentation.
- [ ] rectangle.
- [ ] alpha blending.
- [ ] transform matrix.
- [ ] background.
- [ ] resize/recreate targets.
- [ ] renderer profiling counters.

### Exit
M2 First Pixel, then integrate M3 First Motion.

---

# Epic 6 — Commands / History

### Tasks
- [ ] ProjectEditor mutation boundary.
- [ ] HistoryEntry model.
- [ ] undo/redo cursor.
- [ ] dirty revision model.
- [ ] transaction begin/update/commit/cancel.
- [ ] object commands.
- [ ] keyframe commands.
- [ ] property commands.
- [ ] tests.

---

# Epic 7 — Audio Decode / Playback

### Tasks
- [ ] Symphonia decode spike.
- [ ] target format support.
- [ ] PCM representation.
- [ ] CPAL device/output.
- [ ] buffering.
- [ ] play/pause.
- [ ] seek.
- [ ] playback-position clock.
- [ ] sample-rate mismatch/resampling spike.
- [ ] underrun diagnostics.
- [ ] audio fixture tests.

### Exit
M4 First Sound.

---

# Epic 8 — Waveform

### Tasks
- [ ] peak extraction.
- [ ] multiresolution levels.
- [ ] cache/query API.
- [ ] background preprocess.
- [ ] timeline waveform draw.
- [ ] visible-range rendering.
- [ ] generated waveform tests.

---

# Epic 9 — Timeline Foundation

### Tasks
- [ ] ruler coordinates.
- [ ] time ↔ x.
- [ ] zoom/pan.
- [ ] playhead.
- [ ] bar/beat/subdivision grid.
- [ ] waveform integration.
- [ ] object rows.
- [ ] property rows.
- [ ] visible-range virtualization.

### Exit
M5 First Beat.

---

# Epic 10 — Keyframe Timeline Editing

### Tasks
- [ ] draw keyframes.
- [ ] hit targets.
- [ ] single select.
- [ ] multi-select.
- [ ] box select.
- [ ] grid-constrained drag.
- [ ] multi-drag.
- [ ] delete.
- [ ] copy/paste.
- [ ] duplicate.
- [ ] keyboard rhythm stepping.
- [ ] collision behavior.
- [ ] undo integration.

### Exit
M6 First Rhythm Animation.

---

# Epic 11 — Viewport Editing

### Tasks
- [ ] object selection.
- [ ] pan/zoom.
- [ ] transform gizmo.
- [ ] position drag.
- [ ] scale.
- [ ] rotation.
- [ ] anchor visualization.
- [ ] transient commands.
- [ ] selection synchronization.

---

# Epic 12 — Inspector

### Tasks
- [ ] common property rows.
- [ ] numeric editing transaction.
- [ ] keyframe affordance.
- [ ] current-keyframe indication.
- [ ] animated-state indication.
- [ ] Rectangle/Ellipse properties.
- [ ] Image properties.
- [ ] Text properties.
- [ ] progressive disclosure.

---

# Epic 13 — Additional Visual Objects

### Tasks
- [ ] ellipse renderer.
- [ ] image decode/upload.
- [ ] image object.
- [ ] text stack spike.
- [ ] Cyrillic.
- [ ] multiline.
- [ ] text GPU rendering.
- [ ] preview/export-compatible layout.

---

# Epic 14 — Product Hotkeys / Focus

### Tasks
- [ ] shortcut dispatcher.
- [ ] focus scope model.
- [ ] text-entry protection.
- [ ] transport shortcuts.
- [ ] beat/subdivision stepping.
- [ ] delete/duplicate/copy/paste.
- [ ] shortcut display in tooltips/menus.
- [ ] prototype conflict tests.

---

# Epic 15 — Easing / Curves

### Tasks
- [ ] preset easing UI.
- [ ] apply easing to selection.
- [ ] cubic Bezier evaluator.
- [ ] minimal curve editor.
- [ ] curve handle drag transaction.
- [ ] curve tests.

---

# Epic 16 — Effects

### Tasks
- [ ] effect data model final.
- [ ] effect stack inspector.
- [ ] intermediate targets/pooling.
- [ ] first effect.
- [ ] selected remaining MVP effects.
- [ ] Animated<T> parameters.
- [ ] performance measurements.
- [ ] preview/export parity.

---

# Epic 17 — Serialization

### Tasks
- [ ] choose JSON/RON ADR.
- [ ] schema V1.
- [ ] serializer.
- [ ] parser.
- [ ] semantic validation.
- [ ] stable-ID round trip.
- [ ] safe temp/replace save.
- [ ] Save/Save As/Open.
- [ ] migration framework.
- [ ] fixtures.

### Exit
M8 First Project.

---

# Epic 18 — Recovery

### Tasks
- [ ] recovery directory.
- [ ] session ID/metadata.
- [ ] autosave trigger.
- [ ] background recovery write.
- [ ] clean-shutdown lifecycle.
- [ ] startup scan.
- [ ] Restore/Discard UI.
- [ ] crash/recovery tests.

---

# Epic 19 — Export

### Tasks
- [ ] exact frame-index timestamps.
- [ ] export offscreen target.
- [ ] GPU readback.
- [ ] FFmpeg process spike.
- [ ] raw frame feed.
- [ ] audio mux.
- [ ] progress.
- [ ] cancel.
- [ ] temporary output publication.
- [ ] sync tests.

### Exit
M10 First Export.

---

# Epic 20 — Performance / Stability

### Tasks
- [ ] create benchmark fixtures.
- [ ] profile timeline.
- [ ] profile renderer.
- [ ] profile animation.
- [ ] measure audio underruns.
- [ ] measure memory.
- [ ] measure startup.
- [ ] measure build times.
- [ ] fix major regressions.
- [ ] stress undo/save/open.

---

# Epic 21 — UX / Design Pass

### Tasks
- [ ] establish final design tokens.
- [ ] typography.
- [ ] target sizes.
- [ ] focus visuals.
- [ ] selected states.
- [ ] empty states.
- [ ] tooltip consistency.
- [ ] shortcut discoverability.
- [ ] user observation sessions.
- [ ] action-count review.
- [ ] resolve common friction.

---

# Epic 22 — Packaging / MVP QA

### Tasks
- [ ] release profile.
- [ ] FFmpeg packaging.
- [ ] license notices.
- [ ] portable package.
- [ ] installer decision.
- [ ] clean-machine test.
- [ ] Windows 10/11 test.
- [ ] DPI test.
- [ ] GPU matrix.
- [ ] audio device matrix.
- [ ] full acceptance scenario.
- [ ] known limitations.

---

# Post-MVP bucket

Do not pull into MVP without explicit scope decision:

- 3D;
- particles;
- advanced masks;
- motion tracking;
- scripting/expressions;
- plugins;
- node graph;
- cloud/collaboration;
- multi-track audio;
- automatic BPM detection unless promoted;
- packed project;
- macOS/Linux release;
- sophisticated typography;
- auto-update.
