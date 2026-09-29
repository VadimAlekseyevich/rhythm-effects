//! RGB Split with deterministic signed channel displacement in composition
//! pixels. Alpha follows the original center pixel, never a shifted channel.

use std::num::NonZeroU64;

use crate::temporary_textures::TemporaryTexture;

const RGB_SPLIT_SHADER: &str = r#"
@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct SplitParams {
    amount_px: f32,
    angle_degrees: f32,
    padding0: f32,
    padding1: f32,
};
@group(0) @binding(2) var<uniform> params: SplitParams;

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
    let center = textureSampleLevel(source_texture, source_sampler, input.uv, 0.0);
    if params.amount_px <= 0.0 || center.a <= 0.0 {
        return center;
    }
    let radians = params.angle_degrees * (3.141592653589793 / 180.0);
    let direction = vec2<f32>(cos(radians), sin(radians));
    let texel_delta = direction * params.amount_px
        / vec2<f32>(textureDimensions(source_texture, 0));

    let red_sample = textureSampleLevel(
        source_texture, source_sampler, input.uv + texel_delta, 0.0
    );
    let blue_sample = textureSampleLevel(
        source_texture, source_sampler, input.uv - texel_delta, 0.0
    );

    // Displaced texture samples are premultiplied. Extract straight channel
    // values and premultiply using the unshifted alpha, preventing colored
    // output in the transparent region outside the original object.
    var red = 0.0;
    var blue = 0.0;
    if red_sample.a > 0.000001 {
        red = red_sample.r / red_sample.a;
    }
    if blue_sample.a > 0.000001 {
        blue = blue_sample.b / blue_sample.a;
    }
    return vec4<f32>(
        red * center.a,
        center.g,
        blue * center.a,
        center.a,
    );
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RgbSplitError {
    InvalidAmount,
    InvalidAngle,
    InvalidPreviewScale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbSplitParameters {
    pub amount_px: f32,
    pub angle_degrees: f32,
}

impl RgbSplitParameters {
    pub fn validate(self) -> Result<Self, RgbSplitError> {
        if !self.amount_px.is_finite() || !(0.0..=64.0).contains(&self.amount_px) {
            return Err(RgbSplitError::InvalidAmount);
        }
        if !self.angle_degrees.is_finite() {
            return Err(RgbSplitError::InvalidAngle);
        }
        Ok(self)
    }
}

/// Positive angles rotate clockwise in top-left-origin composition space;
/// red samples the positive displacement, blue the equal opposite.
pub fn channel_offsets(
    parameters: RgbSplitParameters,
) -> Result<([f32; 2], [f32; 2]), RgbSplitError> {
    let params = parameters.validate()?;
    let radians = params.angle_degrees.to_radians();
    let x = radians.cos() * params.amount_px;
    let y = radians.sin() * params.amount_px;
    Ok(([x, y], [-x, -y]))
}

/// Preview works in smaller pixels but Project records full composition units.
pub fn preview_rgb_split_parameters(
    parameters: RgbSplitParameters,
    preview_scale: f32,
) -> Result<RgbSplitParameters, RgbSplitError> {
    let original = parameters.validate()?;
    if !preview_scale.is_finite() || preview_scale <= 0.0 || preview_scale > 1.0 {
        return Err(RgbSplitError::InvalidPreviewScale);
    }
    Ok(RgbSplitParameters {
        amount_px: original.amount_px * preview_scale,
        ..original
    })
}

#[derive(Debug)]
pub struct RgbSplit {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

impl RgbSplit {
    #[must_use]
    pub fn new(device: &wgpu::Device, working_format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rhythm Effects RGB Split bindings"),
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
            label: Some("Rhythm Effects RGB Split clamp sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects RGB Split shader"),
            source: wgpu::ShaderSource::Wgsl(RGB_SPLIT_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Rhythm Effects RGB Split pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rhythm Effects RGB channel displacement"),
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

    /// Encode one isolated pass. Input and output cannot be the same texture.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: RgbSplitParameters,
    ) -> Result<(), RgbSplitError> {
        let parameters = parameters.validate()?;
        let (source, output) = targets;
        debug_assert_eq!(source.key(), output.key());

        let mut bytes = [0_u8; 16];
        bytes[0..4].copy_from_slice(&parameters.amount_px.to_le_bytes());
        bytes[4..8].copy_from_slice(&parameters.angle_degrees.to_le_bytes());
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rhythm Effects RGB Split parameters"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform, 0, &bytes);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rhythm Effects RGB Split source"),
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
            label: Some("Rhythm Effects RGB Split pass"),
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
fn reference_split(
    center: [f32; 4],
    red_sample: [f32; 4],
    blue_sample: [f32; 4],
    amount_px: f32,
) -> [f32; 4] {
    if amount_px <= 0.0 || center[3] <= 0.0 {
        return center;
    }
    let red = if red_sample[3] > 0.000001 {
        red_sample[0] / red_sample[3]
    } else {
        0.0
    };
    let blue = if blue_sample[3] > 0.000001 {
        blue_sample[2] / blue_sample[3]
    } else {
        0.0
    };
    [red * center[3], center[1], blue * center[3], center[3]]
}

#[cfg(test)]
mod tests {
    use super::{
        RgbSplitError, RgbSplitParameters, channel_offsets, preview_rgb_split_parameters,
        reference_split,
    };

    fn params(amount_px: f32, angle_degrees: f32) -> RgbSplitParameters {
        RgbSplitParameters {
            amount_px,
            angle_degrees,
        }
    }

    #[test]
    fn red_blue_offsets_are_opposite_and_clockwise_in_composition_coordinates() {
        let (red, blue) = channel_offsets(params(4.0, 0.0)).expect("zero degrees");
        assert!((red[0] - 4.0).abs() < 0.00001);
        assert!(red[1].abs() < 0.00001);
        assert_eq!(blue, [-red[0], -red[1]]);

        let (red, blue) = channel_offsets(params(4.0, 90.0)).expect("clockwise");
        assert!(red[0].abs() < 0.00001);
        assert!((red[1] - 4.0).abs() < 0.00001);
        assert_eq!(blue, [-red[0], -red[1]]);
        assert_eq!(
            channel_offsets(params(0.0, 90.0)),
            Ok(([0.0, 0.0], [-0.0, -0.0]))
        );
    }

    #[test]
    fn amount_zero_keeps_original_rgba_and_full_preview_offsets() {
        let center = [0.1, 0.2, 0.3, 0.5];
        let red = [0.5, 0.0, 0.0, 0.5];
        let blue = [0.0, 0.0, 0.5, 0.5];
        assert_eq!(reference_split(center, red, blue, 0.0), center);
        assert_eq!(
            reference_split(center, red, blue, 4.0),
            [0.5, 0.2, 0.5, 0.5]
        );
        for (scale, expected) in [(1.0, 64.0), (0.5, 32.0), (0.25, 16.0)] {
            let original = params(64.0, 45.0);
            let scaled = preview_rgb_split_parameters(original, scale).expect("preview");
            assert_eq!(scaled.amount_px, expected);
            assert_eq!(scaled.angle_degrees, original.angle_degrees);
            assert_eq!(original.amount_px, 64.0);
        }
    }

    #[test]
    fn rejects_invalid_amount_angle_and_preview_scale() {
        for amount in [-0.1, 65.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                params(amount, 0.0).validate(),
                Err(RgbSplitError::InvalidAmount)
            );
        }
        assert_eq!(
            params(2.0, f32::NAN).validate(),
            Err(RgbSplitError::InvalidAngle)
        );
        for scale in [0.0, -1.0, 1.01, f32::NAN] {
            assert_eq!(
                preview_rgb_split_parameters(params(4.0, 30.0), scale),
                Err(RgbSplitError::InvalidPreviewScale)
            );
        }
    }
}
