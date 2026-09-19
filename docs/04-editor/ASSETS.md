# Assets

> **Status: Draft**

## 1. Scope

MVP asset types:

- primary audio;
- images;
- fonts if custom font import is accepted.

SVG remains optional.

---

## 2. Project vs runtime

Persisted AssetRecord:

- ID;
- kind;
- source path/reference;
- semantic metadata.

Runtime AssetState:

- loading;
- ready;
- missing;
- failed;
- decoded/GPU cache handles.

---

## 3. Import entry points

- Add Image;
- Import Audio;
- file picker;
- drag & drop desirable.

Do not require opening a separate asset-manager window.

---

## 4. Path policy

Need stable, portable enough references.

MVP proposal:

- store project-relative path when asset lies near/inside project structure;
- otherwise store normalized external path;
- on open, resolve relative to project file first.

Packed project can come later.

---

## 5. Missing file

Asset record remains.

Runtime state = Missing.

UI:

- clear missing indicator;
- Locate/Relink action;
- composition uses placeholder rather than crashing.

Relink updates asset source in one undoable/project edit if considered creative data.

---

## 6. Image decode

Run off UI thread.

Target formats:

- PNG;
- JPEG;
- WebP desirable.

Decode to consistent pixel format before GPU upload.

---

## 7. GPU upload

Upload once when asset becomes ready.

Cache texture by AssetId + relevant decode generation.

Do not upload every frame.

---

## 8. Thumbnails

Object/asset UI may use cached thumbnails.

Do not build thumbnails synchronously in panel render.

---

## 9. Audio asset

Primary audio asset has additional runtime:

- decoded PCM;
- waveform cache;
- duration/sample rate metadata.

Project tempo is separate from audio asset metadata.

---

## 10. Font assets

Decision pending.

If custom font import is in MVP:

- treat font file as Asset;
- runtime font system loads it;
- project TextObject references semantic font/asset relation.

System-font-only MVP is simpler but less portable.

---

## 11. Duplicate imports

Do not over-engineer deduplication.

MVP may reuse same AssetId if user selects an already imported exact path, but behavior should be deterministic.

Content hashing can come later.

---

## 12. Delete asset

If referenced:

- block deletion and explain references;
- or require explicit confirmation causing missing references.

Preferred MVP: block normal delete while referenced.

Unused assets can be removed.

---

## 13. Asset browser

Keep lightweight.

Possible content:

- type icon/thumbnail;
- name;
- missing/loading state.

No nested folder system for MVP.

Search can be added if list grows.

---

## 14. Drag to viewport

Desirable:

- drag image asset into viewport to create Image object.

Not required for first technical milestone.

---

## 15. Replacement/relink

Relinking keeps AssetId so all referencing objects update automatically.

This is a reason not to store raw paths directly on every ImageObject.

---

## 16. Cache invalidation

If source file changes externally during session, MVP does not need full file-watch hot reload.

Manual relink/reload is acceptable.

---

## 17. Import jobs

Each job has identity/generation.

If project closes or asset is replaced:

- result can be discarded safely.

Worker never directly mutates Project.

---

## 18. Error handling

Decode failure reports:

- which asset;
- reason category;
- project remains usable.

Raw decoder details go to logs.

---

## 19. Definition of Done

- audio/image import works;
- missing asset recoverable;
- runtime loading does not block editor;
- GPU textures cached;
- paths survive save/reopen;
- stale background results cannot attach incorrectly.
