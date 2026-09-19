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
    #[allow(dead_code)]
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) adapter: wgpu::Adapter,
    #[allow(dead_code)]
    pub(crate) device: wgpu::Device,
    #[allow(dead_code)]
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
