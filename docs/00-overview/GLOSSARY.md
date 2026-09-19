# Glossary

> **Status: Draft**

Canonical terminology for code, UI, and documentation.

## Terms

**Composition** — the visual canvas and its duration/settings.

**Project** — saved user document containing composition data, assets, rhythm settings, and metadata.

**Musical grid** — discrete rhythm positions derived from BPM, offset, meter, and subdivision.

**Grid position** — a valid authoring location for a keyframe.

**Tick** — integer musical-time unit used internally. Exact PPQ is TBD.

**Beat** — musical beat within a bar according to the current meter.

**Subdivision** — division of a beat, such as 1/4, 1/8, 1/16, or triplet subdivision.

**BPM offset** — alignment between musical grid origin and the audio timeline.

**Keyframe** — authored property value located on a musical grid position.

**Evaluation time** — exact time at which animation values are computed for preview/export. It may fall between keyframes.

**Playhead** — current editor playback/scrub position.

**Asset** — imported external resource such as audio, image, or font.

**Layer/Object** — terminology to finalize in PRODUCT_SPEC and PROJECT_MODEL.

Add new terms here before allowing multiple competing names to spread across code and docs.
