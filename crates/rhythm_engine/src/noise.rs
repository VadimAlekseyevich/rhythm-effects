//! Stateless monochrome Noise in linear-premultiplied working space.
//! Pixel-block size is a creative composition-pixel parameter, not an RNG step.

use std::num::NonZeroU64;

use crate::{noise_hash::NOISE_HASH_WGSL, temporary_textures::TemporaryTexture};

#[cfg(test)]
use crate::noise_hash::deterministic_noise;

const NOISE_SHADER: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct NoiseParams {
    evolution: f32,
    amount: f32,
    size_px: f32,
    seed: u32,
};
@group(0) @binding(2) var<uniform> params: NoiseParams;

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
    if params.amount <= 0.0 || source.a <= 0.0 {
        return source;
    }

    let dimensions = vec2<f32>(textureDimensions(source_texture, 0));
    let pixel = clamp(
        floor(input.uv * dimensions),
        vec2<f32>(0.0),
        dimensions - vec2<f32>(1.0),
    );
    // All pixels in one size_px square get identical seeded noise.
    let cell = vec2<u32>(floor(pixel / max(params.size_px, 1.0)));
    let centered = rhythm_unit_noise(params.seed, params.evolution, cell) * 2.0 - 1.0;
    let rgb = source.rgb + vec3<f32>(centered * params.amount * source.a);
    return vec4<f32>(rgb, source.a);
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseError {
    InvalidAmount,
    InvalidSize,
    InvalidEvolution,
    InvalidPreviewScale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseParameters {
    pub amount: f32,
    pub size_px: f32,
    pub evolution: f32,
    pub seed: u32,
}

impl NoiseParameters {
    pub fn validate(self) -> Result<Self, NoiseError> {
        if !self.amount.is_finite() || !(0.0..=1.0).contains(&self.amount) {
            return Err(NoiseError::InvalidAmount);
        }
        if !self.size_px.is_finite() || !(1.0..=256.0).contains(&self.size_px) {
            return Err(NoiseError::InvalidSize);
        }
        if !self.evolution.is_finite() {
            return Err(NoiseError::InvalidEvolution);
        }
        Ok(self)
    }
}

/// Stable integer grid cell; all working pixels in one block share one hash.
pub fn noise_cell(x: u32, y: u32, size_px: f32) -> Result<(u32, u32), NoiseError> {
    if !size_px.is_finite() || !(1.0..=256.0).contains(&size_px) {
        return Err(NoiseError::InvalidSize);
    }
    Ok((
        ((x as f32) / size_px).floor() as u32,
        ((y as f32) / size_px).floor() as u32,
    ))
}

/// Convert a composition-sized Noise block to the resolved working scale.
/// Blocks smaller than one physical preview pixel are represented as one pixel.
pub fn preview_noise_parameters(
    original: NoiseParameters,
    preview_scale: f32,
) -> Result<NoiseParameters, NoiseError> {
    let original = original.validate()?;
    if !preview_scale.is_finite() || preview_scale <= 0.0 || preview_scale > 1.0 {
        return Err(NoiseError::InvalidPreviewScale);
    }
    Ok(NoiseParameters {
        size_px: (original.size_px * preview_scale).max(1.0),
        ..original
    })
}

#[derive(Debug)]
pub struct Noise {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl Noise {
    #[must_use]
    pub fn new(device: &wgpu::Device, working_format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects Noise bind group layout"),
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
                        min_binding_size: NonZeroU64::new(16),
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Rhythm Effects Noise clamp sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects stateless Noise shader"),
            source: wgpu::ShaderSource::Wgsl(format!("{NOISE_HASH_WGSL}\n{NOISE_SHADER}").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rhythm Effects Noise pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rhythm Effects Noise pass"),
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

    /// Encode one pass from a separate source to destination checkout.
    /// Parameters are validated before allocating uniforms or encoding GPU work.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: NoiseParameters,
    ) -> Result<(), NoiseError> {
        let parameters = parameters.validate()?;
        let (source, output) = targets;
        debug_assert_eq!(source.key(), output.key());
        let mut bytes = [0_u8; 16];
        bytes[0..4].copy_from_slice(&parameters.evolution.to_le_bytes());
        bytes[4..8].copy_from_slice(&parameters.amount.to_le_bytes());
        bytes[8..12].copy_from_slice(&parameters.size_px.to_le_bytes());
        bytes[12..16].copy_from_slice(&parameters.seed.to_le_bytes());
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rhythm Effects Noise parameters"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform, 0, &bytes);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rhythm Effects Noise source"),
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
            label: Some("Rhythm Effects Noise pass"),
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
fn reference_noise(source: [f32; 4], cell: (u32, u32), p: NoiseParameters) -> [f32; 4] {
    if p.amount == 0.0 || source[3] == 0.0 {
        return source;
    }
    let centered = deterministic_noise(p.seed, p.evolution, cell.0, cell.1) * 2.0 - 1.0;
    let delta = centered * p.amount * source[3];
    [
        source[0] + delta,
        source[1] + delta,
        source[2] + delta,
        source[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        NoiseError, NoiseParameters, noise_cell, preview_noise_parameters, reference_noise,
    };

    fn parameters() -> NoiseParameters {
        NoiseParameters {
            amount: 0.5,
            size_px: 8.0,
            evolution: 2.5,
            seed: 777,
        }
    }

    #[test]
    fn amount_zero_and_transparent_input_are_exact_identity() {
        let input = [0.2, 0.1, 0.0, 0.5];
        assert_eq!(
            reference_noise(
                input,
                (3, 4),
                NoiseParameters {
                    amount: 0.0,
                    ..parameters()
                }
            ),
            input
        );
        assert_eq!(reference_noise([0.0; 4], (3, 4), parameters()), [0.0; 4]);
    }

    #[test]
    fn size_groups_pixels_into_stable_spatial_blocks() {
        assert_eq!(noise_cell(0, 0, 8.0), Ok((0, 0)));
        assert_eq!(noise_cell(7, 7, 8.0), Ok((0, 0)));
        assert_eq!(noise_cell(8, 7, 8.0), Ok((1, 0)));
        assert_eq!(noise_cell(7, 8, 8.0), Ok((0, 1)));
        assert_eq!(
            reference_noise(
                [0.2, 0.2, 0.2, 0.5],
                noise_cell(0, 0, 8.0).expect("cell"),
                parameters()
            ),
            reference_noise(
                [0.2, 0.2, 0.2, 0.5],
                noise_cell(7, 7, 8.0).expect("cell"),
                parameters()
            )
        );
    }

    #[test]
    fn changing_amount_changes_only_premultiplied_rgb_not_alpha() {
        let input = [0.2, 0.2, 0.2, 0.5];
        let half = reference_noise(input, (2, 3), parameters());
        let full = reference_noise(
            input,
            (2, 3),
            NoiseParameters {
                amount: 1.0,
                ..parameters()
            },
        );
        assert_eq!(half[3], input[3]);
        assert_eq!(full[3], input[3]);
        for channel in 0..3 {
            assert!(
                (full[channel] - input[channel] - 2.0 * (half[channel] - input[channel])).abs()
                    < 0.000001
            );
        }
    }

    #[test]
    fn preview_size_tracks_composition_pixels_with_one_pixel_floor() {
        assert_eq!(
            preview_noise_parameters(parameters(), 1.0)
                .expect("full")
                .size_px,
            8.0
        );
        assert_eq!(
            preview_noise_parameters(parameters(), 0.5)
                .expect("half")
                .size_px,
            4.0
        );
        assert_eq!(
            preview_noise_parameters(parameters(), 0.25)
                .expect("quarter")
                .size_px,
            2.0
        );
        assert_eq!(
            preview_noise_parameters(
                NoiseParameters {
                    size_px: 1.0,
                    ..parameters()
                },
                0.25
            )
            .expect("minimum size")
            .size_px,
            1.0
        );
    }

    #[test]
    fn validates_all_semantic_noise_parameters_before_encoding() {
        assert_eq!(parameters().validate(), Ok(parameters()));
        for amount in [-0.1, 1.1, f32::NAN] {
            assert_eq!(
                NoiseParameters {
                    amount,
                    ..parameters()
                }
                .validate(),
                Err(NoiseError::InvalidAmount)
            );
        }
        for size_px in [0.0, 257.0, f32::INFINITY] {
            assert_eq!(
                NoiseParameters {
                    size_px,
                    ..parameters()
                }
                .validate(),
                Err(NoiseError::InvalidSize)
            );
        }
        assert_eq!(
            NoiseParameters {
                evolution: f32::NAN,
                ..parameters()
            }
            .validate(),
            Err(NoiseError::InvalidEvolution)
        );
        for scale in [0.0, 1.01, f32::NAN] {
            assert_eq!(
                preview_noise_parameters(parameters(), scale),
                Err(NoiseError::InvalidPreviewScale)
            );
        }
    }
}
