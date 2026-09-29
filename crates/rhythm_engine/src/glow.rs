//! Object-local Glow: bright-region mask -> separable blur -> colored additive
//! composite. Inputs and intermediates use linear-premultiplied Rgba16Float.

use std::num::NonZeroU64;

use rhythm_core::domain::LinearRgba;

use crate::{
    blur::{MAX_BLUR_RADIUS_PX, SeparableBlur},
    temporary_textures::{TemporaryTexture, TemporaryTexturePool},
};

const VERTEX_SHADER: &str = r#"
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
"#;

const THRESHOLD_SHADER: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
struct ThresholdParams {
    threshold: f32,
    padding0: f32,
    padding1: f32,
    padding2: f32,
};
@group(0) @binding(2) var<uniform> params: ThresholdParams;

@fragment
fn fs_threshold(input: VertexOutput) -> @location(0) vec4<f32> {
    let source = textureSampleLevel(source_texture, source_sampler, input.uv, 0.0);
    let alpha = clamp(source.a, 0.0, 1.0);
    if alpha <= 0.000001 {
        return vec4<f32>(0.0);
    }
    // Threshold straight linear-light luminance; retain alpha as the mask.
    let straight = source.rgb / alpha;
    let luminance = dot(straight, vec3<f32>(0.2126, 0.7152, 0.0722));
    let coverage = select(0.0, alpha, luminance >= params.threshold);
    return vec4<f32>(coverage, coverage, coverage, coverage);
}
"#;

const ADDITIVE_SHADER: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var mask_texture: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;
struct CompositeParams {
    color: vec4<f32>,
    intensity: f32,
    padding0: f32,
    padding1: f32,
    padding2: f32,
};
@group(0) @binding(3) var<uniform> params: CompositeParams;

@fragment
fn fs_composite(input: VertexOutput) -> @location(0) vec4<f32> {
    let source = textureSampleLevel(source_texture, source_sampler, input.uv, 0.0);
    let blurred = textureSampleLevel(mask_texture, source_sampler, input.uv, 0.0);
    let coverage = clamp(blurred.a, 0.0, 1.0) * params.color.a * params.intensity;
    // HDR additive RGB keeps working-space headroom; alpha remains normalized.
    let rgb = source.rgb + params.color.rgb * coverage;
    let alpha = clamp(source.a + (1.0 - source.a) * coverage, 0.0, 1.0);
    return vec4<f32>(rgb, alpha);
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowError {
    InvalidRadius,
    InvalidIntensity,
    InvalidThreshold,
    InvalidColor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlowParameters {
    pub radius_px: f32,
    pub intensity: f32,
    pub threshold: f32,
    pub color: LinearRgba,
}

impl GlowParameters {
    pub fn validate(self) -> Result<Self, GlowError> {
        if !self.radius_px.is_finite() || !(0.0..=MAX_BLUR_RADIUS_PX).contains(&self.radius_px) {
            return Err(GlowError::InvalidRadius);
        }
        if !self.intensity.is_finite() || !(0.0..=4.0).contains(&self.intensity) {
            return Err(GlowError::InvalidIntensity);
        }
        if !self.threshold.is_finite() || !(0.0..=1.0).contains(&self.threshold) {
            return Err(GlowError::InvalidThreshold);
        }
        if !self.color.r().is_finite()
            || !self.color.g().is_finite()
            || !self.color.b().is_finite()
            || !self.color.a().is_finite()
            || !(0.0..=1.0).contains(&self.color.a())
        {
            return Err(GlowError::InvalidColor);
        }
        Ok(self)
    }
}

/// Encodes only into caller-owned command streams; the source and destination
/// must be separate checked-out textures with equal keys.
pub struct GlowResources<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub pool: &'a mut TemporaryTexturePool,
}

#[derive(Debug)]
pub struct Glow {
    threshold_layout: wgpu::BindGroupLayout,
    threshold_pipeline: wgpu::RenderPipeline,
    composite_layout: wgpu::BindGroupLayout,
    composite_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_entry(binding: u32, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(size),
        },
        count: None,
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    fragment: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Rhythm Effects Glow pipeline layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Rhythm Effects Glow filter pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn encode_pass(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    layout: &wgpu::BindGroupLayout,
    entries: &[wgpu::BindGroupEntry<'_>],
    output: &TemporaryTexture,
) {
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Rhythm Effects Glow pass input"),
        layout,
        entries,
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Rhythm Effects Glow pass"),
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
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &group, &[]);
    pass.draw(0..3, 0..1);
}

impl Glow {
    #[must_use]
    pub fn new(device: &wgpu::Device, working_format: wgpu::TextureFormat) -> Self {
        let threshold_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects Glow threshold layout"),
            entries: &[texture_entry(0), sampler_entry(1), uniform_entry(2, 16)],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects Glow additive layout"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                sampler_entry(2),
                uniform_entry(3, 32),
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Rhythm Effects Glow clamp sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let threshold_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects Glow threshold shader"),
            source: wgpu::ShaderSource::Wgsl(format!("{VERTEX_SHADER}\n{THRESHOLD_SHADER}").into()),
        });
        let additive_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects Glow additive shader"),
            source: wgpu::ShaderSource::Wgsl(format!("{VERTEX_SHADER}\n{ADDITIVE_SHADER}").into()),
        });
        let threshold_pipeline = create_pipeline(
            device,
            &threshold_layout,
            &threshold_shader,
            "fs_threshold",
            working_format,
        );
        let composite_pipeline = create_pipeline(
            device,
            &composite_layout,
            &additive_shader,
            "fs_composite",
            working_format,
        );
        Self {
            threshold_layout,
            threshold_pipeline,
            composite_layout,
            composite_pipeline,
            sampler,
        }
    }

    /// Encode threshold -> blur -> add, keeping all three inputs separate.
    /// No live scene/composition texture is sampled by an object-local effect.
    pub fn encode(
        &self,
        blur: &SeparableBlur,
        resources: &mut GlowResources<'_>,
        source: &TemporaryTexture,
        output: &TemporaryTexture,
        parameters: GlowParameters,
    ) -> Result<(), GlowError> {
        let parameters = parameters.validate()?;
        debug_assert_eq!(source.key(), output.key());

        let mask = resources.pool.acquire(resources.device, source.key());
        let blurred = resources.pool.acquire(resources.device, source.key());

        let threshold_buffer = resources.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rhythm Effects Glow threshold uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        resources
            .queue
            .write_buffer(&threshold_buffer, 0, &parameters.threshold.to_le_bytes());
        encode_pass(
            resources.device,
            resources.encoder,
            &self.threshold_pipeline,
            &self.threshold_layout,
            &[
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
                    resource: threshold_buffer.as_entire_binding(),
                },
            ],
            &mask,
        );

        blur.encode(
            resources.device,
            resources.queue,
            resources.encoder,
            resources.pool,
            (&mask, &blurred),
            parameters.radius_px,
        );
        resources.pool.release(mask);

        let mut uniform = [0_u8; 32];
        for (i, channel) in [
            parameters.color.r(),
            parameters.color.g(),
            parameters.color.b(),
            parameters.color.a(),
            parameters.intensity,
        ]
        .into_iter()
        .enumerate()
        {
            uniform[i * 4..i * 4 + 4].copy_from_slice(&channel.to_le_bytes());
        }
        let composite_buffer = resources.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rhythm Effects Glow additive uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        resources.queue.write_buffer(&composite_buffer, 0, &uniform);
        encode_pass(
            resources.device,
            resources.encoder,
            &self.composite_pipeline,
            &self.composite_layout,
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(blurred.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: composite_buffer.as_entire_binding(),
                },
            ],
            output,
        );
        resources.pool.release(blurred);
        Ok(())
    }
}

