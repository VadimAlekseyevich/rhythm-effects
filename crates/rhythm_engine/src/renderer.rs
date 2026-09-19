//! GPU renderer boundary for Rhythm Effects.
//!
//! This module owns renderer/backend concerns inside `rhythm_engine`.
//! `rhythm_core` stays independent from wgpu and other graphics APIs.

/// Marker type for the renderer subsystem.
///
/// GPU instance/device/surface ownership is introduced by the following
/// implementation tasks. Keeping the type here establishes the dependency
/// boundary before backend state is added.
#[derive(Debug, Default)]
pub struct Renderer {
    _private: (),
}

impl Renderer {
    #[must_use]
    pub const fn new_uninitialized() -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
mod tests {
    use super::Renderer;

    #[test]
    fn renderer_boundary_can_be_constructed_without_core_gpu_types() {
        let _renderer = Renderer::new_uninitialized();
    }
}
