# Renderer

> **Status: Draft**
>
> Renderer converts evaluated scene state into pixels. It uses wgpu and is shared conceptually by realtime preview and offline export.

## 1. Goals

- stable realtime preview;
- predictable GPU resource ownership;
- offscreen composition rendering;
- effect support;
- preview/export parity;
- low CPU↔GPU synchronization;
- simple MVP architecture;
- future portability through wgpu.

---

## 2. Non-goals

MVP renderer is not:

- a 3D engine;
- a generic game engine;
- HDR pipeline;
- node-based compositor;
- arbitrary plugin renderer;
- full vector graphics engine.

---

## 3. Main rendering model

Render composition to an offscreen texture.

~~~text
EvaluatedScene
→ Scene Pass
→ optional Effect Passes
→ Composition Texture
→ editor viewport display
~~~

The application window is not the composition target.

This allows:

- composition resolution independent from window size;
- viewport zoom/pan;
- export reuse;
- effects;
- clean editor chrome separation.

---

## 4. Device ownership

rhythm_engine renderer owns or receives long-lived:

- wgpu Instance;
- Adapter;
- Device;
- Queue;
- render pipelines;
- bind group layouts;
- samplers;
- reusable buffers/textures.

rhythm_app owns window/event integration.

Exact initialization ownership may evolve, but avoid duplicated wgpu devices for normal preview.

---

## 5. Composition target

Target resolution follows project composition dimensions, subject to preview optimization decisions.

Two possible preview modes:

1. full composition resolution;
2. scaled preview target when window/view is much smaller.

MVP can begin with full resolution if performance is acceptable.

Never let viewport UI pixel size redefine composition pixels.

---

## 6. Color pipeline

MVP output is SDR.

Principles:

- sampled sRGB images should use correct sRGB semantics;
- blending/effect math should happen in a documented linear working representation where practical;
- final display/export converts to expected sRGB output;
- do not accidentally perform blur/glow in gamma space.

Exact intermediate formats require a GPU spike.

Preferred approach:

- standard 8-bit targets where sufficient;
- float intermediate targets only where effect quality requires them;
- avoid defaulting every pass to high-bandwidth float textures without measurement.

---

## 7. Alpha

Use one documented alpha convention.

Preferred renderer convention:

- premultiplied alpha for compositing passes if it simplifies consistent blending;
- asset decode may begin straight-alpha but converts at boundary.

Do not mix conventions silently.

---

## 8. Coordinate systems

Project composition must define:

- origin;
- x direction;
- y direction;
- transform units.

Recommended editor-friendly default:

~~~text
origin: top-left
x: right
y: down
~~~

Anchor/transform math can still use object-local centered geometry.

Do not expose GPU clip space to project model.

---

## 9. Transform order

Proposed 2D object transform:

~~~text
local geometry
→ subtract anchor
→ scale
→ rotate
→ translate position
~~~

Exact matrix convention must match viewport gizmos.

---

## 10. Primitive rendering

MVP primitives:

- solid rectangle;
- ellipse;
- textured image quad;
- text;
- optional rounded rectangle.

Prefer batching where simple, but do not build a universal batching framework before profiling.

---

## 11. Rectangle

Use reusable unit quad + transform.

Rounded corners may be fragment-shader based if included.

Avoid rebuilding unique CPU mesh per simple rectangle.

---

## 12. Ellipse

Candidate:

- transformed quad;
- fragment shader ellipse coverage test.

Need test antialiasing at small/large scale.

---

## 13. Images

Runtime asset manager provides GPU texture + metadata.

Renderer handles:

- sampler;
- transform;
- opacity;
- fit/crop if supported;
- alpha/color semantics.

Never reupload unchanged images each frame.

---

## 14. Text

Composition text delegates shaping/layout/raster responsibilities to text subsystem.

Preferred prototype:

- cosmic-text;
- glyphon;
- shared wgpu device.

See TEXT_RENDERING.md.

---

## 15. Render ordering

2D painter order:

~~~text
back
→ first object
→ ...
→ last object
→ front
~~~

Object vector order defines draw order for MVP.

Depth buffer is unnecessary unless a later technique requires it.

---

## 16. Effects

Objects requiring isolated multi-pass effects render through object-local temporary targets.

~~~text
content
→ temporary object target
→ effect chain
→ composite into scene
~~~

Objects without isolation-requiring effects take a direct path.

---

## 17. Temporary texture pool

Do not allocate effect textures every frame.

Pool keyed by:

- dimensions;
- format;
- usage.

Transient resources return to pool after frame/pass lifetime.

---

## 18. Blur

MVP strategy:

~~~text
horizontal blur
→ vertical blur
~~~

Clamp maximum radius.

Large-radius downsample optimization may be added after measurement.

---

## 19. Glow

Basic glow can reuse blur.

~~~text
source
→ optional bright extraction
→ blur
→ composite/add
~~~

Keep parameter set small.

---

## 20. Single-pass effects

Prefer one pass for effects such as:

- tint;
- simple color adjustments;
- RGB split where feasible;
- noise.

---

## 21. Pipeline caching

Create and reuse pipelines.

Never create render pipeline per object/frame.

Embedded WGSL shaders are release default.

Developer hot reload can come later.

---

## 22. Per-frame data

Start straightforward:

- shared quad buffers;
- small uniform buffers;
- explicit draw calls.

If profiling finds CPU submission bottleneck, consider:

- instancing;
- storage buffers;
- batched object classes.

Do not prematurely turn the renderer into a generic ECS/batcher.

---

## 23. Frame lifecycle

Concept:

1. ensure composition target;
2. receive EvaluatedScene;
3. acquire temporary targets;
4. encode scene/effects;
5. submit;
6. make composition texture available to viewport;
7. encode editor UI;
8. present.

No GPU readback in realtime preview.

---

## 24. Viewport overlays

Editor overlays are not composition content.

Examples:

- selection outline;
- gizmos;
- guides;
- anchor marker.

They may be drawn through egui/custom overlay renderer after composition texture display.

They are never exported.

---

## 25. Export

Offline export reuses scene rendering semantics but renders exact frame timestamps.

Likely MVP path:

~~~text
offscreen GPU texture
→ staging buffer readback
→ FFmpeg input
~~~

Zero-copy hardware encoder interop is post-MVP unless necessary.

---

## 26. Surface/device errors

Window surface failure must not affect project state.

Handle:

- minimized zero-size window;
- resize;
- outdated/lost surface;
- recoverable adapter/device errors.

---

## 27. DPI

DPI affects editor rendering and input mapping.

It must not mutate composition resolution or object coordinates.

---

## 28. Performance rules

- no preview readback;
- no unchanged texture reupload;
- no per-frame shader compilation;
- no unnecessary effect pass for disabled effect;
- cull invisible objects where cheap;
- track draw/pass/transient-resource count.

---

## 29. Developer diagnostics

Expose/log:

- GPU adapter;
- backend;
- limits/features used;
- composition resolution;
- object count;
- draw calls;
- render passes;
- temporary texture count;
- frame CPU timing;
- optional GPU timing.

---

## 30. Required spikes

1. winit + wgpu + egui shared integration;
2. offscreen rectangle shown inside egui;
3. hundreds of simple objects;
4. resize/DPI;
5. two-pass blur;
6. text;
7. image alpha/color reference;
8. export-frame readback.

---

## 31. Definition of Done

- all MVP objects render;
- transform semantics match viewport;
- selected effects work;
- text supports target scripts;
- offscreen texture feeds viewport;
- preview avoids GPU readback;
- deterministic export frame path works;
- benchmark targets are met.
