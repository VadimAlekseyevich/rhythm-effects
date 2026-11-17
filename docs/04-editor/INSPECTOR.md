# Inspector

> **Status: Accepted for MVP**
>
> Inspector exposes context without becoming a dense control wall.

## 1. Section order

For one selected object:

1. Transform
2. Appearance / object-specific content
3. Effects

No nested tab hierarchy.

## 2. Property row

Standard animatable row:

~~~text
Label | value/control | keyframe state/button
~~~

Animation affordance stays in the same horizontal location across property types.

**AI-183–AI-185 implementation scope:** every singly selected object displays the five Transform animatable property rows in a stable Label | numeric value | keyframe state/button layout: Position X/Y in pixels, Scale X/Y percent, Rotation degrees, Anchor X/Y percent and Opacity percent. The keyframe affordance distinguishes Static, Animated off-key and Key present with explicit glyph and text (not color alone) and dispatches the existing property-scoped `K` action at the current authoring grid when a tempo map exists. Locked properties and animated values are displayed read-only; editing animated values is reserved for AI-186. Editing a static numeric field commits valid finite input on Enter or focus loss; Escape restores the original without changing the project, invalid partial input remains an editor-only draft. Switching object selection flushes valid draft edits for the previous static editable object. All edits use an undoable generic core `SetPropertyBase` command, validate ranges/compatibility, and restore previous values on validation errors; each field commit is one history entry and Vec2 edits preserve the current other component. Animated properties are never silently converted to static.

## 3. Transform

Always available for editable object:

- Position X/Y;
- Scale X/Y;
- Rotation;
- Anchor X/Y;
- Opacity.

Friendly display:

- Scale as percent;
- Rotation degrees;
- Opacity percent;
- Position/size composition pixels;
- Anchor normalized or percent representation, consistently labelled.

## 4. Numeric editing

- click/type;
- Enter commits;
- focus loss commits valid value;
- Escape restores pre-edit;
- invalid intermediate string stays only in UI buffer;
- one edit transaction creates one history step.

Optional drag-scrub may be added if it does not reduce target/readability.

## 5. Property animation

Static property:

- value edits base_value;
- keyframe button/K creates first key at nearest current grid point and moves playhead there.

Animated property:

- at an existing key: edits that key;
- off-key: committing a value creates/updates key at nearest grid point and moves playhead there.

This is property-scoped animation behavior, not global Auto-Key.

**AI-186 implementation scope:** edit a numeric value on an already animated Transform row using Enter/focus-loss to resolve the current playhead to the nearest authoring-grid tick. When a key is present at that resolved tick, update its value while preserving KeyframeId, tick and interpolation; otherwise insert one new key with only the edited component replaced, preserving the other evaluated Vec2 axis. Move playhead to the exact grid time after successful change and write one undoable history entry per numeric commit. Never overwrite an animated property's base_value, create a global Auto-Key mode or start a second transaction; static fields continue using AI-185. Animated edits are disabled without a valid tempo/grid resolution.

**AI-187 implementation scope:** the Inspector keyframe indicator/button uses the existing per-property K action; removing the last key makes the property static and assigns its removed value to base_value so the visible value does not jump. Undo restores that key and the prior base_value; Redo removes it again. Tests cover on-grid last-key removal and base restoration.

## 6. Keyframe indicator

The property control distinguishes:

- static;
- animated but no key at current resolved position;
- key exists at current position.

Do not rely on color alone; glyph/shape/state also changes.

## 7. Removing the last keyframe

When the final key is removed, property becomes static.

Its base_value becomes the value of that removed key so the visible result does not unexpectedly jump at that moment.

The action is undoable.

## 8. Multi-object selection

Inspector shows common transform properties.

Mixed values render a clear mixed state.

Editing a common property applies one compound command to all selected objects.

For each selected property:

- static property changes base_value;
- already animated property creates/updates current grid key.

Type-specific sections appear only if selection types are compatible.

