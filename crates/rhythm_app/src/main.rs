#![forbid(unsafe_code)]

mod editor_ui;
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
    egui_renderer: Option<egui_wgpu::Renderer>,
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

                        let egui_renderer = gpu.surface_config.as_ref().map(|config| {
                            egui_wgpu::Renderer::new(
                                &gpu.device,
                                config.format,
                                egui_wgpu::RendererOptions::default(),
                            )
                        });

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
                        self.egui_renderer = egui_renderer;
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
                if let (
                    Some(gpu),
                    Some(egui_context),
                    Some(egui_state),
                    Some(egui_renderer),
                ) = (
                    self.gpu.as_mut(),
                    self.egui_context.as_ref(),
                    self.egui_state.as_mut(),
                    self.egui_renderer.as_mut(),
                ) {
                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_context.run_ui(raw_input, |_root_ui| {
                        editor_ui::draw_editor_shell(egui_context);
                    });
                    let paint_jobs =
                        egui_context.tessellate(full_output.shapes, full_output.pixels_per_point);
                    let mut textures_delta = full_output.textures_delta;

                    egui_state.handle_platform_output(window, full_output.platform_output);

                    for (texture_id, image_deltas) in textures_delta.set.drain() {
                        for image_delta in image_deltas {
                            egui_renderer.update_texture(
                                &gpu.device,
                                &gpu.queue,
                                texture_id,
                                &image_delta,
                            );
                        }
                    }

                    let Some(config) = gpu.surface_config.as_ref() else {
                        return;
                    };

                    let output_frame = match gpu.surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(frame)
                        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                        wgpu::CurrentSurfaceTexture::Timeout
                        | wgpu::CurrentSurfaceTexture::Occluded => return,
                        wgpu::CurrentSurfaceTexture::Outdated
                        | wgpu::CurrentSurfaceTexture::Lost => {
                            gpu.surface.configure(&gpu.device, config);
                            window.request_redraw();
                            return;
                        }
                        wgpu::CurrentSurfaceTexture::Validation => {
                            warn!("surface validation error while drawing egui");
                            return;
                        }
                    };

                    let target_view = output_frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        gpu.device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Rhythm Effects egui encoder"),
                            });
                    let screen = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [config.width, config.height],
                        pixels_per_point: full_output.pixels_per_point,
                    };

                    let user_command_buffers = egui_renderer.update_buffers(
                        &gpu.device,
                        &gpu.queue,
                        &mut encoder,
                        &paint_jobs,
                        &screen,
                    );

                    {
                        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Rhythm Effects egui pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &target_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.035,
                                        g: 0.035,
                                        b: 0.04,
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

                        egui_renderer.render(
                            &mut render_pass.forget_lifetime(),
                            &paint_jobs,
                            &screen,
                        );
                    }

                    let egui_commands = encoder.finish();
                    gpu.queue.submit(
                        user_command_buffers
                            .into_iter()
                            .chain(std::iter::once(egui_commands)),
                    );

                    for texture_id in textures_delta.free.drain() {
                        egui_renderer.free_texture(&texture_id);
                    }

                    window.pre_present_notify();
                    gpu.queue.present(output_frame);
                }
            }
            WindowEvent::Resized(size) => match self.gpu.as_mut() {
                Some(gpu) => match gpu.resize_surface(size.width, size.height) {
                    Ok(reconfigured) => {
                        if reconfigured && self.egui_renderer.is_none() {
                            if let Some(config) = gpu.surface_config.as_ref() {
                                self.egui_renderer = Some(egui_wgpu::Renderer::new(
                                    &gpu.device,
                                    config.format,
                                    egui_wgpu::RendererOptions::default(),
                                ));
                            }
                        }

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
