//! GPU renderer boundary for Rhythm Effects.
//!
//! This module owns renderer/backend concerns inside `rhythm_engine`.
//! `rhythm_core` stays independent from wgpu and other graphics APIs.

pub const INITIAL_COMPOSITION_WIDTH: u32 = 1920;
pub const INITIAL_COMPOSITION_HEIGHT: u32 = 1080;
pub const COMPOSITION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[derive(Debug)]
pub struct Renderer {
    _composition_texture: wgpu::Texture,
    composition_view: wgpu::TextureView,
    composition_size: [u32; 2],
}

impl Renderer {
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let composition_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Rhythm Effects composition"),
            size: wgpu::Extent3d {
                width: INITIAL_COMPOSITION_WIDTH,
                height: INITIAL_COMPOSITION_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COMPOSITION_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let composition_view =
            composition_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _composition_texture: composition_texture,
            composition_view,
            composition_size: [INITIAL_COMPOSITION_WIDTH, INITIAL_COMPOSITION_HEIGHT],
        }
    }

    #[must_use]
    pub const fn composition_size(&self) -> [u32; 2] {
        self.composition_size
    }

    #[must_use]
    pub const fn composition_format(&self) -> wgpu::TextureFormat {
        COMPOSITION_FORMAT
    }

    pub fn clear_composition(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Rhythm Effects composition clear encoder"),
        });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rhythm Effects composition clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composition_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::{COMPOSITION_FORMAT, INITIAL_COMPOSITION_HEIGHT, INITIAL_COMPOSITION_WIDTH};

    #[test]
    fn initial_composition_contract_is_1080p_rgba16float() {
        assert_eq!(INITIAL_COMPOSITION_WIDTH, 1920);
        assert_eq!(INITIAL_COMPOSITION_HEIGHT, 1080);
        assert_eq!(COMPOSITION_FORMAT, wgpu::TextureFormat::Rgba16Float);
    }
}
