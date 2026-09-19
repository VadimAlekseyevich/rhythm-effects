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

## 9. Rectangle

MVP:

- Size X/Y;
- Fill color;
- Corner Radius only if implemented from schema field.

Size and Fill are animatable.

## 10. Ellipse

- Size X/Y;
- Fill color.

Animatable.

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
