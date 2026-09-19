# Project Model

> **Status: Draft — core schema contract accepted; final MVP field cut still open in a few places**
>
> This document defines persisted creative data and its boundary from editor-session, runtime, cache, and device state.

## 1. Principles

Project data must be:

- serializable;
- deterministic enough for fixtures and tests;
- independent from egui, wgpu, CPAL, and FFmpeg;
- explicit about units and IDs;
- validatable;
- evolvable through schema migration;
- free from runtime handles and derived caches.

Canonical supporting documents:

- DOMAIN_TYPES.md
- TIME_MODEL.md
- COORDINATE_SYSTEMS.md
- STATE_OWNERSHIP.md
- SERIALIZATION.md

## 2. Schema wrapper

Schema V1 uses versioned JSON.

Concept:

~~~rust
ProjectFileV1 {
    schema_version: u32,
    created_with_version: String,
    project: Project,
}
~~~

Rules:

- schema_version is independent from application version;
- unknown newer schema fails safely;
- migration happens before the candidate Project becomes active;
- malformed/corrupt input never partially replaces the current Project.

## 3. Project root

Conceptual MVP structure:

~~~rust
Project {
    metadata: ProjectMetadata,
    settings: ProjectSettings,
    tempo_map: TempoMap,
    audio_track: Option<AudioTrack>,
    assets: Vec<AssetRecord>,
    composition: Composition,
    next_entity_id: u64,
}
~~~

A valid project may exist before audio is imported.

## 4. Metadata

~~~rust
ProjectMetadata {
    name: String,
}
~~~

Do not persist transient editor timestamps or session values unless product behavior explicitly requires them.

## 5. IDs

Typed project-local u64 IDs are used:

~~~rust
ObjectId
AssetId
EffectId
KeyframeId
~~~

MVP may use one shared monotonically increasing next_entity_id counter.

Rules:

- reserve zero as invalid;
- never intentionally reuse IDs;
- duplication allocates fresh IDs;
- load validates uniqueness;
- next_entity_id must be greater than all allocated IDs;
- cross-project copy/import remaps identity.

## 6. Project settings

~~~rust
ProjectSettings {
    composition_width: u32,
    composition_height: u32,
    frame_rate: FrameRate,
    duration: DurationNs,
    background: LinearRgba,
}
~~~

Requirements:

- dimensions positive and within supported limits;
- rational frame rate valid;
- duration valid;
- background finite.

Product/export UI owns preset choices; core model owns validity.

## 7. Composition

MVP has one composition:

~~~rust
Composition {
    objects: Vec<Object>,
}
~~~

Object vector order is painter/draw order.

Nested compositions are post-MVP.

## 8. Object

~~~rust
Object {
    id: ObjectId,
    name: String,
    visible: bool,
    locked: bool,
    transform: TransformAnimation,
    content: ObjectContent,
    effects: Vec<Effect>,
}
~~~

Semantics:

- visible affects render/export;
- locked is persisted authoring metadata;
- name is authoring metadata;
- effect order is significant.

## 9. ObjectContent

MVP content variants:

~~~rust
enum ObjectContent {
    Rectangle(RectangleObject),
    Ellipse(EllipseObject),
    Image(ImageObject),
    Text(TextObject),
}
~~~

SVG is not part of schema V1 unless explicitly promoted before schema freeze.

Do not include speculative empty variants for post-MVP features.

## 10. TransformAnimation

~~~rust
TransformAnimation {
    position: Animated<Vec2>,
    scale: Animated<Vec2>,
    rotation_degrees: Animated<f32>,
    anchor: Animated<Vec2>,
    opacity: Animated<f32>,
}
~~~

Semantics come from DOMAIN_TYPES.md and COORDINATE_SYSTEMS.md.

Recommended defaults:

~~~text
position = composition center
scale = (1,1)
rotation = 0
anchor = (0.5,0.5)
opacity = 1
~~~

## 11. Animated<T>

~~~rust
Animated<T> {
    base_value: T,
    keyframes: Vec<Keyframe<T>>,
}
~~~

Invariants:

- keyframes sorted by MusicalTick;
- one keyframe per tick;
- stable KeyframeId;
- finite/valid values;
- evaluation semantics defined by ANIMATION_ENGINE.md.

## 12. Rectangle

Recommended schema V1:

~~~rust
RectangleObject {
    size: Animated<Vec2>,
    fill: Animated<LinearRgba>,
    corner_radius: Animated<f32>,
}
~~~

If corner radius is removed from MVP UI before implementation, omit it from V1 instead of shipping unused schema.

## 13. Ellipse

~~~rust
EllipseObject {
    size: Animated<Vec2>,
    fill: Animated<LinearRgba>,
}
~~~

## 14. Image

Minimal MVP:

~~~rust
ImageObject {
    asset: AssetId,
}
~~~

Intrinsic image dimensions define base local bounds.

Normal resizing can use object transform scale.

If explicit crop/fit/size behavior is required by UX, add the smallest accepted field set before schema freeze.

GPU textures and decoded pixels are runtime state only.

## 15. Text

Recommended MVP:

~~~rust
TextObject {
    text: String,
    font: FontReference,
    font_size: f32,
    color: Animated<LinearRgba>,
    alignment: TextAlignment,
}
~~~

Font size may remain static for MVP because transform scale already animates visual size.

If prototype testing proves direct font-size animation essential, promote it to Animated<f32> before schema freeze.

## 16. FontReference

Concept:

~~~rust
enum FontReference {
    System(FontDescriptor),
    Asset(AssetId),
}
~~~

MVP may start with System only if imported fonts are deferred.

Missing system font must produce recoverable unresolved/fallback state.

