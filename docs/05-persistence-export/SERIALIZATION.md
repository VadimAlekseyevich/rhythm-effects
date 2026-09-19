# Serialization

> **Status: Draft**
>
> This document defines the persisted project format, versioning policy, safe-save behavior, migrations, and the boundary between creative data and runtime/editor state.

## 1. Goals

The project format must be:

- deterministic enough for reliable round trips;
- human-inspectable during development;
- versioned from the first public build;
- resilient to interrupted saves;
- independent from GPU/audio/UI runtime objects;
- able to preserve stable IDs;
- able to represent future-compatible unknown additions through explicit migration/version handling rather than accidental parsing;
- simple enough to debug without custom tooling.

The format is a product-safety boundary. A save operation must never knowingly trade data safety for convenience.

---

## 2. What is persisted

Persist creative/project state only.

Examples:

- project metadata;
- schema version;
- composition settings;
- tempo/BPM configuration;
- grid offset;
- time signature;
- object/layer order;
- stable IDs;
- object properties;
- animated properties;
- keyframes;
- interpolation/easing;
- effects;
- asset records and project-facing paths/references.

---

## 3. What is not persisted in the project document

Do not serialize runtime implementation details such as:

- wgpu handles;
- pipelines;
- textures;
- audio stream/device;
- decoded PCM buffers;
- waveform runtime cache;
- glyph atlas;
- UI widget state;
- pointer/drag state;
- undo stack;
- open dialogs/popovers;
- temporary validation state.

Workspace preferences may later be saved separately from the project.

Examples:

- panel sizes;
- theme;
- recent files;
- shortcut overrides.

These should not contaminate the creative document unless intentionally promoted.

---

## 4. Recommended MVP format

Preferred starting choice:

**RON or JSON through serde, with an explicit schema version.**

Selection criteria:

- good serde support;
- readable diffs;
- low implementation cost;
- straightforward error reporting;
- no native dependency;
- reasonable long-term migration path.

### JSON advantages

- universal tooling;
- easy external inspection;
- easy debugging.

### RON advantages

- Rust-friendly;
- less noisy for enums/structured data;
- readable.

The final choice should be captured in an ADR before implementation.

For MVP, simplicity and recoverability matter more than file-size optimization.

Do not introduce a custom binary format before a measured need exists.

---

## 5. File extension

Use a project-specific extension, final name TBD.

Examples:

- `.rhythm`
- `.rfx`
- `.rhfx`

The extension should identify the project as a Rhythm Effects document while the internal serialization may remain text-based.

The extension decision is product-facing and should be made once naming stabilizes.

---

## 6. Root schema

Conceptually:

~~~rust
ProjectFile {
    schema_version,
    application_version,
    project,
}
~~~

The serialized wrapper may include:

- schema version;
- optional minimum compatible app version;
- project payload;
- optional metadata useful for diagnostics.

Do not use application version alone as schema version.

Application releases and file-format changes evolve independently.

---

## 7. Schema version

Use monotonically increasing integer schema versions.

Example:

~~~text
schema_version: 1
schema_version: 2
...
~~~

Rules:

- every breaking serialized-structure change requires a schema decision;
- readers switch behavior based on schema version;
- migrations are explicit;
- no code should silently guess an unknown future schema.

---

## 8. Compatibility policy

### Older project opened by newer app

Supported through migration where practical.

### Newer project opened by older app

Older app must fail safely and clearly.

Message should explain:

- file was created by a newer project schema;
- current app cannot safely open it;
- file remains untouched.

Never partially load and overwrite a newer project unless compatibility is guaranteed.

---

## 9. Migration model

Preferred pattern:

~~~text
raw file
→ parse version wrapper
→ parse version-specific representation
→ migrate step-by-step
→ current Project
→ validate
~~~

Example:

~~~text
V1 → V2 → V3 → Current
~~~

Do not maintain arbitrary direct migrations from every historical version to current.

Sequential migrations are easier to test.

---

## 10. Stable IDs

Persist typed project-local IDs.

Requirements:

- IDs survive save/load;
- loading preserves object/keyframe/effect relationships;
- duplicate/import commands allocate new IDs when semantically creating new entities;
- migrations must not casually regenerate IDs.

Stable IDs are critical for:

- references;
- effects/objects;
- undo while session is live;
- future features such as parenting or linking.

---

## 11. Keyframe time

Persist keyframes in canonical musical time.

Per TIME_MODEL.md:

- store MusicalTick;
- do not serialize derived floating-point seconds as keyframe identity;
- do not rewrite all keyframe timestamps when BPM changes.

This preserves rhythmic intent.

---

## 12. BPM/grid serialization

Persist enough information to reconstruct musical timing exactly:

- BPM/tempo representation;
- grid offset;
- PPQ/schema assumption;
- time signature;
- future tempo segments if schema evolves.

Current editor subdivision may be:

- workspace/session preference;
- or project state.

That decision must be explicit before schema acceptance.

---

## 13. Asset references

MVP should prefer project-facing asset records with paths.

Concept:

~~~rust
AssetRecord {
    id,
    kind,
    source_path,
    metadata,
}
~~~

