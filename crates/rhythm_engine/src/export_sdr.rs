//! Offscreen SDR sRGB RGBA8 output, separate from the premultiplied UI
//! preview. Both use the same creative linear-to-sRGB transfer function.

use crate::renderer::{PREVIEW_DISPLAY_FORMAT, PREVIEW_SHADER};

const PREMULTIPLIED_RESULT: &str = "return vec4<f32>(straight_srgb * alpha, alpha);";
const STRAIGHT_EXPORT_RESULT: &str =
    "return vec4<f32>(clamp(straight_srgb, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);";

fn export_sdr_shader() -> String {
    // Keep the shared transfer and unpremultiplication math exactly the same
    // as the preview, but hand FFmpeg straight (not premultiplied) SDR RGBA.
    assert!(PREVIEW_SHADER.contains(PREMULTIPLIED_RESULT));
    PREVIEW_SHADER.replace(PREMULTIPLIED_RESULT, STRAIGHT_EXPORT_RESULT)
}

#[derive(Debug)]
pub struct SdrExportConverter {
    texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    size: [u32; 2],
}

impl SdrExportConverter {
    /// The output texture is RGBA8 UNORM (not RGBA8-sRGB: gamma is encoded
    /// explicitly in WGSL). COPY_SRC permits subsequent bounded readback.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        composition_view: &wgpu::TextureView,
        size: [u32; 2],
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Rhythm Effects export SDR RGBA8"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PREVIEW_DISPLAY_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects export SDR input layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Rhythm Effects export SDR sampling"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rhythm Effects export SDR source"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(composition_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects export SDR transfer shader"),
            source: wgpu::ShaderSource::Wgsl(export_sdr_shader().into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rhythm Effects export SDR pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rhythm Effects export straight SDR RGBA"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: PREVIEW_DISPLAY_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            texture,
            _view: view,
            bind_group,
            pipeline,
            size,
        }
    }

    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.size
    }

    #[must_use]
    pub const fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Encode after composing the entire frame in linear Rgba16Float;
    /// the export source and destination textures are distinct.
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let view = self.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Rhythm Effects export SDR conversion"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::{PREMULTIPLIED_RESULT, STRAIGHT_EXPORT_RESULT, export_sdr_shader};
    use crate::renderer::{PREVIEW_DISPLAY_FORMAT, PREVIEW_SHADER};

    #[test]
    fn export_and_preview_share_transfer_function_but_not_alpha_convention() {
        let export = export_sdr_shader();
        assert!(PREVIEW_SHADER.contains(PREMULTIPLIED_RESULT));
        assert!(export.contains(STRAIGHT_EXPORT_RESULT));
        assert!(!export.contains(PREMULTIPLIED_RESULT));
        assert!(export.contains("linear_to_srgb_channel"));
        assert!(export.contains("0.0031308"));
        assert!(export.contains("1.055 * pow(value, 1.0 / 2.4) - 0.055"));
        assert_eq!(PREVIEW_DISPLAY_FORMAT, wgpu::TextureFormat::Rgba8Unorm);
    }

    #[test]
    fn zero_alpha_keeps_zero_rgb_and_output_is_clamped_to_sdr() {
        let export = export_sdr_shader();
        assert!(export.contains("var straight_linear = vec3<f32>(0.0)"));
        assert!(export.contains("if alpha > 0.00001"));
        assert!(export.contains("clamp(straight_srgb, vec3<f32>(0.0), vec3<f32>(1.0))"));
    }
}
