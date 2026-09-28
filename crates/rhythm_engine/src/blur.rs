//! Two-pass, linear-premultiplied Gaussian-like blur in composition pixels.
//! Only one middle texture is checked out; the caller owns source/destination.

use std::num::NonZeroU64;

use crate::temporary_textures::{TemporaryTexture, TemporaryTexturePool};

pub const MAX_BLUR_RADIUS_PX: f32 = 128.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurPreviewScaleError {
    InvalidRadius,
    InvalidPreviewScale,
}

/// Spatial effect parameters are specified in full composition pixels. A
/// Half or Quarter preview adjusts the GPU sampling radius, not the Project.
pub fn preview_blur_radius(
    composition_radius_px: f32,
    preview_scale: f32,
) -> Result<f32, BlurPreviewScaleError> {
    if !composition_radius_px.is_finite()
        || !(0.0..=MAX_BLUR_RADIUS_PX).contains(&composition_radius_px)
    {
        return Err(BlurPreviewScaleError::InvalidRadius);
    }
    if !preview_scale.is_finite() || preview_scale <= 0.0 || preview_scale > 1.0 {
        return Err(BlurPreviewScaleError::InvalidPreviewScale);
    }
    Ok(composition_radius_px * preview_scale)
}

const BLUR_SHADER: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

struct BlurParams {
    radius_px: f32,
    axis: u32,
    padding0: u32,
    padding1: u32,
};
@group(0) @binding(2) var<uniform> blur: BlurParams;

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
    let radius = clamp(blur.radius_px, 0.0, 128.0);
    if radius <= 0.0 {
        return textureSampleLevel(input_texture, input_sampler, input.uv, 0.0);
    }

    let sigma = max(radius / 3.0, 0.5);
    let taps = i32(ceil(radius));
    let dimensions = vec2<f32>(textureDimensions(input_texture, 0));
    var direction = vec2<f32>(1.0, 0.0);
    if blur.axis == 1u {
        direction = vec2<f32>(0.0, 1.0);
    }
    let texel = direction / dimensions;

    var weighted = vec4<f32>(0.0);
    var sum = 0.0;
    for (var offset: i32 = -taps; offset <= taps; offset = offset + 1) {
        let distance = f32(offset);
        let normalized = distance / sigma;
        let weight = exp(-0.5 * normalized * normalized);
        weighted = weighted
            + textureSampleLevel(input_texture, input_sampler,
                                 input.uv + distance * texel, 0.0) * weight;
        sum = sum + weight;
    }
    // Convolve RGB and alpha together: input/output remain linear-premultiplied.
    return weighted / max(sum, 0.000001);
}
"#;

#[derive(Debug)]
pub struct SeparableBlur {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl SeparableBlur {
    #[must_use]
    pub fn new(device: &wgpu::Device, working_format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects blur bindings"),
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
            label: Some("Rhythm Effects blur clamp sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects separable blur shader"),
            source: wgpu::ShaderSource::Wgsl(BLUR_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rhythm Effects blur pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rhythm Effects separable blur"),
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

    /// First render horizontal into a separate pooled middle target, then
    /// vertical into output. Sampling source/output in the same pass is never
    /// allowed. The temporary is released after its final encoded use.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pool: &mut TemporaryTexturePool,
        source: &TemporaryTexture,
        output: &TemporaryTexture,
        radius_px: f32,
    ) {
        debug_assert_eq!(source.key(), output.key());
        let middle = pool.acquire(device, source.key());
        let stride = u64::from(device.limits().min_uniform_buffer_offset_alignment.max(16));
        let mut parameters = vec![0_u8; usize::try_from(stride).expect("device offset fits") + 16];
        let radius_px = radius_px.clamp(0.0, MAX_BLUR_RADIUS_PX);
        parameters[0..4].copy_from_slice(&radius_px.to_le_bytes());
        let second = usize::try_from(stride).expect("device offset fits");
        parameters[second..second + 4].copy_from_slice(&radius_px.to_le_bytes());
        parameters[second + 4..second + 8].copy_from_slice(&1_u32.to_le_bytes());

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rhythm Effects blur parameters"),
            size: stride + 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniforms, 0, &parameters);
        self.encode_one(device, encoder, source, &middle, &uniforms, 0);
        self.encode_one(device, encoder, &middle, output, &uniforms, stride);
        pool.release(middle);
    }

