//! GPU renderer boundary for Rhythm Effects.
//!
//! This module owns renderer/backend concerns inside `rhythm_engine`.
//! `rhythm_core` stays independent from wgpu and other graphics APIs.

use std::{
    cell::Cell,
    collections::HashMap,
    time::{Duration, Instant},
};

use rhythm_core::ids::AssetId;

use crate::{
    blur::{BlurPreviewScaleError, SeparableBlur, preview_blur_radius},
    effect_chain::encode_ordered_effect_chain,
    glow::{
        Glow, GlowError, GlowParameters, GlowPreviewScaleError, GlowResources,
        preview_glow_parameters,
    },
    image_decode::ImageDecodeGeneration,
    isolated_objects::IsolatedObjectCompositor,
    noise::{Noise, NoiseError, NoiseParameters, preview_noise_parameters},
    rgb_split::{RgbSplit, RgbSplitError, RgbSplitParameters, preview_rgb_split_parameters},
    runtime_assets::ValidatedDecodedImage,
    scene_eval::EvaluatedEffect,
    temporary_textures::{
        TemporaryTexture, TemporaryTextureKey, TemporaryTexturePool, TemporaryTexturePoolStats,
    },
    text::TextResources,
    tint::{Tint, TintError, TintParameters},
};

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

    fn estimated_bytes(&self) -> u64 {
        self.entries
            .values()
            .map(|image| {
                u64::from(image.size[0])
                    .saturating_mul(u64::from(image.size[1]))
                    .saturating_mul(4)
            })
            .sum()
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

pub(crate) const PREVIEW_SHADER: &str = r#"
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RendererCpuCounters {
    cpu_nanos: u64,
    render_passes: u64,
    draw_calls: u64,
    isolated_objects: u64,
    texture_uploads: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererFrameDiagnostics {
    pub cpu_time: Duration,
    pub render_passes: u64,
    /// Renderer-owned fullscreen draws. Object-content callback draws are
    /// deliberately not guessed and should be reported by the scene drawer.
    pub draw_calls: u64,
    pub isolated_objects: u64,
    pub texture_uploads: u64,
    pub temporary_textures: TemporaryTexturePoolStats,
    pub gpu_estimated_bytes: u64,
}

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
    temporary_textures: TemporaryTexturePool,
    isolated_compositor: IsolatedObjectCompositor,
    blur: SeparableBlur,
    glow: Glow,
    noise: Noise,
    rgb_split: RgbSplit,
    tint: Tint,
    text_resources: TextResources,
    diagnostics: Cell<RendererCpuCounters>,
}

impl Renderer {
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self::new_with_composition_size(
            device,
            queue,
            [INITIAL_COMPOSITION_WIDTH, INITIAL_COMPOSITION_HEIGHT],
        )
    }

    /// Export owns a separate renderer with its own textures, pipelines and
    /// caches; preview's renderer is never resized or borrowed for this job.
    pub(crate) fn new_with_composition_size(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
    ) -> Self {
        debug_assert!(size[0] > 0 && size[1] > 0);
        let composition_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Rhythm Effects composition"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
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
                width: size[0],
                height: size[1],
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
            composition_size: size,
            _preview_display_texture: preview_display_texture,
            preview_display_view,
            preview_bind_group,
            preview_pipeline,
            image_textures: ImageTextureCache::default(),
            temporary_textures: TemporaryTexturePool::default(),
            isolated_compositor: IsolatedObjectCompositor::new(device, COMPOSITION_FORMAT),
            blur: SeparableBlur::new(device, COMPOSITION_FORMAT),
            glow: Glow::new(device, COMPOSITION_FORMAT),
            noise: Noise::new(device, COMPOSITION_FORMAT),
            rgb_split: RgbSplit::new(device, COMPOSITION_FORMAT),
            tint: Tint::new(device, COMPOSITION_FORMAT),
            text_resources: TextResources::new(device, queue, COMPOSITION_FORMAT, size),
            diagnostics: Cell::new(RendererCpuCounters::default()),
        }
    }

    fn record_diagnostics(
        &self,
        started: Instant,
        render_passes: u64,
        draw_calls: u64,
        isolated_objects: u64,
        texture_uploads: u64,
    ) {
        let current = self.diagnostics.get();
        let elapsed = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.diagnostics.set(RendererCpuCounters {
            cpu_nanos: current.cpu_nanos.saturating_add(elapsed),
            render_passes: current.render_passes.saturating_add(render_passes),
            draw_calls: current.draw_calls.saturating_add(draw_calls),
            isolated_objects: current.isolated_objects.saturating_add(isolated_objects),
            texture_uploads: current.texture_uploads.saturating_add(texture_uploads),
        });
    }

    /// Drain renderer-owned CPU/pass/draw counters for the current UI frame.
    /// Temporary pool allocation/reuse totals remain cumulative and bounded.
    #[must_use]
    pub fn take_frame_diagnostics(&self) -> RendererFrameDiagnostics {
        let counters = self.diagnostics.replace(RendererCpuCounters::default());
        let temporary_textures = self.temporary_textures.stats();
        let [width, height] = self.composition_size;
        let composition_bytes = u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(8);
        let preview_bytes = u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4);
        let gpu_estimated_bytes = composition_bytes
            .saturating_add(preview_bytes)
            .saturating_add(self.image_textures.estimated_bytes())
            .saturating_add(temporary_textures.free_estimated_bytes);
        RendererFrameDiagnostics {
            cpu_time: Duration::from_nanos(counters.cpu_nanos),
            render_passes: counters.render_passes,
            draw_calls: counters.draw_calls,
            isolated_objects: counters.isolated_objects,
            texture_uploads: counters.texture_uploads,
            temporary_textures,
            gpu_estimated_bytes,
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
    pub fn composition_view(&self) -> &wgpu::TextureView {
        &self.composition_view
    }

    #[must_use]
    pub fn preview_display_view(&self) -> &wgpu::TextureView {
        &self.preview_display_view
    }

    #[must_use]
    pub const fn text_resources(&self) -> &TextResources {
        &self.text_resources
    }

    pub const fn text_resources_mut(&mut self) -> &mut TextResources {
        &mut self.text_resources
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
        let started = Instant::now();
        let result = self.image_textures.upload(device, queue, decoded);
        let uploads = if matches!(
            result,
            Ok(ImageTextureUpload::Inserted | ImageTextureUpload::Replaced)
        ) {
            1
        } else {
            0
        };
        self.record_diagnostics(started, 0, 0, 0, uploads);
        result
    }

    pub fn remove_image_texture(&mut self, asset_id: AssetId) -> bool {
        self.image_textures.remove(asset_id)
    }

    #[must_use]
    pub fn temporary_texture_pool(&mut self) -> &mut TemporaryTexturePool {
        &mut self.temporary_textures
    }

    #[must_use]
    pub fn temporary_texture_stats(&self) -> TemporaryTexturePoolStats {
        self.temporary_textures.stats()
    }

    /// The content encoder receives an exclusive transparent Rgba16Float
    /// target; a multipass effect chain can consume the checkout before
    /// calling encode_composite_isolated.
    pub fn encode_isolated_object<F>(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        draw: F,
    ) -> TemporaryTexture
    where
        F: FnOnce(&mut wgpu::RenderPass<'_>),
    {
        let key = TemporaryTextureKey::new(
            self.composition_size[0],
            self.composition_size[1],
            COMPOSITION_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        )
        .expect("composition size and usage are valid");
        let started = Instant::now();
        let target = self.isolated_compositor.encode_object(
            device,
            encoder,
            &mut self.temporary_textures,
            key,
            draw,
        );
        self.record_diagnostics(started, 1, 0, 1, 0);
        target
    }

    /// Run evaluated effects in their ordered stack using distinct pooled
    /// input/output targets; each output feeds the next pass. A source with
    /// zero effects is returned untouched without an intermediate allocation.
    pub fn encode_effect_chain(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: TemporaryTexture,
        effects: &[EvaluatedEffect],
        encode_effect: impl FnMut(
            &mut wgpu::CommandEncoder,
            &EvaluatedEffect,
            &TemporaryTexture,
            &TemporaryTexture,
            &mut TemporaryTexturePool,
        ),
    ) -> TemporaryTexture {
        let started = Instant::now();
        let target = encode_ordered_effect_chain(
            device,
            encoder,
            &mut self.temporary_textures,
            source,
            effects,
            encode_effect,
        );
        self.record_diagnostics(started, 0, 0, 0, 0);
        target
    }

    /// Encode a horizontal/vertical blur between two distinct pooled targets.
    /// The source and output are owned by the caller; the middle target is
    /// acquired/released here after its final encoded use.
    pub fn encode_blur(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        source: &TemporaryTexture,
        output: &TemporaryTexture,
        radius_px: f32,
    ) {
        let started = Instant::now();
        self.blur.encode(
            device,
            queue,
            encoder,
            &mut self.temporary_textures,
            (source, output),
            radius_px,
        );
        self.record_diagnostics(started, 2, 2, 0, 0);
    }

    /// Apply composition-pixel blur semantics at the active preview scale.
    /// Scale validation happens before GPU commands are encoded; the project
    /// radius remains unchanged for full-resolution export and future edits.
    pub fn encode_preview_blur(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        composition_radius_px: f32,
        preview_scale: f32,
    ) -> Result<(), BlurPreviewScaleError> {
        let radius_px = preview_blur_radius(composition_radius_px, preview_scale)?;
        self.encode_blur(device, queue, encoder, targets.0, targets.1, radius_px);
        Ok(())
    }

    /// Encode the object-local Glow mask, its separable blur, and the final
    /// additive color/alpha composite between distinct checked-out targets.
    /// Semantic validation precedes intermediate allocation and encoding.
    pub fn encode_glow(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        source: &TemporaryTexture,
        output: &TemporaryTexture,
        parameters: GlowParameters,
    ) -> Result<(), GlowError> {
        let started = Instant::now();
        let result = self.glow.encode(
            &self.blur,
            &mut GlowResources {
                device,
                queue,
                encoder,
                pool: &mut self.temporary_textures,
            },
            source,
            output,
            parameters,
        );
        if result.is_ok() {
            self.record_diagnostics(started, 4, 4, 0, 0);
        } else {
            self.record_diagnostics(started, 0, 0, 0, 0);
        }
        result
    }

    /// Convert the semantic Glow radius using the resolved preview scale
    /// before encoding. Reject invalid source parameters/scale without
    /// allocating GPU intermediates or changing the creative project.
    pub fn encode_preview_glow(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: GlowParameters,
        preview_scale: f32,
    ) -> Result<(), GlowPreviewScaleError> {
        let working = preview_glow_parameters(parameters, preview_scale)?;
        self.encode_glow(device, queue, encoder, targets.0, targets.1, working)
            .map_err(GlowPreviewScaleError::InvalidParameters)
    }

    /// Encode linear-light Tint into a distinct pooled target, keeping the
    /// original alpha and preserving exact identity at zero amount.
    pub fn encode_tint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: TintParameters,
    ) -> Result<(), TintError> {
        let started = Instant::now();
        let result = self
            .tint
            .encode(device, queue, encoder, targets, parameters);
        self.record_diagnostics(
            started,
            if result.is_ok() { 1 } else { 0 },
            if result.is_ok() { 1 } else { 0 },
            0,
            0,
        );
        result
    }

    /// Encode deterministic, alpha-preserving monochrome Noise. The semantic
    /// project parameters are validated before creating any GPU resources.
    pub fn encode_noise(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: NoiseParameters,
    ) -> Result<(), NoiseError> {
        let started = Instant::now();
        let result = self
            .noise
            .encode(device, queue, encoder, targets, parameters);
        self.record_diagnostics(
            started,
            if result.is_ok() { 1 } else { 0 },
            if result.is_ok() { 1 } else { 0 },
            0,
            0,
        );
        result
    }

    /// Convert composition pixel-block size to the resolved preview scale,
    /// then encode without changing the Project's full-size value.
    pub fn encode_preview_noise(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: NoiseParameters,
        preview_scale: f32,
    ) -> Result<(), NoiseError> {
        let working = preview_noise_parameters(parameters, preview_scale)?;
        self.encode_noise(device, queue, encoder, targets, working)
    }

    /// Red and blue sample equal/opposite signed composition-pixel offsets
    /// along the clockwise angle; center alpha is preserved.
    pub fn encode_rgb_split(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: RgbSplitParameters,
    ) -> Result<(), RgbSplitError> {
        let started = Instant::now();
        let result = self
            .rgb_split
            .encode(device, queue, encoder, targets, parameters);
        self.record_diagnostics(
            started,
            if result.is_ok() { 1 } else { 0 },
            if result.is_ok() { 1 } else { 0 },
            0,
            0,
        );
        result
    }

    /// Convert displacement into working preview pixels without rewriting
    /// the semantic amount or the angle in the creative project.
    pub fn encode_preview_rgb_split(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&TemporaryTexture, &TemporaryTexture),
        parameters: RgbSplitParameters,
        preview_scale: f32,
    ) -> Result<(), RgbSplitError> {
        let working = preview_rgb_split_parameters(parameters, preview_scale)?;
        self.encode_rgb_split(device, queue, encoder, targets, working)
    }

    /// Composite after all effects, then return the checkout to the pool.
    /// The source cannot alias the destination composition texture.
    pub fn encode_composite_isolated(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: TemporaryTexture,
    ) {
        let started = Instant::now();
        self.isolated_compositor
            .encode_composite(device, encoder, &target, &self.composition_view);
        self.temporary_textures.release(target);
        self.record_diagnostics(started, 1, 1, 0, 0);
    }

    /// Encode a deterministic opaque-black composition clear into the caller's
    /// command stream. Export uses this form so creative drawing, SDR conversion
    /// and readback can stay ordered in one submission without a hidden queue
    /// submit between frame stages.
    pub fn encode_clear_composition(&self, encoder: &mut wgpu::CommandEncoder) {
        let started = Instant::now();
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
        self.record_diagnostics(started, 1, 0, 0, 0);
    }

    pub fn clear_composition(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Rhythm Effects composition clear encoder"),
        });
        self.encode_clear_composition(&mut encoder);
        queue.submit([encoder.finish()]);
    }

    pub fn refresh_preview_display(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let started = Instant::now();
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
        self.record_diagnostics(started, 1, 1, 0, 0);
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
