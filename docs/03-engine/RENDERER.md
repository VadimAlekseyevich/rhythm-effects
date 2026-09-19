# Renderer

> **Status: Accepted for MVP**
>
> Renderer converts EvaluatedScene into pixels. Preview and export use the same creative rendering semantics.

## 1. Goals

- stable 60 FPS target for normal 1080p scenes;
- linear-light compositing;
- predictable resource ownership;
- effects without per-frame allocation storms;
- preview/export parity;
- no GPU readback in normal preview;
- composition independent from editor window/DPI.

## 2. Non-goals

MVP renderer is not 3D, HDR/wide-gamut, a generic vector engine, node compositor, plugin renderer, or video compositor.

## 3. Device ownership

One normal wgpu Device/Queue pair is used by composition rendering, glyphon composition text, and egui rendering.

rhythm_app orchestrates window/surface lifecycle.

rhythm_engine owns renderer resources and creative rendering abstractions.

Do not create one wgpu device per subsystem.

## 4. Render target model

~~~text
EvaluatedScene
-> linear premultiplied working target
-> optional object/effect passes
-> composition texture
-> viewport presentation

Export:
same semantics
-> full-resolution output conversion
-> readback
-> encoder
~~~

The swapchain is never the creative composition itself.

## 5. Working format

Accepted working format:

~~~text
Rgba16Float
~~~

Semantic RGB is linear-light and alpha is premultiplied.

This provides effect headroom and avoids severe 8-bit intermediate banding.

Final display/export converts to standard SDR sRGB-compatible output.

## 6. Asset texture semantics

Ordinary color images use correct sRGB sampling semantics.

~~~text
sRGB source bytes
-> linear sample
-> premultiplied working composition
~~~

Never treat gamma-encoded sRGB bytes as linear values.

## 7. Preview quality

PreviewQuality is EditorSession state:

~~~text
Auto
Full
Half
Quarter
~~~

Default: Auto.

Auto chooses 1.0, 0.5, or 0.25 and keeps large compositions near or below a 1920×1080 working target.

Examples:

~~~text
1920×1080 -> Full
3840×2160 -> Half
7680×4320 -> Quarter
~~~

The user may force Full.

## 8. Preview scaling invariant

Reduced preview resolution never changes project units.

Spatial shader parameters are scaled internally so creative semantics remain stable:

- blur radius;
- glow radius;
- RGB displacement;
- other spatial effects.

Export always uses requested full output resolution.

## 9. Coordinates

Project coordinates follow COORDINATE_SYSTEMS.md.

Renderer owns only conversion into GPU clip/NDC space.

## 10. Transform

~~~text
local geometry
-> subtract anchor
-> scale
-> clockwise user rotation
-> translate Position
~~~

Visible semantics must match viewport/hit testing exactly.

## 11. Primitive paths

MVP:

- rectangle;
- ellipse;
- image quad;
- text.

Use reusable geometry/shader approaches rather than unique CPU mesh creation for every simple object.

## 12. Antialiasing

No global MSAA by default.

Shapes use analytic/fragment edge antialiasing where practical.

Text uses text-stack antialiasing.

Revisit MSAA only if reference testing shows a concrete quality gap.

## 13. Images

Runtime asset manager provides immutable GPU texture and intrinsic dimensions.

Renderer does not reupload unchanged images every frame.

MVP has no crop/fit mode.

## 14. Text

Composition text uses cosmic-text + glyphon on the same device.

See TEXT_RENDERING.md.

## 15. Painter order

Object Vec order is back-to-front painter order.

No creative depth buffer is required.

## 16. Isolation and effects

Objects without isolation-requiring effects can render directly.

Objects with multipass effects render:

~~~text
object content
-> isolated target
-> ordered effect chain
-> composite into scene
~~~

Correct effect ordering is semantic.

## 17. Temporary texture pool

Temporary targets are pooled by:

- dimensions;
- format;
- usage class.

Normal effects do not create/destroy textures every frame.

## 18. Pipelines/shaders

- WGSL packaged with app;
- pipelines cached;
- no per-frame compilation;
- development hot reload is post-MVP.

## 19. Per-frame CPU data

Begin with reusable unit geometry, explicit draws, and reusable uniform/storage buffers.

Do not build a generic batch/ECS system without profiling evidence.

## 20. Frame lifecycle

1. resolve preview scale;
2. ensure composition target;
3. obtain EvaluatedScene;
4. encode direct/isolated objects;
5. run effect passes;
6. finalize composition texture;
7. present in viewport;
8. render editor overlays/UI;
9. submit/present.

No preview readback.

## 21. Editor overlays

Selection, gizmos, guides, anchor markers, and hover outlines are editor overlays and never exported.

## 22. Clipping/culling

Creative output clips to composition bounds.

Objects may exist outside bounds.

Cheap broad culling is allowed; semantic transforms remain unchanged.

## 23. Export

Export uses full-resolution offscreen targets.

A final GPU conversion generates standard SDR RGBA/BGRA encoder input, then staging-buffer readback feeds FFmpeg.

Readback buffering may be pipelined later, but total memory remains bounded.

## 24. Device/surface failure

Handle zero-size/minimized, resize, and recoverable surface loss.

On device loss:

- attempt one renderer reinitialization where practical;
- rebuild runtime resources;
- never mutate Project;
- if recovery fails, surface fatal runtime error while project recovery remains available.

## 25. Diagnostics

Expose:

- adapter/backend;
- preview dimensions/scale;
- objects;
- isolated objects;
- passes/draw calls;
- temporary textures;
- CPU encode time;
- optional GPU timing;
- estimated GPU texture memory.

## 26. Required spikes/tests

- offscreen rectangle in egui;
- sRGB image reference;
- transparent premultiplied edge reference;
- clockwise rotation;
- ellipse quality;
- Rgba16Float support on target adapters;
- Half/Quarter effect-size parity;
- two-pass blur;
- text;
- hundreds of simple objects;
- export readback.

If a supported adapter cannot use the working format, add an explicit tested fallback rather than silently changing semantics.

## 27. Definition of Done

Renderer is MVP-ready when:

- all MVP objects render;
- Rgba16Float linear-premultiplied contract holds;
- preview scale preserves creative units;
- no preview readback occurs;
- temporary resources are reused;
- effects/text match export;
- representative 1080p scene meets PERFORMANCE.md.
