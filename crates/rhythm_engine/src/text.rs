use std::{collections::BTreeSet, fmt};

use cosmic_text::FontSystem;

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
        enumerate_system_font_families,
    };

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
