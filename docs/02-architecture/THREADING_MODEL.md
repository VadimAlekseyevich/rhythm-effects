# Threading Model

> **Status: Draft**
>
> This document defines which work may happen on which execution context, how data crosses thread boundaries, and which operations are forbidden from blocking realtime/editor paths.

## 1. Goals

The threading model must support:

- responsive editor interaction;
- glitch-free audio playback;
- background decoding/preprocessing;
- safe autosave;
- deterministic project mutation;
- debuggable ownership;
- bounded communication;
- fast builds and low runtime complexity.

The MVP intentionally avoids a general-purpose task graph and a global async runtime.

---

## 2. Core rule: one canonical project writer

The authoritative mutable Project is owned by the editor/application execution context.

Background workers do not mutate Project directly.

Conceptually:

~~~text
                   ┌─────────────────────┐
                   │ Main/editor thread  │
                   │ owns mutable Project│
                   └──────────┬──────────┘
                              │ immutable job input
                    ┌─────────┴─────────┐
                    ▼                   ▼
             background worker     export/recovery work
                    │                   │
                    └──── result ───────┘
                              │
                              ▼
                   main/editor validates
                   and applies if relevant
~~~

This avoids a shared Arc<Mutex<Project>> architecture.

---

## 3. Execution contexts

MVP has four conceptual execution contexts.

### 3.1. Main/editor thread

Owns or orchestrates:

- winit event loop;
- editor session;
- canonical Project mutation;
- undo/redo history;
- shortcut dispatch;
- selection/focus;
- interaction transactions;
- normal renderer orchestration;
- applying completed background results;
- project lifecycle.

This is the only normal writer of creative project state.

### 3.2. Audio realtime callback

Owns only realtime audio delivery concerns.

May:

- consume already prepared PCM/ring-buffer data;
- write output samples;
- update minimal atomic playback counters;
- observe simple atomic control state where safe.

Must not:

- mutate Project;
- allocate on normal callback path;
- do file I/O;
- decode media;
- wait on blocking mutex;
- log every callback;
- call UI code;
- perform GPU work;
- serialize;
- run arbitrary user commands.

### 3.3. Background workers

Used for bounded non-realtime work such as:

- audio decode;
- waveform generation;
- image decode;
- thumbnails;
- cache generation;
- recovery file writing;
- possibly project parsing before main-thread adoption.

Workers receive owned/immutable job input and return owned results.

### 3.4. Export execution context

Export is logically asynchronous from editor interaction but has stricter GPU/resource constraints than ordinary workers.

MVP contract:

- export operates on an immutable project snapshot;
- export never mutates active Project;
- export has explicit cancellation/progress;
- exact thread/device placement is an implementation decision after renderer spike.

It is acceptable for early MVP export to temporarily restrict editing if sharing the renderer safely would otherwise complicate architecture.

Correctness beats background-export convenience.

---

## 4. Why not a global thread pool first

A generic pool can obscure:

- which jobs may block;
- ordering;
- cancellation;
- memory pressure;
- which subsystem owns a result.

MVP starts with explicit named workers or a very small bounded worker abstraction.

Revisit only after concrete workloads justify pooling.

---

## 5. Worker communication

Prefer message passing over shared mutable state.

Conceptual job:

~~~rust
struct JobEnvelope<J> {
    request_id: RequestId,
    generation: Generation,
    payload: J,
}
~~~

Result:

~~~rust
struct JobResult<R> {
    request_id: RequestId,
    generation: Generation,
    result: Result<R, WorkerError>,
}
~~~

Exact types may differ.

The important properties are:

- result can be correlated;
- stale results can be rejected;
- queue length is bounded;
- ownership is obvious.

---

## 6. Bounded queues

Queues between editor and workers must be bounded where unbounded production is possible.

Examples:

- image decode;
- waveform rebuild;
- thumbnail generation.

If user performs repeated changes quickly, coalesce or supersede older jobs rather than queueing unlimited obsolete work.

---

## 7. Generation tokens

Background work can become stale.

Example:

~~~text
import image A
→ decode job starts

user relinks asset to image B
→ new decode job starts

A finishes late
~~~

Late result A must not overwrite B.

Use a per-resource generation/revision token.

Apply result only if:

- entity still exists;
- generation still matches expected generation.

---

## 8. Project revisions

Project mutation can maintain a monotonically increasing session revision.

Background jobs that depend on a project snapshot may record the revision they were created from.

Not every result must be rejected if project revision changed globally; use narrow generation scopes where possible.

Example:

- waveform depends on audio asset generation, not every color edit.

Avoid invalidating expensive work unnecessarily.

---

## 9. Audio control communication

Control path from editor to audio engine should be narrow.

Typical commands:

- load/replace prepared source;
- play;
- pause;
- seek;
- set gain;
- shutdown.

Do not send entire Project objects into audio engine.

Audio subsystem should receive resolved audio-specific state.

---

## 10. Playback position publication

