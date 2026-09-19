# Glossary

> **Status: Accepted for MVP**

Canonical terminology for product UI, documentation, and code.

## Project and composition

**Project** — the saved creative document represented by a .rhfx file.

**Composition** — the single MVP visual canvas with resolution, frame rate, duration, background, and ordered visual objects.

**Object** — a visual composition entity: Rectangle, Ellipse, Image, or Text. Use "object" as the primary product/code term; "layer" may describe painter ordering informally but is not a separate entity type.

**Asset** — an external source file referenced by the Project, currently Audio or Image. System fonts are FontReferences, not AssetRecords in MVP.

## Time

**Project time** — continuous composition time represented canonically as ProjectTimeNs.

**Musical time** — rhythm-domain time represented by integer MusicalTick positions.

**Tick** — one unit of musical time. MVP uses PPQ = 960, therefore one quarter-note beat equals 960 ticks.

**PPQ** — pulses per quarter note. Fixed at 960 for schema V1.

**Beat** — quarter-note beat used by the MVP authoring grid/tempo model. With PPQ 960, one beat = 960 ticks.

**Bar** — group of beats according to TimeSignature. MVP UI defaults to 4/4.

**Beat division / authoring grid** — equal subdivisions of one beat used when creating/moving keyframes, e.g. 1/4 means four authoring steps per beat.

**Grid position** — a valid position on the currently selected authoring grid.

**Grid offset** — ProjectTimeNs at MusicalTick(0), used to align the musical grid to audio.

**BPM** — tempo stored canonically as BpmMicros fixed-point value.

**Playhead** — continuous editor ProjectTimeNs position. Unlike authored keyframes, it may sit between grid points.

**Audio frame** — one sample per channel at an audio sample rate. AudioFramePosition counts frames, not individual interleaved channel samples.

**Video frame index** — zero-based output/preview frame identity resolved through rational FrameRate.

## Animation

**Animated<T>** — a property containing a static base_value and zero or more musical-time keyframes.

**Keyframe** — an authored property value with stable KeyframeId and MusicalTick position.

**Base value** — value used while a property has no keyframes.

**Interpolation** — rule owned by an outgoing keyframe for the segment to the following keyframe: Hold, Linear, or CubicBezier.

**Easing** — timing remapping of normalized segment progress, implemented as canonical cubic Bezier timing.

**EvaluatedScene** — transient, non-persisted scene state produced from Project + ProjectTimeNs for renderer consumption.

## Editor state

**EditorSession** — non-project editing context: selection, playhead while paused, view state, history, current BeatDivision, interactions.

**Selection** — editor references to stable entity IDs; it is not persisted creative data.

**Transaction** — begin/update/commit-or-cancel lifecycle for a continuous edit such as drag or numeric scrub. One committed transaction produces one logical history entry.

**Command Search** — Ctrl+K searchable action surface used to keep infrequent functionality out of permanent UI.

## Rendering

**Composition space** — top-left-origin, +X right, +Y down, composition-pixel coordinate system.

**Anchor** — normalized pivot relative to object bounds, default (0.5, 0.5).

**Working target** — Rgba16Float linear-light premultiplied render texture used for creative composition/effects.

**Preview quality** — EditorSession render scale: Auto, Full, Half, Quarter. It changes performance/resolution, not creative units.

## Persistence/runtime

**Schema version** — integer .rhfx project-format version independent from application version.

**Runtime asset** — decoded/uploaded representation of an AssetRecord, rebuildable and not persisted in .rhfx.

**Cache** — derived data that can be deleted/rebuilt without loss of creative work.

**Recovery** — rolling autosave state used after abnormal exit; it does not silently replace explicit Save.

## Usage rule

When introducing a new core term, define it here and reuse it consistently. Do not create competing synonyms in UI/code unless there is a distinct semantic concept.
