//! Tint in linear light on premultiplied working textures.
//! Alpha is preserved; no transparent colored fringe can be introduced.

use std::num::NonZeroU64;

use rhythm_core::domain::LinearRgba;

use crate::temporary_textures::TemporaryTexture;

const TINT_SHADER: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct TintParams {
    color: vec4<f32>,
    amount: f32,
    padding0: f32,
    padding1: f32,
    padding2: f32,
};
@group(0) @binding(2) var<uniform> params: TintParams;

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
    let source = textureSampleLevel(source_texture, source_sampler, input.uv, 0.0);
    let factor = clamp(params.amount, 0.0, 1.0) * params.color.a;
    if factor <= 0.0 || source.a <= 0.0 {
        // Zero amount and fully transparent edges are exact identity.
        return source;
    }
    // Work in premultiplied linear RGB: mixing with alpha-weighted tint
    // is equivalent to tinting the straight color and premultiplying again.
    let tinted_rgb = mix(source.rgb, params.color.rgb * source.a, factor);
    return vec4<f32>(tinted_rgb, source.a);
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TintError {
    InvalidAmount,
    InvalidColor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TintParameters {
    pub color: LinearRgba,
    pub amount: f32,
}

impl TintParameters {
    pub fn validate(self) -> Result<Self, TintError> {
        if !self.amount.is_finite() || !(0.0..=1.0).contains(&self.amount) {
            return Err(TintError::InvalidAmount);
        }
        if !self.color.r().is_finite()
            || !self.color.g().is_finite()
            || !self.color.b().is_finite()
            || !self.color.a().is_finite()
            || !(0.0..=1.0).contains(&self.color.a())
        {
            return Err(TintError::InvalidColor);
        }
        Ok(self)
    }
}

#[derive(Debug)]
pub struct Tint {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl Tint {
    #[must_use]
    pub fn new(device: &wgpu::Device, working_format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects Tint bind group layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(32),
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Rhythm Effects Tint clamp sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects Tint linear shader"),
            source: wgpu::ShaderSource::Wgsl(TINT_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rhythm Effects Tint pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rhythm Effects premultiplied Tint"),
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
            layout,
            pipeline,
            sampler,
        }
    }

    /// Never samples from the output target while it is being written.
    /// Source and output are distinct pooled checkouts with the same key.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: TintParameters,
    ) -> Result<(), TintError> {
        let parameters = parameters.validate()?;
        let (source, output) = targets;
        debug_assert_eq!(source.key(), output.key());

        let mut bytes = [0_u8; 32];
        for (index, channel) in [
            parameters.color.r(),
            parameters.color.g(),
            parameters.color.b(),
            parameters.color.a(),
            parameters.amount,
        ]
        .into_iter()
        .enumerate()
        {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&channel.to_le_bytes());
        }
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rhythm Effects Tint parameters"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform, 0, &bytes);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rhythm Effects Tint source"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Rhythm Effects Tint pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output.view(),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[cfg(test)]
fn reference_tint(source: [f32; 4], parameters: TintParameters) -> [f32; 4] {
    let factor = parameters.amount * parameters.color.a();
    if factor <= 0.0 || source[3] <= 0.0 {
        return source;
    }
    [
        source[0] * (1.0 - factor) + parameters.color.r() * source[3] * factor,
        source[1] * (1.0 - factor) + parameters.color.g() * source[3] * factor,
        source[2] * (1.0 - factor) + parameters.color.b() * source[3] * factor,
        source[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::{TintError, TintParameters, reference_tint};
    use rhythm_core::domain::LinearRgba;

    fn params(amount: f32) -> TintParameters {
        TintParameters {
            color: LinearRgba::new(1.0, 0.0, 0.0, 1.0).expect("finite color"),
            amount,
        }
    }

    #[test]
    fn rejects_out_of_range_and_nonfinite_amount_before_encoding() {
        assert_eq!(params(0.0).validate(), Ok(params(0.0)));
        assert_eq!(params(1.0).validate(), Ok(params(1.0)));
        for amount in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            assert_eq!(params(amount).validate(), Err(TintError::InvalidAmount));
        }
    }

    #[test]
    fn amount_zero_is_exact_identity_and_full_amount_preserves_source_alpha() {
        let source = [0.1, 0.3, 0.2, 0.5];
        assert_eq!(reference_tint(source, params(0.0)), source);
        assert_eq!(reference_tint(source, params(1.0)), [0.5, 0.0, 0.0, 0.5]);
        assert_eq!(reference_tint(source, params(0.5)), [0.3, 0.15, 0.1, 0.5]);
    }

    #[test]
    fn fully_transparent_edges_remain_zero_without_colored_fringe() {
        assert_eq!(reference_tint([0.0; 4], params(1.0)), [0.0; 4]);
        assert!(super::TINT_SHADER.contains("source.a <= 0.0"));
        assert!(super::TINT_SHADER.contains("vec4<f32>(tinted_rgb, source.a)"));
    }

    #[test]
    fn color_alpha_modulates_tint_strength_without_changing_output_alpha() {
        let input = [0.0, 0.25, 0.0, 0.5];
        let half_opacity = TintParameters {
            color: LinearRgba::new(1.0, 0.0, 0.0, 0.5).expect("color"),
            amount: 1.0,
        };
        assert_eq!(reference_tint(input, half_opacity), [0.25, 0.125, 0.0, 0.5]);
        assert_eq!(
            reference_tint(
                input,
                TintParameters {
                    color: LinearRgba::new(1.0, 0.0, 0.0, 0.0).expect("transparent tint"),
                    amount: 1.0,
                }
            ),
            input
        );
    }
}
