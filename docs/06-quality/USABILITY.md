# Usability

> **Status: Accepted for MVP**
>
> Interface comfort is a product requirement with measurable review criteria.

## 1. Core goals

The editor is:

- readable;
- shallow;
- predictable;
- forgiving;
- fast by keyboard;
- discoverable by pointer;
- comfortable for long sessions.

A technically reachable feature is not necessarily usable.

## 2. North-star workflow

~~~text
hear moment
-> move to rhythm position
-> add/select keyframe
-> edit property
-> preview
~~~

This loop receives the strongest usability attention.

## 3. Readability gates

At Windows scaling 100%, 125%, and 150%:

- base UI text remains readable;
- no ordinary UI text below DESIGN_SYSTEM minimum;
- clipped labels have tooltip/space recovery rather than unreadable shrink;
- numerical time/value readouts remain distinguishable;
- critical state is not color-only.

## 4. Target gates

- ordinary target >= 32×32 logical px where layout allows;
- keyframe hit area >= 18×18;
- splitters have forgiving >=8 px hit zone;
- handles remain clickable independent of viewport zoom.

Measure misclick/frustration during fast timeline work.

## 5. Navigation depth

Core workflow should remain in the default workspace.

Release blockers include:

- routine tabs inside tabs;
- required modal dialogs for keyframing;
- common actions hidden behind 3+ disclosure layers;
- a new permanent panel that duplicates an existing context.

## 6. Action-count baseline

Track these user tasks:

- import audio;
- set/adjust BPM;
- create rectangle;
- create first Position key;
- advance one beat;
- create second key;
- apply easing preset;
- duplicate a four-beat pattern;
- add effect;
- export.

A change that adds avoidable steps to a high-frequency task requires justification.

## 7. Focus tests

Repeatedly move between:

- timeline;
- viewport;
- object list;
- inspector numeric input;
- text editing;
- command search.

The user should predict what arrows, Delete, P/S/R/O, K, and Space will do.

Hidden focus is a serious defect.

## 8. Mode policy

No global Auto-Key mode in MVP.

Space always means Play/Pause outside text editing.

Temporary drag/modifier states are visible and Esc-cancellable.

Avoid long-lived invisible modes.

## 9. Timeline comfort

Explicitly test:

- grabbing one key;
- dense selected keys;
- 1/16 and 1/32 grids;
- box selection;
- collision preview;
- pattern duplicate + move;
- waveform/grid readability;
- zoom anchor;
- current division visibility.

Timeline is the highest-priority UX surface.

## 10. First-use observation

Recruit at least 3 people familiar with creative software but unfamiliar with Rhythm Effects before MVP candidate.

Without verbal instruction, observe:

1. import audio;
2. set BPM;
3. create object;
4. make two-key rhythm animation;
5. play;
6. save/export.

Record where they become confused.

Do not explain until the observation has captured the problem.

## 11. Experienced workflow observation

After shortcuts are learned, test a repeated 8–16 beat animation pattern.

Look for:

- focus gymnastics;
- excessive mouse travel;
- repeated panel disclosure;
- slow property access;
- difficulty duplicating rhythms.

## 12. Acceptance criteria

Before MVP candidate:

- all 3 first-use testers complete the core two-keyframe animation with no blocking UI defect after normal self-discovery;
- no repeated confusion appears in the same core step for a majority of testers without being addressed/documented;
- experienced workflow can be completed predominantly with keyboard + direct manipulation;
- 1280×720 layout remains functional;
- 100/125/150% DPI remain usable.

This is a small formative test, not statistical proof.

## 13. Long-session review

Perform at least one 60-minute real editing session on a representative demo project.

Record:

- eye strain/readability;
- repeated clicks;
- panel travel;
- focus mistakes;
- hotkey conflicts;
- timeline fatigue;
- accidental mode/state changes.

## 14. Terminology

Use canonical Glossary terms consistently.

Do not alternate layer/object/key/grid terminology arbitrarily.

## 15. Empty/error states

A first-time user can identify the next action for:

- empty project;
- no audio;
- BPM unset;
- no objects;
- no selection;
- missing asset/font;
- lost audio device.

## 16. Accessibility baseline

MVP:

- keyboard reaches core workflows;
- focus is visible;
- critical states are not color-only;
- contrast is intentionally reviewed;
- DPI scaling works.

Formal WCAG/certification is outside MVP, but obvious avoidable barriers are not accepted.

## 17. Definition of Done

Usability is MVP-ready when first-use and experienced observations are recorded, core focus/timeline issues are resolved, long-session review is complete, and design-system size/depth constraints hold.
