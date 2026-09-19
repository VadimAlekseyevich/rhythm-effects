# User Flows

> **Status: Draft**
>
> This document describes complete user journeys. It intentionally focuses on user intent and observable behavior rather than implementation.

## 1. Flow design rules

Every primary flow should optimize for:

- low action count;
- few modal interruptions;
- visible system state;
- reversible actions;
- rhythm-first navigation;
- clear keyboard path and clear pointer path.

A routine flow should not require visiting multiple unrelated panels.

---

## 2. First launch → first rhythm animation

### Goal

A new user reaches a moving object synchronized to music as quickly as possible.

### Flow

1. Launch Rhythm Effects.
2. Choose New Project.
3. Enter editor immediately.
4. Empty audio area presents Import Audio.
5. User imports a track.
6. Waveform processing begins.
7. User enters BPM.
8. Beat grid appears.
9. User adjusts offset until beat lines match waveform.
10. User creates Rectangle.
11. Rectangle appears selected in viewport.
12. Inspector shows transform properties.
13. User places playhead on a grid point.
14. User creates Position keyframe.
15. User moves playhead several grid steps.
16. User moves rectangle.
17. User creates/updates the second Position keyframe.
18. User presses Play.
19. Audio and motion play in sync.

### Success condition

The user creates a synchronized animation without opening a graph editor, settings maze, or nested timeline mode.

---

## 3. BPM alignment

### Goal

Align musical grid to a known-BPM song.

### Pointer path

1. Select BPM field.
2. Enter BPM.
3. Grid appears.
4. Drag/nudge grid offset while watching waveform.
5. Change subdivision if needed.
6. Verify several later beats visually and by playback.

### Keyboard path

1. Focus BPM command/field.
2. Enter BPM.
3. Use offset nudge shortcuts.
4. Jump beat-to-beat to verify.

### Required feedback

- BPM value;
- exact offset;
- current bar/beat;
- visually stronger beat/bar lines;
- waveform remains visible during alignment.

### Failure/edge cases

- invalid BPM input;
- extremely low/high BPM;
- waveform still preprocessing;
- track contains pickup before first downbeat.

---

## 4. Create and animate a shape

1. Create Rectangle.
2. Object appears centered or at a sensible default location.
3. Object is selected.
4. Position/Scale/Rotation/Opacity visible in inspector.
5. Move playhead to target grid point.
6. Add keyframe for desired property.
7. Move to another grid point using rhythm navigation.
8. Change value.
9. Add second keyframe.
10. Preview.

The normal path should not require manually expanding hidden timeline tracks before the first animation can be made.

---

## 5. Animate with keyboard rhythm stepping

### Goal

Rapidly author repeated rhythmic changes.

1. Select object.
2. Select/focus a property or invoke its shortcut.
3. Place first keyframe.
4. Step forward one subdivision.
5. change value;
6. place/update keyframe;
7. repeat.

Optional acceleration:

- duplicate previous rhythmic pattern;
- move selection by N subdivisions;
- jump one beat/bar.

This flow is one of the primary reasons the product exists and should receive dedicated usability testing.

---

## 6. Edit existing keyframes

1. Expand animated property row if needed.
2. Click keyframe or box-select several.
3. Drag left/right.
4. Keyframes snap through valid grid positions.
5. Timeline shows target musical position.
6. Release.
7. One undo step is created.

For multiple selected keyframes:

- musical spacing remains unchanged;
- invalid movement beyond project boundaries is handled predictably;
- collisions follow an explicit policy defined in TIMELINE.md.

---

## 7. Duplicate a rhythmic pattern

### Goal

Repeat an animation pattern without rebuilding it.

1. Select a group of keyframes.
2. Duplicate/copy.
3. Move to target beat/bar.
4. Paste or drag duplicate.
5. Pattern spacing is preserved in musical units.

Future enhancement may provide explicit pattern-repeat tools, but MVP must make copy/paste efficient enough to test the use case.

