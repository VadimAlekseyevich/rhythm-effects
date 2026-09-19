# Project Model

> **Status: Draft**
>
> This document defines persisted creative data and the boundary between project state and runtime/editor state.

## 1. Principles

The project model should be:

- stable;
- easy to serialize;
- deterministic enough to test;
- independent from egui/wgpu/CPAL;
- explicit about IDs;
- simple enough to evolve;
- optimized for correctness before premature database-like complexity.

---

## 2. Top-level structure

Conceptual MVP model:

~~~rust
Project {
    schema_version,
    metadata,
    settings,
    tempo_map,
    audio_track,
    assets,
    composition,
    next_entity_id,
}
~~~

---

## 3. Project metadata

Examples:

~~~rust
ProjectMetadata {
    name,
    created_with_version,
}
~~~

Avoid persisting transient values such as last save timestamp unless a concrete feature needs them.

---

## 4. Schema version

Every project contains a format/schema version.

~~~rust
ProjectSchemaVersion(u32)
~~~

Load path:

~~~text
read
→ identify version
→ migrate if supported
→ validate
→ construct current Project
~~~

Do not deserialize old files directly into whatever the current structs happen to be without migration policy.

---

## 5. Typed project-local IDs

MVP uses project-local integer IDs.

Concept:

~~~rust
ObjectId(u64)
AssetId(u64)
EffectId(u64)
KeyframeId(u64)
~~~

Benefits:

- compact;
- deterministic;
- fast;
- easy serialization;
- avoids UUID dependency.

IDs are never reused within the same project's lifetime after deletion if practical.

Project stores the next allocation counter.

When copying/importing entities from another context, allocate new IDs.

---

## 6. Project settings

~~~rust
ProjectSettings {
    composition_width,
    composition_height,
    frame_rate,
    duration,
    background,
}
~~~

Use validated domain types rather than arbitrary integers/floats where correctness matters.

Example:

- positive dimensions;
- supported rational frame rate;
- non-negative duration.

---

## 7. Composition

MVP has one composition.

~~~rust
Composition {
    objects: Vec<Object>,
}
~~~

Vector order is draw/layer order unless later separated.

Do not introduce nested compositions yet.

---

## 8. Object

Prefer one common object wrapper with typed content.

~~~rust
Object {
    id: ObjectId,
    name: String,
    enabled: bool,
    locked: bool,
    transform: TransformAnimation,
    content: ObjectContent,
    effects: Vec<Effect>,
}
~~~

Potential content enum:

~~~rust
enum ObjectContent {
    Rectangle(RectangleObject),
    Ellipse(EllipseObject),
    Image(ImageObject),
    Text(TextObject),
}
~~~

This keeps common transform/effects uniform.

---

## 9. Transform

~~~rust
TransformAnimation {
    position: Animated<Vec2>,
    scale: Animated<Vec2>,
    rotation: Animated<f32>,
    anchor: Animated<Vec2>,
    opacity: Animated<f32>,
}
~~~

Units must be documented:

- position: composition logical/pixel units;
- rotation: choose degrees for persisted/user model or radians internally, but do not mix;
- opacity: normalized 0..1 internally is recommended;
- scale: explicit convention, for example 1.0 = 100%.

The inspector may display different friendly units.

---

## 10. Animated<T>

Concept:

~~~rust
Animated<T> {
    base_value: T,
    keyframes: Vec<Keyframe<T>>,
}
~~~

Invariant:

- keyframes sorted by MusicalTick;
- no duplicate tick for same property;
- every keyframe owns stable KeyframeId;
- base_value is used when there are no keyframes and for defined before/after behavior.

The exact before-first/after-last evaluation policy is specified in ANIMATION_ENGINE.md.

---

## 11. Rectangle

~~~rust
RectangleObject {
    size: Animated<Vec2>,
    fill: Animated<Color>,
    corner_radius: Option<Animated<f32>>,
}
~~~

Corner radius may be dropped from MVP without changing overall model.

---

## 12. Ellipse

~~~rust
EllipseObject {
    size: Animated<Vec2>,
    fill: Animated<Color>,
}
~~~

---

## 13. Image object

~~~rust
ImageObject {
    asset: AssetId,
    size_or_fit_settings,
}
~~~

The actual decoded texture is runtime state and is not embedded here.

If the asset is missing, the object still exists and can be repaired.

---

## 14. Text object

~~~rust
TextObject {
    text: String,
    font: FontReference,
    font_size: Animated<f32> or static MVP value,
    color: Animated<Color>,
    alignment,
}
~~~

Not every text property must be animatable in MVP.

Only properties explicitly supported by product scope receive Animated<T>.

---

## 15. Asset registry

~~~rust
AssetRegistry {
    assets: Vec<AssetRecord>,
}
~~~

~~~rust
AssetRecord {
    id: AssetId,
    kind: AssetKind,
    source: AssetSource,
    metadata,
}
~~~

