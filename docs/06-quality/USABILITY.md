# Usability

> **Status: Draft**
>
> Interface comfort is a product requirement, not subjective polish. This document converts that principle into review criteria.

## 1. Core usability goals

The editor should be:

- readable;
- shallow;
- predictable;
- forgiving;
- fast with keyboard;
- discoverable with pointer;
- comfortable for long sessions.

A feature is not complete merely because it is technically reachable.

---

## 2. Primary workflow metric

North-star task:

~~~text
hear moment
→ move to rhythm position
→ add/select keyframe
→ edit property
→ preview
~~~

The number of unnecessary actions in this loop should stay extremely low.

---

## 3. Readability

Requirements:

- default text comfortably readable at normal Windows scaling;
- critical labels not compressed into tiny typography;
- numeric values distinguishable;
- sufficient contrast;
- no critical meaning conveyed by subtle color only.

Test at:

- 100%;
- 125%;
- 150% Windows scaling.

---

## 4. Hit targets

Important tiny-looking visual elements receive larger interactive areas.

Examples:

- keyframes;
- playhead handle;
- curve handles;
- visibility/lock icons;
- panel splitters.

Evaluate misclick rate during fast use.

---

## 5. Navigation depth

Routine operations should remain in the main workspace.

Design smell thresholds:

- tabs inside tabs;
- three or more disclosure levels;
- repeated modal dialogs;
- hidden secondary windows needed for common animation.

Any new permanent navigation depth requires justification.

---

## 6. Panel count

Default workspace should keep only high-value persistent regions.

New functionality should first use:

1. existing group;
2. contextual inspector;
3. collapsed advanced section;
4. command search;
5. new panel only when spatial persistence is necessary.

---

## 7. Action-count review

Record action counts for:

- import audio;
- align BPM;
- create shape;
- add position keyframe;
- move one beat;
- add second keyframe;
- apply easing;
- duplicate 4-beat pattern;
- add effect;
- export.

A regression in common-task action count should be questioned.

---

## 8. Shortcut discoverability

A new user should discover shortcuts through:

- menus;
- tooltips;
- command search;
- visible hints.

Experienced users should not need menus for repeated work.

---

## 9. Focus predictability

Test repeatedly switching between:

- timeline;
- viewport;
- inspector numeric field;
- text edit;
- object list.

The user should understand what Delete, arrows, and shortcuts will affect.

Hidden focus is a severe usability problem.

---

## 10. Mode minimization

Long-lived editing modes should be rare.

Prefer temporary modifiers/direct manipulation.

If a mode exists:

- active mode is visible;
- Escape exits;
- cursor/controls reflect state;
- entering mode cannot happen accidentally.

---

## 11. Error recovery

User mistakes should be easy to recover from:

- strong undo;
- Escape cancels drags;
- delete reversible;
- save failure non-destructive;
- missing asset relinkable.

A user should feel safe experimenting.

---

## 12. Timeline comfort

Evaluate:

- ease of grabbing keyframes;
- reading bar/beat hierarchy;
- waveform/keyframe visual competition;
- zoom anchor behavior;
- panning;
- box select;
- multi-drag;
- current subdivision visibility;
- current position readability.

The timeline is the most important usability surface.

---

## 13. Long-session fatigue

Avoid:

- overly bright surfaces;
- constant animations;
- excessive contrast everywhere;
- dense tiny controls;
- repeated mouse travel across entire window;
- modal interruptions;
- fragile precise dragging.

Test with sustained real editing, not only screenshots.

---

## 14. Empty-state clarity

A first-time user should know how to proceed when:

- project empty;
- no audio;
- BPM unset;
- no objects;
- nothing selected.

Use one obvious primary action rather than many equal buttons.

---

## 15. Progressive disclosure

Advanced features should not penalize basic workflow.

When feature count grows:

- defaults remain simple;
- rarely used settings stay collapsed/searchable;
- contextual controls appear only when meaningful.

Measure default UI density over time.

---

## 16. Terminology

Use consistent words.

Do not alternate casually between:

- layer/object;
- beat/grid point;
- key/keyframe;
- asset/media.

Resolve terminology in GLOSSARY.md and product specs.

---

## 17. Icons

An icon-only action is acceptable only if broadly understood or strongly supported by tooltip/context.

Ambiguous actions need text.

Do not grow a private symbolic language users must memorize.

---

## 18. First-use test

Give the build to a person familiar with creative software but not Rhythm Effects.

Without verbal instruction, observe whether they can:

1. import audio;
2. set BPM;
3. create object;
4. animate on grid;
5. play;
6. save/export.

Do not intervene until they become genuinely blocked.

Record confusion, not only completion.

---

## 19. Experienced-user test

After shortcuts are learned, measure speed for a repeated 8–16 beat animation pattern.

Look for:

- focus friction;
- mouse travel;
- repeated panel opening;
- excessive clicks;
- slow property access.

---

## 20. Regression checklist

Every major UI feature review asks:

- Did text get smaller?
- Did hit targets get smaller?
- Did default panel density increase?
- Did navigation become deeper?
- Did a common action require more steps?
- Did shortcut behavior become context-fragile?
- Did new controls crowd timeline/viewport?
- Can the feature be hidden contextually when unused?

---

## 21. Quantitative targets

Exact pixel sizes should follow live prototype testing, but establish minimum tokens once UI exists:

- base font size;
- secondary font size;
- minimum target size;
- row height;
- spacing scale;
- panel minimum widths.

These become design-system tokens, not per-widget improvisation.

---

## 22. Accessibility baseline

MVP should:

- support Windows scaling;
- preserve focus visibility;
- avoid color-only critical states;
- maintain reasonable contrast;
- allow keyboard access to core workflows.

Formal accessibility certification is not MVP, but knowingly hostile patterns are rejected.

---

## 23. Definition of Done

Usability is MVP-ready when:

- core workflows have been observed with fresh users;
- action-count baseline exists;
- shortcut-heavy workflow tested;
- text/hit targets work at common DPI settings;
- no routine workflow requires deep nested navigation;
- timeline editing is comfortable under realistic density;
- UI regressions have a repeatable review checklist.