### Path policy

Prefer relative paths when the asset is inside/near a project-controlled directory and a stable relative representation exists.

Otherwise retain an absolute source path with clear missing-file recovery behavior.

Long-term packed-project behavior is out of MVP scope.

### Missing asset

A missing asset must not make the entire project unparsable.

Load the semantic project and mark the asset unresolved.

---

## 14. Safe save algorithm

Never overwrite the only valid project file in-place.

Preferred flow:

~~~text
serialize current project
→ write sibling temporary file
→ flush write
→ validate/close temporary file
→ replace destination atomically where OS/filesystem allows
→ update backup/recovery policy
~~~

On failure:

- original project remains untouched whenever possible;
- dirty state remains;
- user is informed.

---

## 15. Save As

Save As writes a new destination using the same safe-save procedure.

Only after successful completion should the project session adopt the new canonical path.

If Save As fails, the previous project path remains active.

---

## 16. Validation

After deserialization and migration, validate invariants before project becomes active.

Examples:

- IDs unique;
- referenced IDs exist;
- BPM finite and valid;
- keyframe ticks valid integer values;
- keyframes sorted/normalizable;
- no duplicate keyframes at same property/tick unless migration explicitly resolves;
- composition dimensions/FPS valid;
- effect parameters valid;
- strings/path sizes within sane limits.

Invalid input returns a structured load error.

Do not trust project files simply because they were produced by us.

---

## 17. Partial/corrupt input

Parser failures should include:

- high-level message;
- stage: parse/migration/validation;
- useful internal diagnostic for logs.

Do not mutate the currently open project until the new file is fully parsed, migrated, and validated.

Open should be transactional:

~~~text
load candidate
→ validate
→ only then replace active project
~~~

---

## 18. Deterministic ordering

Where order matters, use explicit ordered collections.

Examples:

- object order;
- keyframe order;
- effect order.

Avoid relying on hash map iteration order in serialized output.

This improves:

- stable diffs;
- reproducible tests;
- debugging.

---

## 19. Floating-point data

Some creative values may remain floats.

Requirements:

- reject NaN and infinity before persistence;
- do not compare serialized floats for semantic identity;
- avoid unnecessary serialization churn;
- use dedicated wrappers for validated values where appropriate.

Musical time identity remains integer-based.

---

## 20. Autosave interaction

Autosave uses the same semantic serialization pipeline but writes to recovery storage, not the main project file.

Autosave must never replace the user’s explicit save unless the recovery policy deliberately promotes it.

See PROJECT_RECOVERY.md.

---

## 21. Project backups

MVP should keep a minimal previous-good-save strategy if inexpensive.

Possible policy:

~~~text
project.rhythm
project.rhythm.bak
~~~

or recovery storage outside the project directory.

Final backup location/rotation belongs to PROJECT_RECOVERY.md.

---

## 22. Security / resource limits

Project files are local but should still be treated as untrusted input.

Set sensible limits or validate against pathological values:

- absurd composition dimensions;
- enormous string lengths;
- extreme counts where allocation could be dangerous;
- invalid path encodings;
- impossible BPM/FPS values.

Do not allow a small malformed file to trigger unbounded allocation.

---

## 23. Performance

Typical project save/load should not require expensive derived cache serialization.

Do not serialize:

- waveform mipmaps;
- decoded images;
- GPU-ready resources.

These are regenerated/cached separately.

For large creative project data, profile before considering binary serialization.

---

## 24. Testing

Required automated tests:

### Round trip

~~~text
Project
→ serialize
→ deserialize
→ semantic equality
~~~

### Stable IDs

All references survive round trip.

### Migration

Fixtures for each historical schema migrate to current expected state.

### Corruption

- truncated file;
- unknown schema;
- invalid enum/value;
- missing referenced ID;
- duplicate IDs;
- invalid BPM;
- invalid dimensions.

### Safe save

Simulate write failure where practical and verify previous project remains valid.

### Determinism

Equivalent project produces stable ordering/output where intended.

---

## 25. Implementation sequence

1. Finalize current Project model.
2. Choose text format via ADR.
3. Add schema wrapper/version.
4. Implement current-version serializer/deserializer.
5. Add validation.
6. Implement safe-save temporary replacement.
7. Add load transaction.
8. Add migration framework with V1 baseline.
9. Add fixture tests.
10. Integrate autosave/recovery.

---

## 26. Open decisions

- JSON vs RON;
- final project extension;
- exact grid-offset persisted integer unit;
- whether current authoring subdivision is project state;
- backup location/rotation;
- path normalization policy;
- app-version metadata semantics;
- when/if packed projects are introduced.

---

## 27. Definition of Done

Serialization is MVP-ready when:

- one basic project round-trips without semantic loss;
- schema version is present;
- safe save cannot intentionally destroy the previous valid file on ordinary write failure;
- older fixture migration framework exists;
- unknown future schema fails safely;
- missing assets do not prevent semantic project load;
- keyframe musical ticks and stable IDs survive exactly;
- load validation rejects corrupt semantic state;
- project files contain no runtime/UI/GPU/audio device state.
