# Curve Editor

> **Status: Accepted for MVP**
>
> Curve Editor edits timing easing only. It is not a full value/speed graph.

## 1. Fast path

Most users should apply:

- Linear;
- Ease In;
- Ease Out;
- Ease In-Out

without opening the custom editor.

## 2. Representation

Custom easing is cubic Bezier timing owned by the outgoing keyframe.

~~~text
(0,0) -> (x1,y1) -> (x2,y2) -> (1,1)
~~~

For MVP:

- x handles constrained to [0,1];
- y handles constrained to [0,1];
- no overshoot easing;
- no spring/bounce procedural curves.

This keeps timing monotonic and property ranges predictable.

## 3. Placement

Curve editor appears contextually within/above the timeline area.

It does not open a detached modal window.

Suggested working height: about 180–220 logical px.

## 4. Selection context

Custom curve editor targets one active outgoing segment at a time.

The contextual panel is shown when exactly one selected keyframe owns an outgoing segment. Selecting a terminal keyframe or keeping a multi-key selection leaves the custom panel hidden; preset easing remains available for multi-selection.

This avoids ambiguous multi-curve handle manipulation without introducing a hidden primary-key state.

## 5. Handles

Two cubic-Bezier control points are draggable.

Pointer coordinates are normalized against the graph and clamped independently to `[0,1]` on both x and y, so a handle cannot leave the accepted MVP unit square even when the pointer moves outside the graph.

Hit areas are larger than visible points.

While dragging, the Curve Editor previews the constrained timing curve and handle values. The released value is then queued as the keyframe interpolation change.

Dragging becomes one history transaction with Escape restore in the next dedicated interaction task.

## 6. Live preview

Changing handles updates scene preview immediately at current playhead.

No playback restart required.

## 7. Hold/Linear

Hold has no editable Bezier handles.

Linear maps to exact diagonal timing and can be converted to custom Bezier representation only when semantically identical/explicit.

## 8. No value graph

MVP does not graph property magnitude, velocity, or spatial path.

Only normalized timing curve is shown.

This prevents graph-editor complexity from dominating the rhythm workflow.

## 9. Keyboard/accessibility

Tab/focus can reach preset controls and numeric handle values if exposed.

Mouse drag is not the sole way to reset/apply presets.

## 10. Performance

Curve UI evaluates only selected segment(s).

No scanning/rendering of all project curves.

## 11. Tests

- preset canonical points;
- handle bounds;
- exact endpoints;
- solver monotonic behavior;
- outgoing-key ownership;
- undo/cancel;
- multi-selection preset application.

## 12. Definition of Done

Preset path is fast, one-segment custom editing is clear, handles are comfortable, edits are undoable, and engine evaluation matches displayed timing curve.