Exact FontDescriptor fields depend on text-stack prototype and remain one of the final schema-open items.

## 17. Assets

~~~rust
AssetRecord {
    id: AssetId,
    kind: AssetKind,
    source: AssetSource,
    metadata: AssetMetadata,
}
~~~

MVP asset kinds:

- Audio;
- Image;
- optional Font if imported fonts are promoted.

Persist source/reference information, not decoded runtime bytes.

## 18. Audio track

MVP supports zero or one primary audio track:

~~~rust
AudioTrack {
    asset_id: AssetId,
    gain: f32,
}
~~~

Gain defaults to 1.0.

Playback cursor is runtime/session state and never part of Project.

## 19. TempoMap

Concept:

~~~rust
TempoMap {
    grid_offset: GridOffsetNs,
    segments: Vec<TempoSegment>,
}
~~~

MVP normally contains one segment:

~~~rust
TempoSegment {
    start_tick: MusicalTick,
    bpm: BpmMicros,
    meter: TimeSignature,
}
~~~

Initial segment starts at tick 0.

PPQ is schema/core semantics and remains 960.

Tempo-change editing UI is post-MVP.

## 20. Current beat division

Current authoring BeatDivision is EditorSession/workspace state, not creative Project state.

Changing 1/4 to 1/16 does not change project semantics.

It may later be saved in workspace preferences.

## 21. Effects

~~~rust
Effect {
    id: EffectId,
    enabled: bool,
    kind: EffectKind,
}
~~~

Typed effect variants only.

Candidate MVP set:

~~~rust
enum EffectKind {
    Blur(BlurEffect),
    Glow(GlowEffect),
    Tint(TintEffect),
    Noise(NoiseEffect),
    RgbSplit(RgbSplitEffect),
}
~~~

The final list may shrink before schema freeze.

Effect parameters use Animated<T> only where the product exposes animation.

Do not build a generic string parameter map before plugin support exists.

## 22. Color

Every project color uses LinearRgba semantic values.

UI converts user-facing sRGB values at the boundary.

Do not persist egui/wgpu-specific color types.

## 23. Ordering

Explicit Vec order defines:

- object draw order;
- effect stack order;
- keyframe sorted order.

No user-visible order depends on hash-map iteration.

## 24. Runtime indexes

Runtime may derive indexes such as:

~~~text
ObjectId -> object index
AssetId -> asset index
EffectId -> owner/index
~~~

Indexes are:

- rebuildable;
- not serialized;
- updated by mutation layer.

Start without an index if simple linear lookup is sufficient.

## 25. Editor/session separation

Not part of Project:

- selection;
- playhead;
- timeline zoom/scroll;
- viewport pan/zoom;
- focused panel/property;
- hover;
- active drag;
- undo stack;
- current beat division.

## 26. App preferences separation

Not part of Project:

- theme;
- UI scale;
- preferred audio device;
- recent files;
- shortcut remaps.

## 27. Deletion

Deleting an Object:

- removes contained effects and keyframes;
- does not automatically remove asset records;
- undo stores enough object subtree data for exact restore.

Deleting an AssetRecord:

- should be blocked while semantically referenced, or require explicit handling;
- normal commands must not silently create dangling AssetIds.

A missing external file is different from deleting the AssetRecord: the record remains valid but unresolved.

## 28. Duplication

Duplicating Object:

- allocate fresh ObjectId;
- fresh EffectIds;
- fresh KeyframeIds;
- preserve asset references;
- preserve values/ticks/easing;
- choose deterministic copy name.

Duplicating a keyframe pattern:

- allocate fresh KeyframeIds;
- preserve relative MusicalTick offsets.

## 29. Validation

On load and controlled debug/test boundaries validate:

- IDs nonzero if zero reserved;
- all IDs unique;
- next_entity_id greater than allocated IDs;
- dimensions valid;
- frame rate valid;
- duration valid;
- persisted floats finite;
- opacity within [0,1];
- BPM valid;
- asset references point to records;
- asset kind matches semantic reference;
- keyframes sorted and unique;
- object/effect variant payload valid.

Missing external source file is recoverable runtime state, not invalid schema.

## 30. Serialization boundary

Persist semantic project data only.

Never persist:

- GPU handles;
- egui IDs;
- decoder state;
- PCM buffers;
- waveform mip data in main JSON;
- glyph atlas;
- selection;
- undo stack;
- EvaluatedScene;
- audio device handles;
- FFmpeg process state.

## 31. Forward evolution

Likely later additions:

- parenting;
- masks;
- additional object types;
- multiple compositions;
- packed assets;
- richer text;
- tempo changes.

Add them through real schema migrations when they exist.

Do not reserve speculative placeholder fields.

## 32. Schema V1 freeze gate

Before schema V1 becomes Implemented, resolve only these remaining field-cut decisions:

1. exact FontReference shape after text spike;
2. final MVP EffectKind list;
3. whether ImageObject needs explicit size/crop/fit data;
4. final project file extension.

Core schema architecture outside those details is accepted.

## 33. Required tests

- default project validates;
- ID allocator never collides/reuses;
- object duplication refreshes nested IDs;
- JSON round trip preserves semantic equality;
- missing file leaves AssetRecord valid;
- keyframe order/uniqueness survives commands;
- invalid NaN/Infinity/ranges rejected;
- next_entity_id invalid state rejected or explicitly repaired;
- current beat division absent from creative schema.

## 34. Definition of Done

Project schema is ready to freeze when:

- four remaining field-cut decisions are resolved;
- every persisted field has an explicit unit and owner;
- no runtime/cache state leaks into JSON;
- V1 fixture round-trips;
- load validation covers all invariants;
- duplication/deletion behavior is tested.
