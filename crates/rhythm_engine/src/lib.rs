#![forbid(unsafe_code)]

pub mod audio;
pub mod blur;
pub mod effect_chain;
pub mod glow;
pub mod image_decode;
pub mod isolated_objects;
pub mod noise;
pub mod noise_hash;
pub mod renderer;
pub mod runtime_assets;
pub mod scene_eval;
pub mod temporary_textures;
pub mod text;
pub mod tint;
pub mod waveform;

#[must_use]
pub const fn status() -> &'static str {
    "engine scaffold ready"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_status_is_available() {
        assert_eq!(status(), "engine scaffold ready");
    }
}
