# Text Rendering

> **Status: Draft**
>
> Composition text is a high-risk subsystem: shaping, fonts, fallback, layout, caching, and GPU rendering must work without coupling creative content to editor UI text.

## 1. MVP requirements

- Latin;
- Cyrillic;
- multiline;
- font family selection;
- font size;
- text color;
- basic alignment;
- transform/opacity animation;
- reliable preview/export.

Advanced typography is not MVP.

---

## 2. Preferred prototype stack

Evaluated 2026-09-19:

- cosmic-text 0.19.x;
- glyphon 0.12.x;
- wgpu 30.x.

Current glyphon release targets wgpu 30 and uses cosmic-text for text handling.

This alignment is valuable because it avoids a second graphics backend.

---

## 3. Why separate from egui text

egui text belongs to application chrome.

Creative text requires:

- project font references;
- composition coordinate space;
- export consistency;
- object transforms;
- composition clipping;
- runtime layout caching.

Do not serialize or depend on egui font identifiers.

---

## 4. Font system lifetime

Maintain shared runtime font system.

Do not rebuild system font database per object/frame.

Potentially initialize lazily if startup scan is measurable.

---

## 5. Font reference

Persist semantic reference:

~~~rust
FontReference {
    family,
    weight/style,
}
~~~

Exact schema TBD.

Runtime resolves to face.

Do not persist ephemeral font database index.

---

## 6. Missing fonts

Project remains editable.

Behavior:

- render fallback;
- mark missing requested font in inspector;
- offer replacement;
- do not silently rewrite saved font choice just because fallback rendered.

---

## 7. Shaping

Use full shaping for composition text.

Validate:

- Latin;
- Cyrillic;
- combining marks;
- punctuation;
- common Unicode.

RTL support can be inherited from stack, but full RTL editing UX is not required for MVP unless explicitly promoted.

---

## 8. Layout cache

Cache shaped layout by semantic inputs:

- text;
- font;
- size;
- wrap width;
- alignment;
- other layout attributes.

Object position/rotation/opacity changes must not trigger reshape.

---

## 9. Glyph cache

Reuse glyph atlas/raster cache.

Do not rasterize/upload same glyphs every frame.

Need bounded/growing atlas strategy appropriate for long editor sessions.

---

## 10. Object bounds

Text subsystem provides layout bounds for:

- selection;
- hit test;
- anchor;
- alignment.

Viewport and renderer must use the same bounds semantics.

---

## 11. Font-size animation

Font-size animation is expensive because it can reshape/rasterize repeatedly.

Recommended MVP:

- font size static;
- animate apparent size using object Scale.

This keeps motion fast and cache-friendly.

If font-size animation is promoted later, profile it explicitly.

---

## 12. Text content animation

Not MVP.

Text content is static property.

Transform, opacity, and optionally color animate.

---

## 13. Color

Text fill color may be Animated<Color>.

Stroke/shadow are not dedicated typography features in MVP.

Use generic effects such as Glow where possible.

---

## 14. Alignment

Minimum:

- left;
- center;
- right.

Need decide model:

- point text;
- bounded text box.

Prototype both interaction implications before locking data schema.

---

## 15. Composition vs UI DPI

Composition text layout uses composition space.

Editor DPI scaling only changes how the already-rendered composition is displayed.

Export uses same composition layout.

---

## 16. Startup

If system font discovery is slow:

- show app shell first;
- initialize font system lazily/background where architecture permits;
- project containing text may show loading placeholder briefly rather than blocking startup excessively.

Measure before adding complexity.

---

## 17. Build-time cost

Text stack is substantial.

Measure:

- clean compile;
- incremental compile;
- transitive dependencies;
- binary contribution.

Ensure glyphon/cosmic-text/wgpu versions do not cause duplicate major wgpu builds.

---

## 18. Prototype checklist

1. Latin + Cyrillic;
2. system font lookup;
3. fallback;
4. multiline;
5. alignment;
6. 100+ objects;
7. transform animation without reshape;
8. text edit invalidation;
9. offscreen render;
10. export/readback;
11. compile-time impact.

---

## 19. Tests

- Cyrillic fixture;
- multiline bounds;
- missing font;
- cache invalidation;
- no reshape on transform-only change;
- semantic font reference serialization.

---

## 20. Open decisions

- exact font schema;
- custom imported fonts in MVP;
- point vs box text;
- exact stack features;
- bundled test font licensing;
- whether text color is animated in MVP.

---

## 21. Definition of Done

- Latin/Cyrillic correct;
- separate from egui font system;
- missing font recoverable;
- layout/glyph cache effective;
- preview/export layout matches;
- build/runtime costs accepted.
