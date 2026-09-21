//! GPU renderer boundary for Rhythm Effects.
//!
//! This module owns renderer/backend concerns inside `rhythm_engine`.
//! `rhythm_core` stays independent from wgpu and other graphics APIs.

use std::collections::HashMap;

use rhythm_core::ids::AssetId;

use crate::{image_decode::ImageDecodeGeneration, runtime_assets::ValidatedDecodedImage};

pub const INITIAL_COMPOSITION_WIDTH: u32 = 1920;
pub const INITIAL_COMPOSITION_HEIGHT: u32 = 1080;
pub const COMPOSITION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const PREVIEW_DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const IMAGE_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageTextureUpload {
    Inserted,
    Replaced,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageTextureUploadError {
    InvalidDimensions,
    SizeOverflow,
    ByteLengthMismatch {
        expected: usize,
        actual: usize,
    },
    OlderThanCached {
        cached: ImageDecodeGeneration,
        incoming: ImageDecodeGeneration,
    },
}

#[derive(Debug)]
pub struct CachedImageTexture {
    generation: ImageDecodeGeneration,
    size: [u32; 2],
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl CachedImageTexture {
    #[must_use]
    pub const fn generation(&self) -> ImageDecodeGeneration {
        self.generation
    }

    #[must_use]
    pub const fn size(&self) -> [u32; 2] {
        self.size
    }

    #[must_use]
    pub const fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageTextureCacheDecision {
    Insert,
    Replace,
    Unchanged,
}

#[derive(Debug, Default)]
struct ImageTextureCache {
    entries: HashMap<AssetId, CachedImageTexture>,
}

impl ImageTextureCache {
    fn get(&self, asset_id: AssetId) -> Option<&CachedImageTexture> {
        self.entries.get(&asset_id)
    }

    fn remove(&mut self, asset_id: AssetId) -> bool {
        self.entries.remove(&asset_id).is_some()
    }

    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        decoded: ValidatedDecodedImage,
    ) -> Result<ImageTextureUpload, ImageTextureUploadError> {
        let asset_id = decoded.asset_id();
        let generation = decoded.generation();
        let decision = image_texture_cache_decision(
            self.entries
                .get(&asset_id)
                .map(CachedImageTexture::generation),
            generation,
        )?;
        if decision == ImageTextureCacheDecision::Unchanged {
            return Ok(ImageTextureUpload::Unchanged);
        }

        let image = decoded.image();
        let bytes_per_row = image
            .width
            .checked_mul(4)
            .ok_or(ImageTextureUploadError::SizeOverflow)?;
        validate_rgba8_payload(image.width, image.height, image.rgba8.len())?;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Rhythm Effects image asset"),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: IMAGE_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.entries.insert(
            asset_id,
            CachedImageTexture {
                generation,
                size: [image.width, image.height],
                _texture: texture,
                view,
            },
        );

        Ok(match decision {
            ImageTextureCacheDecision::Insert => ImageTextureUpload::Inserted,
            ImageTextureCacheDecision::Replace => ImageTextureUpload::Replaced,
            ImageTextureCacheDecision::Unchanged => unreachable!("handled before upload"),
        })
    }
}

fn image_texture_cache_decision(
    cached: Option<ImageDecodeGeneration>,
    incoming: ImageDecodeGeneration,
) -> Result<ImageTextureCacheDecision, ImageTextureUploadError> {
    match cached {
        None => Ok(ImageTextureCacheDecision::Insert),
        Some(cached) if cached == incoming => Ok(ImageTextureCacheDecision::Unchanged),
        Some(cached) if cached.get() < incoming.get() => Ok(ImageTextureCacheDecision::Replace),
        Some(cached) => Err(ImageTextureUploadError::OlderThanCached { cached, incoming }),
    }
}

fn validate_rgba8_payload(
    width: u32,
    height: u32,
    actual: usize,
) -> Result<(), ImageTextureUploadError> {
    if width == 0 || height == 0 {
        return Err(ImageTextureUploadError::InvalidDimensions);
    }

    let expected_u64 = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ImageTextureUploadError::SizeOverflow)?;
    let expected =
        usize::try_from(expected_u64).map_err(|_| ImageTextureUploadError::SizeOverflow)?;
    if expected != actual {
        return Err(ImageTextureUploadError::ByteLengthMismatch { expected, actual });
    }
    Ok(())
}

const PREVIEW_SHADER: &str = r#"
@group(0) @binding(0) var composition_texture: texture_2d<f32>;
@group(0) @binding(1) var composition_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
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
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

fn linear_to_srgb_channel(value: f32) -> f32 {
    if value <= 0.0031308 {
        return value * 12.92;
    }
    return 1.055 * pow(value, 1.0 / 2.4) - 0.055;
}

fn linear_to_srgb(rgb: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb_channel(rgb.r),
        linear_to_srgb_channel(rgb.g),
        linear_to_srgb_channel(rgb.b),
    );
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let premultiplied_linear = textureSample(composition_texture, composition_sampler, input.uv);
    let alpha = clamp(premultiplied_linear.a, 0.0, 1.0);

    var straight_linear = vec3<f32>(0.0);
    if alpha > 0.00001 {
        straight_linear = max(premultiplied_linear.rgb / alpha, vec3<f32>(0.0));
    }

    let straight_srgb = linear_to_srgb(straight_linear);
    return vec4<f32>(straight_srgb * alpha, alpha);
}
"#;

