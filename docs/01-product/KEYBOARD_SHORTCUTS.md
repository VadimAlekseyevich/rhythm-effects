# Keyboard Shortcuts

> **Status: Draft**
>
> This document defines shortcut architecture and an initial MVP map. Exact bindings may change after prototype testing, but shortcut families and scope rules should remain coherent.

## 1. Goals

Shortcuts should make the rhythm-authoring loop dramatically faster.

They must be:

- learnable;
- grouped by purpose;
- discoverable in UI/tooltips;
- predictable by context;
- safe around text entry;
- efficient for repeated beat/subdivision operations.

---

## 2. Scope classes

### Global

Work almost everywhere unless text editing requires interception.

### Timeline

Require timeline context/focus.

### Viewport

Require viewport context/focus.

### Property/Object

Operate on selected objects/properties.

### Text Entry

When a field is active, normal editing shortcuts win.

---

## 3. Proposed global bindings

| Action | Proposed binding | Notes |
|---|---|---|
| Save | Ctrl+S | Standard |
| Save As | Ctrl+Shift+S | Standard |
| Open | Ctrl+O | Standard |
| New Project | Ctrl+N | Standard |
| Undo | Ctrl+Z | Standard |
| Redo | Ctrl+Shift+Z / Ctrl+Y | support Windows convention |
| Copy | Ctrl+C | contextual semantic selection |
| Paste | Ctrl+V | contextual |
| Duplicate | Ctrl+D | object/keyframe context |
| Select All | Ctrl+A | active region |
| Delete | Delete | active semantic selection |
| Play/Pause | Space | unless text entry / temporary pan conflict |
| Command Search | Ctrl+K or Ctrl+Shift+P | final choice TBD |
| Cancel | Esc | interaction hierarchy |

Bindings that conflict with viewport Space-pan require careful resolution.

---

## 4. Rhythm navigation family

These actions are core product functionality and need exceptionally easy bindings.

Required actions:

- previous subdivision;
- next subdivision;
- previous beat;
- next beat;
- previous bar;
- next bar;
- go to previous keyframe;
- go to next keyframe.

Candidate model:

| Action | Candidate |
|---|---|
| Previous subdivision | Left |
| Next subdivision | Right |
| Previous beat | Ctrl+Left |
| Next beat | Ctrl+Right |
| Previous bar | Ctrl+Shift+Left |
| Next bar | Ctrl+Shift+Right |
| Previous keyframe | Alt+Left |
| Next keyframe | Alt+Right |

This is provisional and must be tested against selection/nudge expectations.

Important invariant: rhythm stepping should work without forcing the user to click a tiny timeline control first.

---

## 5. Grid subdivision

The user needs a fast way to make the authoring grid finer/coarser.

Required commands:

- finer subdivision;
- coarser subdivision;
- choose common subdivision directly if useful.

Candidate bindings are TBD after keyboard layout testing.

The UI must always show current subdivision when these commands are used.

---

## 6. Keyframe actions

Required commands:

- add/remove keyframe for focused property;
- delete selected keyframes;
- move selected keyframes one subdivision left/right;
- move one beat left/right;
- duplicate selected pattern;
- apply common easing presets.

Potential model:

| Action | Candidate |
|---|---|
| Move keyframes one grid step | Alt+Left/Right or another family |
| Apply Linear | shortcut TBD |
| Apply Ease In | shortcut TBD |
| Apply Ease Out | shortcut TBD |
| Apply Ease In-Out | shortcut TBD |

Do not assign arbitrary mnemonic shortcuts before testing frequent workflows.

---

## 7. Property quick access

A high-value possibility is direct access to common transform properties.

Candidates inspired by motion-design conventions:

- P — Position;
- S — Scale;
- R — Rotation;
- O or T — Opacity.

However, these conventions conflict with typing/search and potentially tool keys.

Decision must consider:

- whether pressing the key focuses the property;
- expands property row;
- creates a keyframe;
- or merely selects a transform tool.

Do not overload one press with surprising destructive behavior.

A safer MVP approach may be:

1. property shortcut focuses/reveals property;
2. dedicated keyframe action creates/removes keyframe at current grid position.

---

## 8. Viewport navigation

Required:

- pan;
- zoom;
- fit composition;
- frame selection;
- reset view.

Candidate:

- middle mouse drag: pan;
- wheel: zoom;
- Space + drag: optional temporary pan;
- F: frame selection or fit;
- Home: fit composition candidate.

Space interaction must not make Play/Pause unreliable.

One possible policy:

- Space tap = Play/Pause;
- Space hold + pointer drag = temporary pan.

This needs prototype validation because timing ambiguity may feel bad.

Middle-drag-only pan is simpler.

---

## 9. Timeline navigation

Required:

- zoom;
- horizontal pan;
- center playhead;
- follow playhead toggle if included;
- jump start/end.

Candidate standards:

- wheel = vertical/appropriate scroll;
- Shift+wheel = horizontal;
- Ctrl+wheel = zoom;
- Home = project start;
- End = project end.

Exact behavior depends on platform conventions and egui/winit event quality.

---

## 10. Object actions

Required:

- create object through menu/command search;
- duplicate;
- delete;
- rename;
- reorder if shortcuts are useful;
- visibility/lock may remain pointer-first for MVP.

Rename candidate: F2.

---

## 11. Shortcut discoverability

Shortcuts should appear in:

- tooltips;
- menus;
- context menus;
- command search results.

Do not require a separate cheat sheet to discover basic commands.

A dedicated shortcut reference page can still exist.

---

## 12. Remapping

Custom remapping is not required for MVP.

However shortcut definitions should be data-driven enough that remapping can be added later without rewriting every panel.

Avoid hardcoding key checks throughout UI code.

---

## 13. Keyboard layout considerations

Do not assume only US QWERTY for essential non-letter rhythm operations.

Test:

- standard Latin layout;
- switching to Cyrillic/Russian input;
- modifier behavior;
- numpad where relevant.

Critical transport and rhythm navigation should preferably use physical/navigation keys or command mapping independent of typed character where technically appropriate.

---

## 14. Shortcut conflict rules

Priority:

1. active text-entry control;
2. active transient interaction cancellation/commit;
3. explicit global non-destructive action;
4. focused-panel contextual action;
5. fallback command.

No key should silently trigger two actions.

---

## 15. Repetition

Holding a rhythm navigation key may repeat.

Key repeat must not:

- queue excessive commands;
- create accidental keyframes;
- continue after focus changes;
- lag behind input.

For destructive actions, repeat behavior should be conservative.

---

## 16. Shortcut usability test scenarios

Measure keyboard-only or keyboard-dominant completion of:

1. move playhead four subdivisions;
2. create a transform keyframe;
3. move one beat;
4. create second keyframe;
5. select several keyframes;
6. shift pattern by one beat;
7. apply easing;
8. play/pause;
9. save.

If this sequence requires awkward focus gymnastics, the shortcut model has failed.

---

## 17. Open decisions

- exact command search binding;
- Space playback vs temporary pan;
- exact rhythm-navigation modifier family;
- exact property quick-access behavior;
- whether common easing presets need dedicated hotkeys;
- whether grid subdivision gets direct number-key shortcuts;
- whether numpad becomes a rhythm-entry surface later.

These should be resolved with a working prototype, not by convention alone.
