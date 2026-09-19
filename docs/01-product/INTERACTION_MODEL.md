# Interaction Model

> **Status: Draft**
>
> This document defines editor-wide interaction rules. Individual panels may specialize them but should not contradict them without an explicit reason.

## 1. Core principle

The user should not need to remember which invisible mode the application is currently in.

Prefer direct manipulation, explicit selection, temporary modifier behavior, and contextual controls over long-lived tool modes.

---

## 2. Focus model

The application has keyboard focus, but shortcuts are classified by scope.

### Global actions

Examples:

- Save;
- Undo/Redo;
- Play/Pause;
- command search;
- Escape/cancel.

They work unless a text-entry field needs the same key sequence.

### Contextual actions

Examples:

- Delete selection;
- duplicate;
- nudge keyframes;
- frame selected object;
- timeline zoom.

They apply to the currently active editor region.

### Text-entry protection

When typing in a text or numeric field:

- printable editor shortcuts must not unexpectedly trigger;
- Escape cancels/ends edit according to control semantics;
- Enter commits where appropriate;
- global safe shortcuts such as Save may remain available.

Focus state must be visually understandable.

---

## 3. Selection model

### Single click

Select the clicked item and clear unrelated selection unless modifier is held.

### Add/remove selection

Ctrl is the default candidate for toggling selection on Windows.

Exact modifier mapping will be finalized in KEYBOARD_SHORTCUTS.md.

### Empty-space click

Clears relevant selection.

It should not accidentally clear unrelated context if doing so would make the workflow frustrating; exact panel rules are documented per panel.

### Selection ownership

Viewport and timeline selections should refer to the same underlying objects/keyframes where appropriate.

Selecting an object in viewport should select its object/layer context.

Selecting a keyframe should identify its object/property without causing disorienting panel navigation.

---

## 4. Box selection

Dragging from empty timeline/viewport space may create a selection rectangle where appropriate.

Rules:

- clear or additive based on modifier;
- visible in real time;
- edge inclusion deterministic;
- Escape cancels before commit.

Timeline box select targets keyframes.

Viewport box select targets visible/unlocked objects.

---

## 5. Drag lifecycle

Every meaningful drag has three stages:

~~~text
begin
→ transient update
→ commit or cancel
~~~

### Begin

Capture:

- initial state;
- target;
- relevant modifiers;
- snap rules.

### Transient update

UI/preview may update continuously.

Do not create one undo command per mouse-move event.

### Commit

On pointer release, create one semantic edit.

### Cancel

Escape restores initial state.

This pattern applies to:

- object transform;
- keyframe move;
- panel resize where undo is not required;
- curve handle movement;
- BPM offset manipulation.

---

## 6. Snapping behavior

Keyframe time snapping is fundamental, not optional decoration.

### Keyframe creation

Always creates at a valid musical position.

### Keyframe drag

Pointer movement is continuous; resolved keyframe position is discrete.

### Temporary override

Because MVP philosophy says authored keyframes live on the grid, a temporary modifier must not allow arbitrary off-grid keyframe placement.

A modifier may instead:

- choose a finer supported subdivision;
- temporarily disable non-time snapping, such as spatial guides.

This distinction is important.

### Visual feedback

While moving a keyframe, show enough information to understand:

- destination bar/beat/subdivision;
- delta in musical units where useful.

---

## 7. Playhead behavior

Clicking timeline ruler/time area moves playhead.

Dragging playhead:

- updates visual time continuously or at defined resolution;
- may ignore keyframe-authoring snap because playhead is not an authored keyframe;
- can optionally snap when a modifier/action requests it.

This is intentionally different from keyframe placement.

The playhead can exist between grid positions because playback/evaluation time is continuous.

---

## 8. Value editing

Numeric properties support:

- click/type;
- keyboard commit;
- drag-to-adjust where useful;
- modifier for finer adjustment if implemented.

Changing a property at a time where no keyframe exists must have explicit behavior.

Default MVP proposal:

- unanimated property: edit base value;
- animated property: edit current keyframe only if playhead is exactly on one;
- otherwise edit is not silently converted into an unexpected keyframe unless auto-key is enabled.

The final behavior must be tested for friction and may be refined in PRODUCT_SPEC.

---

## 9. Auto-key policy

Auto-key is potentially powerful and potentially dangerous.

MVP default:

