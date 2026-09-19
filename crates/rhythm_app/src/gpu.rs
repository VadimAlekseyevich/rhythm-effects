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
    pub(crate) adapter: wgpu::Adapter,
    #[allow(dead_code)]
    pub(crate) device: wgpu::Device,
    #[allow(dead_code)]
    pub(crate) queue: wgpu::Queue,
}

impl GpuContext {
    pub fn initialize(window: Arc<Window>) -> Result<Self, GpuInitError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        // AI-022 uses a temporary unconfigured surface only to guarantee that
        // the selected adapter can present to this application window.
        // Persistent surface ownership/configuration is introduced by AI-023.
        let compatibility_surface = instance.create_surface(window)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&compatibility_surface),
            force_fallback_adapter: false,
            ..Default::default()
        }))?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Rhythm Effects GPU device"),
                ..Default::default()
            }))?;

        drop(compatibility_surface);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
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
