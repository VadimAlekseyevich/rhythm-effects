# Milestones

> **Status: Draft**
>
> Milestones are vertical-slice capability gates. They exist to prevent months of isolated subsystem work without a usable product.

## M0 — Product Contract

### Goal
Enough documentation exists to start implementation deliberately.

### Exit criteria
- philosophy accepted;
- MVP scope drafted;
- architecture/time model coherent;
- core risks identified;
- initial tech stack chosen;
- first implementation backlog available.

---

## M1 — Window

### Goal
Native application starts quickly and presents editor shell.

### Exit criteria
- winit window;
- wgpu device;
- egui integration;
- base workspace regions;
- logging;
- debug frame timing;
- CI builds/tests scaffold.

No project functionality required yet.

---

## M2 — First Pixel

### Goal
Composition renderer draws a visual object inside viewport.

### Exit criteria
- offscreen composition target;
- rectangle primitive;
- viewport displays result;
- resize/DPI works;
- basic composition coordinates defined.

---

## M3 — First Motion

### Goal
A project property is animated through core animation evaluator.

### Exit criteria
- Project model exists;
- MusicalTick/time model implemented;
- Animated<T>/keyframes;
- Hold/Linear minimum;
- renderer consumes evaluated transform;
- deterministic unit tests.

Audio not required yet.

---

## M4 — First Sound

### Goal
User imports and plays a music file.

### Exit criteria
- decode one target format minimum;
- audio output;
- play/pause;
- seek;
- playback position exposed independently of editor FPS;
- no normal callback underruns in simple test.

---

## M5 — First Beat

### Goal
Waveform and BPM grid align to music.

### Exit criteria
- waveform preprocessing/cache;
- timeline ruler;
- BPM;
- offset;
- beat/subdivision grid;
- playhead;
- rhythm stepping;
- grid visuals remain smooth while zooming.

---

## M6 — First Rhythm Animation

### Goal
Prove the product thesis.

### Exit criteria
- create/select rectangle;
- add keyframe on valid grid point;
- move to another grid point;
- second keyframe;
- playback audio;
- motion visibly synchronized;
- keyframe drag constrained to musical grid;
- undo basic keyframe edit.

This is the first true product proof-of-concept.

---

## M7 — Editor Core

### Goal
Basic motion-design workflow is coherent.

### Exit criteria
- rectangle/ellipse/image/text;
- object list;
- inspector;
- viewport transforms;
- position/scale/rotation/opacity animation;
- multi-keyframe selection;
- copy/paste;
- easing presets;
- consistent focus/shortcuts;
- meaningful undo/redo.

---

## M8 — First Project

### Goal
Creative work survives application restart.

### Exit criteria
- schema v1;
- Save/Save As/Open;
- safe save;
- stable IDs;
- asset references;
- semantic round-trip tests;
- minimal autosave/recovery.

---

## M9 — Visual Depth

### Goal
MVP has enough visual capability to produce convincing examples.

### Exit criteria
- selected MVP effects;
- effect parameter animation;
- reliable text;
- basic curve editor/custom easing if still in scope;
- preview performance remains acceptable.

---

## M10 — First Export

### Goal
Create a shareable final video.

### Exit criteria
- deterministic frame timestamps;
- offline offscreen rendering;
- H.264 MP4;
- audio mux;
- progress/cancel;
- tested A/V sync;
- external playback success.

---

## M11 — MVP Candidate

### Goal
Entire workflow works without release blockers.

### Exit criteria
- primary acceptance scenario passes;
- performance baseline passes;
- critical crash/data-loss bugs resolved;
- clean Windows package;
- QA matrix executed;
- UX review complete.

Feature freeze begins except release blockers.

---

## M12 — MVP

### Goal
Release build can be handed to external users for real creative work.

### Exit criteria
- release smoke test passes on clean machine;
- recovery tested;
- export tested;
- known limitations documented;
- build/version packaged;
- milestone backlog closed or explicitly deferred.

---

## Milestone rule

Do not mark a milestone complete because individual modules exist.

The end-to-end user-visible capability must work.