#[derive(Debug)]
pub struct Renderer {
    _composition_texture: wgpu::Texture,
    composition_view: wgpu::TextureView,
    composition_size: [u32; 2],
    _preview_display_texture: wgpu::Texture,
    preview_display_view: wgpu::TextureView,
    preview_bind_group: wgpu::BindGroup,
    preview_pipeline: wgpu::RenderPipeline,
    image_textures: ImageTextureCache,
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

        let preview_display_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Rhythm Effects preview display"),
            size: wgpu::Extent3d {
                width: INITIAL_COMPOSITION_WIDTH,
                height: INITIAL_COMPOSITION_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PREVIEW_DISPLAY_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let preview_display_view =
            preview_display_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let preview_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Rhythm Effects preview bind group layout"),
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

        let preview_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Rhythm Effects preview sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let preview_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rhythm Effects preview bind group"),
            layout: &preview_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&composition_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&preview_sampler),
                },
            ],
        });

        let preview_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rhythm Effects preview conversion shader"),
            source: wgpu::ShaderSource::Wgsl(PREVIEW_SHADER.into()),
        });
        let preview_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rhythm Effects preview pipeline layout"),
                bind_group_layouts: &[Some(&preview_bind_group_layout)],
                immediate_size: 0,
            });
        let preview_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rhythm Effects preview pipeline"),
            layout: Some(&preview_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &preview_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &preview_shader,
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
            _composition_texture: composition_texture,
            composition_view,
            composition_size: [INITIAL_COMPOSITION_WIDTH, INITIAL_COMPOSITION_HEIGHT],
            _preview_display_texture: preview_display_texture,
            preview_display_view,
            preview_bind_group,
            preview_pipeline,
            image_textures: ImageTextureCache::default(),
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

    #[must_use]
    pub fn preview_display_view(&self) -> &wgpu::TextureView {
        &self.preview_display_view
    }

    #[must_use]
    pub fn image_texture(&self, asset_id: AssetId) -> Option<&CachedImageTexture> {
        self.image_textures.get(asset_id)
    }

    pub fn upload_validated_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        decoded: ValidatedDecodedImage,
    ) -> Result<ImageTextureUpload, ImageTextureUploadError> {
        self.image_textures.upload(device, queue, decoded)
    }

    pub fn remove_image_texture(&mut self, asset_id: AssetId) -> bool {
        self.image_textures.remove(asset_id)
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

    pub fn refresh_preview_display(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Rhythm Effects preview conversion encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Rhythm Effects preview conversion pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.preview_display_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.preview_pipeline);
            pass.set_bind_group(0, &self.preview_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COMPOSITION_FORMAT, IMAGE_TEXTURE_FORMAT, INITIAL_COMPOSITION_HEIGHT,
        INITIAL_COMPOSITION_WIDTH, ImageTextureCacheDecision, ImageTextureUploadError,
        PREVIEW_DISPLAY_FORMAT, image_texture_cache_decision, validate_rgba8_payload,
    };
    use crate::image_decode::ImageDecodeGeneration;

    #[test]
    fn initial_composition_contract_is_1080p_rgba16float() {
        assert_eq!(INITIAL_COMPOSITION_WIDTH, 1920);
        assert_eq!(INITIAL_COMPOSITION_HEIGHT, 1080);
        assert_eq!(COMPOSITION_FORMAT, wgpu::TextureFormat::Rgba16Float);
        assert_eq!(PREVIEW_DISPLAY_FORMAT, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(IMAGE_TEXTURE_FORMAT, wgpu::TextureFormat::Rgba8UnormSrgb);
    }

    #[test]
    fn image_texture_cache_skips_same_generation_and_replaces_newer() {
        let first = ImageDecodeGeneration::new(1);
        let second = ImageDecodeGeneration::new(2);

        assert_eq!(
            image_texture_cache_decision(None, first),
            Ok(ImageTextureCacheDecision::Insert)
        );
        assert_eq!(
            image_texture_cache_decision(Some(first), first),
            Ok(ImageTextureCacheDecision::Unchanged)
        );
        assert_eq!(
            image_texture_cache_decision(Some(first), second),
            Ok(ImageTextureCacheDecision::Replace)
        );
        assert_eq!(
            image_texture_cache_decision(Some(second), first),
            Err(ImageTextureUploadError::OlderThanCached {
                cached: second,
                incoming: first,
            })
        );
    }

    #[test]
    fn image_texture_upload_validates_rgba8_payload_shape() {
        assert_eq!(validate_rgba8_payload(2, 1, 8), Ok(()));
        assert_eq!(
            validate_rgba8_payload(0, 1, 0),
            Err(ImageTextureUploadError::InvalidDimensions)
        );
        assert_eq!(
            validate_rgba8_payload(2, 2, 15),
            Err(ImageTextureUploadError::ByteLengthMismatch {
                expected: 16,
                actual: 15,
            })
        );
    }
}
