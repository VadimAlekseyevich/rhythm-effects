//! Reusable renderer-owned targets for multipass composition/effects.
//! Checked-out textures are owned by the caller and cannot be handed out twice.

use wgpu::{Device, Texture, TextureFormat, TextureUsages, TextureView};

/// A small retention bound avoids unbounded GPU memory from arbitrary sizes.
pub const MAX_FREE_TEMPORARY_TEXTURES: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TemporaryTextureKey {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub usage: TextureUsages,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporaryTextureError {
    ZeroDimension,
    EmptyUsage,
}

impl TemporaryTextureKey {
    pub fn new(
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
    ) -> Result<Self, TemporaryTextureError> {
        if width == 0 || height == 0 {
            return Err(TemporaryTextureError::ZeroDimension);
        }
        if usage.is_empty() {
            return Err(TemporaryTextureError::EmptyUsage);
        }
        Ok(Self {
            width,
            height,
            format,
            usage,
        })
    }
}

/// An owned checkout. The texture and its view stay alive while encoding
/// passes, and must not be released until all uses in that frame are encoded.
#[derive(Debug)]
pub struct TemporaryTexture {
    key: TemporaryTextureKey,
    texture: Texture,
    view: TextureView,
}

impl TemporaryTexture {
    #[must_use]
    pub const fn key(&self) -> TemporaryTextureKey {
        self.key
    }

    #[must_use]
    pub const fn texture(&self) -> &Texture {
        &self.texture
    }

    #[must_use]
    pub const fn view(&self) -> &TextureView {
        &self.view
    }
}

/// Small, bounded LIFO lookup shared by GPU pool and deterministic unit tests.
#[derive(Debug)]
struct FreeSlots<T> {
    entries: Vec<(TemporaryTextureKey, T)>,
}

impl<T> Default for FreeSlots<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> FreeSlots<T> {
    fn take(&mut self, key: TemporaryTextureKey) -> Option<T> {
        let index = self.entries.iter().rposition(|(other, _)| *other == key)?;
        Some(self.entries.swap_remove(index).1)
    }

    fn put(&mut self, key: TemporaryTextureKey, target: T) {
        if self.entries.len() < MAX_FREE_TEMPORARY_TEXTURES {
            self.entries.push((key, target));
        }
        // A saturated pool drops excess targets instead of retaining
        // unbounded textures after transient high-resolution frames.
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn estimated_bytes(&self) -> u64 {
        self.entries
            .iter()
            .map(|(key, _)| {
                let bytes_per_pixel = match key.format {
                    TextureFormat::Rgba16Float => 8_u64,
                    TextureFormat::Rgba8Unorm
                    | TextureFormat::Rgba8UnormSrgb
                    | TextureFormat::Bgra8Unorm
                    | TextureFormat::Bgra8UnormSrgb => 4_u64,
                    _ => 0_u64,
                };
                u64::from(key.width)
                    .saturating_mul(u64::from(key.height))
                    .saturating_mul(bytes_per_pixel)
            })
            .sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporaryTexturePoolStats {
    pub free: usize,
    pub allocations: usize,
    pub reuses: usize,
    pub free_estimated_bytes: u64,
}

#[derive(Debug, Default)]
pub struct TemporaryTexturePool {
    free: FreeSlots<TemporaryTexture>,
    allocations: usize,
    reuses: usize,
}

impl TemporaryTexturePool {
    /// Reuse only an exactly matching size/format/usage target. A checkout
    /// removes it from the free list, so simultaneous passes never alias.
    #[must_use]
    pub fn acquire(&mut self, device: &Device, key: TemporaryTextureKey) -> TemporaryTexture {
        if let Some(texture) = self.free.take(key) {
            self.reuses += 1;
            return texture;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Rhythm Effects pooled effect target"),
            size: wgpu::Extent3d {
                width: key.width,
                height: key.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: key.format,
            usage: key.usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.allocations += 1;
        TemporaryTexture { key, texture, view }
    }

    /// Only release after the last encoded use; the next acquisition may
    /// clear or overwrite the texture. This transfers ownership to the pool.
    pub fn release(&mut self, texture: TemporaryTexture) {
        self.free.put(texture.key, texture);
    }

    #[must_use]
    pub fn stats(&self) -> TemporaryTexturePoolStats {
        TemporaryTexturePoolStats {
            free: self.free.len(),
            allocations: self.allocations,
            reuses: self.reuses,
            free_estimated_bytes: self.free.estimated_bytes(),
        }
    }

    /// Drop retained targets (e.g. after preview size/quality switches).
    pub fn clear(&mut self) {
        self.free.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FreeSlots, MAX_FREE_TEMPORARY_TEXTURES, TemporaryTextureError, TemporaryTextureKey,
    };
    use wgpu::{TextureFormat, TextureUsages};

    fn key(size: u32, usage: TextureUsages) -> TemporaryTextureKey {
        TemporaryTextureKey::new(size, size, TextureFormat::Rgba16Float, usage).expect("valid key")
    }

    #[test]
    fn rejects_invalid_target_dimensions_and_usage() {
        assert_eq!(
            TemporaryTextureKey::new(
                0,
                8,
                TextureFormat::Rgba16Float,
                TextureUsages::RENDER_ATTACHMENT
            ),
            Err(TemporaryTextureError::ZeroDimension)
        );
        assert_eq!(
            TemporaryTextureKey::new(
                8,
                0,
                TextureFormat::Rgba16Float,
                TextureUsages::RENDER_ATTACHMENT
            ),
            Err(TemporaryTextureError::ZeroDimension)
        );
        assert_eq!(
            TemporaryTextureKey::new(8, 8, TextureFormat::Rgba16Float, TextureUsages::empty()),
            Err(TemporaryTextureError::EmptyUsage)
        );
    }

    #[test]
    fn pooled_checkout_is_exclusive_and_reuses_exact_target_key() {
        let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
        let mut slots = FreeSlots::default();
        let match_key = key(64, usage);
        let other_size = key(128, usage);
        let other_format =
            TemporaryTextureKey::new(64, 64, TextureFormat::Rgba8Unorm, usage).expect("key");
        let other_usage = key(64, TextureUsages::RENDER_ATTACHMENT);

        slots.put(match_key, 17_u32);
        assert_eq!(slots.take(other_size), None);
        assert_eq!(slots.take(other_format), None);
        assert_eq!(slots.take(other_usage), None);
        assert_eq!(slots.take(match_key), Some(17));
        assert_eq!(
            slots.take(match_key),
            None,
            "checked-out target cannot alias"
        );
        slots.put(match_key, 17);
        assert_eq!(slots.take(match_key), Some(17), "returned target is reused");
    }

    #[test]
    fn idle_pool_retention_is_bounded() {
        let mut slots = FreeSlots::default();
        let usage = TextureUsages::RENDER_ATTACHMENT;
        for value in 0..MAX_FREE_TEMPORARY_TEXTURES + 7 {
            slots.put(key(64, usage), value);
        }
        assert_eq!(slots.len(), MAX_FREE_TEMPORARY_TEXTURES);
    }
}
