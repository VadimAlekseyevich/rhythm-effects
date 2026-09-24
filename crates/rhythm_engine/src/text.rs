use std::{
    collections::{BTreeSet, HashMap},
    fmt,
};

use cosmic_text::{Align, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Wrap};
use rhythm_core::{
    domain::Vec2,
    geometry::LocalBounds2d,
    project::{FontReference, FontStyle, FontWeight, TextAlignment},
};

pub const COMPOSITION_FALLBACK_FAMILY: &str = "Inter";

const INTER_VARIABLE: &[u8] = include_bytes!("../assets/fonts/inter/InterVariable.ttf");
const INTER_VARIABLE_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/inter/InterVariable-Italic.ttf");

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextLayoutKey {
    text: String,
    font: FontReference,
    font_size_bits: u32,
    alignment: TextAlignment,
}

impl TextLayoutKey {
    fn new(text: &str, font: &FontReference, font_size: f32, alignment: TextAlignment) -> Self {
        Self {
            text: text.to_owned(),
            font: font.clone(),
            font_size_bits: font_size.to_bits(),
            alignment,
        }
    }
}

fn create_composition_font_system_and_cache() -> (FontSystem, Vec<String>) {
    let mut font_system = FontSystem::new();
    let system_font_families = enumerate_system_font_families(&font_system);
    install_bundled_inter(&mut font_system);
    (font_system, system_font_families)
}

fn enumerate_system_font_families(font_system: &FontSystem) -> Vec<String> {
    font_system
        .db()
        .faces()
        .filter_map(|face| face.families.first().map(|(family, _)| family.clone()))
        .filter(|family| !family.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn cosmic_weight(weight: FontWeight) -> cosmic_text::fontdb::Weight {
    match weight {
        FontWeight::Thin => cosmic_text::fontdb::Weight::THIN,
        FontWeight::ExtraLight => cosmic_text::fontdb::Weight::EXTRA_LIGHT,
        FontWeight::Light => cosmic_text::fontdb::Weight::LIGHT,
        FontWeight::Normal => cosmic_text::fontdb::Weight::NORMAL,
        FontWeight::Medium => cosmic_text::fontdb::Weight::MEDIUM,
        FontWeight::SemiBold => cosmic_text::fontdb::Weight::SEMIBOLD,
        FontWeight::Bold => cosmic_text::fontdb::Weight::BOLD,
        FontWeight::ExtraBold => cosmic_text::fontdb::Weight::EXTRA_BOLD,
        FontWeight::Black => cosmic_text::fontdb::Weight::BLACK,
    }
}

fn cosmic_style(style: FontStyle) -> cosmic_text::fontdb::Style {
    match style {
        FontStyle::Normal => cosmic_text::fontdb::Style::Normal,
        FontStyle::Italic => cosmic_text::fontdb::Style::Italic,
    }
}

fn cosmic_alignment(alignment: TextAlignment) -> Align {
    match alignment {
        TextAlignment::Left => Align::Left,
        TextAlignment::Center => Align::Center,
        TextAlignment::Right => Align::Right,
    }
}

fn font_family_available(font_system: &FontSystem, family: &str) -> bool {
    font_system.db().faces().any(|face| {
        face.families
            .iter()
            .any(|(candidate, _)| candidate == family)
    })
}

fn shape_with_font_system(
    font_system: &mut FontSystem,
    text: &str,
    font: &FontReference,
    font_size: f32,
    alignment: TextAlignment,
) -> ShapedText {
    let requested_font_missing = !font_family_available(font_system, &font.family);
    let resolved_family = if requested_font_missing {
        COMPOSITION_FALLBACK_FAMILY
    } else {
        font.family.as_str()
    };
    let metrics = Metrics::new(font_size, font_size);
    let attrs = Attrs::new()
        .family(Family::Name(resolved_family))
        .weight(cosmic_weight(font.weight))
        .style(cosmic_style(font.style));
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_wrap(Wrap::None);
    buffer.set_text(
        text,
        &attrs,
        Shaping::Advanced,
        Some(cosmic_alignment(alignment)),
    );
    buffer.shape_until_scroll(font_system, false);

    let layout_width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0_f32, f32::max);
    buffer.set_size(Some(layout_width), None);
    buffer.shape_until_scroll(font_system, false);

    ShapedText {
        buffer,
        requested_font_missing,
    }
}

fn shape_cached<'a>(
    font_system: &mut FontSystem,
    layout_cache: &'a mut HashMap<TextLayoutKey, ShapedText>,
    text: &str,
    font: &FontReference,
    font_size: f32,
    alignment: TextAlignment,
) -> &'a ShapedText {
    let key = TextLayoutKey::new(text, font, font_size, alignment);

    match layout_cache.entry(key) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => entry.insert(shape_with_font_system(
            font_system,
            text,
            font,
            font_size,
            alignment,
        )),
    }
}