Asset kinds:

- Audio;
- Image;
- Font;
- future SVG if accepted.

Persist references, not runtime decoded bytes, unless a later packed-project format intentionally changes this.

---

## 16. Primary audio track

MVP has one primary track.

~~~rust
AudioTrack {
    asset_id: AssetId,
    gain: f32, // optional, default 1
}
~~~

Playback cursor is editor/runtime state and is not persisted as creative data unless "last position" is later treated as workspace state.

---

## 17. Tempo map

Project contains:

~~~rust
TempoMap {
    ppq,
    grid_offset,
    segments,
}
~~~

MVP normally contains one segment.

Current editor subdivision is likely workspace/editor state, not creative project data, unless the product explicitly wants reopening to preserve it.

BPM itself is project data.

---

## 18. Effects

~~~rust
Effect {
    id: EffectId,
    enabled: bool,
    kind: EffectKind,
}
~~~

Typed variants are preferred for MVP:

~~~rust
enum EffectKind {
    Blur(BlurEffect),
    Glow(GlowEffect),
    Tint(TintEffect),
    Noise(NoiseEffect),
    RgbSplit(RgbSplitEffect),
}
~~~

Avoid a generic stringly-typed plugin parameter map before plugin support exists.

Effect parameters use Animated<T> where meaningful.

---

## 19. Color

Define one canonical persisted color model for MVP.

Recommended initial internal project representation:

~~~rust
Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}
~~~

with documented color-space semantics.

Do not mix editor UI color types into core.

Advanced color management/HDR is outside MVP.

---

## 20. Ordering

Object vector order defines drawing order.

Effect vector order defines effect-stack order.

Keyframe vector order is sorted by tick.

Do not use hash-map iteration order for user-visible ordering.

---

## 21. Runtime indexes

Persisted Vec structures may have runtime acceleration indexes.

Examples:

~~~text
ObjectId -> object index
AssetId  -> asset index
Keyframe tick -> binary search in sorted vec
~~~

Indexes are rebuilt after load and updated on mutation.

They are not serialized.

Start simple; add indexes only where useful.

---

## 22. Editor session is separate

Examples not stored in Project by default:

- current selection;
- playhead;
- timeline scroll/zoom;
- viewport pan/zoom;
- open/collapsed rows;
- focused property;
- hover state;
- drag state;
- undo stack.

Possible separate type:

~~~rust
EditorSession {
    selection,
    playhead,
    workspace,
    interaction,
    history,
}
~~~

---

## 23. App preferences are separate

Examples:

- theme;
- recent files;
- audio device preference;
- shortcut remaps later;
- UI scale preference.

These belong in application settings, not project serialization.

---

## 24. Validation

After load and after risky operations, project invariants can be validated in debug/tests.

Examples:

- IDs unique;
- asset references resolvable or explicitly missing;
- dimensions valid;
- BPM valid;
- keyframes sorted;
- no duplicate keyframe ticks per property;
- opacity ranges valid if clamped by model;
- no NaN/Infinity in persisted numeric values.

Do not permit invalid floats to silently enter project state.

---

## 25. Deletion semantics

Deleting object:

- removes object and contained effects/keyframes;
- asset records are not necessarily deleted automatically;
- undo stores enough object data to restore exact state.

Deleting asset:

- should be restricted if actively referenced, or produce explicit missing reference behavior;
- final UX defined in ASSETS.md.

---

## 26. Duplication semantics

Duplicating object:

- allocate new ObjectId;
- allocate new EffectId values;
- allocate new KeyframeId values;
- preserve values/ticks/easing;
- reference same assets;
- choose deterministic new name.

Duplicating keyframes:

- allocate new KeyframeIds;
- preserve relative musical offsets.

---

## 27. Serialization boundary

Persist only semantic user data.

Do not persist:

- GPU texture IDs;
- egui IDs;
- raw pointers/handles;
- audio device names unless as preference;
- decoder internals;
- waveform cache bytes in the core project structure;
- computed animation values.

Cache sidecars may exist separately.

---

## 28. Forward evolution

Design for adding later:

- additional object types;
- parenting;
- masks;
- multiple compositions;
- richer effects;
- tempo changes;
- packed assets.

Do not implement placeholder fields for speculative features.

Use schema migration when real features arrive.

---

## 29. Required tests

- default project validates;
- ID allocation never collides;
- duplicate object gets fully fresh entity IDs;
- project serialization round-trip preserves semantic equality;
- missing asset reference produces recoverable state;
- keyframes remain sorted after command operations;
- invalid NaN/Infinity rejected;
- old schema migration tests once version 2 exists.

---

## 30. Open decisions

- exact project file format;
- exact composition duration type;
- whether font files become Asset records or font descriptors;
- whether object ordering remains direct Vec order after parenting exists;
- final Color space convention;
- which text/effect properties are animatable in MVP;
- whether editor grid subdivision is project or workspace state.