---

## 8. Change grid resolution

1. User changes grid subdivision, for example 1/8 → 1/16.
2. Ruler/grid updates immediately.
3. Existing keyframes remain semantically stable.
4. Future moves/creates use the newly selected authoring grid according to TIME_MODEL rules.

Changing visual grid density must never silently move existing animation.

---

## 9. Add image asset

1. Choose Add Image or import image asset.
2. Select file.
3. Image appears in project assets.
4. Image object appears in composition or can be placed with one additional obvious action.
5. Image is selected.
6. Transform properties are immediately animatable.

Missing asset later:

1. Project opens.
2. Missing image is represented safely.
3. User sees clear relink/locate action.
4. Other project content remains editable.

---

## 10. Add and edit text

1. Create Text.
2. Text object appears with default content.
3. Text editing is immediately discoverable.
4. User changes text/font/size/alignment/color.
5. Transform and supported text properties can be animated.
6. Cyrillic renders correctly.

Routine text editing should not require leaving the main workspace.

---

## 11. Add an effect

1. Select object.
2. Choose Add Effect.
3. Search/list shows small supported set.
4. Select effect.
5. Effect appears in inspector under the object.
6. Main effect parameters are visible.
7. Animatable parameters expose same keyframe affordance as transform properties.
8. User can reorder/remove effect if the effect model supports stacking in MVP.

---

## 12. Apply easing

### Fast path

1. Select one or more keyframes or a transition.
2. Choose Ease In / Ease Out / Ease In-Out from context/shortcut.
3. Preview.

### Custom path

1. Open curve editor only when needed.
2. Edit cubic Bezier handles.
3. Preview updates immediately.
4. Close/collapse curve editor without losing timeline context.

The fast path should be dramatically easier than custom-curve editing.

---

## 13. Save and continue

1. User invokes Save.
2. First save asks for location.
3. Save completes without blocking the editor longer than necessary.
4. Dirty indicator clears.
5. User continues editing.

On failure:

- project remains open;
- dirty state remains;
- error explains that save did not complete.

---

## 14. Crash/recovery flow

1. Application previously terminated unexpectedly.
2. On next launch/open, recoverable state is detected.
3. User receives a simple choice: Restore or Discard.
4. Restore opens recovered project.
5. User can then Save normally.

Avoid complex recovery browsers for MVP.

---

## 15. Export

1. User chooses Export.
2. Compact export view opens.
3. User chooses path, resolution, FPS, quality preset.
4. Start Export.
5. Progress is visible.
6. User may cancel.
7. On success, UI clearly shows completed output path and optional reveal/open action.
8. Editor project remains unchanged.

Export UI should not expose codec internals irrelevant to MVP.

---

## 16. Undo/redo flow

1. User performs a meaningful edit.
2. Ctrl+Z restores previous semantic state.
3. Ctrl+Shift+Z or Ctrl+Y reapplies.
4. Repeated drag motion is undone as one action.
5. Undo after delete restores object with animation.
6. Undo must never produce partial broken state.

---

## 17. Long-session workflow

The user may spend hours alternating between:

- listening;
- stepping through beats;
- moving playhead;
- editing values;
- moving keyframes;
- previewing.

Therefore:

- transport controls stay reachable;
- timeline focus behavior stays predictable;
- commonly used shortcuts do not depend on fragile hidden modes;
- UI should not accumulate temporary dialogs/windows;
- selection state is visually obvious.

---

## 18. Flow quality metrics

During usability review, record for primary flows:

- pointer clicks;
- shortcut presses;
- modal openings;
- panel switches;
- focus mistakes;
- accidental deselection;
- time to first synchronized animation;
- time to duplicate a 4-beat pattern;
- time to correct BPM offset;
- time to apply easing;
- time to recover from an error.

Metrics are diagnostic, not vanity targets. The goal is to find avoidable friction.
