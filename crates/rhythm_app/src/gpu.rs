use std::sync::Arc;

use thiserror::Error;
use winit::window::Window;

#[derive(Debug, Error)]
pub enum GpuInitError {
    #[error("failed to create a window-compatible GPU surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),

    #[error("failed to find a compatible GPU adapter: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),

    #[error("failed to create the logical GPU device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),

    #[error("the selected GPU adapter cannot provide a default surface configuration")]
    UnsupportedSurfaceConfiguration,

    #[error("the selected GPU adapter cannot render to Rgba16Float composition targets")]
    UnsupportedCompositionFormat,
}

#[derive(Debug, Clone)]
pub struct GpuAdapterSummary {
    pub name: String,
    pub backend: String,
    pub device_type: String,
}

#[derive(Debug)]
pub struct GpuContext {
    #[allow(dead_code)]
    pub(crate) instance: wgpu::Instance,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) surface_config: Option<wgpu::SurfaceConfiguration>,
}

impl GpuContext {
    pub fn initialize(window: Arc<Window>) -> Result<Self, GpuInitError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let initial_size = window.inner_size();
        let surface = instance.create_surface(window)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            ..Default::default()
        }))?;

        let composition_features =
            adapter.get_texture_format_features(wgpu::TextureFormat::Rgba16Float);
        if !composition_features
            .allowed_usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(GpuInitError::UnsupportedCompositionFormat);
        }

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Rhythm Effects GPU device"),
                ..Default::default()
            }))?;

        let surface_config = if initial_size.width == 0 || initial_size.height == 0 {
            None
        } else {
            let config = surface
                .get_default_config(&adapter, initial_size.width, initial_size.height)
                .ok_or(GpuInitError::UnsupportedSurfaceConfiguration)?;
            surface.configure(&device, &config);
            Some(config)
        };

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            surface_config,
        })
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<bool, GpuInitError> {
        if width == 0 || height == 0 {
            return Ok(false);
        }

        let mut config = match self.surface_config.take() {
            Some(config) => config,
            None => self
                .surface
                .get_default_config(&self.adapter, width, height)
                .ok_or(GpuInitError::UnsupportedSurfaceConfiguration)?,
        };

        config.width = width;
        config.height = height;
        self.surface.configure(&self.device, &config);
        self.surface_config = Some(config);

        Ok(true)
    }

    #[must_use]
    pub fn surface_size(&self) -> Option<[u32; 2]> {
        self.surface_config
            .as_ref()
            .map(|config| [config.width, config.height])
    }

    #[must_use]
    pub fn adapter_summary(&self) -> GpuAdapterSummary {
        let info = self.adapter.get_info();

        GpuAdapterSummary {
            name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
        }
    }
}
