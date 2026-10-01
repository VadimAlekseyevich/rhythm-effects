#![forbid(unsafe_code)]

pub mod audio;
pub mod av_sync_fixture;
pub mod benchmark_fixtures;
pub mod blur;
pub mod effect_chain;
pub mod export_ffmpeg;
pub mod export_job;
pub mod export_progress;
pub mod export_publication;
pub mod export_readback;
pub mod export_renderer;
pub mod export_resolution;
pub mod export_resources;
pub mod export_sdr;
pub mod export_timeline;
pub mod glow;
pub mod image_decode;
pub mod isolated_objects;
pub mod noise;
pub mod noise_hash;
pub mod reference_scenes;
pub mod renderer;
pub mod rgb_split;
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
mod export_acceptance;

#[cfg(test)]
mod preview_effect_parity;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_status_is_available() {
        assert_eq!(status(), "engine scaffold ready");
    }
}