#[cfg(test)]
fn reference_additive(source: [f32; 4], mask_coverage: f32, params: GlowParameters) -> [f32; 4] {
    let coverage = mask_coverage.clamp(0.0, 1.0) * params.color.a() * params.intensity;
    [
        source[0] + params.color.r() * coverage,
        source[1] + params.color.g() * coverage,
        source[2] + params.color.b() * coverage,
        (source[3] + (1.0 - source[3]) * coverage).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::{GlowError, GlowParameters, reference_additive};
    use rhythm_core::domain::LinearRgba;

    fn params() -> GlowParameters {
        GlowParameters {
            radius_px: 16.0,
            intensity: 1.0,
            threshold: 0.5,
            color: LinearRgba::new(0.8, 0.3, 0.1, 0.75).expect("color"),
        }
    }

    #[test]
    fn reject_invalid_semantic_parameters_before_gpu_acquisition() {
        assert_eq!(params().validate(), Ok(params()));
        assert_eq!(
            GlowParameters {
                radius_px: 129.0,
                ..params()
            }
            .validate(),
            Err(GlowError::InvalidRadius)
        );
        assert_eq!(
            GlowParameters {
                intensity: f32::NAN,
                ..params()
            }
            .validate(),
            Err(GlowError::InvalidIntensity)
        );
        assert_eq!(
            GlowParameters {
                intensity: 4.01,
                ..params()
            }
            .validate(),
            Err(GlowError::InvalidIntensity)
        );
        assert_eq!(
            GlowParameters {
                threshold: -0.01,
                ..params()
            }
            .validate(),
            Err(GlowError::InvalidThreshold)
        );
        assert_eq!(
            GlowParameters {
                threshold: f32::INFINITY,
                ..params()
            }
            .validate(),
            Err(GlowError::InvalidThreshold)
        );
    }

    #[test]
    fn intensity_zero_and_transparent_mask_are_exact_identity() {
        let source = [0.2, 0.1, 0.0, 0.5];
        assert_eq!(
            reference_additive(
                source,
                1.0,
                GlowParameters {
                    intensity: 0.0,
                    ..params()
                }
            ),
            source
        );
        assert_eq!(reference_additive(source, 0.0, params()), source);
    }

    #[test]
    fn colored_glow_adds_linear_rgb_and_keeps_alpha_normalized() {
        let original = [0.2, 0.1, 0.0, 0.5];
        let result = reference_additive(original, 1.0, params());
        assert!((result[0] - 0.8).abs() < 0.00001);
        assert!((result[1] - 0.325).abs() < 0.00001);
        assert!((result[3] - 0.875).abs() < 0.00001);
        let boosted = reference_additive(
            original,
            1.0,
            GlowParameters {
                intensity: 4.0,
                ..params()
            },
        );
        assert!(
            boosted[0] > 1.0,
            "linear working color retains HDR headroom"
        );
        assert_eq!(boosted[3], 1.0, "alpha stays normalized");
    }

    #[test]
    fn threshold_uses_straight_linear_light_but_mask_is_alpha_weighted() {
        let alpha = 0.5_f32;
        let straight = [0.8_f32, 0.1, 0.0];
        let luminance = 0.2126 * straight[0] + 0.7152 * straight[1];
        assert!(luminance > 0.2);
        assert_eq!(if luminance >= 0.2 { alpha } else { 0.0 }, alpha);
        assert!(super::THRESHOLD_SHADER.contains("source.rgb / alpha"));
        assert!(super::THRESHOLD_SHADER.contains("luminance >= params.threshold"));
    }
}