Realtime callback/output logic publishes minimal clock state.

Possible implementation:

- atomic output frame count;
- atomic playback generation;
- separately known latency/buffer state.

The editor samples this clock once per visual frame.

Exact formula belongs in AUDIO_ENGINE.md.

---

## 11. No blocking waits on main interaction path

Main/editor thread should not synchronously wait for:

- full audio decode;
- waveform build;
- image decode;
- autosave disk write;
- export completion.

Allowed synchronous work must be predictably short.

File dialogs are externally modal OS interactions and are a separate UX concern.

---

## 12. GPU ownership

For MVP, renderer state should have clear ownership and avoid unnecessary locking.

Preferred normal-preview model:

- renderer orchestrated from main/editor thread;
- GPU resource mutation performed in renderer boundary;
- background CPU decode returns bytes/images, then renderer uploads on safe owner context.

Do not pass wgpu resource handles freely through unrelated workers without need.

---

## 13. Image import path

Preferred:

~~~text
main:
create/update AssetRecord
enqueue decode request

worker:
read/decode image
return decoded pixels + metadata

main/renderer:
check generation
upload GPU texture
publish runtime asset ready
~~~

The worker never mutates project or renderer cache directly.

---

## 14. Waveform path

Preferred:

~~~text
audio import/decode
→ PCM/source representation
→ waveform worker
→ multi-resolution peaks
→ runtime/cache result
~~~

Timeline reads immutable prepared peak data.

Waveform generation never runs in audio callback.

---

## 15. Autosave path

Preferred:

~~~text
main reaches consistent semantic state
→ create serialization snapshot/bytes
→ worker writes safe recovery file
→ result reports success/failure
~~~

If project copying becomes expensive, optimize snapshot strategy later.

Do not solve this preemptively with shared project locks.

---

## 16. Project open path

Safer model:

~~~text
select path
→ worker may read/parse/migrate candidate
→ return complete validated candidate
→ main adopts candidate atomically
~~~

No partially loaded candidate mutates active project.

If migration logic remains fast, it may run synchronously initially, but adoption boundary remains transactional.

---

## 17. Cancellation

Long-running jobs should support cooperative cancellation.

Candidates:

- waveform rebuild;
- large image decode if library permits;
- export;
- recovery write only where useful.

Cancellation token can be an atomic flag or generation mismatch.

Do not build a generic cancellation framework until needed.

---

## 18. Shutdown

Shutdown order:

1. stop accepting new edits/jobs;
2. resolve dirty-project close UX;
3. stop audio playback;
4. signal workers/export cancellation;
5. wait only for bounded critical cleanup;
6. finalize recovery/save if required;
7. release renderer/window/runtime.

Do not hang indefinitely waiting for noncritical background work.

---

## 19. Panic policy

Panics are bugs, not normal error transport.

Rules:

- audio callback code should be written so normal invalid external state cannot panic;
- worker failure returns Result where recoverable;
- main thread logs fatal panic diagnostics where possible;
- recovery system protects user work independently of panic catching.

Do not use catch_unwind as routine control flow.

---

## 20. Thread-safe types

Do not mark domain types Send/Sync through unsafe implementation to satisfy architecture.

If a type cannot safely cross threads, keep it behind its owner boundary.

Prefer owned plain data messages.

---

## 21. Memory pressure

Bounded workers prevent hidden memory explosions.

Examples:

- never queue hundreds of decoded 4K images waiting for GPU upload;
- do not queue every autosave tick;
- do not keep unlimited export frames.

Backpressure/coalescing must be explicit.

---

## 22. Debug diagnostics

Developer diagnostics should expose:

- worker queue lengths;
- active jobs;
- stale/discarded results;
- autosave duration;
- decode duration;
- waveform duration;
- export queue/progress;
- audio underruns.

This makes concurrency problems observable.

---

## 23. Forbidden patterns

Avoid:

~~~text
Arc<Mutex<Project>> shared by UI/audio/workers
~~~

Avoid:

~~~text
background thread directly calling egui
~~~

Avoid:

~~~text
audio callback sends unbounded allocations/messages per buffer
~~~

Avoid:

~~~text
worker directly updates GPU/editor selection
~~~

Avoid:

~~~text
unbounded channel for repeated obsolete jobs
~~~

---

## 24. Revisit conditions

Revisit threading architecture if:

- asset workloads justify a shared bounded pool;
- export needs true concurrent editor/render use;
- platform APIs require async;
- project snapshot cost becomes measurable;
- worker count becomes difficult to manage.

Any change must preserve single-writer project semantics unless a strong reason emerges.

---

## 25. Definition of Done

Threading model is implementation-ready when:

- canonical Project writer is unambiguous;
- audio callback restrictions are encoded in subsystem design;
- background jobs use bounded communication;
- stale-result handling is defined;
- autosave/import/waveform paths do not require global project locks;
- export snapshot boundary is explicit;
- shutdown/cancellation paths are testable.
