#![forbid(unsafe_code)]

pub mod audio;
pub mod image_decode;
pub mod renderer;
pub mod runtime_assets;
pub mod scene_eval;
pub mod text;
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
