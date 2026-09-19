# Inspector

> **Status: Draft**
>
> Inspector exposes context-sensitive properties without turning the editor into a wall of controls.

## 1. Primary rule

Show what matters for the current selection.

Do not make the user navigate a deep property hierarchy.

---

## 2. Section order

For a selected object, recommended order:

1. identity/basic;
2. Transform;
3. Appearance/object-specific;
4. Effects.

Advanced details remain collapsed/contextual.

---

## 3. Property row

Canonical pattern:

~~~text
Label        Value / control        Animation control
~~~

Keep alignment consistent.

Do not place animation controls differently for every property type.

---

## 4. Transform section

Always available for visual objects:

- Position X/Y;
- Scale X/Y;
- Rotation;
- Anchor X/Y;
- Opacity.

Frequently used values should not require expanding submenus.

---

## 5. Numeric editing

Support:

- click/type;
- Enter commit;
- Escape cancel;
- optional drag scrub.

During incomplete typing, invalid text stays local UI state until valid/commit.

Never insert NaN/Infinity into Project.

---

## 6. Friendly display units

Internal vs UI:

- scale 1.0 ↔ 100%;
- opacity 0..1 ↔ 0..100%;
- rotation stored/documented, displayed degrees;
- position in composition units/pixels.

Conversion belongs in control layer.

---

## 7. Animation affordance

Each animatable property communicates:

- static;
- animated;
- keyframe exists at current musical position;
- keyframe can be added/removed.

Use one coherent icon/control language.

---

## 8. Keyframe at continuous playhead

Because keyframes live on musical grid, inspector add-key action resolves to the current authoring grid position.

UI should avoid implying arbitrary-time key creation.

If playhead is between divisions, target grid position should be predictable/visible.

---

## 9. Animated property editing

Policy proposal:

- at existing keyframe: edit that key value;
- static property: edit base value;
- animated property with no keyframe at target: do not silently alter an interpolated temporary value;
- user explicitly adds keyframe or enables future auto-key.

This keeps state predictable.

---

## 10. Multi-object selection

MVP may support limited common-property editing.

At minimum:

- inspector can indicate multiple objects selected;
- common transform fields may show mixed values;
- bulk edit can be deferred if implementation becomes costly.

Do not show misleading single values.

---

## 11. Rectangle

Expose:

- size;
- fill;
- corner radius only if included.

---

## 12. Ellipse

Expose:

- size;
- fill.

---

## 13. Image

Expose:

- asset/source;
- intrinsic dimensions info;
- fit behavior if supported.

Missing source shows relink action.

---

## 14. Text

Expose:

- content;
- font family;
- font size;
- alignment;
- color.

Avoid advanced typography controls in MVP.

---

## 15. Effects

Effects displayed as shallow stacked sections/cards.

Each:

- enabled;
- name;
- parameters;
- animation controls;
- reorder handle/action;
- remove.

No effect editor popup for normal parameters.

---

## 16. Add Effect

One searchable/list popover.

Small MVP list.

Do not create a separate Effects Browser panel.

---

## 17. Collapsing

Transform may default expanded.

Object-specific section expanded.

Effects individually collapsible if stack grows.

Avoid nested collapsibles inside effects unless essential.

---

## 18. Property search

Not required for MVP.

If future property count grows, search is preferable to more nested tabs.

---

## 19. Focus

Focused numeric/text control captures typing.

Global safe shortcuts remain where appropriate.

Rhythm arrow navigation must not steal arrow keys from active text field.

---

## 20. Validation

Inline errors for user-fixable values.

Examples:

- invalid font;
- invalid asset;
- out-of-range BPM elsewhere;
- impossible size.

Prefer clamp only when expected; otherwise explain.

---

## 21. Performance

Inspector only renders selected context.

No asset decode/font scan/file IO directly in UI draw.

Large dropdown lists use lazy/searchable presentation.

---

## 22. Definition of Done

- all MVP object properties accessible;
- animation state obvious;
- no deep navigation;
- invalid input cannot corrupt project;
- effect controls stay compact;
- missing assets/fonts recoverable.
