# Project Philosophy

> **Status: Accepted**
>
> This document is a project-level constraint. Feature, UX, architecture, and technology decisions should be checked against it.

Rhythm Effects is not intended to become a smaller clone of a traditional motion-design suite. Its purpose is to make rhythm-synchronized visual animation unusually fast, clear, and pleasant.

The project has three primary pillars.

---

## 1. Interface comfort comes first

The editor must remain visually calm and operationally fast even as the feature set grows.

### 1.1. The interface should feel spacious, soft, and legible

We prefer:

- large, comfortable interaction targets;
- readable typography;
- generous spacing;
- clear hierarchy;
- visually soft panels and controls;
- a small, coherent set of type styles;
- obvious primary actions;
- predictable placement of controls.

We avoid:

- tiny text;
- tiny hit targets;
- dense control walls;
- decorative complexity;
- seven unrelated font styles;
- excessive borders and separators;
- deep nested navigation;
- tabs inside tabs inside tabs;
- modal dialogs for routine editing;
- hidden state that the user must memorize.

### 1.2. Complexity belongs in the system, not in the user's head

A powerful feature is acceptable only if it can be exposed without degrading the normal workflow.

When functionality expands, the default workspace must not become proportionally more complicated.

Preferred strategies:

- progressive disclosure;
- context-sensitive controls;
- searchable commands;
- keyboard shortcuts;
- sensible defaults;
- collapsible detail where appropriate;
- one strong primary workflow instead of many competing workflows.

### 1.3. Keyboard-first, not keyboard-only

Frequent actions should have fast hotkeys.

A user should be able to work quickly without constantly moving the pointer across the application.

At the same time:

- every core action should remain discoverable in the UI;
- shortcuts should follow a coherent system;
- focus behavior must be predictable;
- destructive shortcuts must be difficult to trigger accidentally.

### 1.4. Avoid navigation depth

Routine editing should happen in a small number of persistent regions:

- viewport;
- timeline;
- object/layer context;
- inspector/properties;
- transport/rhythm controls.

New features should first try to fit into these concepts before adding another permanent panel, navigation hierarchy, or mode.

### 1.5. UI quality is an MVP feature

Interface design is not a polish phase that happens after the engine.

Prototype and implementation of the editor must repeatedly test:

- readability;
- target sizes;
- panel density;
- shortcut speed;
- number of clicks/actions per common task;
- accidental mode changes;
- discoverability;
- timeline comfort;
- long-session fatigue.

The detailed visual language belongs in [../01-product/DESIGN_SYSTEM.md](../01-product/DESIGN_SYSTEM.md).

---

## 2. Runtime performance and build speed are first-class requirements

Rhythm Effects should feel immediate both to the user and to the developer building it.

### 2.1. Runtime performance

The editor should prioritize:

- low input latency;
- stable realtime preview;
- fast startup;
- fast project loading;
- responsive timeline zoom/pan;
- responsive keyframe manipulation;
- efficient waveform rendering;
- GPU-accelerated visual rendering;
- predictable memory use;
- minimal blocking work on the UI thread.

A feature that makes normal editing feel heavy must justify its cost.

### 2.2. Performance architecture, not late optimization

Performance-sensitive boundaries should be designed from the beginning:

- audio clock;
- animation evaluation;
- visible-range timeline rendering;
- waveform cache;
- GPU resource lifetime;
- effect passes;
- serialization;
- background preprocessing.

We should measure before optimizing, but we should not design obviously expensive hot paths and hope to repair them at the end.

### 2.3. Fast builds and fast iteration

Developer iteration speed is a product constraint because it determines how quickly UX can improve.

Prefer:

- a modest dependency graph;
- intentionally chosen crates;
- few heavyweight compile-time dependencies;
- restrained macro use where compile cost is substantial;
- a small number of crates until separation is justified;
- incremental compilation-friendly boundaries;
- fast unit tests;
- isolated expensive integration tests.

Avoid architecture that is elegant on paper but makes every small UI change expensive to build.

Concrete build-time budgets will be defined in [../06-quality/PERFORMANCE.md](../06-quality/PERFORMANCE.md).

---

## 3. BPM grid + motion design is the product, not a feature

Rhythm Effects is rhythm-native.

Traditional animation tools primarily organize keyframes against continuous time or video frames. Rhythm Effects organizes authored animation events against musical time.

### 3.1. Keyframes live on the musical grid

For MVP, authored keyframes are placed on valid musical grid positions.

Examples:

- bar;
- beat;
- 1/2;
- 1/4;
- 1/8;
- 1/16;
- 1/32;
- triplet subdivisions where enabled.

The exact internal representation is defined by the time model, but the user-facing rule is simple:

**keyframes belong to rhythm positions, not arbitrary floating timestamps.**

Continuous animation still exists between keyframes through interpolation. The restriction applies to authored keyframe positions, not to evaluated motion.

### 3.2. Music is a primary clock domain

The system must explicitly understand:

- BPM;
- offset;
- bars;
- beats;
- subdivisions;
- musical ticks;
- audio sample time;
- video frame time.

Conversion between these domains must be deterministic and centralized.

### 3.3. Rhythm operations should be faster than generic timeline operations

Examples of first-class actions:

- jump one beat;
- jump one subdivision;
- place a keyframe at the current grid point;
- move selected keyframes by N subdivisions;
- duplicate a rhythmic pattern;
- change grid resolution;
- align BPM/offset to the track;
- inspect bar/beat/tick position.

### 3.4. Motion-design primitives remain familiar

Rhythm-first does not mean inventing a strange animation model.

Objects and assets still use familiar properties:

- position;
- scale;
- rotation;
- opacity;
- color;
- effect parameters;
- text/image-specific properties.

They are animated with keyframes and easing. The differentiator is how quickly and precisely those animations are authored against music.

---

## Decision filter

When choosing between two implementations or features, ask in order:

1. Does this preserve or improve interface comfort?
2. Does this preserve runtime responsiveness and development iteration speed?
3. Does this strengthen the rhythm-first motion-design workflow?
4. Is the added complexity justified for MVP?

If a feature fails all three primary pillars, it probably does not belong in the product yet.

---

## North Star interaction loop

The editor should minimize friction in this loop:

~~~text
hear a musical moment
→ locate the rhythmic position
→ place/select a keyframe
→ change a visual property
→ immediately preview the result
→ repeat
~~~

The long-term product advantage should come from making this loop dramatically faster and more pleasant than in general-purpose motion-design software.
