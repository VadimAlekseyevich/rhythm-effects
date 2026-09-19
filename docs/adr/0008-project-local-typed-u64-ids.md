# ADR 0008 — Use Typed Project-Local u64 Entity IDs

> **Status: Accepted for MVP**
>
> **Date: 2026-09-19**

## Context

Objects, assets, effects, and keyframes need stable identity across selection, undo/redo, serialization, background job correlation, duplication, and future references.

Globally unique UUIDs are convenient but add dependency and representation cost that MVP does not currently need.

## Decision

Use distinct typed wrappers around project-local u64 IDs.

Examples:

~~~rust
ObjectId(u64)
AssetId(u64)
EffectId(u64)
KeyframeId(u64)
~~~

Project maintains monotonically increasing allocation state.

Deleted IDs are not intentionally reused within a project lifetime.

Copying an entity into a new semantic identity allocates fresh IDs.

## Consequences

Positive:

- compact serialization;
- cheap hashing/comparison;
- compile-time separation between ID kinds;
- deterministic fixtures;
- no UUID dependency.

Negative:

- IDs are not globally unique across independent projects;
- cross-project import must remap IDs;
- allocator state must persist correctly.

## Revisit condition

Adopt globally unique IDs if collaboration, distributed references, external plugin references, or merging projects makes project-local identity insufficient.
