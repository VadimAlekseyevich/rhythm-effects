# Project Recovery

> **Status: Draft**
>
> This document defines autosave, crash recovery, and minimum data-loss protection for MVP.

## 1. Goal

A crash, power loss, GPU/audio failure, or application bug should not casually destroy hours of user work.

Recovery must be useful without becoming a complicated version-control system.

---

## 2. Principles

1. Explicit Save remains the canonical user save.
2. Autosave creates recovery state, not surprising hidden replacement of the canonical file.
3. Recovery should be mostly invisible during normal work.
4. Recovery operations must avoid noticeable editor stalls.
5. Recovery files must be distinguishable from normal project files.
6. The user should receive a simple restore/discard decision after an abnormal termination.

---

## 3. Failure scenarios

Recovery must consider:

- process crash;
- panic;
- GPU/device-loss crash;
- forced termination;
- operating-system restart;
- power loss;
- crash during autosave;
- crash during explicit save;
- corrupt recovery file;
- canonical project moved/deleted;
- unsaved new project.

---

## 4. Session identity

Every open editing session should have a recovery identity.

For a saved project, identity can be derived from:

- canonical project path;
- project/session ID.

For unsaved projects, allocate a temporary session ID.

Recovery must work before the first explicit Save.

---

## 5. Recovery storage location

Preferred MVP strategy:

store recovery files in an application-controlled per-user directory, not beside every source project.

Benefits:

- avoids clutter;
- works for unsaved projects;
- easier cleanup;
- no permission dependence on project folder.

The recovery record stores enough metadata to show:

- original project path if any;
- display name;
- last autosave time;
- schema version.

Exact Windows path belongs to PACKAGING.md.

---

## 6. Autosave trigger

Autosave should be based on dirty state plus time/activity.

Initial proposal:

- only autosave when project is dirty;
- target interval around 30–60 seconds;
- avoid firing continuously during heavy interaction;
- if a transient drag/value edit is active, autosave after a stable semantic state is available.

Exact interval should be tested for save cost.

---

## 7. Autosave architecture

Do not serialize directly from the audio callback or GPU thread.

Concept:

~~~text
dirty project
→ autosave due
→ obtain consistent semantic snapshot / serialized representation
→ background write recovery temp file
→ atomic replace recovery file
~~~

The main/UI thread should not block on slow disk longer than necessary.

How to obtain the snapshot depends on Project size and ownership; implementation must preserve consistency.

---

## 8. Recovery file safety

Recovery file itself uses safe replacement:

~~~text
recovery.tmp
→ completed write
→ replace recovery.current
~~~

If autosave crashes halfway, prior good recovery remains where practical.

---

## 9. Explicit save interaction

After successful explicit Save:

- canonical file becomes current known-good user save;
- recovery state may be deleted or marked no longer newer.

Do not delete useful recovery state before explicit save succeeds.

If explicit save fails:

- recovery remains;
- dirty state remains.

---

## 10. Clean shutdown marker

The app needs a way to distinguish normal close from abnormal termination.

Possible pattern:

- active recovery/session record exists while dirty editing is in progress;
- on clean shutdown after state is safely handled, mark/remove session record.

Avoid relying only on an in-memory flag.

---

## 11. Startup recovery detection

On startup, inspect recovery directory for sessions that were not cleanly finalized.

If one relevant recovery exists, present a simple UX.

If many recoveries exist, MVP may use a minimal list rather than an elaborate recovery browser.

Information shown:

- project name/path if known;
- recovery timestamp;
- canonical save timestamp if known.

---

## 12. Restore flow

1. User chooses Restore.
2. Recovery file is parsed/migrated/validated like any project.
3. Editor opens recovered semantic state.
4. The restored state is considered dirty unless it exactly matches canonical saved state.
5. User can Save/Save As.

Do not silently overwrite the canonical project merely because recovery was restored.

---

## 13. Discard flow

Discard removes/archives recovery record only after explicit user action.

If canonical project exists, it remains untouched.

For an unsaved project, warn that discard removes the only recovery copy.

---

## 14. Recovery file is corrupt

If recovery cannot load:

- do not touch canonical save;
- provide clear error;
- offer to open canonical project if available;
- keep corrupt recovery for diagnostics until user chooses cleanup where practical.

---

## 15. Canonical project newer than recovery

If normal project file timestamp/state is clearly newer than stale recovery, do not aggressively prompt.

The system may clean stale recovery automatically only when safety is certain.

When uncertain, prefer a low-friction informational choice over data loss.

---

## 16. Autosave and undo

Autosave stores current project semantic state.

It does not need to persist the undo stack for MVP.

After restoring:

- project content returns;
- undo history may start fresh.

This limitation should be acceptable for MVP and may be documented in recovery UI if relevant.

---

## 17. Recovery and assets

Recovery stores project references, not copies of every external asset.

If an asset disappears:

- recovery still loads;
- missing asset behavior follows ASSETS.md.

Packed recovery is out of MVP scope.

---

## 18. Performance requirements

Autosave should:

- avoid visible input hitching;
- avoid audio interruption;
- avoid GPU synchronization;
- not serialize derived caches;
- coalesce repeated autosave requests if a previous write is still running.

At most one recovery write per project/session should be active.

---

## 19. Logging

Log:

- autosave start/end duration;
- autosave failure;
- recovery discovery;
- restore success/failure;
- cleanup failures.

Do not show normal successful autosaves as noisy user notifications.

---

## 20. Testing

Required scenarios:

- dirty saved project autosaves;
- clean project does not autosave repeatedly;
- unsaved new project gets recovery;
- simulated crash leaves detectable recovery;
- clean shutdown does not prompt incorrectly;
- corrupt recovery does not damage canonical project;
- explicit save failure retains recovery;
- recovery write failure retains previous recovery;
- restore loads expected semantic state;
- discard leaves canonical file untouched.

---

## 21. Implementation sequence

1. Define recovery directory.
2. Define recovery metadata/session ID.
3. Implement safe recovery-file writer.
4. Add dirty + timer trigger.
5. Add background/coalesced autosave job.
6. Add clean-shutdown lifecycle.
7. Add startup scan.
8. Add restore/discard UI.
9. Add automated failure tests.
10. Measure hitching with medium project fixture.

---

## 22. Open decisions

- autosave interval;
- exact recovery directory;
- stale recovery retention period;
- whether previous explicit save backup is separate from autosave;
- recovery list UX if multiple sessions exist;
- whether app records a lightweight clean-shutdown marker separately.

---

## 23. Definition of Done

Recovery is MVP-ready when:

- dirty work is periodically recoverable;
- unsaved projects are recoverable;
- abnormal exit is detected without corrupting canonical saves;
- Restore/Discard flow is understandable;
- explicit save failure does not destroy recovery;
- corrupt recovery fails safely;
- normal autosave does not cause perceptible audio/editor interruption under target project size.
