# Project Recovery

> **Status: Accepted for MVP**
>
> Autosave is recovery state, not a hidden replacement for explicit Save.

## 1. Storage

Windows MVP recovery root:

~~~text
%LOCALAPPDATA%\RhythmEffects\recovery\
~~~

AI-242 resolves this root from the absolute `LOCALAPPDATA` environment value.
An absent, empty or relative value is an explicit error, never a fallback to the
working directory. The directory is created idempotently at application startup;
a setup error is logged without preventing the editor from opening. Tests cover
normal resolution, repeated creation, invalid environment values and a blocked
parent directory.

AI-243 creates a unique per-editing-session directory using a timestamp,
process ID and atomic counter, with `create_dir` collision protection.
Its synchronized `session.json` records the stable session ID, project display
name, optional canonical file path and creation timestamp. Save As updates
metadata through a same-directory temporary file and safe replacement without
changing the ID; opening a different document allocates a new session.
Metadata is read back only if its session ID matches the directory name.
These metadata records are not yet recovery project snapshots (AI-244 onward).

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

The AI-244–247 implementation polls dirty state from the application event
loop even when UI input is idle. Its 30-second due time is measured from the
last successfully completed generation; failures have a five-second retry
backoff. A live editor transaction blocks capture and the first eligible
snapshot waits at least one second after that transaction ends.

A cloned committed `Project` crosses to a named background recovery worker,
so serialization, synchronization and publication do not run on the UI/audio
path. The coordinator allows only one writer in flight and replaces any queued
snapshot with the latest committed revision while it is busy. A successful
generation writes `current.rhfx` through a sibling temporary file. When an
older current is valid, it is staged independently and only published to
`previous.rhfx` after the new current succeeds. Malformed current is never
rotated over a known-good previous; failed publication does not delete the
previous success. The current snapshot and previous fallback remain ordinary
bounded, validated V1 project files in the session directory.

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

AI-252 deletes only the obsolete `current.rhfx` and `previous.rhfx`
generations after successful explicit Save/Save As, preserving stable
`session.json` for future editing in the same session. If a recovery worker
is already in flight, cleanup waits until its completion rather than racing
the writer. A new dirty edit cancels pending clean-save cleanup; the next
generation is again due immediately. Clean close joins an existing worker
before removing obsolete generations. A dirty project's close keeps recovery
intact. Successful Save is always the prerequisite: a failed Save cannot
request cleanup.

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

AI-253 replaces unconditional close of a dirty project with explicit Save,
Don't Save or Cancel choices. Save must publish successfully before closing;
cancel or Save failure keeps the editor and its recovery. Don't Save requires
a positive click and waits for an already-running recovery worker before
deleting only the active session directory. It does not delete any canonical
project or another session. If discard itself fails, closing is cancelled
rather than silently claiming that the recovery was removed.

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

AI-248 startup discovery scans non-symlink session directories, validates that
each `session.json` ID agrees with its directory name, and retains only sessions
with a regular `current.rhfx` or `previous.rhfx` generation. It also observes
the last-modified timestamps of recovery and canonical files, including unsaved
projects with no canonical file; results are ordered newest first. Broken
individual records are skipped without blocking other recoveries. Discovery
does not parse/open the recovery project or change/delete canonical files.
Its candidate list is retained by the application for the Restore/Discard UX
in later tasks, and summarized in startup diagnostics.

## 9. Restore

Restore:

1. parse/migrate/validate recovery like normal project;
2. open recovered state;
3. recovered session is dirty;
4. user must Save/Save As to replace/create canonical file.

Restoring never silently overwrites the canonical project.

AI-249/251 expose a validated, explicit Restore action. The candidate is loaded
through the normal bounded read, schema parse/migration and semantic validation
gates. If current fails, the previous successful generation is attempted. The
active editor and canonical path are swapped only after a complete candidate
has been prepared; the restored project starts dirty with no saved revision
and must be explicitly saved. Restore refuses to replace an already-dirty
active editor and never automatically publishes the recovered bytes to the
canonical file. The UI shows the recovery timestamp and canonical file
timestamp so the user can choose.

## 10. Discard

Discard removes recovery only after explicit user action.

Canonical .rhfx remains untouched.

For unsaved project, UI clearly indicates recovery may be the only copy.

AI-250 displays a separate confirmation before Discard and warns that
an unsaved project may have no other copy. Discard revalidates selected
session metadata and rejects paths outside the immediate recovery root,
removing only that session's recovery directory. It never follows the
optional canonical project path as a deletion target.

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

AI-254 startup maintenance applies a conservative 14-day rule. It deletes
only discovered, identity-validated, non-active recovery sessions whose latest
generation is older than the threshold **and** whose existing canonical file
has a modification time at least as new as that generation. Unsaved-only
sessions, missing canonical files, and recoveries newer than their canonical
file are retained regardless of age. Any filesystem error is logged without
blocking recovery discovery. Deletion uses the same checked direct-child
operation as explicit Discard; no canonical project file is removed.

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
