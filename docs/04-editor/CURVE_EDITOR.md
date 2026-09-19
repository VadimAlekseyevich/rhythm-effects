# Curve Editor

> **Status: Draft**
>
> MVP curve editing should make easing powerful without turning the product into a full graph-editor clone.

## 1. Scope

Support timing easing between keyframes.

MVP does not require:

- full value graph across entire timeline;
- spatial path editing;
- separate speed/value graph modes;
- expressions;
- per-component graph complexity.

---

## 2. Fast path first

Most users should use presets without opening curve editor:

- Linear;
- Ease In;
- Ease Out;
- Ease In-Out.

Curve editor is the advanced path.

---

## 3. Representation

Cubic Bezier timing curve:

~~~text
(0,0)
  \
   P1 ---- P2
             \
             (1,1)
~~~

Control points:

- x constrained according to valid easing semantics;
- y may be allowed outside 0..1 only if overshoot semantics are intentionally supported.

Initial MVP should likely keep a safe bounded curve.

---

## 4. UI placement

Prefer one of:

- temporary lower timeline subpanel;
- contextual inspector section.

Avoid opening a separate full application mode/window.

The user should retain timeline context.

---

## 5. Selection context

Curve editor operates on:

- selected keyframe/transition;
- or selected multiple transitions if compatible.

If selection is ambiguous, show clear neutral state.

---

## 6. Preset interaction

Preset selection updates Bezier parameters.

Custom drag turns it into custom curve.

Reset returns to selected preset/default.

---

## 7. Handles

Large forgiving hit targets.

Display:

- curve;
- control points;
- handles;
- optionally numeric values.

No tiny professional-VFX-style graph controls in MVP.

---

## 8. Live preview

Dragging handle updates animation preview immediately.

One drag = one undo entry.

Escape restores previous curve.

---

## 9. Time-only easing

Curve changes normalized progress, not property value graph topology.

Same curve system works for:

- position;
- scale;
- rotation;
- opacity;
- effect parameters.

---

## 10. Hold

Hold has no editable Bezier.

UI communicates that curve editing is unavailable.

---

## 11. Linear

Linear preset = straight line.

Can be represented without special curve data or canonical Bezier.

---

## 12. Multi-selection

MVP can apply preset easing to many selected transitions.

Custom simultaneous Bezier editing for heterogeneous existing curves can be deferred.

---

## 13. Keyboard

Required:

- Escape cancel drag;
- Delete not destructive to keyframes while handle focus is ambiguous;
- arrow fine-adjust optional;
- preset shortcuts optional.

Do not overload timeline navigation keys inside active numeric curve field.

---

## 14. Performance

Curve UI is tiny.

Main requirement is no expensive full-scene rebuild beyond normal animation re-evaluation.

---

## 15. Tests

- preset maps to expected parameters;
- linear exact;
- handle drag transaction undo;
- invalid curve prevented/clamped;
- evaluator matches visual curve;
- selected transition changes correctly.

---

## 16. Definition of Done

- presets require no graph opening;
- custom cubic curve editable;
- interaction is simple and readable;
- one drag = one undo;
- curve semantics match animation engine.