- off;
- obvious status if included;
- no hidden automatic keyframe creation.

If enabled later, it must still create keyframes only at valid grid positions.

---

## 10. Double click

Use sparingly.

Potential uses:

- rename object;
- enter text editing;
- fit/focus specific content.

No critical capability should be available only by double click.

---

## 11. Right click / context menus

Context menus are accelerators, not the only location for core actions.

Use them for:

- duplicate;
- delete;
- interpolation presets;
- object-specific secondary commands.

Avoid massive nested context menus.

---

## 12. Escape hierarchy

Escape should predictably move one level out of the current transient interaction.

Priority:

1. cancel active drag/transform;
2. cancel text/value edit;
3. close temporary popover/menu;
4. clear temporary tool state;
5. optionally clear selection only when no higher-priority transient state exists.

Escape should not unexpectedly close the project/application.

---

## 13. Enter behavior

Enter generally commits the current text/numeric edit.

Do not use Enter for unrelated global actions while a field is active.

---

## 14. Delete behavior

Delete removes the active contextual selection.

Examples:

- selected keyframes in timeline;
- selected object(s) in viewport/object list;
- selected effect in inspector only when effect selection is explicit.

Destructive scope must be obvious.

Undo must restore the deletion.

---

## 15. Copy/paste

Copy operates on the active semantic selection.

### Keyframes

Copy stores:

- source property compatibility;
- musical offsets within selection;
- values;
- easing/interpolation.

Paste target is determined by current playhead/grid context.

### Objects

Copy duplicates object data and references assets rather than duplicating raw asset files.

Cross-type incompatible paste should fail clearly or offer only meaningful behavior.

---

## 16. Duplicate

Duplicate is optimized for speed.

For keyframes, duplicate should preserve pattern spacing and make immediate rhythmic repositioning easy.

For objects, duplicate should preserve animation/effects.

---

## 17. Timeline zoom/pan

Zoom:

- should center around cursor when practical;
- smooth enough to maintain orientation;
- should not change project data.

Pan:

- direct;
- does not alter playhead;
- should support common wheel/shift/middle-drag conventions where sensible.

Exact mappings go in KEYBOARD_SHORTCUTS.md.

---

## 18. Viewport zoom/pan

Viewport navigation must not conflict with object transforms.

Preferred pattern:

- wheel/pinch zoom;
- middle-drag or temporary space-pan;
- fit composition command.

Holding a navigation modifier temporarily suspends manipulation rather than permanently changing tool mode.

---

## 19. Inspector disclosure

Properties should be grouped by meaning:

- Transform;
- Appearance;
- object-specific;
- Effects.

Avoid nested collapsible sections deeper than necessary.

Default view should show the most frequently edited properties.

Advanced controls can be disclosed contextually.

---

## 20. Modal windows

Routine editing should avoid modal windows.

Allowed/likely MVP modal use:

- file picker;
- destructive project-level confirmation when data loss is possible;
- minimal export configuration if implemented as modal-like surface.

Prefer in-workspace popovers/panels for ordinary commands.

---

## 21. Notifications

Use lightweight notifications for:

- save success only when useful;
- export completion;
- recoverable asset issue;
- background task completion/failure.

Do not spam toasts for every normal edit.

Errors requiring user action should remain visible long enough to understand.

---

## 22. Pointer target policy

Interactive visual glyph and hit target are not the same thing.

Examples:

- keyframe diamond may look compact but has a larger invisible hit region;
- curve handle points have enlarged hit area;
- resize separators have forgiving hit area.

This is essential for a comfortable editor.

---

## 23. Disabled state

A disabled control should communicate why when the reason is non-obvious.

Example:

- keyframe action unavailable because BPM/grid is not configured.

Do not leave controls mysteriously inert.

---

## 24. Animation during UI transitions

UI animation should be subtle and must never delay editing.

No decorative transition should:

- block input;
- hide current state;
- add noticeable latency;
- animate large panels constantly.

The viewport animation itself is the content; editor chrome should stay calm.

---

## 25. Interaction consistency test

Before accepting a new interaction, ask:

- Can the user predict it from existing behavior?
- Can it be undone?
- Does Escape cancel it?
- Does focus change shortcut meaning unexpectedly?
- Is the active target obvious?
- Does it add a persistent mode?
- Can it be done efficiently by keyboard?
- Can it still be discovered by pointer?
