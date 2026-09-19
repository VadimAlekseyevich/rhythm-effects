# Text Rendering

> **Status: Accepted for MVP**
>
> Composition text is separate from egui UI text.

## 1. Stack

- cosmic-text: shaping/layout/font database;
- glyphon: wgpu rendering.

## 2. MVP features

Supported:

- Unicode;
- Latin and Cyrillic;
- explicit multiline via newlines;
- installed/system font selection;
- normal/italic;
- practical font weights;
- static font size;
- left/center/right alignment;
- animated LinearRgba text color;
- transform/opacity animation.

Post-MVP:

- text on path;
- per-character animation;
- rich spans;
- automatic paragraph wrapping boxes;
- variable-font axis UI;
- imported/embedded fonts;
- guaranteed color emoji.

## 3. FontReference

~~~rust
FontReference {
    family: String,
    weight: FontWeight,
    style: FontStyle,
}
~~~

Never persist an internal cosmic-text font ID.

## 4. Fallback

Bundle Inter as deterministic fallback.

When requested font is missing:

- project still opens;
- text renders with Inter;
- editor visibly marks missing requested font;
- export uses the same fallback.

No silent invisible substitution.

## 5. Font system lifetime

Use one long-lived font system/database.

System enumeration may be cached for the font picker.

No per-object/per-frame font database creation.

## 6. Content and size

Text String is static in MVP.

Font size is static f32 in composition pixels.

Visual size animation uses Transform Scale.

This avoids reshaping every frame.

## 7. Color

Color is Animated<LinearRgba>.

UI sRGB controls convert at the project boundary.

## 8. Layout

Explicit newline controls line breaks.

No automatic wrap box is stored.

Alignment:

- Left;
- Center;
- Right.

Use a sensible line-height default; advanced typography controls are post-MVP.

## 9. Bounds

Text subsystem produces deterministic local bounds for anchor, selection, hit testing, and viewport outline.

Layout invalidates on text/font/size/alignment changes.

Transform, opacity, and color do not require reshaping.

## 10. Cache

Cache identity includes:

- text;
- FontReference;
- font size;
- alignment;
- shaping settings.

Glyph atlas is shared runtime state and never serialized.

## 11. DPI

Font size is composition-space pixels.

Editor DPI/zoom changes only presentation.

Preview/export use the same composition layout.

## 12. Editing UX

Double click selected text or use inspector edit action.

MVP may use inspector text editing instead of a rich inline canvas editor.

## 13. Missing glyphs / emoji

Missing glyph behavior must not crash.

Color emoji is not a release requirement.

## 14. Performance fixture

At least:

- 50 text objects;
- Latin/Cyrillic mix;
- several installed fonts;
- animated transforms/colors.

Unchanged text is not reshaped every frame.

## 15. Tests

- Cyrillic;
- Latin;
- mixed text;
- multiline;
- alignment;
- missing font -> Inter;
- bounds/anchor;
- preview/export parity;
- DPI independence.

## 16. Definition of Done

System fonts work, fallback is deterministic and visible, shaping is cached, Cyrillic is reliable, and preview/export match.
