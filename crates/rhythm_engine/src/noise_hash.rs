//! Stateless, reproducible Noise hash. No wall clock, frame counter or RNG
//! state may influence a pixel. The WGSL and Rust reference share constants.

/// Embed this helper in the Noise shader (AI-265) to avoid temporal state.
pub const NOISE_HASH_WGSL: &str = r#"
fn rhythm_noise_hash(seed: u32, evolution: f32, pixel: vec2<u32>) -> u32 {
    var hash = seed ^ bitcast<u32>(evolution)
        ^ (pixel.x * 0x9e3779b9u) ^ (pixel.y * 0x85ebca6bu);
    hash = hash ^ (hash >> 16u);
    hash = hash * 0x7feb352du;
    hash = hash ^ (hash >> 15u);
    hash = hash * 0x846ca68bu;
    hash = hash ^ (hash >> 16u);
    return hash;
}

fn rhythm_unit_noise(seed: u32, evolution: f32, pixel: vec2<u32>) -> f32 {
    return f32(rhythm_noise_hash(seed, evolution, pixel) & 0x00ffffffu)
        / 16777215.0;
}
"#;

/// Portable 32-bit wrapping integer avalanche, matching the WGSL helper.
#[must_use]
pub fn noise_hash(seed: u32, evolution: f32, x: u32, y: u32) -> u32 {
    let mut hash = seed
        ^ evolution.to_bits()
        ^ x.wrapping_mul(0x9e37_79b9)
        ^ y.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846c_a68b);
    hash ^= hash >> 16;
    hash
}

/// A reproducible unit-range value from the lower 24 hash bits (representable
/// exactly as an integer f32 before normalization). Coordinates are integral
/// working-space pixel/grid positions supplied by the effect shader.
#[must_use]
pub fn deterministic_noise(seed: u32, evolution: f32, x: u32, y: u32) -> f32 {
    ((noise_hash(seed, evolution, x, y) & 0x00ff_ffff) as f32) / 16_777_215.0
}

#[cfg(test)]
mod tests {
    use super::{NOISE_HASH_WGSL, deterministic_noise, noise_hash};

    #[test]
    fn hash_has_stable_known_vectors_matching_wgsl_constants() {
        assert_eq!(noise_hash(0, 0.0, 0, 0), 0);
        assert_eq!(noise_hash(7, 0.5, 10, 20), 0xa67e_03fa);
        assert_eq!(noise_hash(7, 0.5, 11, 20), 0xffd9_e45f);
        assert_eq!(noise_hash(8, 0.5, 10, 20), 0x60d5_4fbf);
        assert_eq!(noise_hash(7, 0.6, 10, 20), 0x681f_e817);
    }

    #[test]
    fn noise_is_deterministic_regardless_of_iteration_order_and_bounded() {
        let coordinates = [(3, 4), (100, 100), (3, 5), (0, 0), (3, 4)];
        let first: Vec<_> = coordinates
            .iter()
            .map(|&(x, y)| deterministic_noise(777, 2.5, x, y))
            .collect();
        let reversed: Vec<_> = coordinates
            .iter()
            .rev()
            .map(|&(x, y)| deterministic_noise(777, 2.5, x, y))
            .collect();
        assert_eq!(first[0], first[4]);
        assert_eq!(first[0], reversed[0]);
        for value in first {
            assert!((0.0..=1.0).contains(&value));
        }
        assert_ne!(
            deterministic_noise(777, 2.5, 3, 4),
            deterministic_noise(777, 2.75, 3, 4)
        );
        assert_ne!(
            deterministic_noise(777, 2.5, 3, 4),
            deterministic_noise(778, 2.5, 3, 4)
        );
    }

    #[test]
    fn shader_hash_uses_integer_coordinates_and_evolution_bits_only() {
        assert!(NOISE_HASH_WGSL.contains("bitcast<u32>(evolution)"));
        assert!(NOISE_HASH_WGSL.contains("pixel.x * 0x9e3779b9u"));
        assert!(NOISE_HASH_WGSL.contains("pixel.y * 0x85ebca6bu"));
        assert!(!NOISE_HASH_WGSL.contains("time"));
        assert!(!NOISE_HASH_WGSL.contains("random"));
    }
}
