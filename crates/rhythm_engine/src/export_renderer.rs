//! Export-only renderer ownership. Planning and dependency checks precede
//! GPU allocation; the live preview Renderer is never borrowed or mutated.

use std::path::Path;

use rhythm_core::time::FrameRate;

use crate::{
    export_job::ExportJobSnapshot,
    export_readback::{
        ExportReadbackError, ExportReadbackLayout, ExportReadbackPool, ExportReadbackTicket,
    },
    export_resolution::{ExportOutputResolution, ExportResolutionError},
    export_resources::{ExportResourceErrors, PreparedExportResources},
    export_sdr::SdrExportConverter,
    export_timeline::{ExportFrameError, ExportFrameTimeline, ExportTimelineError},
    renderer::Renderer,
    scene_eval::EvaluatedScene,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ExportPreparationError {
    Resolution(ExportResolutionError),
    Timeline(ExportTimelineError),
    Resources(ExportResourceErrors),
}

/// A separate, validated job plan. Only capture this from an immutable
/// ExportJobSnapshot, not from the editable ProjectEditor or preview caches.
#[derive(Debug)]
pub struct ExportRendererPlan {
    snapshot: ExportJobSnapshot,
    resources: PreparedExportResources,
    timeline: ExportFrameTimeline,
    resolution: ExportOutputResolution,
}

impl ExportRendererPlan {
    /// Do filesystem and font preflight on the export worker, before a GPU
    /// target or the first encoded frame is allocated.
    pub fn prepare(
        snapshot: ExportJobSnapshot,
        canonical_project_path: Option<&Path>,
        output_fps: FrameRate,
        output_size: [u32; 2],
    ) -> Result<Self, ExportPreparationError> {
        let resolution = ExportOutputResolution::new(
            &snapshot.project().settings,
            output_size[0],
            output_size[1],
        )
        .map_err(ExportPreparationError::Resolution)?;
        let timeline = ExportFrameTimeline::full_composition(&snapshot, output_fps)
            .map_err(ExportPreparationError::Timeline)?;
        let resources = snapshot
            .prepare_resources(canonical_project_path)
            .map_err(ExportPreparationError::Resources)?;
        Ok(Self {
            snapshot,
            resources,
            timeline,
            resolution,
        })
    }

    #[must_use]
    pub const fn snapshot(&self) -> &ExportJobSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub const fn resources(&self) -> &PreparedExportResources {
        &self.resources
    }

    #[must_use]
    pub const fn timeline(&self) -> ExportFrameTimeline {
        self.timeline
    }

    #[must_use]
    pub const fn resolution(&self) -> ExportOutputResolution {
        self.resolution
    }

    pub fn evaluate_frame(&self, index: u64) -> Result<EvaluatedScene, ExportFrameError> {
        self.timeline.evaluate_frame(&self.snapshot, index)
    }

    /// Allocate new GPU textures, independent image/text/effect caches and
    /// the very same creative shader pipelines used by the preview Renderer.
    pub fn initialize_gpu(self, device: &wgpu::Device, queue: &wgpu::Queue) -> ExportRendererState {
        let renderer =
            Renderer::new_with_composition_size(device, queue, self.resolution.output_size());
        let sdr = SdrExportConverter::new(
            device,
            renderer.composition_view(),
            self.resolution.output_size(),
        );
        let layout = ExportReadbackLayout::new(self.resolution.output_size())
            .expect("previously validated nonzero resolution fits readback rows");
        let readback = ExportReadbackPool::new(device, layout);
        ExportRendererState {
            plan: self,
            renderer,
            sdr,
            readback,
        }
    }
}

/// Only owned by the export worker. It cannot mutate the app's preview GPU
/// state because that renderer is never supplied to this constructor.
#[derive(Debug)]
pub struct ExportRendererState {
    plan: ExportRendererPlan,
    renderer: Renderer,
    sdr: SdrExportConverter,
    readback: ExportReadbackPool,
}

impl ExportRendererState {
    #[must_use]
    pub const fn plan(&self) -> &ExportRendererPlan {
        &self.plan
    }

    #[must_use]
    pub const fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// Start one exact-index full-resolution creative frame in the export
    /// renderer's offscreen composition target.
    ///
    /// The frame is evaluated from the immutable snapshot, the output-sized
    /// linear composition is cleared in the caller's command stream, and the
    /// supplied draw callback receives the same Renderer type used by preview.
    /// Keeping the callback here is deliberate while AI-279 finishes the shared
    /// EvaluatedScene primitive/image/text rasterizer; this method owns frame
    /// ordering without pretending that rasterizer already exists.
    pub fn encode_creative_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame_index: u64,
        draw_scene: impl FnOnce(&mut Renderer, &EvaluatedScene, &mut wgpu::CommandEncoder),
    ) -> Result<EvaluatedScene, ExportFrameError> {
        let scene = self.plan.evaluate_frame(frame_index)?;
        self.renderer.encode_clear_composition(encoder);
        draw_scene(&mut self.renderer, &scene, encoder);
        Ok(scene)
    }

    #[must_use]
    pub const fn sdr_output(&self) -> &SdrExportConverter {
        &self.sdr
    }

    /// After the creative frame is composed, encode shared linear-to-sRGB
    /// conversion into a distinct straight-alpha RGBA8 COPY_SRC target.
    pub fn encode_sdr_frame(&self, encoder: &mut wgpu::CommandEncoder) {
        self.sdr.encode(encoder);
    }

    /// Encode only after the SDR conversion pass in the same command stream.
    /// When all three slots are busy, apply backpressure rather than allocate.
    pub fn encode_readback(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame_index: u64,
    ) -> Result<ExportReadbackTicket, ExportReadbackError> {
        self.readback.encode_copy(encoder, &self.sdr, frame_index)
    }

    pub fn readback_pool(&mut self) -> &mut ExportReadbackPool {
        &mut self.readback
    }

    pub fn evaluate_frame(&self, index: u64) -> Result<EvaluatedScene, ExportFrameError> {
        self.plan.evaluate_frame(index)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportPreparationError, ExportRendererPlan};
    use crate::{
        export_job::ExportJobSnapshot, export_resolution::ExportResolutionError,
        reference_scenes::five_effects_reference_scene,
    };
    use rhythm_core::{
        editor::ProjectEditor,
        time::{FrameRate, ProjectTimeNs},
    };

    #[test]
    fn preflighted_export_uses_independent_snapshot_and_requested_render_size() {
        let reference = five_effects_reference_scene();
        let editor = ProjectEditor::new(reference.project.clone()).expect("editor");
        let snapshot = ExportJobSnapshot::capture(&editor).expect("snapshot");
        let plan = ExportRendererPlan::prepare(
            snapshot,
            None,
            FrameRate::new(30, 1).expect("30 fps"),
            [1280, 720],
        )
        .expect("prepared plan");
        assert_eq!(plan.resolution().output_size(), [1280, 720]);
        assert_eq!(plan.timeline().frame_count(), 300);
        assert!(plan.resources().images.is_empty());
        assert!(plan.resources().audio.is_none());
        assert_eq!(
            plan.evaluate_frame(0).expect("start scene"),
            plan.snapshot()
                .evaluate_at(ProjectTimeNs::new(0))
                .expect("same evaluator")
        );
        assert_eq!(editor.project(), &reference.project);
        assert_eq!(plan.snapshot().project(), &reference.project);
    }

    #[test]
    fn unsupported_stretched_resolution_is_rejected_before_resource_or_gpu_work() {
        let reference = five_effects_reference_scene();
        let editor = ProjectEditor::new(reference.project).expect("editor");
        let snapshot = ExportJobSnapshot::capture(&editor).expect("snapshot");
        assert!(matches!(
            ExportRendererPlan::prepare(
                snapshot,
                None,
                FrameRate::new(60, 1).expect("fps"),
                [1280, 800]
            ),
            Err(ExportPreparationError::Resolution(
                ExportResolutionError::AspectRatioMismatch
            ))
        ));
    }
}
