//! Aspect-preserving export output dimensions and composition pixel scaling.
//! Creative coordinates and the stored Project remain in composition units.

use rhythm_core::project::{MAX_COMPOSITION_DIMENSION, MIN_COMPOSITION_DIMENSION, ProjectSettings};

use crate::export_job::ExportJobSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportResolutionError {
    InvalidDimensions,
    AspectRatioMismatch,
    NonFiniteSpatialValue,
    SpatialOverflow,
}

/// Exact integer ratio check before any float conversion, preventing
/// distorted shapes, fonts, or spatial effects at custom export resolutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportOutputResolution {
    composition: [u32; 2],
    output: [u32; 2],
}

impl ExportOutputResolution {
    pub fn new(
        settings: &ProjectSettings,
        output_width: u32,
        output_height: u32,
    ) -> Result<Self, ExportResolutionError> {
        let original = [settings.composition_width, settings.composition_height];
        let valid =
            |value: u32| (MIN_COMPOSITION_DIMENSION..=MAX_COMPOSITION_DIMENSION).contains(&value);
        if !valid(original[0])
            || !valid(original[1])
            || !valid(output_width)
            || !valid(output_height)
        {
            return Err(ExportResolutionError::InvalidDimensions);
        }
        if u64::from(output_width) * u64::from(original[1])
            != u64::from(output_height) * u64::from(original[0])
        {
            return Err(ExportResolutionError::AspectRatioMismatch);
        }
        Ok(Self {
            composition: original,
            output: [output_width, output_height],
        })
    }

    pub fn native(snapshot: &ExportJobSnapshot) -> Result<Self, ExportResolutionError> {
        let settings = &snapshot.project().settings;
        Self::new(
            settings,
            settings.composition_width,
            settings.composition_height,
        )
    }

    #[must_use]
    pub const fn composition_size(self) -> [u32; 2] {
        self.composition
    }

    #[must_use]
    pub const fn output_size(self) -> [u32; 2] {
        self.output
    }

    /// The exact un-reduced numerator/denominator remain integer values for
    /// geometry/effect callers that must avoid premature float rounding.
    #[must_use]
    pub const fn scale_ratio(self) -> (u32, u32) {
        (self.output[0], self.composition[0])
    }

    /// All composition-pixel spatial quantities (geometry, font sizes,
    /// blur/glow radius, Noise block size and RGB Split offsets) share this
    /// conversion. Color, opacity and effect intensity remain unscaled.
    pub fn spatial_pixels(self, composition_pixels: f32) -> Result<f32, ExportResolutionError> {
        if !composition_pixels.is_finite() {
            return Err(ExportResolutionError::NonFiniteSpatialValue);
        }
        let result = f64::from(composition_pixels) * f64::from(self.output[0])
            / f64::from(self.composition[0]);
        if !result.is_finite() || result.abs() > f64::from(f32::MAX) {
            return Err(ExportResolutionError::SpatialOverflow);
        }
        Ok(result as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportOutputResolution, ExportResolutionError};
    use crate::{export_job::ExportJobSnapshot, reference_scenes::five_effects_reference_scene};
    use rhythm_core::{editor::ProjectEditor, project::ProjectSettings};

    #[test]
    fn native_and_720p_and_4k_preserve_exact_16_9_aspect_ratio() {
        let settings = ProjectSettings::default();
        let native = ExportOutputResolution::new(&settings, 1920, 1080).expect("native");
        let half = ExportOutputResolution::new(&settings, 1280, 720).expect("720p");
        let double = ExportOutputResolution::new(&settings, 3840, 2160).expect("4K");
        assert_eq!(native.scale_ratio(), (1920, 1920));
        assert_eq!(half.scale_ratio(), (1280, 1920));
        assert_eq!(double.scale_ratio(), (3840, 1920));
        assert_eq!(half.output_size(), [1280, 720]);
        assert_eq!(half.spatial_pixels(120.0), Ok(80.0));
        assert_eq!(double.spatial_pixels(24.0), Ok(48.0));
        assert_eq!(half.spatial_pixels(-120.0), Ok(-80.0));
        assert_eq!(half.spatial_pixels(0.0), Ok(0.0));
    }

    #[test]
    fn rejects_stretch_even_if_integer_sizes_look_visually_close() {
        let settings = ProjectSettings::default();
        assert_eq!(
            ExportOutputResolution::new(&settings, 1280, 800),
            Err(ExportResolutionError::AspectRatioMismatch)
        );
        assert_eq!(
            ExportOutputResolution::new(&settings, 854, 480),
            Err(ExportResolutionError::AspectRatioMismatch)
        );
        assert_eq!(
            ExportOutputResolution::new(&settings, 0, 1080),
            Err(ExportResolutionError::InvalidDimensions)
        );
        assert_eq!(
            ExportOutputResolution::new(&settings, 9000, 5062),
            Err(ExportResolutionError::InvalidDimensions)
        );
    }

    #[test]
    fn arbitrary_4_3_compositions_can_be_scaled_without_16_9_assumptions() {
        let mut settings = ProjectSettings::default();
        settings.composition_width = 1024;
        settings.composition_height = 768;
        let resolution = ExportOutputResolution::new(&settings, 640, 480).expect("4:3 scale");
        assert_eq!(resolution.scale_ratio(), (640, 1024));
        assert_eq!(resolution.spatial_pixels(256.0), Ok(160.0));
    }

    #[test]
    fn snapshot_and_editor_values_do_not_change_when_export_resolution_changes() {
        let project = five_effects_reference_scene().project;
        let editor = ProjectEditor::new(project.clone()).expect("project");
        let snapshot = ExportJobSnapshot::capture(&editor).expect("snapshot");
        let native = ExportOutputResolution::native(&snapshot).expect("native");
        let reduced =
            ExportOutputResolution::new(&snapshot.project().settings, 1280, 720).expect("reduced");
        assert_eq!(native.output_size(), [1920, 1080]);
        assert_eq!(reduced.output_size(), [1280, 720]);
        assert_eq!(editor.project(), &project);
        assert_eq!(snapshot.project(), &project);
        assert_eq!(
            reduced.spatial_pixels(f32::NAN),
            Err(ExportResolutionError::NonFiniteSpatialValue)
        );
    }
}
