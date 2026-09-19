# Coordinate Systems and Transform Semantics

> **Status: Accepted for MVP**
>
> This document defines composition, object-local, viewport, and GPU coordinate conventions.

## 1. Composition space

Project composition coordinates use:

~~~text
origin: top-left
+X: right
+Y: down
unit: composition pixel
~~~

Objects may exist outside visible composition bounds.

## 2. Object-local geometry

Common object geometry uses local top-left coordinates before transform.

A rectangle of width W and height H occupies:

~~~text
(0,0) -------- (W,0)
  |              |
  |              |
(0,H) -------- (W,H)
~~~

## 3. Anchor

Anchor is stored normalized relative to resolved object bounds.

~~~text
(0.0, 0.0) = top-left
(0.5, 0.5) = center
(1.0, 1.0) = bottom-right
~~~

Default anchor is center.

Values outside 0..1 may be allowed to place pivot outside the object.

Anchor in local pixels is:

~~~text
anchor_px = anchor * bounds_size
~~~

## 4. Position

Position is the composition-space location where the object anchor lands.

A newly created centered object normally has:

~~~text
position = composition center
anchor = (0.5, 0.5)
~~~

## 5. Transform order

For local point p:

~~~text
anchor_px = anchor * bounds_size

p0 = p - anchor_px
p1 = p0 * scale
p2 = rotate(p1, rotation)
p3 = p2 + position
~~~

Therefore:

~~~text
composition_point =
    position
    + Rotation(rotation)
      * (scale * (local_point - anchor_px))
~~~

The same semantics must be used by renderer, viewport hit testing, selection bounds, gizmos, and export.

## 6. Scale

Scale acts around anchor before rotation.

Scale 1.0 means 100 percent.

Negative scale is valid and mirrors around anchor.

Geometry size and transform scale are separate properties.

## 7. Rotation

MVP user/project convention:

- positive rotation appears clockwise on screen;
- persisted unit is degrees;
- values are not normalized.

Renderer/math-library conversions must preserve this visible convention.

## 8. Viewport space

Viewport screen space is editor UI space.

A viewport transform maps:

~~~text
composition space
<-> viewport screen space
~~~

Viewport state includes:

- pan;
- zoom;
- panel origin.

It is editor/session state, never creative Project state.

## 9. DPI

DPI affects editor rendering and input mapping.

It must not alter:

- composition dimensions;
- object positions;
- object scale;
- export dimensions;
- animation values.

Keep composition units, UI logical points, and physical framebuffer pixels conceptually distinct.

## 10. GPU clip space

NDC/clip space is a renderer implementation detail.

Convert at renderer boundary.

Never store GPU clip-space positions in Project.

## 11. Text bounds

Text layout subsystem resolves local bounds.

Anchor applies to those resolved bounds.

If text or font size changes, local bounds may change while composition Position remains fixed.

A center anchor therefore keeps the text centered around Position.

## 12. Image bounds

Image object local bounds are derived from its intrinsic dimensions and explicit fit/size behavior.

Texture dimensions are runtime metadata, but resolved object bounds must be deterministic.

## 13. Viewport hit testing

Preferred MVP path:

~~~text
pointer screen position
-> inverse viewport transform
-> composition position
-> inverse object transform
-> local position
-> object-specific geometric hit test
~~~

Do not use GPU readback for ordinary object picking.

## 14. Selection bounds

Selection and gizmos use the same object transform as rendering.

For rotated objects, broad-phase hit tests may be simpler, but final semantic placement must match renderer geometry.

## 15. Gizmos

Gizmo handles are editor overlays.

Their visual/clickable size is in UI/viewport units so they remain usable at any zoom.

Their location is derived from composition-space object state.

Gizmos are never exported.

## 16. Effect units

Every spatial effect parameter must document its semantic unit.

Preferred MVP convention:

- spatial radii are expressed in composition-pixel semantics.

A reduced preview target must not silently change the apparent blur/glow radius.

## 17. Export

Export uses composition coordinates directly.

It is independent from:

- viewport pan;
- viewport zoom;
- window size;
- display DPI.

## 18. Precision

MVP transform values use finite f32.

This is adequate for ordinary 2D composition dimensions.

Time uses stronger integer/fixed-point semantics because its identity and long-duration requirements differ.

## 19. Required tests

Test at minimum:

- center anchor;
- top-left anchor;
- outside-bounds anchor;
- scale 2;
- negative scale;
- positive 90 degree clockwise rotation;
- transform inverse round trip;
- viewport conversion round trip;
- DPI-independent export coordinates.

Create one visual reference scene with labeled corners and axes.

## 20. Definition of Done

Coordinate semantics are implementation-ready when:

- renderer and viewport agree on object placement;
- transform order is represented once semantically;
- positive rotation direction is tested;
- anchor behavior remains stable under bounds changes;
- hit testing uses inverse transforms;
- UI DPI/view state cannot affect exported composition coordinates.


---

## Accepted geometry policy

CPU geometric hit testing is the MVP picking implementation.

No GPU ID-buffer/readback picking is used.

Spatial effect radii are expressed in composition-pixel semantics and must be corrected for reduced preview resolution so preview scale does not change the apparent creative result.
