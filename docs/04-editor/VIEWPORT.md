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

Fit Composition and Frame Selection modify only camera state. **Fit Composition** resets pan and zoom to the centered aspect-correct composition preview; **Frame Selection** centers the union of visible selected Rectangle/Ellipse bounds in the viewport canvas with an 8% margin on each side. Bounds reflect evaluated Position, Rotation, non-uniform/negative Scale, Anchor and the actual ellipse curve (analytic extrema). The existing 0.125–16× zoom limits apply; the frame action does not alter the project, playhead, selection or undo history. Frame Selection is a no-op for empty selection, invisible shapes and unsupported Image/Text without runtime geometry. The UI labels this limitation explicitly; it never invents missing media bounds.

## 4. Zoom

Mouse wheel over the viewport canvas zooms around the pointer (not the panel center): the composition point under the cursor remains stationary to floating-point precision. EditorSession stores a camera scale relative to the fitted preview (1.0 = fit) and a pan in egui logical points. Updating zoom adjusts pan so the image, editor-only wireframe overlays, hit testing and marquee all share the same camera rectangle; zoom never edits project coordinates or transport time.

The interactive scale is clamped to **0.125–16×** the fitted preview. Wheel input is exponential for smooth step sizes, ignored outside the viewport and while a middle-button pan is active. Reset View restores both default zoom and pan without changing object selection. Fit Composition/Frame Selection camera actions are implemented by AI-171; Image/Text layout-aware framing remains deferred until runtime metadata is available.

## 5. Pan

Middle-mouse drag pans the composition image in egui logical pixels, beginning from anywhere in the viewport canvas (including the letterbox background). The camera pan is EditorSession-only state; no Project transform values or timeline/playhead change. The image, CPU hit-testing coordinates, editor outline overlays and marquee share the same shifted composition rectangle. Draw and input are clipped to the viewport canvas. An explicit Reset Pan action recenters the fitted composition even when it is fully outside the panel.

Viewport wheel zoom and Fit/Frame camera controls are implemented; DPI-specific camera controls remain future work.

Space is reserved for Play/Pause.

## 6. Selection

Single click selects topmost unlocked visible object whose hit-test passes.

Click empty space clears object selection.

Ctrl+click toggles individual object IDs, preserving all other selected objects. Ctrl+click on empty composition leaves selection unchanged; a plain click on empty composition clears it. Multiple selected objects are outlined simultaneously. The Inspector currently shows the most recently selected object as primary, but mixed-value editing remains separate work.

Box selection from empty space selects intersecting visible/unlocked objects. A drag starts only when the initial point misses every selectable object; beginning a drag on an object is reserved for its move gizmo. The displayed marquee is clipped to the fitted composition texture; pointer motion beyond its edges clamps to composition bounds. An ordinary marquee replaces the selection, and Ctrl-marquee toggles intersected objects in painter order. A zero-area marquee yields no hits, without fabricating a rectangle.

AI-168 uses pure CPU affine-aware intersection: transformed Rectangle and Image/Text rectangles use convex polygon overlap, and Ellipse uses the exact ellipse/segment intersection rather than a bounding-box shortcut or a sample polygon. Image/Text remain ineligible in the current UI until their runtime bounds caches are connected; absent metadata never means a guessed hit. Both the preview overlay and the hit test use the same composition coordinate mapping.

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

AI-164 adds `rhythm_core::hit_test::hit_test_image`. Image local bounds are the **source's decoded intrinsic pixel dimensions**; the semantic `ImageObject` still stores only `AssetId`. The caller supplies optional prepared metadata `(AssetId, [width_px, height_px])`; missing, stale (mismatched AssetId) or zero-size dimensions must miss instead of guessing a fallback box or performing synchronous file decode. The same CPU inverse Position/Rotation/Scale/Anchor helper used by Rectangle and Ellipse is reused for image rectangular bounds, including mirrored or nonuniform transforms. No project-schema change, disk access, GPU readback or viewport integration is introduced by this task.

AI-165 adds `rhythm_core::hit_test::hit_test_text` with optional `TextLayoutBounds { min, max }` measured by the composition text shaper. A text's local picking box has the measured extent `max - min`, positioned by its evaluated normalized Anchor; nonzero/negative local bounds origins from shaping/alignment do not shift the anchor a second time. Neither text length nor font size is used as an approximate width. Missing or degenerate layout misses, and the caller must supply bounds from the same up-to-date cache entry used by the renderer (invalidated on text/font/size/alignment changes); transforms and colors do not reshape text. Text shaping, caching, renderer parity and UI selection remain unimplemented by this *hook* task.

AI-166 connects CPU hit testing to a real click on the fitted egui composition image: cursor coordinates are converted using the exact image rectangle (not the viewport panel), and objects are checked **from last drawn to first** in painter order. Hidden and locked objects are excluded from pointer selection; an empty click clears the single-object selection stored in EditorSession. An Object list and a minimal undoable Add Rectangle action make this interaction exercisable in an empty project. In projects without a tempo map, the static tick-zero pose is used for this provisional editor-only view; no audio transport is implied.

**Rendering gap remains visible:** the composition GPU renderer still clears to black. Until GPU scene rendering is implemented, the app displays explicit *editor-only wireframe overlays* for Rectangle/Ellipse, calculated from the same evaluated geometry as picking. These outlines are not creative output or export content. Image and Text picking is supported by core only when prepared intrinsic-size and shaped-layout metadata are supplied; no cache exists in the UI yet, so the viewport deliberately passes None and does not fabricate clickable media bounds. Ctrl-click multi-selection, pan/zoom, real object GPU rendering, and Inspector property editing remain separate tasks.

### Selection bounds and pivot overlays (AI-172)

Selected, visible Rectangle/Ellipse objects show separate editor-only **axis-aligned selection bounds** on top of their existing transformed wireframes. Bounds derive from evaluated continuous-tick geometry: Rectangle uses transformed corners; Ellipse uses analytic curve extrema, so rotated and negatively scaled objects cannot be incorrectly framed by a local, unrotated box. Each selectable outline gets a fixed-screen-size crosshair/ring at the actual world **Position** of its normalized Anchor. With a noncentral Anchor, that pivot **differs from the geometric center**; the marker follows evaluated position animation and is not draggable yet. When more than one selected shape has usable geometry, a second group bounds rectangle surrounds their union.

All bounds and pivot coordinates pass through the same fitted/panned/zoomed preview rectangle as hit-testing and box selection. Overlays are clipped to the viewport canvas and never enter the GPU composition texture, project serialization or video export. Invisible objects, invalid geometry and Image/Text without prepared runtime bounds are omitted rather than assigned synthetic bounds. Locked objects can display their selection overlays when selected through the Objects list, but are not picked by pointer.

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
