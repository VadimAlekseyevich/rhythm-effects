# State Ownership

> **Status: Draft**
>
> This document defines where each category of state lives, whether it is persisted, and who is allowed to mutate it.

## 1. Purpose

Complex editor bugs often come from the same state being represented in multiple places.

Rhythm Effects should make one owner authoritative for every important state category.

Rule:

> **One semantic fact has one authoritative owner. Other representations are caches, views, or derived values.**

---

## 2. State categories

| State | Owner | Persisted | Mutable by |
|---|---|---:|---|
| Creative project | rhythm_app session / ProjectEditor | Yes | main/editor project mutation path |
| Undo history | EditorSession | No | command/history system |
| Selection | EditorSession | No | editor interaction |
| Playhead while paused | EditorSession | No | editor/timeline |
| Playback clock while playing | AudioEngine | No | audio runtime |
| Timeline zoom/scroll | EditorSession | No | timeline UI |
| Viewport pan/zoom | EditorSession | No | viewport UI |
| Runtime decoded assets | RuntimeAssetManager | No | engine asset lifecycle |
| GPU resources | Renderer | No | renderer |
| Waveform cache | runtime/cache subsystem | derived sidecar | waveform subsystem |
| App preferences | AppSettings | separate | settings UI/app |
| Recovery metadata | Recovery subsystem | separate | recovery manager |
| Export snapshot | Export job | ephemeral | immutable after job start |

---

## 3. Project is semantic truth

Project contains only creative data required to reproduce the work.

Examples:

- composition;
- objects;
- animation;
- BPM;
- asset references;
- effects.

It does not contain:

- current hover;
- timeline x offset;
- active audio device;
- decoded texture;
- undo cursor.

---

## 4. EditorSession

Conceptual structure:

~~~rust
EditorSession {
    project: Project,
    history: History,
    selection: SelectionState,
    playhead: EditorPlayhead,
    workspace: WorkspaceState,
    interaction: InteractionState,
    dirty/save_revision,
}
~~~

Exact code may split these fields.

The important boundary is that session owns editing context while Project owns creative document semantics.

---

## 5. Derived evaluated scene

EvaluatedScene is not persisted.

It is a per-frame or cached derivation:

~~~text
Project + EvaluationTime
→ EvaluatedScene
→ Renderer
~~~

It must never become an independently editable truth source.

Renderer cannot write animation values back to Project.

---

## 6. Runtime assets

Persisted AssetRecord and runtime loaded asset are distinct.

~~~text
Project:
AssetId -> path/type/metadata

Runtime:
AssetId + generation -> decoded/GPU resource
~~~

A runtime asset can be missing/rebuilding while project remains valid.

---

## 7. Cache authority

Caches never become authoritative.

If cache and source disagree:

- source/project wins;
- cache is invalidated/rebuilt.

This applies to:

- waveform cache;
- thumbnails;
- decoded image cache;
- glyph atlas;
- pipeline cache.

---

## 8. Selection

Selection uses stable IDs, never pointers or screen coordinates.

Possible references:

- ObjectId;
- KeyframeId plus owning property context;
- EffectId.

When selected entity disappears:

- selection cleans itself safely.

Undo may optionally restore selection as editor behavior, but Project does not know selection exists.

---

## 9. Playhead authority transition

When paused:

- EditorSession playhead is authoritative.

When playback starts:

1. editor resolves desired audio/project start;
2. AudioEngine starts;
3. AudioEngine playback clock becomes authoritative.

When paused/stopped:

1. editor samples final resolved audio position;
2. writes paused playhead;
3. paused editor state becomes authority again.

This transition must be explicit.

---

## 10. BPM/grid state

Creative timing:

- BPM;
- grid offset;
- meter;
- tempo map

belongs to Project.

View/edit preference:

- current visible/authoring subdivision;
- grid label density

may belong to EditorSession/AppSettings.

Changing a view preference must not change persisted keyframe timing.

---

## 11. Current authoring subdivision

Recommended MVP ownership:

- EditorSession / workspace preference, not core Project semantics.

Reason:

the project remains musically valid independently of which subdivision the current user happens to be editing with.

Optionally persist it in a separate workspace state later.

---

## 12. Undo state

History is session state.

It references project entities through stable IDs/snapshots but is not part of saved project file.

Save marks a history revision as canonical saved state.

Recovery may restore project semantics without restoring undo history.

---

## 13. UI text buffers

Temporary input buffers are UI state.

Example user types:

~~~text
-
12.
~~~

These may be temporarily invalid numeric representations.

They must not enter Project until validated/committed.

This separates text-control state from domain validity.

---

## 14. Transient drag state

Drag stores before-state and preview state in InteractionState/transaction.

Do not add dozens of intermediate undo entries.

On cancel:

- restore before-state.

On commit:

- one semantic command/history entry.

---

## 15. Renderer state

Renderer owns:

- pipelines;
- buffers;
- textures;
- temporary texture pool;
- composition targets.

Project holds none of these.

Renderer resources can be destroyed/recreated without creative-data loss.

---

## 16. Audio engine state

AudioEngine owns:

- device/stream;
- playback buffers;
- resampler;
- current runtime cursor;
- playback generation;
- gain runtime state.

Project stores only creative/source audio configuration.

---

## 17. AppSettings

Separate persisted settings file may contain:

- theme;
- UI scale;
- preferred audio device;
- recent files;
- future shortcut remaps.

Failure/corruption of AppSettings must not corrupt project files.

Defaults should recreate a usable app.

---

## 18. Background results

A background result is not authoritative until owner accepts it.

Example:

~~~text
DecodedImageResult(asset_id, generation, pixels)
~~~

Renderer/asset manager verifies ID/generation before installing it.

---

## 19. Save snapshot

Serialization reads a coherent project semantic state.

Do not serialize while a transaction is half-applied.

Policies:

- serialize committed state;
- or complete/cancel active edit first.

Exact UX can vary, but saved project must satisfy Project invariants.

---

## 20. Ownership review checklist

For every new feature ask:

- What is the authoritative state?
- Who owns it?
- Is it project, session, runtime, cache, or preference?
- Who can mutate it?
- Is it serialized?
- Can it be rebuilt?
- How is stale derived state detected?
- Does it need stable ID?
- Does it cross threads?

If these answers are unclear, implementation should not begin.

---

## 21. Definition of Done

State ownership is ready when:

- every existing subsystem maps into one state category;
- Project contains no runtime handles;
- EditorSession contains no duplicated creative truth;
- runtime caches can be dropped/rebuilt;
- playhead authority transition is explicit;
- background results require owner validation;
- future features have a mandatory ownership review.
