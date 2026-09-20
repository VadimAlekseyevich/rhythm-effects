#![forbid(unsafe_code)]

mod editor_session;
mod editor_ui;
mod gpu;
mod timeline;
mod viewport;

use std::sync::Arc;
use std::time::Instant;

use editor_session::{EditorSession, ViewportCameraAction};
use editor_ui::DiagnosticsView;
use gpu::GpuContext;
use rhythm_core::APP_NAME;
use rhythm_engine::renderer::Renderer;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{Window, WindowId},
};

fn fail_startup(
    event_loop: &ActiveEventLoop,
    stage: &'static str,
    message: impl std::fmt::Display,
) {
    error!(stage, error = %message, "fatal startup failure");
    event_loop.exit();
}

struct RhythmApp {
    session: EditorSession,
    project_editor: rhythm_core::editor::ProjectEditor,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    egui_context: Option<egui::Context>,
    egui_state: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    composition_texture_id: Option<egui::TextureId>,
    diagnostics: Option<DiagnosticsView>,
    waveform: Option<rhythm_engine::waveform::WaveformData>,
    modifiers: ModifiersState,
    last_frame_instant: Option<Instant>,
}

impl Default for RhythmApp {
    fn default() -> Self {
        let project = rhythm_core::project::Project::new(
            "Untitled",
            rhythm_core::project::ProjectSettings::default(),
            rhythm_core::time::TempoMap::unset(rhythm_core::time::GridOffsetNs::new(0)),
        );
        let project_editor =
            rhythm_core::editor::ProjectEditor::new(project).expect("default project is valid");

        Self {
            session: EditorSession::default(),
            project_editor,
            window: None,
            gpu: None,
            renderer: None,
            egui_context: None,
            egui_state: None,
            egui_renderer: None,
            composition_texture_id: None,
            diagnostics: None,
            waveform: None,
            modifiers: ModifiersState::default(),
            last_frame_instant: None,
        }
    }
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
                        renderer.refresh_preview_display(&gpu.device, &gpu.queue);

                        let egui_context = egui::Context::default();
                        editor_ui::configure_theme(&egui_context);
                        let egui_state = egui_winit::State::new(
                            egui_context.clone(),
                            egui::ViewportId::ROOT,
                            event_loop,
                            Some(window.scale_factor() as f32),
                            event_loop.system_theme(),
                            Some(gpu.device.limits().max_texture_dimension_2d as usize),
                        );

                        let mut egui_renderer = gpu.surface_config.as_ref().map(|config| {
                            egui_wgpu::Renderer::new(
                                &gpu.device,
                                config.format,
                                egui_wgpu::RendererOptions::default(),
                            )
                        });
                        let composition_texture_id = egui_renderer.as_mut().map(|egui_renderer| {
                            egui_renderer.register_native_texture(
                                &gpu.device,
                                renderer.preview_display_view(),
                                wgpu::FilterMode::Linear,
                            )
                        });

