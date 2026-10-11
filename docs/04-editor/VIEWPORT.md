# Viewport

> **Status: Accepted for MVP**

## 1. Responsibilities

Viewport displays composition and supports:

- object selection;
- pan/zoom;
- move/scale/rotation gizmos;
- selection bounds;
- direct manipulation;
- editor-only overlays.

It does not own creative rendering or Project state.

## 2. Composition display

Viewport displays the renderer's offscreen composition texture.

Outside-composition area is editor chrome/background.

Preview quality may be Auto/Full/Half/Quarter without changing composition-space coordinates.

## 3. Camera

ViewportCamera is EditorSession state:

- pan;
- zoom.

Fit Composition and Frame Selection modify only camera state.

## 4. Zoom

Mouse wheel zooms around pointer location so the composition point under cursor remains approximately stable.

Clamp to practical min/max.

## 5. Pan

Middle-mouse drag pans.

Space is reserved for Play/Pause.

## 6. Selection

Single click selects topmost unlocked visible object whose hit-test passes.

Click empty space clears object selection.

Ctrl+click toggles.

Box selection from empty space selects intersecting visible/unlocked objects.

## 7. Hit testing

Use CPU geometry:

~~~text
pointer screen
-> inverse viewport camera
-> composition point
-> inverse object transform
-> local object test
~~~

No GPU readback picking.

Text uses layout bounds, image uses local rectangle, rectangle/ellipse use geometry.

AI-162 implements `rhythm_core::hit_test::hit_test_rectangle` for a composition-space pointer and an evaluated continuous musical tick. The inverse is: subtract evaluated Position, undo Rotation (degrees), undo non-uniform or negative Scale, then add normalized Anchor × evaluated Rectangle Size. Edge hits are inclusive with a minimal floating-point tolerance. Singular scales, nonpositive sizes, non-finite rotation/tick and non-rectangles do not hit. Visibility/locking and topmost selection remain separate AI-166 responsibilities. Screen-to-composition camera conversion and UI hookup are not implemented by this geometry-only task.

AI-163 adds `rhythm_core::hit_test::hit_test_ellipse`, reusing the exact same inverse object transform as Rectangle. Its local shape is the mathematical ellipse within evaluated Size (center = half Size, radii = half Size) irrespective of Anchor; transformed bounding-box corners must not count as hits. The ellipse curve is inclusive with a minimal normalized floating-point tolerance. Nonpositive Size, zero Scale, invalid rotation/tick and the wrong object kind miss. Viewport camera conversion and selection remain separate work.

## 8. Locked/hidden objects

Hidden objects are not rendered or pointer-selectable.

Locked objects render but are skipped by normal viewport selection/manipulation.

They remain selectable from Object list if product UX allows inspection; direct transform editing stays disabled.

## 9. Multi-selection

Multiple selected objects show one combined bounding context.

MVP transform behavior:

- Move can move all selected objects by one composition-space delta.
- Scale/Rotate multi-selection may be limited to a group bounding transform only if implementation remains predictable.
- If multi-scale/rotate becomes risky, MVP may expose them through inspector rather than shipping inconsistent gizmo behavior.

Single-object gizmos are release-critical; multi-object move is required.

## 10. Move gizmo

Direct drag of selected object/bounds moves Position.

Shift constrains dominant movement axis.

One drag = one history transaction.

Animated Position follows per-property keyframing semantics: at an animated property without a key at current playhead, direct manipulation creates/updates the nearest-grid key and resolves playhead there.

## 11. Scale gizmo

Single object:

- corner handles adjust X/Y scale;
- Shift constrains to uniform scale;
- edge handles adjust one axis.

Scale acts around Anchor.

Negative scale through crossing the anchor may be allowed if interaction remains stable; numeric inspector always supports negative values.

## 12. Rotation gizmo

Rotation handle adjusts rotation around Anchor.

Shift snaps preview/commit to 15-degree increments.

Without Shift, rotation is continuous degrees.

Rotation animation uses the same keyframing rules.

## 13. Anchor

Viewport displays anchor marker for selected object.

MVP edits Anchor primarily through Inspector.

Dragging anchor in viewport is post-MVP because preserving visual placement while changing normalized anchor adds interaction complexity.

## 14. Text editing

Double click TextObject selects/opens text-edit intent.

MVP may focus Inspector text field rather than implementing rich inline canvas editing.

## 15. Image aspect

Image intrinsic bounds come from source pixels.

Scale X/Y can distort intentionally.

Shift-constrained corner scale gives convenient aspect-preserving resize.

## 16. Guides

MVP includes minimal visual helpers:

- composition center axes while moving near center;
- composition boundary.

General rulers/custom guides/snapping are post-MVP.

These guides do not affect BPM/keyframe snapping.

## 17. Safe frame

No broadcast/title-safe overlay required for MVP.

Could be added later as editor-only overlay.

## 18. Performance

Viewport interaction does not trigger:

- image re-decode;
- text reshape for pure transform;
- project serialization;
- GPU readback.

Direct manipulation updates EvaluatedScene/renderer immediately.

## 19. DPI

Pointer/UI logical coordinates convert through viewport camera into composition units.

DPI never changes Project transform values.

## 20. Resize

Viewport panel resize changes camera display region only.

Fit mode may recompute if explicitly active; normal manual camera should not jump unexpectedly.

## 21. Empty state

No objects:

- show composition;
- unobtrusive Add Object hint;
- audio/timeline workflow remains available.

## 22. Tests

- topmost hit selection;
- locked skip;
- rotated hit test;
- negative scale hit inversion;
- center anchor;
- move transaction;
- Shift axis constraint;
- scale/Shift uniform;
- rotation +15 snap;
- animated transform creates grid key;
- DPI/camera round trip.

## 23. Definition of Done

Viewport is MVP-ready when selection, pan/zoom, single-object move/scale/rotation, direct animated edits, hit testing, overlays, and coordinate parity with renderer all pass.
