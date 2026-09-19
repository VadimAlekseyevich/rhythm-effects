# Product Specification

> **Status: Accepted for MVP**
>
> This document defines the MVP as a user-facing product. Architecture documents may choose how to implement these behaviors, but must not silently change them.

## 1. Product statement

Rhythm Effects is a desktop motion-design editor built around musical time.

The primary authoring surface is a BPM grid. Visual objects and imported assets are animated with keyframes placed on valid rhythm positions. Animation between keyframes remains continuous through interpolation and easing.

The product is optimized for a repeated loop:

~~~text
hear a moment
→ locate its grid position
→ create/select a keyframe
→ change a visual property
→ preview immediately
→ repeat
~~~

The product should feel closer to a fast rhythm editor or sequencer than to a reduced copy of a traditional compositing suite.

---

## 2. MVP product goals

The MVP must prove four things.

### 2.1. Rhythm-native animation is useful

A user can author a visually meaningful animation while thinking primarily in bars, beats, and subdivisions rather than arbitrary timestamps.

### 2.2. The workflow is faster than generic motion-design editing for music-synchronized work

Common rhythm operations require very few actions and have direct shortcuts.

### 2.3. The interface remains calm

The editor can expose useful animation functionality without dense nested interfaces, tiny controls, or mode confusion.

### 2.4. Preview and export are trustworthy

What the user previews against audio is what gets exported.

---

## 3. Target user

The MVP primarily targets a creator who:

- already understands basic keyframe animation;
- works with music-driven visuals;
- may create edits, lyric visuals, abstract motion graphics, loops, visualizers, social-media motion, rhythm-based overlays, or short music-synchronized scenes;
- values speed over extremely deep compositing functionality;
- prefers direct manipulation and hotkeys;
- does not want to spend time building beat markers manually in a general-purpose editor.

The MVP does not need to teach motion design from zero, but its basic workflow must still be discoverable.

---

## 4. Supported project model

An MVP project contains:

- one composition;
- one primary audio track;
- one BPM configuration;
- one ordered list of visual objects/layers;
- imported assets;
- animation keyframes;
- effects supported by the MVP;
- composition/output settings.

Nested compositions are not required.

Multiple audio tracks are not required.

---

## 5. New project

The user can create a project without going through a long setup wizard.

### Required behavior

A new project has sensible defaults:

- composition resolution: 1920×1080;
- frame rate: 60 FPS;
- duration: 10 seconds before audio import; after first primary-audio import, extend to at least the audio duration;
- background: opaque black;
- BPM: unset; default meter is 4/4; default authoring division is 1/4 beat once BPM is set;
- grid: visible only when BPM can be resolved.

The project should become useful as soon as audio is imported.

### UX rule

Do not require the user to configure every technical export setting before entering the editor.

---

## 6. Audio import

The user can import one primary music file.

### Required behavior

After import:

- the file is validated;
- duration is known;
- waveform preprocessing starts;
- the timeline gains an audio waveform;
- the project can play the audio;
- the project can seek;
- the editor exposes BPM and offset setup.

If preprocessing is incomplete, the editor should remain usable where possible and visibly improve as cached data becomes available.

### Failure behavior

Unsupported, unreadable, or missing audio must produce a clear user-facing error without damaging the project.

---

## 7. BPM and grid setup

The user can configure the musical grid for the imported track.

### MVP controls

- BPM numeric input;
- BPM offset;
- subdivision selector;
- optional meter display, initially 4/4;
- fine offset nudge;
- tap tempo is post-MVP; BPM entry and fine offset nudge are sufficient for MVP.

### Required behavior

The user can visually align the grid against waveform transients.

The timeline distinguishes at least:

- bar lines;
- beat lines;
- subdivision lines.

Visual emphasis must make hierarchy obvious without excessive color noise.

### Grid validity

Authored keyframes must occupy valid musical grid positions.

The system may internally evaluate at arbitrary time, but the user cannot leave a keyframe at an arbitrary timestamp between enabled grid positions.

Changing visible grid density must not silently invalidate existing keyframes. The exact rule is defined in TIME_MODEL.md.

---

## 8. Main editor workspace

The default workspace contains a small number of persistent regions:

1. viewport;
2. timeline;
3. object/layer context;
4. inspector;
5. compact transport/rhythm controls.

The workspace should avoid permanent panels that are not necessary for the current task.

### Required properties

- panels can resize within sensible limits;
- important text remains readable;
- no core workflow depends on tiny icons with no labels/tooltips;
- no routine workflow requires navigating more than one meaningful level of contextual detail;
- the timeline receives substantial vertical and horizontal space;
- the viewport remains visible during normal animation work.

---

## 9. Object creation

MVP object types:

- Rectangle;
- Ellipse;
- Image;
- Text.

Object creation should require minimal configuration.

