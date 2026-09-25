# Serialization

> **Status: Accepted for MVP**
>
> The .rhfx file is the canonical persisted creative document.

## 1. Format

MVP uses:

- extension: .rhfx;
- encoding: UTF-8;
- payload: versioned JSON through serde/serde_json;
- schema version: integer, independent from app version.

Human readability/debuggability is preferred over compact binary size for MVP.

## 2. Root wrapper

~~~rust
ProjectFileV1 {
    schema_version: 1,
    created_with_version: String,
    project: Project,
}
~~~

The schema wrapper lives in `rhythm_core::serialization`. Its schema version is fixed independently from the application/package version; serde support for the wrapper and all nested semantic types is introduced by the following serialization task.

Unknown future schema versions fail safely without modifying the current/open file.

## 3. Persisted state

Persist creative/project semantics only:

- metadata;
- composition settings;
- tempo/BPM/offset/meter;
- audio reference;
- assets;
- objects/order;
- transforms/content;
- effects/order;
- Animated values/keyframes/easing;
- stable IDs;
- next ID allocator state.

## 4. Not persisted

Not in .rhfx:

- selection;
- playhead;
- timeline/viewport view state;
- undo stack;
- decoded PCM;
- waveform peaks;
- GPU resources;
- thumbnails;
- glyph atlas;
- audio device;
- export process state;
- runtime generation tokens.

## 5. Deterministic structure

Serialized semantic ordering is explicit:

- Vec order for objects/effects;
- keyframes sorted by MusicalTick;
- struct fields through schema types;
- avoid HashMap iteration in persisted user-visible order.

Pretty JSON is acceptable for MVP fixtures/debugging.

Bit-for-bit identical JSON is not a product guarantee, but semantically identical round trips are.

## 6. Numeric rules

Persist:

- BpmMicros;
- MusicalTick;
- ProjectTime/Duration integer units;
- rational FrameRate;
- finite f32 creative values.

Reject NaN/Infinity.

Never serialize presentation strings as semantic numbers.

## 7. Asset paths

Per ASSETS.md/ADR 0020:

- relative when source is under project directory and relative representation is practical;
- otherwise absolute;
- external source is never automatically copied;
- missing source does not invalidate JSON schema.

## 8. Safe explicit save

Save never writes directly over the only known-good file.

Concept:

~~~text
serialize validated Project
-> write sibling temporary file
-> flush/close
-> publish via platform-safe replacement
-> only then mark revision saved
~~~

On Windows, implementation uses a platform replacement operation that preserves the old file until the new temp file is complete.

If replacement fails:

- original stays the known-good canonical file;
- dirty state remains;
- temp is cleaned or retained diagnostically;
- user gets Save failure/Save As option.

## 9. Save As

Save As writes the new destination transactionally.

Only after success:

- session canonical project path changes;
- relative asset paths may be recalculated against new project directory;
- saved_revision updates.

A failed Save As leaves previous project path/state unchanged.

## 10. Load transaction

~~~text
read bytes
-> parse wrapper/version
-> migrate
-> semantic validation
-> resolve nonfatal missing assets
-> construct candidate Project
-> only then replace active Project
~~~

A broken candidate can never half-mutate currently open work.

## 11. Migration

Migration is sequential:

~~~text
V1 -> V2 -> V3 -> current
~~~

Every historical schema that shipped publicly keeps a fixture.

Do not deserialize old versions directly into whatever current structs happen to be.

## 12. Compatibility

Older file in newer app:

- migrate if supported.

Newer file in older app:

- fail read-only/safely;
- explain that a newer app/schema is required;
- never overwrite it.

## 13. Validation limits

Protect against malformed/untrusted local files.

MVP validation limits are intentionally generous:

- project file input: maximum 256 MiB;
- composition dimensions: 16..8192 per axis;
- composition duration: maximum 24 hours;
- individual user text string: maximum 1 MiB UTF-8;
- total object count: maximum 100,000;
- total serialized keyframes: maximum 2,000,000.

These are safety limits, not performance promises.

Renderer/device limits may impose stricter usable dimensions.

## 14. ID validation

- zero is invalid/reserved;
- IDs unique across their type domain;
- references resolve to the correct entity kind;
- next_entity_id is above all allocated IDs;
- duplicate keyframe tick per property is invalid.

Do not silently regenerate stable IDs during ordinary load.

## 15. Missing assets

Missing file is a recoverable asset state.

Project still opens.

Asset record remains semantically present and can be relinked.

## 16. Backups vs recovery

Explicit Save safety and autosave recovery are separate systems.

MVP does not maintain a visible endless version history beside the project.

Recovery manager keeps rolling recovery generations in app data.

## 17. Security

Project files and referenced media are treated as untrusted input.

All parser/decode/resource-allocation paths validate lengths/dimensions before large allocation where practical.

No project load executes scripts/plugins because MVP has none.

## 18. Tests

- V1 round trip;
- stable IDs;
- object/effect/keyframe order;
- relative/absolute paths;
- missing source;
- invalid future schema;
- truncated JSON;
- NaN-like invalid input;
- duplicate IDs;
- duplicate keyframe tick;
- resource limits;
- failed temp write;
- failed publish;
- Save As failure;
- current project survives failed Open.

## 19. Definition of Done

Serialization is MVP-ready when .rhfx V1 fixtures round-trip, transactional Save/Open failure tests pass, unknown schemas are safe, missing assets are recoverable, and no runtime/editor state leaks into the document.
