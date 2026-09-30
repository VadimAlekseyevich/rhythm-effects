//! Hardware-free release acceptance checks for supported export variants.
//! These tests validate planning and structured encoder configuration; they
//! do not substitute for GPU/FFmpeg bitstream validation on release hardware.

#[cfg(test)]
mod tests {
    use std::{ffi::{OsStr, OsString}, path::PathBuf};

    use crate::{
        export_ffmpeg::{ExportQuality, FfmpegExportOptions},
        export_job::ExportJobSnapshot,
        export_renderer::ExportRendererPlan,
        reference_scenes::five_effects_reference_scene,
    };
    use rhythm_core::{
        editor::ProjectEditor,
        time::{DurationNs, FrameRate},
    };

    fn contains_pair(args: &[OsString], flag: &str, value: &str) -> bool {
        args.windows(2).any(|pair| {
            pair[0].as_os_str() == OsStr::new(flag) && pair[1].as_os_str() == OsStr::new(value)
        })
    }

    fn captured_ten_seconds() -> ExportJobSnapshot {
        let mut fixture = five_effects_reference_scene();
        fixture.project.settings.duration = DurationNs::new(10_000_000_000);
        ExportJobSnapshot::capture(
            &ProjectEditor::new(fixture.project).expect("valid reference project"),
        )
        .expect("snapshot")
    }

    fn options_for(size: [u32; 2], fps: FrameRate) -> (ExportRendererPlan, FfmpegExportOptions) {
        let snapshot = captured_ten_seconds();
        let plan = ExportRendererPlan::prepare(snapshot, None, fps, size).expect("valid variant");
        let root = std::env::temp_dir();
        let options = FfmpegExportOptions::from_plan(
            &plan,
            root.join("rhythm-test-ffmpeg.exe"),
            root.join("rhythm-test-output.partial.mp4"),
            ExportQuality::Balanced,
        );
        (plan, options)
    }

    #[test]
    fn release_variants_keep_resolution_fps_frame_count_and_rawvideo_contract_aligned() {
        let variants = [
            ([1920, 1080], FrameRate::new(60, 1).expect("60"), 600_u64, "1920x1080", "60/1"),
            ([1280, 720], FrameRate::new(60, 1).expect("60"), 600_u64, "1280x720", "60/1"),
            ([1920, 1080], FrameRate::new(30, 1).expect("30"), 300_u64, "1920x1080", "30/1"),
        ];

        for (size, fps, frames, video_size, frame_rate) in variants {
            let (plan, options) = options_for(size, fps);
            assert_eq!(plan.resolution().output_size(), size);
            assert_eq!(plan.timeline().output_fps(), fps);
            assert_eq!(plan.timeline().frame_count(), frames);
            assert_eq!(
                options.validate().expect("packed RGBA frame size"),
                usize::try_from(u64::from(size[0]) * u64::from(size[1]) * 4).expect("frame size")
            );

            let command = options.command().expect("structured FFmpeg command");
            let args: Vec<PathBuf> = command.get_args().map(PathBuf::from).collect();
            let args: Vec<OsString> = args
                .into_iter()
                .map(|arg| arg.into_os_string())
                .collect();
            assert!(contains_pair(&args, "-video_size", video_size));
            assert!(contains_pair(&args, "-framerate", frame_rate));
            assert!(contains_pair(&args, "-frames:v", &frames.to_string()));
            assert!(contains_pair(&args, "-pix_fmt", "rgba"));
            assert!(contains_pair(&args, "-pix_fmt", "yuv420p"));
        }
    }

    #[test]
    fn variant_overrides_never_mutate_the_captured_composition_settings() {
        let snapshot = captured_ten_seconds();
        let original = snapshot.project().settings.clone();
        for (size, fps) in [
            ([1920, 1080], FrameRate::new(60, 1).expect("60")),
            ([1280, 720], FrameRate::new(60, 1).expect("60")),
            ([1920, 1080], FrameRate::new(30, 1).expect("30")),
        ] {
            let plan = ExportRendererPlan::prepare(snapshot.clone(), None, fps, size)
                .expect("variant plan");
            assert_eq!(plan.snapshot().project().settings, original);
        }
        assert_eq!(snapshot.project().settings, original);
    }
}
