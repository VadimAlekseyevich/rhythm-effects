# Commands & Undo/Redo

> **Status: Accepted for MVP**
>
> All user-authored project mutation must be compatible with predictable undo/redo.

## 1. Goals

The history system must:

- undo semantic user actions;
- group continuous drags into one step;
- restore exact project state;
- support live preview during interaction;
- avoid cloning the entire project per mouse movement;
- keep editor/session state separate from creative data where appropriate;
- remain easy to test.

---

## 2. What is undoable

Undoable project edits include:

- add/delete/duplicate object;
- rename object;
- reorder object;
- property change;
- add/delete/move keyframe;
- multi-keyframe move;
- interpolation/easing change;
- add/remove/reorder effect;
- effect parameter change;
- asset reference change;
- BPM/offset change;
- composition setting change.

Not every editor navigation action is undoable.

Not undoable by project history:

- playhead movement;
- timeline pan/zoom;
- viewport pan/zoom;
- selection;
- focused panel;
- opening a popover.

---

## 3. Architecture

The editor dispatches semantic edit operations to a project editor layer.

Concept:

~~~text
UI intent
→ ProjectEditor
→ validate
→ mutate Project
→ create HistoryEntry
→ mark dirty
~~~

Do not scatter direct field mutation across widgets.

---

## 4. Command representation

Prefer explicit enums/data over trait-object command frameworks for MVP.

Concept:

~~~rust
enum EditCommand {
    AddObject { ... },
    DeleteObject { ... },
    SetProperty { ... },
    AddKeyframe { ... },
    DeleteKeyframes { ... },
    MoveKeyframes { ... },
    SetInterpolation { ... },
    AddEffect { ... },
    ...
}
~~~

Actual implementation may separate command input from stored undo entry.

Reason:

- exhaustive matching;
- easy tests;
- no dynamic-dispatch requirement;
- easy logging/debug formatting;
- clear domain boundaries.

---

## 5. History entry

History needs both undo and redo capability.

Two viable strategies:

### A. Store forward + inverse commands

~~~text
HistoryEntry {
    forward,
    inverse
}
~~~

### B. Store semantic before/after payload

~~~text
SetPropertyHistory {
    target,
    before,
    after
}
~~~

For MVP, prefer specialized history payloads that contain the minimal before/after state necessary.

Do not snapshot the entire Project for every edit.

---

## 6. Transactions for drag interactions

A drag is not hundreds of commands.

Lifecycle:

~~~text
begin transaction
  capture initial affected state

update
  mutate current project for live preview
  no history push yet

commit
  capture final state
  if changed:
      push one HistoryEntry

cancel
  restore initial state
  push nothing
~~~

This pattern applies to:

- object move/scale/rotate;
- keyframe drag;
- multi-keyframe drag;
- curve handles;
- numeric scrub;
- BPM offset drag.

---

## 7. Property editing

Text/numeric edit should not create history on each keystroke.

Accepted behavior:

- focus begins edit transaction;
- intermediate values update preview when valid;
- Enter/focus loss commits one history step;
- Escape reverts;
- invalid intermediate text remains editor-control state and does not corrupt Project.

This is especially important for numbers like "-" or "12." while typing.

---

## 8. Command validation

Commands validate before destructive mutation where practical.

Examples:

- target ID exists;
- value valid;
- keyframe target property supports type;
- target tick valid;
- no illegal duplicate;
- object reorder index valid.

A failed command:

- must not partially mutate;
- does not enter history;
- returns structured error.

---

## 9. Dirty state

Project session tracks save revision.

Concept:

~~~text
history_revision
saved_revision
dirty = history_revision != saved_revision
~~~

Need handle branches after undo:

~~~text
edit A
save
edit B
undo B
→ project may be semantically back at saved state
~~~

A revision/history-position model can represent this accurately.

If history is truncated after editing from an undone state, saved revision may become unreachable and project remains dirty.

---

## 10. Undo

Undo:

1. if active transient interaction exists, cancel/resolve it first according to UI policy;
2. fetch previous HistoryEntry;
3. apply inverse/before state atomically;
4. move history cursor;
5. update dirty state;
6. keep project valid.

Selection restoration is optional and should be conservative.

For example, undoing object deletion may reasonably reselect restored object, but this is editor behavior layered on top of project undo, not core project data.

---

## 11. Redo

Redo reapplies the stored after/forward state.

Any new committed edit after undo removes the redo branch.

MVP does not need branching history.

---

## 12. History memory

Deleted objects can contain many keyframes/effects.

Undo must preserve enough data to restore them.

Use owned snapshots of affected entities for destructive edits.

Set a practical history memory policy later if projects become large.

Do not prematurely write history to disk.

---

## 13. Compound commands

Some user actions touch multiple fields.

Examples:

- create image object + asset reference;
- duplicate object;
- paste multiple keyframes;
- delete multiple objects.

Represent them as one semantic history entry.

Do not expose internal implementation steps as separate undo presses.

---