fn install_bundled_inter(font_system: &mut FontSystem) {
    let system_inter_faces: Vec<_> = font_system
        .db()
        .faces()
        .filter(|face| {
            face.families
                .iter()
                .any(|(family, _)| family == COMPOSITION_FALLBACK_FAMILY)
        })
        .map(|face| face.id)
        .collect();

    let database = font_system.db_mut();
    for face_id in system_inter_faces {
        database.remove_face(face_id);
    }

    database.load_font_data(INTER_VARIABLE.to_vec());
    database.load_font_data(INTER_VARIABLE_ITALIC.to_vec());
}

/// A shaped composition text buffer backed by cosmic-text.
#[derive(Debug)]
pub struct ShapedText {
    buffer: Buffer,
    requested_font_missing: bool,
}

impl ShapedText {
    #[must_use]
    pub const fn requested_font_missing(&self) -> bool {
        self.requested_font_missing
    }

    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.buffer.layout_runs().map(|run| run.glyphs.len()).sum()
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        self.buffer.lines.len()
    }

    #[must_use]
    pub fn local_bounds(&self) -> LocalBounds2d {
        let width = self.buffer.size().0.unwrap_or(0.0);
        let height = self
            .buffer
            .layout_runs()
            .map(|run| run.line_top + run.line_height)
            .fold(0.0_f32, f32::max);
        let size = Vec2::new(width, height).expect("shaped text layout bounds must be finite");

        LocalBounds2d::from_size(size)
    }

    #[must_use]
    pub fn missing_glyph_count(&self) -> usize {
        self.buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter())
            .filter(|glyph| glyph.glyph_id == 0)
            .count()
    }
}

/// Borrowed glyphon state for one prepare/render operation.
pub struct GlyphonResources<'a> {
    pub font_system: &'a mut FontSystem,
    pub atlas: &'a mut glyphon::TextAtlas,
    pub viewport: &'a glyphon::Viewport,
    pub swash_cache: &'a mut glyphon::SwashCache,
    pub renderer: &'a mut glyphon::TextRenderer,
}

/// Long-lived composition text state.
///
/// Font discovery, glyph rasterization caches, atlas allocation, viewport state,
/// and the glyphon renderer are created once with the renderer and reused across
/// frames. Later shaping/layout tasks borrow this state rather than rebuilding it.
pub struct TextResources {
    font_system: FontSystem,
    system_font_families: Vec<String>,
    layout_cache: HashMap<TextLayoutKey, ShapedText>,
    _glyphon_cache: glyphon::Cache,
    atlas: glyphon::TextAtlas,
    viewport: glyphon::Viewport,
    swash_cache: glyphon::SwashCache,
    renderer: glyphon::TextRenderer,
}

impl fmt::Debug for TextResources {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextResources")
            .field("viewport_resolution", &self.viewport.resolution())
            .field("system_font_family_count", &self.system_font_families.len())
            .field("text_layout_cache_count", &self.layout_cache.len())
            .finish_non_exhaustive()
    }
}