### Default creation behavior

Creating a shape should produce a visible, editable object immediately.

Creating an image should prompt for/select an asset if necessary.

Creating text should produce editable text with a sensible default style.

The user should not encounter a configuration dialog before seeing the object.

---

## 10. Object list / layer context

The user can:

- see object order;
- select an object;
- multi-select where supported;
- rename;
- reorder;
- toggle visibility;
- lock;
- delete;
- duplicate.

The object list should not become a second timeline.

Animation detail belongs in the timeline.

---

## 11. Inspector

The inspector shows relevant properties for the current selection.

### Core transform properties

- Position;
- Scale;
- Rotation;
- Anchor/Pivot;
- Opacity.

### Object-specific properties

Rectangle/Ellipse:
- size;
- fill;
- Rectangle corner radius.

Image:
- source asset;
- size;
- basic fit behavior if required.

Text:
- content;
- font;
- size;
- alignment;
- color.

### Animation affordance

An animatable property clearly communicates:

- current evaluated value;
- whether it is animated;
- whether a keyframe exists at the current grid position;
- how to add/remove a keyframe.

The UI should not require a separate animation mode for ordinary property keyframing.

---

## 12. Keyframe creation

A keyframe is created at the current valid grid position.

### Required interactions

The user can create a keyframe by:

- pressing the property's keyframe control;
- using a shortcut for a frequent property/action;
- changing an already-animated property at a grid position, if auto-key behavior is later explicitly enabled.

Auto-key is not required for MVP and should default off unless deliberately specified.

### Position invariant

The resulting keyframe is always associated with a valid musical grid position.

---

## 13. Keyframe editing

The user can:

- select one keyframe;
- select multiple keyframes;
- box-select keyframes;
- delete;
- move;
- duplicate;
- copy/paste;
- nudge by grid units;
- change interpolation/easing.

### Dragging

Dragging a keyframe moves it through valid grid positions.

The visual motion of the pointer may be continuous, but the committed keyframe position is discrete.

The UI should show the target bar/beat/subdivision while dragging.

### Multi-drag

Relative spacing between selected keyframes should remain stable in musical units.

---

## 14. Timeline

The timeline is the product's primary editing surface.

It contains:

- time/rhythm ruler;
- waveform;
- musical grid;
- playhead;
- object rows;
- expandable animated-property rows;
- keyframes.

### Required navigation

- horizontal pan;
- zoom;
- jump to start;
- jump to playhead;
- next/previous beat;
- next/previous subdivision;
- next/previous keyframe where context allows.

### Zoom behavior

Zoom should preserve context, preferably around the cursor or another clearly defined anchor.

At lower zoom, minor grid divisions may disappear visually, but their logical timing remains intact.

---

## 15. Playback

Required transport:

- play/pause;
- stop or return-to-start behavior;
- click/drag playhead;
- seek.

During playback:

- audio is the timing authority;
- visual animation follows the resolved playback time;
- timeline/viewport remain responsive;
- editing behavior during playback is deliberately defined rather than accidental.

For MVP, it is acceptable to restrict some destructive edits during active playback if needed for correctness, but ordinary selection and stop/pause must remain immediate.

---

## 16. Easing

MVP supports:

- Hold;
- Linear;
- Ease In;
- Ease Out;
- Ease In-Out;
- custom cubic Bezier if the curve editor is included in the MVP cut.

The user should be able to apply a useful easing preset without opening a complex graph editor.

---

## 17. Effects

The MVP effect set is fixed:

- Blur;
- Glow;
- Tint/Color adjustment;
- Noise;
- RGB Split.

Effect controls appear contextually in the inspector and must follow the same UI-density rules as object properties.

---

## 18. Undo / redo

All normal destructive editing operations must participate in undo/redo.

Examples:

- create/delete object;
- property edit;
- add/delete/move keyframe;
- reorder object;
- add/remove effect;
- effect parameter edit;
- asset-changing operation where practical.

A continuous drag should normally produce one undo step, not hundreds.

Undo/redo must not affect playback position unless the command itself concerns timeline/playhead state and is intentionally undoable.

---

## 19. Save and open

The user can:

- Save;
- Save As;
- Open;
- reopen recent work through a simple path if implemented.

Required behavior:

- dirty state is visible but unobtrusive;
- save failure is explicit;
- the project is not silently corrupted by an interrupted write;
- reopening preserves project semantics.

Autosave/recovery is required in minimal form for MVP.

---

## 20. Export

The user can export a video without understanding the internal renderer.

### Required settings

- output path;
- resolution;
- FPS;
- H.264/MP4 quality setting or preset.

### Required behavior

- audio is included;
- rendering is deterministic;
- progress is visible;
- export can be cancelled safely;
- the resulting file can be played in common video software;
- sync matches editor preview within defined tolerance.