    fn encode_one(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &TemporaryTexture,
        output: &TemporaryTexture,
        uniforms: &wgpu::Buffer,
        offset: u64,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rhythm Effects blur pass inputs"),
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
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniforms,
                        offset,
                        size: NonZeroU64::new(16),
                    }),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Rhythm Effects blur axis pass"),
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
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
fn reference_kernel(radius_px: f32) -> Vec<f32> {
    let radius = radius_px.clamp(0.0, MAX_BLUR_RADIUS_PX);
    if radius == 0.0 {
        return vec![1.0];
    }
    let sigma = (radius / 3.0).max(0.5);
    let samples = radius.ceil() as i32;
    let mut weights: Vec<_> = (-samples..=samples)
        .map(|x| {
            let distance = x as f32 / sigma;
            (-0.5 * distance * distance).exp()
        })
        .collect();
    let total: f32 = weights.iter().sum();
    for weight in &mut weights {
        *weight /= total;
    }
    weights
}

#[cfg(test)]
mod tests {
    use super::{
        BlurPreviewScaleError, MAX_BLUR_RADIUS_PX, preview_blur_radius, reference_kernel,
    };

    #[test]
    fn preview_blur_radius_keeps_composition_pixel_semantics() {
        assert_eq!(preview_blur_radius(0.0, 0.25), Ok(0.0));
        assert_eq!(preview_blur_radius(80.0, 1.0), Ok(80.0));
        assert_eq!(preview_blur_radius(80.0, 0.5), Ok(40.0));
        assert_eq!(preview_blur_radius(80.0, 0.25), Ok(20.0));
        assert_eq!(preview_blur_radius(128.0, 0.25), Ok(32.0));
        assert_eq!(
            preview_blur_radius(f32::NAN, 1.0),
            Err(BlurPreviewScaleError::InvalidRadius)
        );
        assert_eq!(
            preview_blur_radius(129.0, 1.0),
            Err(BlurPreviewScaleError::InvalidRadius)
        );
        for scale in [0.0, -1.0, 1.01, f32::NAN, f32::INFINITY] {
            assert_eq!(
                preview_blur_radius(10.0, scale),
                Err(BlurPreviewScaleError::InvalidPreviewScale)
            );
        }
    }

    #[test]
    fn radius_zero_is_identity_and_reference_kernel_is_normalized_symmetric() {
        assert_eq!(reference_kernel(0.0), vec![1.0]);
        for radius in [0.25, 1.0, 8.0, 64.0, 128.0] {
            let kernel = reference_kernel(radius);
            assert!(kernel.len() <= 257);
            assert!((kernel.iter().sum::<f32>() - 1.0).abs() < 0.0001);
            for (a, b) in kernel.iter().zip(kernel.iter().rev()) {
                assert!((a - b).abs() < 0.0001);
            }
        }
        assert_eq!(
            reference_kernel(1000.0),
            reference_kernel(MAX_BLUR_RADIUS_PX)
        );
    }

    #[test]
    fn linear_premultiplied_convolution_preserves_zero_alpha_and_channel_bounds() {
        let weights = reference_kernel(2.0);
        let center = weights.len() / 2;
        let mut output = [0.0_f32; 4];
        for (i, weight) in weights.iter().enumerate() {
            let sample = if i == center {
                [0.25_f32, 0.10_f32, 0.05_f32, 0.5_f32]
            } else {
                [0.0_f32; 4]
            };
            for channel in 0..4 {
                output[channel] += sample[channel] * weight;
            }
        }
        assert!(output[0] <= output[3]);
        assert!(output[1] <= output[3]);
        assert!(output[2] <= output[3]);
        assert!(output[3] > 0.0);
        assert!(super::BLUR_SHADER.contains("weighted / max(sum"));
    }
}
