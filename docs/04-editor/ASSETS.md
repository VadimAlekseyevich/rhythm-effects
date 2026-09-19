# Assets

> **Status: Accepted for MVP**

## 1. Scope

MVP asset kinds:

- one primary Audio asset;
- Image assets;
- system fonts are references, not AssetRecords.

Supported image formats:

- PNG;
- JPEG;
- WebP.

SVG is post-MVP.

## 2. Project vs runtime

Project stores:

- AssetId;
- kind;
- source path reference;
- basic metadata.

Runtime stores:

- decoded image pixels while loading;
- GPU textures;
- prepared audio PCM;
- waveform peaks;
- thumbnails.

Derived runtime data is rebuildable.

## 3. AssetSource path

Schema concept:

~~~rust
AssetSource::File {
    path: String,
    relative_to_project: bool,
}
~~~

If project already has a path:

- asset under project directory tree is stored relative where practical;
- other files remain absolute.

For unsaved project:

- imports begin as absolute paths;
- on first Save/Save As, sources under new project directory may be rewritten to relative references.

The source file is never copied automatically in MVP.

## 4. Resolution on load

For relative path:

~~~text
project directory + stored relative path
~~~

For absolute path:

~~~text
stored absolute path
~~~

Missing source does not invalidate Project.

It creates unresolved runtime state.

## 5. Missing asset UX

Object/project opens.

Missing image/audio is shown clearly.

Actions:

- Relink;
- Locate/reselect file.

Other project content remains editable.

Missing primary audio disables playback/waveform but not visual editing.

## 6. Import

Entry points:

- Import Audio;
- Add Image from File;
- drag-and-drop file into app/viewport where unambiguous.

Import runs probe/decode in background.

Project mutation uses compound commands where creative state changes.

## 7. Duplicate imports

If the same normalized/canonical file path is imported again in the same session/project, reuse existing compatible AssetRecord rather than creating duplicates.

Do not deduplicate by filename alone.

## 8. Image decode/upload

Worker:

~~~text
read file
-> image decode
-> finite dimensions/metadata
-> decoded pixels result
~~~

Main/renderer:

~~~text
validate AssetId generation
-> GPU upload
-> runtime ready
~~~

Worker never mutates renderer/project directly.

## 9. Image limits

Reject absurd dimensions/allocation sizes before decoding/uploading where library metadata allows.

Exact max dimension may follow GPU limits; user gets clear unsupported-image error.

## 10. Audio

One Project AudioTrack references one Audio AssetId.

Replacing primary audio is an explicit project command and invalidates prepared playback/waveform runtime generations.

## 11. Thumbnails

Asset browser image thumbnails are derived runtime/cache data.

They are not required for creative correctness.

Thumbnail failure does not make asset unusable.

## 12. Asset browser

MVP browser is simple:

- name;
- type;
- thumbnail/icon;
- missing state;
- Relink where needed.

No folder/tag/library management.

## 13. Drag image to viewport

Dragging image asset into viewport creates one ImageObject at a sensible composition location.

The object references existing AssetId.

If dragging a new OS file, import + object creation is one compound undoable action.

## 14. Delete asset

Normal delete of an AssetRecord is blocked while referenced.

Deleting an unreferenced asset removes only the project record/runtime cache.

It never deletes the user's source file from disk.

## 15. Relink

Relink keeps AssetId stable.

It updates source reference and increments runtime generation.

Objects continue referencing the same AssetId.

Late decode results from old generation are discarded.

## 16. External file changes

Automatic filesystem watching/reload is post-MVP.

MVP provides explicit Reload/Relink behavior or reloads on project reopen.

This avoids hidden live mutation of creative work.

## 17. Cache invalidation

Runtime caches key at least on:

- AssetId;
- source generation;
- relevant decoding options.

Changing source invalidates decoded/GPU/thumbnail/waveform state.

## 18. Error handling

Unsupported/corrupt file:

- import command fails atomically;
- no half-valid AssetRecord remains unless it is explicitly useful unresolved state;
- user sees readable error;
- raw decoder detail is logged.

## 19. Definition of Done

Assets are MVP-ready when relative/absolute paths, missing state, background import, generation safety, image GPU upload, primary audio replacement, compound image creation, relink, and referenced-delete protection all pass.
