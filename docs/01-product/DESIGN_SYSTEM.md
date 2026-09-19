# Design System

> **Status: Draft**
>
> This document defines the visual and ergonomic constraints of the editor. Exact colors, font family, and pixel values are intentionally not finalized yet; the system rules are.

## 1. Design intent

Rhythm Effects should feel:

- spacious;
- soft;
- fast;
- focused;
- modern without decorative excess;
- professional without becoming dense;
- suitable for long editing sessions.

The editor is not trying to maximize the number of visible controls.

It is trying to maximize the amount of useful editing state the user can understand at a glance.

---

## 2. Visual hierarchy

Use a small number of hierarchy levels.

Recommended conceptual levels:

1. application/workspace;
2. panel;
3. section;
4. primary control/content;
5. secondary metadata.

Avoid creating new hierarchy through arbitrary font-size changes.

Hierarchy should primarily come from:

- spacing;
- grouping;
- weight;
- surface contrast;
- alignment;
- restrained typography.

---

## 3. Typography

### Rule: few text styles

Target a compact type system, for example:

- display/major title only where necessary;
- panel/section title;
- normal UI text;
- secondary/metadata text.

Do not create a unique font style for every component.

### Readability

The default UI must not depend on tiny text to fit more controls.

Numeric/timeline metadata may be somewhat denser than normal labels, but should remain comfortably readable at standard Windows scaling.

### Font selection

Final font is TBD.

Requirements:

- excellent Latin;
- excellent Cyrillic;
- clear numerals;
- good distinction between similar glyphs;
- efficient desktop rendering;
- appropriate licensing/distribution.

---

## 4. Spacing

Spacing is functional.

Use a consistent scale rather than arbitrary margins.

The final token values will be selected after a live UI prototype.

Rules:

- related controls are visually close;
- unrelated groups receive meaningful separation;
- panel edges have breathing room;
- dense timeline content may use tighter spacing than inspector controls;
- hit targets remain comfortable even when visual spacing is compact.

---

## 5. Interaction target sizes

Small visual elements may have larger hit regions.

Important targets:

- keyframes;
- playhead head/handle;
- curve points;
- panel splitters;
- visibility/lock toggles;
- keyframe enable/add buttons;
- effect reorder handles.

A user should not need pixel-perfect pointer placement.

Exact minimums will be validated during prototype usability testing.

---

## 6. Surfaces and panels

Prefer a shallow surface system.

Potential conceptual surfaces:

- workspace background;
- panel background;
- raised/active control;
- selected/interactive state;
- overlay/popover.

Avoid outlines around every container.

Use borders only when they clarify structure.

Panel separation should often come from spacing and subtle surface differences.

---

## 7. Corners and softness

Controls/panels may use restrained rounding to create a softer workspace.

Avoid:

- excessive pill-shaped everything;
- giant rounded cards wasting editor space;
- decorative glass effects that reduce contrast.

Softness should improve comfort, not become branding noise.

---

## 8. Color

Color is functional first.

Primary semantic uses:

- selection;
- active/focused state;
- playhead/current time;
- musical hierarchy;
- warnings/errors;
- keyframe state where necessary.

Avoid using many unrelated accent colors simultaneously.

The viewport content may be colorful; editor chrome should remain controlled.

### Musical grid

Grid should communicate hierarchy through a combination of:

- intensity;
- thickness;
- spacing;
- restrained tonal variation.

Do not turn every subdivision into a competing color.

---

## 9. Timeline density

Timeline is allowed to be denser than the inspector because density represents useful temporal information.

However:

- keyframes need forgiving hit targets;
- row labels remain readable;
- selected state is obvious;
- beat/bar hierarchy remains clear;
- waveform does not visually overpower keyframes;
- minor subdivisions fade with zoom.

---

## 10. Inspector design

Inspector should optimize for scanning.

Property row pattern should be consistent:

~~~text
Label          Value/control      Animation affordance
~~~

Rules:

- align related value fields;
- avoid excessive icons;
- use text labels when an icon is ambiguous;
- keep animation affordance consistently placed;
- advanced controls collapse only one logical level where possible.

---

## 11. Toolbar philosophy

The app should not have several permanent toolbars full of tiny icons.

A small primary toolbar may contain:

- create/add;
- essential transform/navigation tools if needed;
- project/export entry points where appropriate.