impl TextResources {
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
        resolution: [u32; 2],
    ) -> Self {
        let (font_system, system_font_families) = create_composition_font_system_and_cache();
        let glyphon_cache = glyphon::Cache::new(device);
        let mut atlas = glyphon::TextAtlas::new(device, queue, &glyphon_cache, target_format);
        let mut viewport = glyphon::Viewport::new(device, &glyphon_cache);
        viewport.update(
            queue,
            glyphon::Resolution {
                width: resolution[0],
                height: resolution[1],
            },
        );
        let swash_cache = glyphon::SwashCache::new();
        let renderer =
            glyphon::TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            font_system,
            system_font_families,
            layout_cache: HashMap::new(),
            _glyphon_cache: glyphon_cache,
            atlas,
            viewport,
            swash_cache,
            renderer,
        }
    }

    #[must_use]
    pub const fn font_system(&self) -> &FontSystem {
        &self.font_system
    }

    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        self.layout_cache.clear();
        &mut self.font_system
    }

    #[must_use]
    pub fn system_font_families(&self) -> &[String] {
        &self.system_font_families
    }

    #[must_use]
    pub fn font_family_available(&self, family: &str) -> bool {
        font_family_available(&self.font_system, family)
    }

    #[must_use]
    pub fn shape_text(
        &mut self,
        text: &str,
        font: &FontReference,
        font_size: f32,
        alignment: TextAlignment,
    ) -> &ShapedText {
        shape_cached(
            &mut self.font_system,
            &mut self.layout_cache,
            text,
            font,
            font_size,
            alignment,
        )
    }

    #[must_use]
    pub fn viewport_resolution(&self) -> glyphon::Resolution {
        self.viewport.resolution()
    }

    pub fn update_viewport(&mut self, queue: &wgpu::Queue, resolution: [u32; 2]) {
        self.viewport.update(
            queue,
            glyphon::Resolution {
                width: resolution[0],
                height: resolution[1],
            },
        );
    }

    pub fn glyphon_resources_mut(&mut self) -> GlyphonResources<'_> {
        GlyphonResources {
            font_system: &mut self.font_system,
            atlas: &mut self.atlas,
            viewport: &self.viewport,
            swash_cache: &mut self.swash_cache,
            renderer: &mut self.renderer,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cosmic_text::{
        FontSystem,
        fontdb::{Family, Query, Source, Stretch, Style, Weight},
    };

    use super::{
        COMPOSITION_FALLBACK_FAMILY, ShapedText, create_composition_font_system_and_cache,
        enumerate_system_font_families, font_family_available, shape_cached, shape_with_font_system,
    };
    use rhythm_core::{
        domain::Vec2,
        geometry::{ObjectTransform2d, hit_test_text_layout_bounds},
        project::{FontReference, FontStyle, FontWeight, TextAlignment},
    };

    fn inter_font(weight: FontWeight, style: FontStyle) -> FontReference {
        FontReference {
            family: COMPOSITION_FALLBACK_FAMILY.to_owned(),
            weight,
            style,
        }
    }

    fn assert_shapes_without_missing_glyphs(text: &str, font: &FontReference) {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let shaped =
            shape_with_font_system(&mut font_system, text, font, 48.0, TextAlignment::Left);

        assert!(shaped.glyph_count() > 0);
        assert_eq!(shaped.missing_glyph_count(), 0);
    }

    #[test]
    fn missing_requested_family_shapes_with_bundled_inter() {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let inter_face_ids: Vec<_> = font_system
            .db()
            .faces()
            .filter(|face| {
                face.families
                    .iter()
                    .any(|(family, _)| family == COMPOSITION_FALLBACK_FAMILY)
            })
            .map(|face| face.id)
            .collect();
        let missing_font = FontReference {
            family: "Rhythm Effects Definitely Missing Font".to_owned(),
            weight: FontWeight::SemiBold,
            style: FontStyle::Italic,
        };

        assert!(!font_family_available(&font_system, &missing_font.family));

        let shaped = shape_with_font_system(
            &mut font_system,
            "FallbackПривет",
            &missing_font,
            48.0,
            TextAlignment::Left,
        );

        assert!(shaped.requested_font_missing());
        assert_eq!(shaped.missing_glyph_count(), 0);
        assert!(shaped.buffer.layout_runs().all(|run| {
            run.glyphs
                .iter()
                .all(|glyph| inter_face_ids.contains(&glyph.font_id))
        }));
    }

    #[test]
    fn bundled_inter_is_not_reported_as_missing() {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let shaped = shape_with_font_system(
            &mut font_system,
            "Inter",
            &inter_font(FontWeight::Normal, FontStyle::Normal),
            48.0,
            TextAlignment::Left,
        );

        assert!(!shaped.requested_font_missing());
    }

    #[test]
    fn shapes_latin_with_bundled_inter() {
        assert_shapes_without_missing_glyphs(
            "Rhythm Effects",
            &inter_font(FontWeight::Normal, FontStyle::Normal),
        );
    }

    #[test]
    fn shapes_cyrillic_with_bundled_inter() {
        assert_shapes_without_missing_glyphs(
            "Привет ритм",
            &inter_font(FontWeight::SemiBold, FontStyle::Normal),
        );
    }

    #[test]
    fn shapes_mixed_latin_and_cyrillic_with_bundled_inter() {
        assert_shapes_without_missing_glyphs(
            "Rhythm Привет 123",
            &inter_font(FontWeight::Bold, FontStyle::Italic),
        );
    }

    #[test]
    fn explicit_newlines_create_multiline_layout() {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let shaped = shape_with_font_system(
            &mut font_system,
            "First line\nВторая строка\nThird line",
            &inter_font(FontWeight::Normal, FontStyle::Normal),
            48.0,
            TextAlignment::Left,
        );

        assert_eq!(shaped.line_count(), 3);
        assert_eq!(shaped.missing_glyph_count(), 0);
    }

    #[test]
    fn empty_explicit_line_is_preserved() {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let shaped = shape_with_font_system(
            &mut font_system,
            "Latin\n\nКириллица",
            &inter_font(FontWeight::Normal, FontStyle::Normal),
            48.0,
            TextAlignment::Left,
        );

        assert_eq!(shaped.line_count(), 3);
        assert_eq!(shaped.missing_glyph_count(), 0);
    }

    fn line_start_and_width(shaped: &ShapedText, line_index: usize) -> (f32, f32) {
        let run = shaped
            .buffer
            .layout_runs()
            .nth(line_index)
            .expect("expected shaped line");
        let start = run.glyphs.first().map_or(0.0, |glyph| glyph.x);
        (start, run.line_w)
    }

    #[test]
    fn left_center_and_right_align_shorter_lines_to_widest_line() {
        let font = inter_font(FontWeight::Normal, FontStyle::Normal);
        let text = "Wide alignment line\nshort";

        let (mut left_fonts, _) = create_composition_font_system_and_cache();
        let left = shape_with_font_system(&mut left_fonts, text, &font, 48.0, TextAlignment::Left);

        let (mut center_fonts, _) = create_composition_font_system_and_cache();
        let center =
            shape_with_font_system(&mut center_fonts, text, &font, 48.0, TextAlignment::Center);

        let (mut right_fonts, _) = create_composition_font_system_and_cache();
        let right =
            shape_with_font_system(&mut right_fonts, text, &font, 48.0, TextAlignment::Right);

        let (left_wide_x, wide_width) = line_start_and_width(&left, 0);
        let (left_short_x, short_width) = line_start_and_width(&left, 1);
        let (center_wide_x, _) = line_start_and_width(&center, 0);
        let (center_short_x, _) = line_start_and_width(&center, 1);
        let (right_wide_x, _) = line_start_and_width(&right, 0);
        let (right_short_x, _) = line_start_and_width(&right, 1);

        let remaining = wide_width - short_width;
        assert!(remaining > 0.0);
        assert!((left_wide_x - center_wide_x).abs() < 0.01);
        assert!((left_wide_x - right_wide_x).abs() < 0.01);
        assert!((center_short_x - left_short_x - remaining / 2.0).abs() < 0.01);
        assert!((right_short_x - left_short_x - remaining).abs() < 0.01);
    }

    #[test]
    fn local_bounds_cover_widest_line_and_full_multiline_height() {
        let font = inter_font(FontWeight::Normal, FontStyle::Normal);
        let text = "Wide alignment line\n\nshort";
        let mut widths = Vec::new();

        for alignment in [
            TextAlignment::Left,
            TextAlignment::Center,
            TextAlignment::Right,
        ] {
            let (mut font_system, _) = create_composition_font_system_and_cache();
            let shaped = shape_with_font_system(&mut font_system, text, &font, 48.0, alignment);
            let bounds = shaped.local_bounds();

            assert_eq!(bounds.min, Vec2::new(0.0, 0.0).expect("finite origin"));
            assert!((bounds.size.y() - 144.0).abs() < 0.01);
            assert!(bounds.size.x() > 0.0);
            widths.push(bounds.size.x());
        }

        assert!((widths[0] - widths[1]).abs() < 0.01);
        assert!((widths[0] - widths[2]).abs() < 0.01);
    }

    #[test]
    fn local_bounds_feed_text_anchor_hit_testing() {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let shaped = shape_with_font_system(
            &mut font_system,
            "Anchor\nПривязка",
            &inter_font(FontWeight::Normal, FontStyle::Normal),
            48.0,
            TextAlignment::Center,
        );
        let bounds = shaped.local_bounds();
        let position = Vec2::new(200.0, 100.0).expect("finite position");
        let transform = ObjectTransform2d::new(
            position,
            Vec2::new(1.0, 1.0).expect("finite scale"),
            0.0,
            Vec2::new(0.5, 0.5).expect("finite anchor"),
        );

        assert!(hit_test_text_layout_bounds(position, transform, bounds));

        let outside = Vec2::new(position.x() + bounds.size.x() * 0.5 + 1.0, position.y())
            .expect("finite outside point");
        assert!(!hit_test_text_layout_bounds(outside, transform, bounds));
    }

    #[test]
    fn layout_cache_reuses_identical_layout_key() {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let mut cache = HashMap::new();
        let font = inter_font(FontWeight::Normal, FontStyle::Normal);

        let first_ptr = shape_cached(
            &mut font_system,
            &mut cache,
            "Cached Привет",
            &font,
            48.0,
            TextAlignment::Center,
        ) as *const ShapedText;
        assert_eq!(cache.len(), 1);

        let second_ptr = shape_cached(
            &mut font_system,
            &mut cache,
            "Cached Привет",
            &font,
            48.0,
            TextAlignment::Center,
        ) as *const ShapedText;

        assert_eq!(cache.len(), 1);
        assert_eq!(first_ptr, second_ptr);
    }

    #[test]
    fn layout_cache_key_tracks_all_layout_inputs_only() {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let mut cache = HashMap::new();
        let normal = inter_font(FontWeight::Normal, FontStyle::Normal);
        let bold = inter_font(FontWeight::Bold, FontStyle::Normal);

        let cases = [
            ("Text", &normal, 48.0, TextAlignment::Left),
            ("Text changed", &normal, 48.0, TextAlignment::Left),
            ("Text", &normal, 49.0, TextAlignment::Left),
            ("Text", &normal, 48.0, TextAlignment::Right),
            ("Text", &bold, 48.0, TextAlignment::Left),
        ];

        for (text, font, size, alignment) in cases {
            let _ = shape_cached(&mut font_system, &mut cache, text, font, size, alignment);
        }

        assert_eq!(cache.len(), cases.len());
    }

    #[test]
    fn system_font_family_cache_is_sorted_and_unique() {
        let font_system = FontSystem::new();
        let families = enumerate_system_font_families(&font_system);

        assert!(families.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn composition_font_system_caches_system_families_before_bundled_inter_install() {
        let expected = enumerate_system_font_families(&FontSystem::new());
        let (_, cached) = create_composition_font_system_and_cache();

        assert_eq!(cached, expected);
    }

    #[test]
    fn bundled_inter_replaces_system_inter_with_binary_faces() {
        let (font_system, _) = create_composition_font_system_and_cache();
        let faces: Vec<_> = font_system
            .db()
            .faces()
            .filter(|face| {
                face.families
                    .iter()
                    .any(|(family, _)| family == COMPOSITION_FALLBACK_FAMILY)
            })
            .collect();

        assert_eq!(faces.len(), 2);
        assert!(
            faces
                .iter()
                .all(|face| matches!(&face.source, Source::Binary(_)))
        );
        assert!(faces.iter().any(|face| face.style == Style::Normal));
        assert!(faces.iter().any(|face| face.style == Style::Italic));
    }

    #[test]
    fn bundled_inter_is_queryable_for_supported_styles_and_weights() {
        let (font_system, _) = create_composition_font_system_and_cache();
        let families = [Family::Name(COMPOSITION_FALLBACK_FAMILY)];

        for style in [Style::Normal, Style::Italic] {
            for weight in [
                Weight::THIN,
                Weight::EXTRA_LIGHT,
                Weight::LIGHT,
                Weight::NORMAL,
                Weight::MEDIUM,
                Weight::SEMIBOLD,
                Weight::BOLD,
                Weight::EXTRA_BOLD,
                Weight::BLACK,
            ] {
                let face = font_system.db().query(&Query {
                    families: &families,
                    weight,
                    stretch: Stretch::Normal,
                    style,
                });
                assert!(
                    face.is_some(),
                    "Inter must resolve for {style:?} {weight:?}"
                );
            }
        }
    }
}
