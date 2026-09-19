# Project Recovery

> **Status: Accepted for MVP**
>
> Autosave is recovery state, not a hidden replacement for explicit Save.

## 1. Storage

Windows MVP recovery root:

~~~text
%LOCALAPPDATA%\RhythmEffects\recovery\
~~~

Each editing session has a stable recovery directory/id.

Unsaved new projects receive recovery before first explicit Save.

## 2. Recovery generations

Keep two successful generations per active/recent session:

~~~text
current
previous
~~~

Writing current uses temp + safe replace.

A failed new recovery must not destroy previous successful recovery.

## 3. Autosave cadence

Autosave runs when Project is dirty.

Policy:

- target maximum interval: 30 seconds since last successful recovery;
- wait at least 1 second after an active edit transaction ends before snapshotting;
- at most one recovery write in flight;
- repeated requests coalesce to newest committed project state.

This provides frequent protection without writing on every drag/keystroke.

## 4. Snapshot semantics

Recovery captures only a valid committed semantic Project.

Do not serialize a half-applied drag/text transaction.

If autosave becomes due during a transaction, defer until commit/cancel.

Undo history is not recovered in MVP.

## 5. Background write

Main/editor ownership produces a coherent serialization snapshot/bytes.

Disk write happens outside realtime/audio paths.

Project is never shared through a global mutex solely for autosave.

## 6. Clean explicit Save

After successful explicit Save:

- canonical project is known-good;
- dirty state clears;
- obsolete recovery older/equal to save may be removed or marked stale.

Do not delete recovery before Save succeeds.

## 7. Clean close

If user closes a clean project:

- remove active recovery record.

If dirty:

- normal unsaved-changes choice appears.

If user chooses Save and it succeeds:
- remove obsolete recovery.

If user explicitly chooses Don't Save:
- delete current recovery because data loss is intentional/confirmed.

If user Cancels:
- keep editing/recovery.

## 8. Crash/abnormal exit

Recovery/session record remains.

Next startup detects it.

Show:

- project display/path if known;
- recovery timestamp;
- canonical file timestamp if known;
- Restore;
- Discard.

Keep UX simple.

## 9. Restore

Restore:

1. parse/migrate/validate recovery like normal project;
2. open recovered state;
3. recovered session is dirty;
4. user must Save/Save As to replace/create canonical file.

Restoring never silently overwrites the canonical project.

## 10. Discard

Discard removes recovery only after explicit user action.

Canonical .rhfx remains untouched.

For unsaved project, UI clearly indicates recovery may be the only copy.

## 11. Corrupt recovery

- preserve canonical file;
- try previous recovery generation if current fails;
- report failure;
- allow opening canonical file;
- log diagnostics.

This is the reason MVP keeps current + previous.

## 12. Stale cleanup

Recovery data older than 14 days with no active/relevant session may be cleaned on startup.

Never delete a recovery that appears newer than its canonical project solely because it is old without checking state.

## 13. Assets

Recovery stores the same external asset references as Project.

It does not pack/copy media.

Missing media remains relinkable.

## 14. Performance

Recovery must not cause visible input/audio hitching in medium project.

Track serialization snapshot time and write duration.

If serialization snapshot itself becomes too expensive, optimize snapshot generation rather than introducing project-wide locking.

## 15. Tests

- saved dirty project autosaves;
- unsaved project autosaves;
- active transaction defers;
- coalescing;
- crash leaves current;
- corrupt current falls back previous;
- explicit Save failure keeps recovery;
- Don't Save removes recovery;
- Restore opens dirty;
- Discard leaves canonical untouched;
- cleanup policy.

## 16. Definition of Done

Recovery is MVP-ready when a forced crash after ordinary editing can restore recent committed work, corrupt-current fallback works, and autosave does not disrupt audio/editor responsiveness.
