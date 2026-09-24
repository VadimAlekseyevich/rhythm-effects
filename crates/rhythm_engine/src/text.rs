use std::{collections::BTreeSet, fmt};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use rhythm_core::project::{FontReference, FontStyle, FontWeight};

pub const COMPOSITION_FALLBACK_FAMILY: &str = "Inter";

const INTER_VARIABLE: &[u8] = include_bytes!("../assets/fonts/inter/InterVariable.ttf");
const INTER_VARIABLE_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/inter/InterVariable-Italic.ttf");

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

fn shape_with_font_system(
    font_system: &mut FontSystem,
    text: &str,
    font: &FontReference,
    font_size: f32,
) -> ShapedText {
    let metrics = Metrics::new(font_size, font_size);
    let attrs = Attrs::new()
        .family(Family::Name(&font.family))
        .weight(cosmic_weight(font.weight))
        .style(cosmic_style(font.style));
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_text(text, &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);

    ShapedText { buffer }
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
}

impl ShapedText {
    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.buffer.layout_runs().map(|run| run.glyphs.len()).sum()
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

    pub const fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    #[must_use]
    pub fn system_font_families(&self) -> &[String] {
        &self.system_font_families
    }

    #[must_use]
    pub fn shape_text(&mut self, text: &str, font: &FontReference, font_size: f32) -> ShapedText {
        shape_with_font_system(&mut self.font_system, text, font, font_size)
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
    use cosmic_text::{
        FontSystem,
        fontdb::{Family, Query, Source, Stretch, Style, Weight},
    };

    use super::{
        COMPOSITION_FALLBACK_FAMILY, create_composition_font_system_and_cache,
        enumerate_system_font_families, shape_with_font_system,
    };
    use rhythm_core::project::{FontReference, FontStyle, FontWeight};

    fn inter_font(weight: FontWeight, style: FontStyle) -> FontReference {
        FontReference {
            family: COMPOSITION_FALLBACK_FAMILY.to_owned(),
            weight,
            style,
        }
    }

    fn assert_shapes_without_missing_glyphs(text: &str, font: &FontReference) {
        let (mut font_system, _) = create_composition_font_system_and_cache();
        let shaped = shape_with_font_system(&mut font_system, text, font, 48.0);

        assert!(shaped.glyph_count() > 0);
        assert_eq!(shaped.missing_glyph_count(), 0);
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
