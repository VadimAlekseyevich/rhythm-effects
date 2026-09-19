# Viewport

> **Status: Draft**

## 1. Responsibilities

Viewport displays the composition and provides direct object manipulation.

It owns editor interaction overlays, not creative rendering semantics.

---

## 2. Composition display

Renderer produces offscreen composition texture.

Viewport:

- positions it within available panel;
- scales according to viewport zoom;
- pans according to viewport camera;
- clips to viewport rectangle.

Composition texture remains at project resolution.

---

## 3. Viewport camera

Editor-only state:

~~~rust
ViewportCamera {
    zoom,
    pan,
}
~~~

Not part of Project.

---

## 4. Zoom

Required:

- wheel/pinch;
- fit composition;
- 100% view optional;
- frame selection desirable.

Zoom anchor should feel predictable, ideally under cursor.

---

## 5. Pan

Preferred:

- middle mouse drag;
- optional temporary Space+drag only if it does not conflict with Play/Pause.

Do not create permanent "hand tool" unless usability proves necessary.

---

## 6. Object selection

Click visible unlocked object.

Selection order when objects overlap:

- frontmost hit first;
- possible cycling modifier later.

Locked objects are not directly selectable in viewport unless explicit override exists.

---

## 7. Hit testing

MVP options:

- CPU geometric hit-test from evaluated object bounds;
- avoid GPU picking unless necessary.

Need handle:

- transforms;
- rotation;
- text/image bounds;
- ellipse shape vs bounding box tradeoff.

Bounding-box hit may be acceptable initially for text/image but shapes should feel sensible.

---

## 8. Multi-selection

Ctrl-click adds/removes.

Box selection selects visible unlocked objects intersecting selection rect according to explicit containment/intersection rule.

---

## 9. Selection outline

Editor overlay only.

Must remain readable over varied composition colors.

Use strong but restrained visual treatment.

Not exported.

---

## 10. Transform gizmos

MVP needs:

- move;
- scale;
- rotation.

Possible single universal selection box with handles.

Avoid a large traditional toolbar of separate tools unless necessary.

---

## 11. Move

Direct drag selected object body can move when not conflicting with text editing.

Movement updates transient project value.

Commit one undo step.

Spatial snapping/guides are optional; musical time snapping is unrelated.

---

## 12. Scale

Selection handles.

Rules:

- obvious corner/edge handles;
- larger invisible hit region;
- modifier for aspect lock if supported;
- transform around anchor or opposite handle according to defined model.

Need consistency with Transform math.

---

## 13. Rotation

Rotation handle or contextual gesture.

Display angle during drag.

No automatic shortest-path rule affects stored keyframe interpolation; this is property editing only.

---

## 14. Anchor

Anchor/pivot is visible when relevant.

MVP can provide simple draggable anchor if implementation is manageable.

Changing anchor while preserving visual position requires explicit transform math.

Do not ship confusing anchor behavior.

---

## 15. Animated transforms

When playhead is at exact keyframe:

- dragging transform edits that keyframe value.

When property is animated but no keyframe at current playhead:

Default policy should not silently create off-context keys.

Options:

- require explicit keyframe first;
- if auto-key enabled, snap/create at grid.

Final behavior coordinated with Inspector/Interaction Model.

---

## 16. Playhead between grid points

Directly changing an animated property between keyframes is potentially ambiguous.

MVP safest behavior:

- allow preview manipulation only after user intentionally creates/selects keyframe;
- or snap creation through explicit animation control.

Avoid hidden key creation.

---

## 17. Text editing

Double click may enter text edit mode.

While editing text:

- viewport transform drag is suppressed;
- typing stays with text editor;
- Escape exits/cancels according to policy;
- object remains visually selected.

Exact text editing may use inspector first for MVP if in-canvas editing becomes complex.

---

## 18. Image aspect

Scaling behavior must distinguish:

- object transform scale;
- intrinsic image size/fit.

Keep transform model simple.

---

## 19. Guides

Post-MVP unless inexpensive.

Potential:

- composition center;
- edge alignment;
- smart guides.

Do not let spatial snapping interfere with BPM time snapping concepts.

---

## 20. Safe frame

Not required.

Can add composition bounds and center indicators only.

---

## 21. Performance

Viewport itself should not rerender project through CPU.

It displays GPU texture + light overlays.

Hit testing should use evaluated/bounds cache, not reconstruct full render scene repeatedly.

---

## 22. DPI

Pointer coordinates:

~~~text
window physical/logical
→ egui point coordinates
→ viewport local
→ composition coordinates
~~~

Centralize transforms and test at 125%, 150%, 200% Windows scale.

---

## 23. Resize

Viewport panel resize changes visible area only.

Composition target resolution stays project-defined.

---

## 24. Empty state

With no object:

- composition still visible;
- object creation discoverable;
- audio/timeline work unaffected.

---

## 25. Required tests

- coordinate conversion round-trip;
- rotated object selection;
- zoom anchor behavior;
- pan;
- multi-select;
- transform drag undo;
- DPI;
- viewport resize;
- locked object behavior.

---

## 26. Definition of Done

- composition texture displays correctly;
- select/multi-select works;
- move/scale/rotation work;
- interactions create one undo entry;
- overlays not exported;
- DPI mapping correct;
- viewport remains responsive.
