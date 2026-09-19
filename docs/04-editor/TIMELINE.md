# Timeline

> **Status: Draft**
>
> The timeline is the central product surface. It combines waveform, BPM grid, playhead, object/property rows, and musical keyframes.

## 1. Product objective

The timeline must make this loop exceptionally fast:

~~~text
hear moment
→ identify beat/subdivision
→ navigate there
→ place/move keyframe
→ preview
~~~

It is not a generic video NLE timeline.

---

## 2. Time coordinate system

One canonical timeline transform maps ProjectTime to screen x.

Concept:

~~~rust
TimelineTransform {
    visible_start,
    seconds_per_pixel,
    viewport_rect,
}
~~~

All layers use this transform:

- waveform;
- grid;
- playhead;
- keyframes;
- markers later.

Never let waveform and keyframes use independent rounding.

---

## 3. Musical coordinate helpers

Timeline also needs:

- ProjectTime ↔ continuous musical tick;
- MusicalTick → ProjectTime;
- current GridResolution.

Keyframes draw from their MusicalTick-derived absolute position.

---

## 4. Horizontal zoom

Zoom should preserve orientation.

Preferred behavior:

- Ctrl+wheel or equivalent;
- anchor at mouse cursor when over timeline;
- otherwise anchor playhead/center according to explicit rule.

Zoom changes view only, never project data.

---

## 5. Horizontal pan

Support:

- wheel/shift-wheel convention;
- middle drag where useful;
- scrollbar only if visually unobtrusive.

Panning never moves playhead.

---

## 6. Ruler

Ruler emphasizes musical structure.

At appropriate zoom show:

- bar number;
- beat;
- subdivisions.

Minor labels disappear before lines become visually crowded.

---

## 7. Grid hierarchy

Visual strength:

~~~text
Bar      strongest
Beat     medium
Subdiv   subtle
~~~

Do not assign unrelated colors to every division.

Current grid resolution is visually clear.

---

## 8. Grid density

At far zoom-out:

- hide minor subdivision lines;
- retain bars/beats as space permits.

This is display density only.

The current authoring GridResolution remains logically active.

---

## 9. Existing fine-grid keyframes

Important invariant:

A keyframe can remain at a fine MusicalTick position even after the user switches to a coarser authoring grid.

Example:

~~~text
created on 1/16 grid
switch current grid to 1/8
old keyframe remains exactly where it was
~~~

It must still render visibly.

Do not quantize or hide it merely because it is not on the current coarse step.

---

## 10. Playhead

Playhead is continuous.

Interactions:

- click ruler/time space to seek;
- drag;
- keyboard musical stepping;
- keyboard keyframe stepping.

Playhead does not need to remain on grid.

A separate snap-to-grid action may place it exactly on grid.

---

## 11. Waveform row

Waveform stays near ruler/transport context and spans composition/audio time.

Requirements:

- enough height to align transients;
- does not dominate visual hierarchy;
- clipped to visible range;
- grid lines may overlay it with controlled contrast.

---

## 12. Object rows

Each composition object has a primary row.

Primary row displays:

- name;
- visibility/lock state if part of row header;
- compact keyframe summary markers if useful.

Expandable child rows show animated properties.

---

## 13. Property rows

Default behavior should avoid showing dozens of static properties.

Candidates:

- show animated properties automatically;
- show currently focused property;
- allow explicit reveal of all animatable properties.

Final UX should minimize vertical clutter.

---

## 14. Keyframe glyph

Use a simple recognizable glyph, e.g. diamond.

Visual size can be compact, but hit target is larger.

States:

- normal;
- hover;
- selected;
- multiple-selected;
- collision/invalid preview if needed.

---

## 15. Keyframe creation

Keyframe is created at current valid authoring grid position.

If playhead is between grid points:

- creation resolves to nearest/current explicit snap policy;
- UI should show where key will be created before/at action if ambiguity exists.

Potential alternative: keyframe command first snaps playhead. Decide in usability prototype.

---

## 16. Keyframe drag

Single keyframe:

1. pointer drag begins;
2. continuous pointer x maps to time;
3. time snaps to current authoring grid;
4. preview keyframe at target MusicalTick;
5. release commits one command.

Show musical destination.

---

## 17. Multi-keyframe drag

Preserve selected pattern spacing.

Recommended model:

- choose drag anchor keyframe;
- anchor resolves to target grid position;
- compute integer delta ticks;
- add same delta to every selected keyframe.

This allows selected keys created on finer grids to retain internal rhythmic phase.

---

## 18. Keyboard movement

Subdivision move:

