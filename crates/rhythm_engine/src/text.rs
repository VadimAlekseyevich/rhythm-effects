use cosmic_text::FontSystem;

/// Long-lived composition text state.
///
/// Creating a cosmic-text FontSystem performs system-font discovery, so the
/// renderer owns exactly one instance and shares mutable access with later
/// shaping/rendering stages.
#[derive(Debug)]
pub struct TextResources {
    font_system: FontSystem,
}

impl TextResources {
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
        }
    }

    #[must_use]
    pub const fn font_system(&self) -> &FontSystem {
        &self.font_system
    }

    pub const fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }
}

impl Default for TextResources {
    fn default() -> Self {
        Self::new()
    }
}
