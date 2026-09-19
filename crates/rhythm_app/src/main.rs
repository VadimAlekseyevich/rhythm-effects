#![forbid(unsafe_code)]

mod gpu;

use std::sync::Arc;

use gpu::GpuContext;
use rhythm_core::APP_NAME;
use rhythm_engine::renderer::Renderer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct RhythmApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    egui_context: Option<egui::Context>,
    egui_state: Option<egui_winit::State>,
}

impl ApplicationHandler for RhythmApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(APP_NAME)
            .with_inner_size(LogicalSize::new(1440.0, 900.0))
            .with_min_inner_size(LogicalSize::new(1280.0, 720.0));

        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);

                match GpuContext::initialize(Arc::clone(&window)) {
                    Ok(gpu) => {
                        let renderer = Renderer::new(&gpu.device);
                        renderer.clear_composition(&gpu.device, &gpu.queue);

                        let egui_context = egui::Context::default();
                        let egui_state = egui_winit::State::new(
                            egui_context.clone(),
                            egui::ViewportId::ROOT,
                            event_loop,
                            Some(window.scale_factor() as f32),
                            event_loop.system_theme(),
                            Some(gpu.device.limits().max_texture_dimension_2d as usize),
                        );

                        let adapter = gpu.adapter_summary();
                        let surface_size = gpu.surface_size();
                        let composition_size = renderer.composition_size();
                        let composition_format = renderer.composition_format();
                        info!(
                            window_id = ?window.id(),
                            engine = rhythm_engine::status(),
                            gpu_name = %adapter.name,
                            gpu_backend = %adapter.backend,
                            gpu_device_type = %adapter.device_type,
                            surface_size = ?surface_size,
                            composition_size = ?composition_size,
                            composition_format = ?composition_format,
                            "application window, GPU context, surface, and composition target created"
                        );
                        self.egui_context = Some(egui_context);
                        self.egui_state = Some(egui_state);
                        self.renderer = Some(renderer);
                        self.gpu = Some(gpu);
                        window.request_redraw();
                        self.window = Some(window);
                    }
                    Err(error) => {
                        warn!(%error, "failed to initialize GPU");
                        event_loop.exit();
                    }
                }
            }
            Err(error) => {
                warn!(%error, "failed to create application window");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        if let Some(egui_state) = self.egui_state.as_mut() {
            let response = egui_state.on_window_event(window, &event);
            if response.repaint {
                window.request_redraw();
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                info!("close requested");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let (Some(egui_context), Some(egui_state)) =
                    (self.egui_context.as_ref(), self.egui_state.as_mut())
                {
                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_context.run_ui(raw_input, |_ui| {});
                    egui_state.handle_platform_output(window, full_output.platform_output);
                }
            }
            WindowEvent::Resized(size) => match self.gpu.as_mut() {
                Some(gpu) => match gpu.resize_surface(size.width, size.height) {
                    Ok(reconfigured) => {
                        info!(
                            width = size.width,
                            height = size.height,
                            surface_reconfigured = reconfigured,
                            "window resized"
                        );
                    }
                    Err(error) => {
                        warn!(%error, "failed to reconfigure GPU surface");
                        event_loop.exit();
                    }
                },
                None => {
                    info!(
                        width = size.width,
                        height = size.height,
                        "window resized before GPU initialization"
                    );
                }
            },
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        info!("application exiting");
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    info!(app = APP_NAME, "starting application");

    let event_loop = EventLoop::new()?;
    let mut app = RhythmApp::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