Advanced codec controls are not required.

---

## 21. Discoverability

Keyboard speed must not make the mouse UI cryptic.

Every core action should be discoverable through one or more of:

- visible control;
- context menu where appropriate;
- tooltip showing shortcut;
- command search/palette if included.

The product should teach hotkeys gradually through labels and tooltips rather than a blocking tutorial.

---

## 22. Empty states

### No audio

Show a clear primary action to import audio.

### No selection

Inspector should show useful neutral state, not stale controls.

### No objects

Viewport/timeline should make object creation obvious.

### No BPM

Rhythm grid area should clearly indicate that BPM must be configured.

---

## 23. Error handling principles

User-facing errors must answer:

1. what happened;
2. what was not completed;
3. whether project data is safe;
4. what the user can do next.

Avoid technical stack traces in normal UI.

Detailed logs may exist separately.

---

## 24. Accessibility and readability baseline

MVP should at minimum:

- avoid essential information encoded only by subtle color differences;
- use readable default text sizes;
- maintain strong enough contrast;
- provide adequate hit targets;
- keep keyboard focus understandable;
- support common Windows scaling/DPI configurations.

Full formal accessibility certification is outside MVP scope, but obvious accessibility-hostile design is not acceptable.

---

## 25. Product performance expectations

The product spec does not define exact benchmark numbers; PERFORMANCE.md does.

However, user-visible expectations are:

- opening a basic editor window feels immediate;
- dragging a keyframe feels direct;
- zooming/panning timeline feels smooth;
- waveform interaction does not hitch under normal project sizes;
- property edits update preview immediately;
- play/pause reacts immediately;
- no routine action causes multi-second UI blocking.

---

## 26. MVP non-goals

Explicitly excluded unless later promoted:

- 3D;
- particles;
- full compositing suite;
- advanced masks/roto;
- motion tracking;
- expressions;
- scripting;
- plugin ecosystem;
- cloud collaboration;
- accounts;
- template marketplace;
- advanced audio editing;
- multi-track DAW behavior;
- nested composition systems;
- AE project compatibility.

---

## 27. MVP acceptance scenario

A release candidate must support the following uninterrupted scenario:

1. Launch app.
2. Create project.
3. Import a song.
4. Enter BPM.
5. Align grid offset to waveform.
6. Choose a subdivision.
7. Create a rectangle.
8. Place position keyframe on a grid point.
9. Move to another grid point.
10. Create second position keyframe.
11. Preview synchronized motion.
12. Add scale/rotation/opacity animation.
13. Add image.
14. Add text.
15. Multi-select and move keyframes.
16. Apply easing.
17. Add at least one visual effect and animate one parameter if effects survive MVP cut.
18. Save.
19. Close app.
20. Reopen project.
21. Continue editing.
22. Undo/redo several edits.
23. Export MP4.
24. Play exported file externally.
25. Confirm audiovisual sync and expected visual state.

---

## 28. Open product decisions

These require later explicit decisions rather than accidental implementation:

- exact default composition resolution/FPS;
- exact base grid terminology shown in UI;
- whether keyframes remain on their original fine grid when user switches to a coarser visible subdivision;
- auto-key inclusion and default state;
- exact object list terminology: layer vs object;
- parenting in MVP;
- SVG in MVP;
- exact MVP effect list;
- command palette in MVP;
- audible scrubbing;
- tap tempo;
- recent-project screen.

These decisions should be resolved before their implementation becomes expensive.


---

## 29. Accepted MVP product decisions

The following choices are frozen for MVP implementation:

- default composition: 1920×1080 at 60 FPS;
- default pre-audio duration: 10 seconds;
- default composition background: opaque black;
- BPM starts unset;
- default meter: 4/4;
- default authoring grid after BPM setup: 1/4 beat;
- SVG is post-MVP;
- object parenting is post-MVP;
- global Auto-Key mode is not part of MVP;
- keyframing is enabled per property;
- once a property has at least one keyframe, editing that property at a time without a keyframe creates a new keyframe at the nearest current authoring-grid position and moves the playhead to that resolved grid position;
- if the property is not animated, normal editing changes base_value;
- clicking the keyframe control on an unanimated property creates its first keyframe using the current base value at the resolved grid position;
- removing the final keyframe returns the property to a static base-value state using the removed keyframe value at that moment;
- ImageObject uses intrinsic image bounds plus Transform for MVP; crop/fit modes are post-MVP;
- composition text uses installed/system fonts plus one bundled fallback family; imported font assets are post-MVP;
- MVP effect set is Blur, Glow, Tint, Noise, and RGB Split;
- one composition and one primary audio track remain hard MVP boundaries;
- the default workspace is fixed/resizable rather than a fully dockable IDE workspace.

Changes to these decisions require an explicit scope/ADR update rather than incidental implementation.