## 14. Keyframe movement

History record stores:

- affected KeyframeIds;
- before ticks;
- after ticks.

For values unchanged during a horizontal timeline move, no need to duplicate value payloads.

Collision resolution must occur before commit and final history entry must describe actual resulting state.

---

## 15. Keyframe value change

Store:

- property target;
- keyframe ID/tick;
- before value;
- after value.

If editing base value of unanimated property, store base value before/after instead.

---

## 16. Object transform drag

Store transform component(s) changed.

A free 2D position drag should be one history entry even if both x/y change.

If modifier duplicates object while dragging in future, that becomes a compound command.

---

## 17. BPM and offset history

BPM/offset affect derived timing of all animation.

They are ordinary project edits but potentially visually large.

History stores old/new tempo settings.

Undo must restore exact previous tempo configuration.

No per-keyframe timestamps are rewritten because keyframes remain in MusicalTick coordinates.

This is a major benefit of musical-time storage.

---

## 18. Serialization and history

Undo stack is not saved in MVP project files.

Save serializes current Project only.

Autosave/recovery likewise stores current semantic project state, not necessarily history.

---

## 19. Command IDs and observability

For debugging, edit operations should be loggable by semantic name.

Examples:

~~~text
MoveKeyframes(count=12, delta_ticks=960)
SetProperty(object=42, property=opacity)
ChangeTempo(128.0 -> 130.0)
~~~

Do not log huge object payloads by default.

---

## 20. Threading

Project mutation occurs on editor/main ownership path.

Background workers do not issue arbitrary project commands directly.

They return results:

~~~text
worker result
→ app validates it is still relevant
→ app commits project/runtime change
~~~

Example: waveform completion changes runtime cache state, not creative project history.

---

## 21. Command coalescing

Besides explicit drag transactions, nearby edits may be coalesced selectively.

Accepted scope:

- repeated arrow-key nudge;
- repeated numeric increment;
- typing a property.

Do not implement generic time-window coalescing until UX requires it.

Explicit transaction boundaries are safer.

---

## 22. History labels

Each entry should provide human-readable action name for future UI/debugging.

Examples:

- Move 4 Keyframes
- Delete Rectangle
- Change BPM
- Add Glow
- Edit Position

MVP does not require a visible history panel, but labels make debugging and future UI easier.

---

## 23. Undo failure

Applying a stored history entry should normally be infallible because it originated from valid state.

If internal inconsistency prevents undo:

- do not silently continue with partial state;
- log diagnostic;
- preserve as much valid state as possible;
- surface a serious recoverable/fatal error according to app policy.

Tests should make this path extremely rare.

---

## 24. Required tests

- one drag = one history step;
- Escape during drag restores exact before state;
- undo/redo add object;
- undo/redo delete object with effects/keyframes;
- undo/redo multi-keyframe move;
- redo branch cleared after new edit;
- dirty state across save/edit/undo;
- invalid command causes no partial mutation;
- BPM undo changes derived absolute timing without rewriting keyframe ticks;
- duplicate object undo/redo preserves correct fresh IDs.

---

## 25. Accepted remaining policies

### Command/history representation

Use explicit EditCommand intent plus specialized HistoryEntry before/after payloads.

Do not store full-Project snapshots per edit.

### History capacity

Keep at most 500 committed logical history entries in MVP.

When capacity is exceeded, drop oldest entries.

This is a predictable guardrail, not a claim that every history entry has equal memory cost. PERFORMANCE.md requires stress measurement of destructive large-object edits.

### Selection restoration

Project history does not persist selection.

The app may opportunistically reselect a restored/created object after undo/redo when the target identity is unambiguous, but selection restoration is not part of semantic undo correctness.

### Keyboard nudge coalescing

A continuous OS key-repeat burst affecting the same target/operation is one logical history entry.

End the burst when:

- the key is released;
- selection changes;
- another edit begins;
- approximately 300 ms passes without another matching repeat event.

### Import + create grouping

"Add Image from File" is one compound edit:

- create AssetRecord if needed;
- create ImageObject;
- reference the asset.

Undo removes the object and removes the newly-created asset record only if that record was introduced by the same compound action and has no remaining references.

### Settings editing

Project-setting and property fields commit on Enter or focus loss.

Escape restores the pre-edit value.

No explicit Apply button is used for routine inspector/project numeric settings.

## 26. Revision semantics

Each committed history state receives a monotonically increasing session revision ID.

Save records the current revision as saved_revision.

Dirty state is false only when current history state is the recorded saved state.

If the saved state is dropped from bounded history and the user continues editing, dirty remains true until the next successful Save.

## 27. Definition of Done

The command/history contract is implementation-ready when:

- all creative mutations route through ProjectEditor;
- continuous transactions produce one history entry;
- invalid operations are atomic/no-op on failure;
- redo branch truncation works;
- dirty state follows revision position;
- 500-entry cap behaves predictably;
- compound image import/create undo is tested;
- key-repeat coalescing is tested.
