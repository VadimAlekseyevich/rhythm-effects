//! Transparent object isolation and premultiplied-linear compositing.
//!
//! A caller encodes object content into its own pooled target and may insert
//! effect passes before compositing. The scene composition never doubles as
//! an effect input, so neighboring objects cannot bleed into multipass filters.

use crate::{
    scene_eval::EvaluatedEffectKind,
    temporary_textures::{TemporaryTexture, TemporaryTextureKey, TemporaryTexturePool},
};

const COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0) var object_texture: texture_2d<f32>;
@group(0) @binding(1) var object_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = uvs[index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // The working format is linear, premultiplied RGBA; do NOT apply gamma
    // conversion or divide by alpha when layering an isolated object.
    return textureSample(object_texture, object_sampler, input.uv);
}
"#;

#[must_use]
pub fn effect_requires_isolation(kind: &EvaluatedEffectKind) -> bool {
    matches!(
        kind,
        EvaluatedEffectKind::Blur { .. } | EvaluatedEffectKind::Glow { .. }
    )
}

#[derive(Debug)]
pub struct IsolatedObjectCompositor {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl IsolatedObjectCompositor {
    #[must_use]
    pub fn new(device: &wgpu::Device, working_format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects isolated object composite layout"),
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
            label: Some("Rhythm Effects isolated object sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects isolated object compositor"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rhythm Effects isolated object pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rhythm Effects premultiplied object compositing"),
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
                    format: working_format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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
            layout,
            pipeline,
            sampler,
        }
    }

    /// Clear the isolated object target to transparent and let the caller
    /// draw just this object's evaluated content into the target.
    /// The returned target remains checked out through any effect passes.
    pub fn encode_object<F>(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pool: &mut TemporaryTexturePool,
        key: TemporaryTextureKey,
        draw_object: F,
    ) -> TemporaryTexture
    where
        F: FnOnce(&mut wgpu::RenderPass<'_>),
    {
        let target = pool.acquire(device, key);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rhythm Effects isolated object content"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
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
            draw_object(&mut pass);
        }
        target
    }

    /// Blend a fully processed isolated object over the existing scene in
    /// painter order. The source and target MUST be distinct textures.
    pub fn encode_composite(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &TemporaryTexture,
        composition_view: &wgpu::TextureView,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rhythm Effects isolated object input"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Rhythm Effects isolated object blend"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: composition_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::effect_requires_isolation;
    use crate::scene_eval::EvaluatedEffectKind;
    use rhythm_core::domain::LinearRgba;

    #[test]
    fn multipass_blur_and_glow_require_object_isolation() {
        assert!(effect_requires_isolation(&EvaluatedEffectKind::Blur {
            radius_px: 12.0
        }));
        assert!(effect_requires_isolation(&EvaluatedEffectKind::Glow {
            radius_px: 12.0,
            intensity: 1.0,
            threshold: 0.5,
            color: LinearRgba::black_opaque()
        }));
        assert!(!effect_requires_isolation(&EvaluatedEffectKind::Tint {
            amount: 0.25,
            color: LinearRgba::black_opaque()
        }));
        assert!(!effect_requires_isolation(&EvaluatedEffectKind::Noise {
            amount: 0.5,
            size_px: 8.0,
            evolution: 0.0,
            seed: 4
        }));
        assert!(!effect_requires_isolation(&EvaluatedEffectKind::RgbSplit {
            amount_px: 4.0,
            angle_degrees: 45.0
        }));
    }

    #[test]
    fn composite_shader_keeps_linear_premultiplied_source() {
        assert!(super::COMPOSITE_SHADER.contains("return textureSample"));
        assert!(!super::COMPOSITE_SHADER.contains("pow("));
    }
}
