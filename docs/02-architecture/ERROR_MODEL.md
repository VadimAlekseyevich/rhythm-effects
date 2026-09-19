# Error Model

> **Status: Accepted for MVP**
>
> This document defines how failures move from low-level subsystems to logs and user-facing UI without leaking implementation details or corrupting project state.

## 1. Goals

Errors should be:

- structured;
- actionable;
- safe;
- contextual;
- observable;
- recoverable when possible;
- separated from user-facing wording.

Do not use opaque strings as the primary internal error protocol.

---

## 2. Error classes

### 2.1. Domain validation error

The requested state is invalid.

Examples:

- BPM out of range;
- duplicate keyframe tick where operation forbids it;
- invalid dimensions;
- incompatible paste target.

Usually operation is rejected without partial mutation.

### 2.2. Operation failure

A requested external/runtime operation failed.

Examples:

- image decode;
- project save;
- file open;
- export;
- audio device initialization.

Project remains valid.

### 2.3. Degraded runtime state

Feature cannot currently function, but editor can continue.

Examples:

- audio device disconnected;
- missing asset;
- thumbnail failed;
- optional effect shader unavailable.

UI shows degraded state and recovery action.

### 2.4. Fatal application/runtime failure

Editor cannot safely continue normal operation.

Examples:

- unrecoverable GPU/device initialization failure;
- core invariant violation;
- catastrophic initialization failure.

Attempt to preserve recovery data/log diagnostics where safe.

---

## 3. Error layers

Low-level errors remain close to implementation.

Example:

~~~text
std::io::Error
→ ProjectSaveError::Write
→ AppOperationError::SaveProject
→ user message policy
~~~

Do not make renderer/audio/core depend on UI notification types.

---

## 4. User-facing message

A useful error message answers:

1. What failed?
2. What did not happen?
3. Is existing project data safe?
4. What can the user do next?

Example:

~~~text
Could not save the project.

The existing project file was not replaced.
Check that the destination is writable or choose Save As.
~~~

Not:

~~~text
OS error 5 at src/save.rs:412
~~~

Raw diagnostics belong in logs/details.

---

## 5. Error identity/context

Structured errors should carry useful machine context:

- operation;
- path if appropriate;
- AssetId/ObjectId if relevant;
- source error;
- request ID;
- subsystem.

Avoid embedding sensitive project content in logs unnecessarily.

---

## 6. Core crate errors

rhythm_core should expose domain-specific error enums.

Avoid depending on heavy generic application error crates if not necessary.

Examples:

- InvalidBpm;
- InvalidFrameRate;
- KeyframeCollision;
- MissingEntity;
- TypeMismatch;
- ValidationFailure.

Core errors should remain deterministic and testable.

---

## 7. Engine errors

Engine may wrap:

- wgpu errors;
- decoder errors;
- CPAL errors;
- font errors;
- asset I/O.

Expose subsystem-level context rather than raw backend errors alone.

---

## 8. App orchestration errors

rhythm_app decides:

- log only;
- toast/banner;
- inline panel error;
- blocking dialog;
- fatal shutdown.

Subsystem does not choose presentation.

---

## 9. Inline vs global errors

Use inline errors when tied to a visible context.

Examples:

- invalid BPM field;
- missing asset row;
- unsupported image.

Use global/banner/dialog when:

- save failed;
- export failed;
- audio output lost;
- project cannot open.

Avoid modal dialogs for every recoverable issue.

---

## 10. Transaction safety

If an operation can partially mutate external state, design transactional boundaries.

Examples:

### Project open
candidate loads fully before active Project is replaced.

### Save
temp file completes before canonical replacement.

### Command
validation occurs before mutation where possible.

### Asset relink
new asset is validated before old reference is discarded where practical.

---

## 11. Background job errors

Workers return error results.

Main owner verifies request/generation then decides whether error is still relevant.

A stale job error should usually not notify the user after the source asset/request has already changed.

---

## 12. Audio errors

Audio failure must never corrupt Project.

Examples:

- device unavailable;
- stream error;
- resampler setup failure.

Policy:

- stop playback safely;
- keep editor usable;
- preserve/resolve playhead;
- show recoverable message;
- permit retry/device reinit.

Realtime callback should not format complex errors.

---

## 13. Renderer errors

Surface errors can be transient.

Classify:

- recoverable resize/outdated surface;
- minimized/zero-size;
- device lost;
- initialization failure.

Project remains independent.

Device-loss recovery can be limited in MVP, but error must not imply project corruption.

---

## 14. Export errors

Export error result includes:

- stage: prepare/render/readback/encode/mux/publish;
- current frame if relevant;
- FFmpeg diagnostic source;
- temp-output cleanup status.

Never present partial/corrupt output as success.

---

## 15. Serialization errors

Distinguish:

- read;
- syntax parse;
- unsupported schema;
- migration;
- semantic validation;
- save serialization;
- write;
- publish/replace.

This makes recovery instructions accurate.

---

## 16. Missing assets

Missing asset is a recoverable project condition, not a parse failure.

Load project with unresolved asset state.

Render clear placeholder where appropriate.

Offer relink.

---

## 17. Logging levels

Suggested semantics:

- TRACE: high-volume diagnostics, normally off;
- DEBUG: developer state;
- INFO: lifecycle events;
- WARN: recoverable unexpected condition;
- ERROR: failed operation or serious subsystem issue.

Do not log normal per-frame/per-audio-buffer work at INFO.

---

## 18. Panic/invariant violations

Use assertions/debug assertions for programmer invariants.

Production-facing invalid input should return Result rather than panic.

A project file should never be able to intentionally trigger a normal-path panic through malformed values.

---

## 19. Error deduplication

Repeated frame-level error conditions must not flood UI/log.

Examples:

- missing texture every frame;
- audio stream callback repeats same device error;
- shader resource unavailable.

Rate-limit or report state transition once.

---

## 20. Recovery actions

User-facing error objects/presentation may expose actions:

- Retry;
- Relink;
- Save As;
- Reveal log/details;
- Reinitialize audio;
- Choose destination.

Do not invent an action when there is no useful recovery path.

---

## 21. Testing

Test:

- commands fail without partial mutation;
- open failure leaves current project untouched;
- save failure leaves canonical file intact;
- stale background failure produces no irrelevant UI notification;
- missing asset loads as degraded state;
- audio failure preserves project;
- repeated error does not spam;
- unknown project schema gives correct classification.

---

## 22. Definition of Done

Error model is implementation-ready when:

- each subsystem exposes structured error types;
- app controls user-facing presentation;
- save/open/export failures have stage-specific errors;
- recoverable runtime failures do not mutate creative state;
- project files are validated rather than trusted;
- repeated errors are deduplicated/rate-limited where necessary.


---

## Accepted app error presentation

MVP uses three user-facing presentation levels:

- inline validation for field/context errors;
- non-modal persistent banner/toast area for recoverable subsystem failures;
- modal confirmation/dialog only for project-open failure, unsaved-data close decisions, or another action that cannot safely continue without user choice.

Backend error strings are always logged; user-facing text is authored at the app boundary.
