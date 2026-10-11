# Timeline

> **Status: Accepted for MVP**
>
> Timeline is the primary rhythm-animation authoring surface.

## 1. Objective

Optimize:

~~~text
hear moment
-> find musical position
-> create/select keyframe
-> edit visual value
-> preview
~~~

It is not a generic NLE timeline.

## 2. Shared horizontal transform

One TimelineTransform maps ProjectTimeNs to screen x.

Waveform, musical grid, playhead, and keyframes use the same conversion.

No independent rounding per layer.

## 3. Vertical structure

From top to bottom:

- ruler: about 28 px;
- waveform: about 64 px;
- object/property rows.

Object row: about 30 px.

Property row: about 28 px.

Actual token tuning may move by a few logical pixels without changing contract.

## 4. Ruler/grid hierarchy

Visual priority:

~~~text
bar       strongest
beat      medium
subdivision subtle
~~~

Labels disappear before lines become crowded.

Current authoring division remains logically active even if some minor lines are hidden at zoom-out.

## 5. Waveform

Waveform remains fixed near ruler and does not scroll vertically with object rows.

It is visually behind playhead/keyframes/grid emphasis.

## 6. Zoom

Ctrl+wheel zooms horizontally around mouse pointer.

Zoom changes only view state.

Clamp zoom to a practical range that permits:

- fine 1/32-beat editing at high BPM;
- whole-song overview.

Exact numeric zoom limits are implementation constants and can be tuned without schema impact.

## 7. Pan

- Shift+wheel: horizontal pan;
- middle-mouse drag: horizontal pan;
- panning never moves playhead.

## 8. Playhead

Playhead is continuous ProjectTimeNs.

Ruler click/drag seeks continuously.

Musical keyboard navigation moves it exactly by grid/beat/bar commands.

The playhead may sit between grid positions.

## 9. Keyframe creation

Authored keyframes always resolve to the current authoring grid.

When K/keyframe button creates a key while playhead is between grid points:

1. resolve nearest grid point using TIME_MODEL snap rule;
2. move playhead to that exact point;
3. create/update keyframe there.

This removes ambiguity between visible playhead and authored key.

## 10. Existing fine-grid keys

Changing BeatDivision never moves/hides existing keys.

Example:

~~~text
key at tick 60 created on 1/16
switch grid to 1/4
key remains tick 60
~~~

## 11. Object/property rows

Object rows show primary object identity.

Property rows are shown for:

- animated properties;
- currently focused/revealed property;
- explicitly expanded compatible properties.

Do not display every static property by default.

## 12. Keyframe glyph/hit target

Visible diamond may be compact.

Interactive hit box is at least 18×18 logical px.

States:

- normal;
- hover;
- selected.

Do not encode too many semantics into glyph color.

## 13. Single drag

1. begin transaction;
2. pointer x maps to continuous time;
3. convert to continuous musical position;
4. snap to current BeatDivision;
5. preview target tick;
6. commit on release as one history entry;
7. Esc restores original state.

## 14. Multi-drag

Use one selected anchor keyframe.

Snap anchor to target grid, derive integer delta ticks, apply identical delta to all selected keys.

Internal pattern spacing is preserved even if some keys originated from a finer grid.

## 15. Collision

One keyframe per property/tick.

Accepted move/paste collision rule:

- incoming/moved keyframe wins;
- occupied unselected key is replaced;
- replaced key data is included in undo;
- preview should indicate replacement before commit where practical.

### Compound delete final-key rule

When a compound Delete removes every keyframe of one property, selected keys are resolved in ascending MusicalTick order for that property. The value of the latest removed key becomes the new static base_value. This is the deterministic compound extension of the Inspector rule that removing the final key promotes that removed key value to static state.

## 16. Box selection

Drag from empty keyframe canvas.

Normal box replaces selection.

Ctrl+box adds/toggles according to standard selection semantics.

Only visible-range candidate keys are hit-tested.

## 17. Keyboard movement

Per KEYBOARD_SHORTCUTS.md:

- Alt+Left/Right: selected keys ± current grid step;
- Ctrl+Alt+Left/Right: ± one beat.

Movement is integer tick arithmetic, not pointer-style resnap.

## 18. Copy/paste

Copied keyframe packet stores:

- compatible property identity/type;
- values;
- interpolation;
- relative MusicalTick offsets from anchor.

Paste:

1. resolve current playhead to grid anchor;
2. allocate new KeyframeIds;
3. preserve relative tick offsets;
4. apply collision policy;
5. one compound history entry.

Cross-property paste requires semantic/type compatibility.

## 19. Duplicate