**AI-188 implementation scope:** for two or more selected objects the Inspector renders shared Position, Scale, Rotation, Anchor and Opacity rows, with a visible `Mixed` placeholder for axes/components whose evaluated values differ. Identical values are displayed normally; keyframe states distinguish all-static, all-animated and mixed static/animated selections. One edited component is applied to **every** selected object without changing the other Vec2 component. Locked group members disable the whole group edit rather than silently editing a subset. Every member's property is resolved independently: static members update base_value; already-animated members update an existing key or create a new nearest-grid key and move the playhead there on success. The edit is staged on a cloned candidate project, validated atomically and recorded as **one** compound Undo/Redo history entry for all static and animated changes, with existing keyframe identity/interpolation preserved. Invalid values, duplicate members, missing objects or failure in any member abort the entire compound edit without changing history or ID allocation. Type-specific sections appear only for compatible single selection; mixed multi-selection shows the common Transform section only.

## 9. Rectangle

MVP:

- Size X/Y;
- Fill color;
- Corner Radius only if implemented from schema field.

Size and Fill are animatable.

**AI-189 implementation scope:** when a Rectangle is the sole selected, unlocked object, its Inspector shows Size X/Y (composition pixels), a Fill with a visual swatch and four explicitly labeled **linear-light RGBA percent** channel fields, and Corner Radius (pixels). Every value is edited via Enter/focus-loss with Escape discard. Static edits use one generic undoable `SetPropertyBase` command; animated Size/Fill/Corner Radius edits create a key on the nearest authoring-grid tick, or update an existing key while preserving ID and interpolation. The linear RGBA channels are edited as floats rather than converting semantic color values to 8-bit sRGB (a display-only preview swatch clamps to output range), so negative/HDR linear channel values and alpha semantics are not silently lost. Invalid alpha outside 0..100 percent is rejected. The shared property keyframe state/affordance also works for Rectangle content.

## 10. Ellipse

- Size X/Y;
- Fill color.

Animatable.

**AI-190 implementation scope:** the Ellipse Inspector exposes Size X/Y and the same four-channel linear Fill and keyframe affordance. Existing animated Ellipse Fill edits preserve stable keyframe IDs and interpolation. There is no Rectangle-only Corner Radius field on Ellipse. The same numeric draft lifecycle, group-selection exclusion, core validation and Undo/Redo semantics apply.

## 11. Image

MVP:

- source asset/path summary;
- intrinsic dimensions readout;
- Relink action.

Sizing is via Transform Scale.

No crop/fit controls.

## 12. Text

- content;
- system font;
- weight;
- style;
- font size;
- alignment;
- color.

Text/color semantics follow TEXT_RENDERING.md.

Font size/content are static in MVP; Color is animatable.

## 13. Effects

Effect group shows:

- enabled;
- reorder;
- remove;
- typed parameters with keyframe affordance.

Add Effect opens one shallow searchable list of the five MVP effects.

## 14. Collapsing

Sections may collapse one level.

No collapsible section inside repeated nested collapsible sections unless unavoidable.

Transform starts expanded.

## 15. Search

Dedicated Inspector property search is not required for MVP.

Command Search handles infrequent global actions.

## 16. Focus

Active text/numeric editor owns keyboard typing/arrows according to control behavior.

Global Save/Undo may remain available where safe.

## 17. Validation

Core validates semantic ranges.

UI shows friendly range errors/clamping where appropriate.

Invalid text is never written into Project.

## 18. Performance

Inspector only lays out current selection context.

Font list enumeration/cache is not repeated every frame.

No project-wide property scan for one selected object.

## 19. Definition of Done

Inspector is MVP-ready when static/animated edit semantics, last-key removal, mixed selection, object-specific controls, effect controls, focus/cancel/undo, and comfortable design-system sizing all pass.

## Incremental runtime integration: static Position

The first live Inspector Transform control edits a single unlocked object's
**static Position X/Y** in composition pixels. Each text field commits on Enter
or focus loss, Escape discards the draft, and invalid/non-finite input never
reaches Project. Selecting another object flushes a valid pending edit.
Undo/Redo buttons in Inspector use the existing ProjectEditor history.

The unaffected axis is read from current Project state at commit time to
avoid losing a simultaneous edit or overwriting a viewport move. A successful
change creates exactly one Undo entry; no-op entries are omitted. Multi-object,
locked, and animated Position edits are intentionally unavailable in this slice.
The full Inspector definition of done remains open.