Transport/rhythm controls belong near timeline context rather than in an unrelated top toolbar if that improves workflow.

---

## 12. Iconography

Icons must be:

- simple;
- coherent;
- recognizable at actual UI size;
- paired with tooltip;
- not the sole carrier of obscure meaning.

Prefer text for actions whose icon would be ambiguous.

---

## 13. Motion in the editor chrome

Use motion sparingly.

Acceptable:

- subtle hover/focus transitions;
- short panel disclosure animation if it does not delay;
- progress indicators.

Avoid:

- slow panel slides;
- bouncing controls;
- decorative hover motion;
- constantly animated chrome.

Editor UI should visually get out of the way of the actual animation.

---

## 14. Selection states

Selection is one of the most important visual states.

Need distinct treatment for:

- object selected;
- keyframe selected;
- multiple selection;
- focused panel;
- active property;
- playhead/current grid position.

Do not rely only on tiny color shifts.

---

## 15. Hover states

Hover should help discover interaction without causing visual flicker.

Good uses:

- reveal larger keyframe hit region;
- highlight draggable splitter;
- show tooltip after short delay;
- emphasize clickable property animation control.

Do not reveal essential controls only during extremely precise hover.

---

## 16. Focus states

Keyboard focus must be visible enough to understand where contextual shortcuts will apply.

This is particularly important between:

- timeline;
- viewport;
- object list;
- inspector text fields.

Focus ring/style should be noticeable but not visually loud.

---

## 17. Disabled states

Disabled elements remain legible.

Do not reduce opacity so far that labels become unreadable.

Where helpful, tooltip explains prerequisite.

---

## 18. Error and warning states

Errors are visually clear but not alarming beyond their severity.

Levels:

- informational;
- warning/recoverable issue;
- blocking error.

Use plain language.

Do not show raw Rust/FFmpeg/backend error text as the primary message.

---

## 19. Popovers and menus

Use for short contextual choices.

Rules:

- shallow hierarchy;
- searchable list if choices grow;
- sensible keyboard navigation;
- Escape closes;
- click outside closes when safe.

Avoid submenu chains.

---

## 20. Command/search surface

If a command palette is included, it becomes a pressure-release valve for discoverability as features grow.

It can expose less-frequent actions without adding permanent UI.

Design goals:

- open instantly;
- fuzzy/searchable;
- show shortcut;
- show context/disabled reason;
- keyboard navigable.

---

## 21. Scaling / DPI

UI must remain comfortable under common Windows scaling settings.

Do not hardcode assumptions that only look correct at 100%.

Viewport pixel mapping and editor UI scaling are separate concerns.

---

## 22. Minimum window size

Define a minimum usable window size during prototype stage.

Below it:

- avoid overlapping controls;
- allow panels to collapse intentionally;
- do not silently shrink text into unreadability.

---

## 23. Layout adaptability

The interface may allow resizing and some panel collapsing, but MVP does not need a fully dockable IDE-style workspace.

A stable default layout is more important than unlimited layout customization.

This directly supports simplicity.

---

## 24. Progressive disclosure rules

When adding new functionality, prefer in order:

1. use existing property group;
2. show contextually for selected object/effect;
3. add optional collapsed advanced subsection;
4. expose through command/search;
5. add a new panel only if the task truly needs persistent space.

A new tab nested inside another tab should be treated as a design smell.

---

## 25. Design review checklist

Any new editor feature should be reviewed against:

- Does it reduce readable text size?
- Does it introduce another permanent panel?
- Does it create another nested navigation level?
- Does it add an unexplained icon?
- Does it increase common-task click count?
- Can frequent use get a shortcut?
- Can infrequent use stay out of the default view?
- Is the target comfortable to click?
- Is state visible?
- Does it work at common DPI scaling?
- Does it visually compete with timeline/viewport content?

---

## 26. Prototype deliverables

Before finalizing tokens/colors, build a functional UI prototype containing:

- top-level workspace;
- viewport;
- timeline with waveform;
- bar/beat/subdivision grid;
- several object rows;
- several keyframes;
- inspector with animated properties;
- object list;
- transport;
- effect section;
- popover/menu;
- focused/selected/disabled/error states.

Evaluate the system at realistic content density, not in isolated component screenshots.