Ctrl+D duplicates active keyframe selection and leaves the duplicate selected for immediate Alt+Arrow repositioning.

The copied selection begins one current authoring-grid step after the latest selected keyframe (measured from the earliest selected keyframe). A single selected key moves forward one grid step. The offset is applied in integer MusicalTicks to every selected key, regardless of property, without requiring BPM or moving the playhead. Relative pattern timing remains unchanged. The originals remain intact; an unrelated key at a destination tick follows the normal paste collision rule. Duplication is one undoable compound insertion and does not change the clipboard.

## 20. Easing

Right-click a keyframe diamond to open outgoing easing actions for the active keyframe selection. Right-clicking an unselected diamond selects that key; right-clicking an already selected diamond keeps the multi-selection. A keyboard-accessible Easing menu is also available in the timeline header.

Actions include Hold, Linear, Ease In, Ease Out, and Ease In-Out. Presets apply only to selected keys that have a following key on the same object/property. Terminal keys have no outgoing segment and are skipped. Each multi-key action is one undoable history entry; applying an unchanged preset or selecting only terminal keys creates no history entry. Existing IDs, key values, and musical tick spacing stay unchanged.

Timeline does not attempt to visualize detailed curve shape in each key glyph.

## 21. Playback follow

Follow Playhead is available as a timeline-header toggle and is OFF by default.

When enabled, **authoritative audible playback positions** may move the horizontal viewport as follows:

- within the middle 75% of the visible time span, the view does not move;
- if the audible playhead enters the outer 1/8 of either edge, scroll just enough to place it 1/5 of the span inside that edge (with project-bound clamping), retaining the same zoom/span;
- user manual pan via Shift+wheel or middle-button drag disables follow until explicitly re-enabled, even if a pan hits the project boundary;
- manual paused seeks and editing/navigation actions never auto-scroll via Follow Playhead. Timeline zoom does not disable the toggle.

This avoids constant recentring and maintains a stable view while listening. **Integration note:** the editor's audio transport is not yet wired into `EditorSession::update_playhead_from_audio_clock`. The toggle and viewport-follow policy are implemented and unit-tested, but live audio-driven following remains dependent on transport integration. AI-160 is not complete until that wiring is present.

## 22. Focus

Timeline focus is visible.

Text field focus elsewhere prevents timeline arrows/letters stealing input.

## 23. Virtualization

Vertical:

- lay out visible object/property rows only.

Horizontal:

- query sorted keyframes by visible MusicalTick range plus margin.

Frame cost should depend primarily on visible rows/keys, not project total.

The AI-161 regression fixture creates 10,000 sorted keys on one animated property and validates the project. It verifies a narrow inclusive MusicalTick query returns only its matching slice, then runs a headless egui timeline frame over the same two-second window for both 64-key and 10,000-key projects. The 10,000-key case must emit fewer than 1,000 painter shapes and no more than ten additional shapes relative to the 64-key case. This checks that nonvisible keyframes are not emitted as draw geometry; it is not a wall-clock performance benchmark. Visible key queries use sorted-slice partition-point range lookups.

## 24. Overlapping visual keys

At extreme zoom-out, multiple distinct MusicalTicks may occupy the same few pixels.

MVP:

- preserve all semantic keys;
- hit-test nearest time/key;
- selected keys remain emphasized.

Aggregated/stacked visualization can be post-MVP.

## 25. BPM/offset editing

Waveform remains stationary in project/audio time.

Changing BPM/offset moves musical grid and therefore the absolute derived positions of keyframes.

Keyframe MusicalTicks remain unchanged.

## 26. Empty states

No audio:
- ruler/grid may exist if BPM is set;
- Import Audio remains prominent.

No BPM:
- waveform/playhead work;
- rhythm grid/keyframe creation disabled with clear Set BPM action.

No objects:
- ruler/waveform visible;
- Add Object path visible.

## 27. Stress target

Architecture test fixture:

- 500 objects;
- 10,000+ keyframes.

MVP release smoothness is required primarily for realistic medium projects, but stress case must not reveal obvious O(total keys every frame) design.

## 28. Required usability scenarios

- align BPM to waveform;
- create 1/16 pattern;
- switch to 1/4 without moving old keys;
- multi-drag one beat;
- collision replacement + undo;
- box-select 20 keys;
- duplicate/paste four-beat pattern;
- rapid zoom/pan;
- playback follow toggle;
- 10k-key stress fixture.

## 29. Definition of Done

Timeline is MVP-ready when shared transforms, grid-native key creation, fine-grid preservation, collision/undo, multi-drag, virtualization, keyboard rhythm flow, and realistic density comfort all pass.