~~~text
tick += current_grid_step
~~~

Beat move:

~~~text
tick += PPQ
~~~

Bar move uses time signature.

Exact integer arithmetic; no snap round trip.

---

## 19. Collision policy

One keyframe per property per tick.

Proposed MVP rule:

- moving/pasting selection onto occupied unselected keyframe: moved/pasted keyframe wins;
- overwritten keyframe is included in undo history;
- preview should visibly indicate collision/replacement.

This prioritizes fast editing but remains reversible.

Must be usability-tested before Accepted status.

---

## 20. Box selection

Drag from empty keyframe area.

Modifiers control replace/add/remove selection.

Selection rectangle only processes visible candidate keyframes.

---

## 21. Vertical virtualization

Only lay out visible object/property rows.

Need stable row-height model.

Expanded/collapsed state lives in editor session.

---

## 22. Horizontal virtualization

Only inspect/render keyframes in visible tick/time range plus small margin.

Because keyframes are sorted by tick, use binary-search range.

No iteration through every keyframe every frame.

---

## 23. Selection model

Keyframe selection stores KeyframeId plus property/object context as needed.

Do not use screen position or tick alone as identity.

Selection survives timeline pan/zoom.

If keyframe is deleted, remove from selection.

---

## 24. Copy/paste

Copy selected keyframes stores:

- relative tick offsets from an anchor;
- values;
- interpolation;
- compatible property identity/type.

Paste:

- uses playhead/current target as anchor;
- resolves anchor to current grid;
- allocates fresh KeyframeIds;
- preserves relative offsets.

Cross-property paste only if types/semantics are compatible.

---

## 25. Duplicate pattern

Fast duplicate should be equivalent to copy + immediately movable selection.

Future "repeat N times" can be added post-MVP.

---

## 26. Easing indication

Do not overload keyframe glyph with too much visual encoding.

A subtle marker or inspector/context state is enough.

Detailed curve belongs in Curve Editor.

---

## 27. Playback follow

Optional mode:

- timeline scrolls when playhead approaches edge.

Default behavior must not constantly fight user's manual navigation.

Potential simple MVP policy:

- follow only during playback if explicitly enabled;
- user pan disables follow until re-enabled.

---

## 28. Row height

Keep readable but compact.

Object rows slightly stronger than property rows.

Do not solve density by tiny typography.

---

## 29. Timeline focus

Focused timeline should be visually subtle but clear.

When text field is active elsewhere, timeline letter/navigation shortcuts must not steal typing.

Arrow rhythm navigation policy must coordinate with numeric fields.

---

## 30. Performance budget

Timeline frame cost should depend primarily on:

- visible rows;
- visible keyframes;
- viewport pixels;

not total project duration/keyframe count.

Stress fixture:

- 500 objects;
- 10,000 keyframes.

Even if full stress target is not silky in MVP, architecture must avoid obvious O(total_keyframes_per_frame) work.

---

## 31. Hit testing

Use spatially simple visible-range lists.

Hit area larger than glyph.

When multiple keys overlap visually due zoom:

- prefer nearest exact time;
- potentially show stacked/combined representation later.

MVP can cycle/select topmost with clear behavior.

---

## 32. BPM editing interaction

BPM/offset controls remain near timeline.

When adjusting offset:

- grid moves over stationary waveform;
- existing keyframe MusicalTicks move in absolute time because their musical meaning follows grid;
- this is intentional.

This is one of the core semantic benefits of storing keyframes in musical time.

---

## 33. Tempo change implication

MVP supports one BPM.

Timeline APIs should call TempoMap rather than hardcode BPM formulas so future tempo changes do not require replacing coordinate infrastructure.

---

## 34. Empty states

No audio:
- grid can still exist if BPM set, but import CTA should be prominent.

No BPM:
- waveform visible;
- rhythm grid unavailable/disabled with clear setup action.

No objects:
- timeline still shows waveform/ruler and Add Object path.

---

## 35. Required prototype scenarios

1. align BPM to waveform;
2. create 1/16 keys;
3. switch grid to 1/8;
4. old keys stay;
5. drag group by one beat;
6. box select 20 keys;
7. copy/paste four-beat pattern;
8. rapid zoom/pan;
9. play while timeline follows;
10. 10k-key stress data.

---

## 36. Definition of Done

- waveform/grid/playhead share coordinate transform;
- keyframes are grid-native;
- fine-grid keys survive grid changes;
- multi-drag preserves tick spacing;
- visible-range virtualization works;
- common rhythm navigation has keyboard path;
- timeline remains comfortable at realistic density.