                        let adapter = gpu.adapter_summary();
                        let surface_size = gpu.surface_size();
                        let window_size = window.inner_size();
                        let diagnostics = DiagnosticsView {
                            adapter_name: adapter.name.clone(),
                            backend: adapter.backend.clone(),
                            window_size: [window_size.width, window_size.height],
                            frame_time_ms: 0.0,
                        };
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
                        self.diagnostics = Some(diagnostics);
                        self.last_frame_instant = Some(Instant::now());
                        self.egui_context = Some(egui_context);
                        self.egui_state = Some(egui_state);
                        self.egui_renderer = egui_renderer;
                        self.composition_texture_id = composition_texture_id;
                        self.renderer = Some(renderer);
                        self.gpu = Some(gpu);
                        window.request_redraw();
                        self.window = Some(window);
                    }
                    Err(error) => {
                        fail_startup(event_loop, "gpu_initialization", error);
                    }
                }
            }
            Err(error) => {
                fail_startup(event_loop, "window_creation", error);
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
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if !is_synthetic
                    && event.state == ElementState::Pressed
                    && !self
                        .egui_context
                        .as_ref()
                        .is_some_and(|context| context.egui_wants_keyboard_input())
                    && let PhysicalKey::Code(code) = event.physical_key
                {
                    let control = self.modifiers.control_key();
                    let shift = self.modifiers.shift_key();
                    let alt = self.modifiers.alt_key();
                    let super_key = self.modifiers.super_key();
                    let direction = match code {
                        KeyCode::ArrowLeft => -1,
                        KeyCode::ArrowRight => 1,
                        _ => 0,
                    };

                    let handled = if control
                        && !shift
                        && !alt
                        && !super_key
                        && code == KeyCode::KeyC
                    {
                        self.session
                            .copy_selected_keyframes(self.project_editor.project())
                    } else if control && !shift && !alt && !super_key && code == KeyCode::KeyV {
                        match self
                            .session
                            .paste_keyframe_clipboard(&mut self.project_editor)
                        {
                            Ok(changed) => changed,
                            Err(error) => {
                                warn!(?error, "paste keyframes failed");
                                false
                            }
                        }
                    } else if control && !shift && !alt && !super_key && code == KeyCode::KeyD {
                        match self
                            .session
                            .duplicate_selected_keyframes(&mut self.project_editor)
                        {
                            Ok(changed) => changed,
                            Err(error) => {
                                warn!(?error, "duplicate selected keyframes failed");
                                false
                            }
                        }
                    } else if direction != 0 && alt && !shift && !super_key {
                        let result = if control {
                            self.session.move_selected_keyframes_by_beat(
                                &mut self.project_editor,
                                direction,
                            )
                        } else {
                            self.session.move_selected_keyframes_by_grid(
                                &mut self.project_editor,
                                direction,
                            )
                        };
                        match result {
                            Ok(changed) => changed,
                            Err(error) => {
                                warn!(?error, "selected keyframe keyboard move failed");
                                false
                            }
                        }
                    } else if !control && !shift && !alt && !super_key && code == KeyCode::Delete {
                        match self
                            .session
                            .delete_selected_keyframes(&mut self.project_editor)
                        {
                            Ok(changed) => changed,
                            Err(error) => {
                                warn!(?error, "delete selected keyframes failed");
                                false
                            }
                        }
                    } else if !control && !shift && !alt && !super_key && code == KeyCode::Escape {
                        let cancelled_keyframe_drag = self.session.cancel_keyframe_drag();
                        let cancelled_position_drag = self.session.cancel_viewport_position_drag();
                        let cancelled_multi_position_drag =
                            self.session.cancel_viewport_multi_position_drag();
                        let cancelled_scale_drag = self.session.cancel_viewport_scale_drag();
                        let cancelled_rotation_drag = self.session.cancel_viewport_rotation_drag();
                        cancelled_keyframe_drag
                            || cancelled_position_drag
                            || cancelled_multi_position_drag
                            || cancelled_scale_drag
                            || cancelled_rotation_drag
                    } else if !control && !alt && !super_key && code == KeyCode::KeyF {
                        let action = if shift {
                            ViewportCameraAction::FitComposition
                        } else {
                            ViewportCameraAction::FrameSelection
                        };
                        self.session.request_viewport_camera_action(action);
                        true
                    } else if !control && !shift && !alt && !super_key && code == KeyCode::KeyK {
                        match self.session.keyframe_action(&mut self.project_editor) {
                            Ok(changed) => changed,
                            Err(error) => {
                                warn!(?error, "keyframe action failed");
                                false
                            }
                        }
                    } else if direction != 0 && !alt && !super_key {
                        let project = self.project_editor.project();
                        if control && shift {
                            self.session.step_playhead_bar(
                                &project.tempo_map,
                                project.settings.duration,
                                direction,
                            )
                        } else if control && !shift {
                            self.session.step_playhead_beat(
                                &project.tempo_map,
                                project.settings.duration,
                                direction,
                            )
                        } else if !control && !shift {
                            self.session.step_playhead_grid(
                                &project.tempo_map,
                                project.settings.duration,
                                direction,
                            )
                        } else {
                            false
                        }
                    } else if !control && !shift && !alt && !super_key {
                        match code {
                            KeyCode::BracketLeft => self.session.change_authoring_division(false),
                            KeyCode::BracketRight => self.session.change_authoring_division(true),
                            _ => false,
                        }
                    } else {
                        false
                    };

                    if handled {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CloseRequested => {
                info!("close requested");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let (Some(gpu), Some(egui_context), Some(egui_state), Some(egui_renderer)) = (
                    self.gpu.as_mut(),
                    self.egui_context.as_ref(),
                    self.egui_state.as_mut(),
                    self.egui_renderer.as_mut(),
                ) {
                    let now = Instant::now();
                    let frame_time_ms = self
                        .last_frame_instant
                        .replace(now)
                        .map_or(0.0, |previous| {
                            now.duration_since(previous).as_secs_f32() * 1000.0
                        });

                    if let Some(diagnostics) = self.diagnostics.as_mut() {
                        let size = window.inner_size();
                        diagnostics.window_size = [size.width, size.height];
                        diagnostics.frame_time_ms = frame_time_ms;
                    }

                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_context.run_ui(raw_input, |root_ui| {
                        if let Some(diagnostics) = self.diagnostics.as_ref() {
                            editor_ui::draw_editor_shell(
                                root_ui,
                                &mut self.session,
                                self.project_editor.project(),
                                diagnostics,
                                self.composition_texture_id,
                                self.waveform.as_ref(),
                            );
                        }
                    });
                    match self
                        .session
                        .sync_viewport_position_drag(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "viewport position drag failed"),
                    }
                    match self
                        .session
                        .sync_viewport_multi_position_drag(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "viewport multi-position drag failed"),
                    }
                    match self
                        .session
                        .sync_viewport_scale_drag(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "viewport scale drag failed"),
                    }
                    match self
                        .session
                        .sync_viewport_rotation_drag(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "viewport rotation drag failed"),
                    }
                    match self
                        .session
                        .commit_pending_object_list_actions(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "object list state edit failed"),
                    }
                    match self
                        .session
                        .commit_pending_inspector_static_property_edit(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "inspector static property edit failed"),
                    }
                    match self
                        .session
                        .commit_pending_focused_keyframe_action(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "inspector keyframe action failed"),
                    }
                    match self
                        .session
                        .commit_pending_keyframe_interpolation(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "keyframe interpolation change failed"),
                    }
                    match self
                        .session
                        .commit_pending_keyframe_move(&mut self.project_editor)
                    {
                        Ok(true) => window.request_redraw(),
                        Ok(false) => {}
                        Err(error) => warn!(?error, "keyframe drag commit failed"),
                    }
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
                        if reconfigured
                            && self.egui_renderer.is_none()
                            && let Some(config) = gpu.surface_config.as_ref()
                        {
                            let mut egui_renderer = egui_wgpu::Renderer::new(
                                &gpu.device,
                                config.format,
                                egui_wgpu::RendererOptions::default(),
                            );
                            self.composition_texture_id = self.renderer.as_ref().map(|renderer| {
                                egui_renderer.register_native_texture(
                                    &gpu.device,
                                    renderer.preview_display_view(),
                                    wgpu::FilterMode::Linear,
                                )
                            });
                            self.egui_renderer = Some(egui_renderer);
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
