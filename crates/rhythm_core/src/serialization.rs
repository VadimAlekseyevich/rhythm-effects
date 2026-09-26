use crate::project::Project;
use serde::{Deserialize, Serialize};

pub const PROJECT_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFileV1 {
    pub schema_version: u32,
    pub created_with_version: String,
    pub project: Project,
}

impl ProjectFileV1 {
    pub const SCHEMA_VERSION: u32 = PROJECT_SCHEMA_VERSION_V1;

    #[must_use]
    pub fn new(project: Project, created_with_version: impl Into<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            created_with_version: created_with_version.into(),
            project,
        }
    }

    #[must_use]
    pub fn for_current_app(project: Project) -> Self {
        Self::new(project, env!("CARGO_PKG_VERSION"))
    }
}

/// A structural parse error. Semantic validation and schema compatibility are separate load steps.
#[derive(Debug)]
pub enum ProjectFileParseError {
    InvalidUtf8(std::str::Utf8Error),
    InvalidJson(serde_json::Error),
}

impl std::fmt::Display for ProjectFileParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8(error) => write!(formatter, "project file is not UTF-8: {error}"),
            Self::InvalidJson(error) => write!(formatter, "invalid project JSON: {error}"),
        }
    }
}

impl std::error::Error for ProjectFileParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            Self::InvalidJson(error) => Some(error),
        }
    }
}

/// Parses .rhfx bytes into a detached V1 candidate without modifying an active editor.
///
/// This checks UTF-8 and the JSON/schema shape only. Version compatibility, semantic
/// validation, migrations, and active-project replacement belong to later load steps.
pub fn parse_project_file_v1(bytes: &[u8]) -> Result<ProjectFileV1, ProjectFileParseError> {
    let text = std::str::from_utf8(bytes).map_err(ProjectFileParseError::InvalidUtf8)?;
    serde_json::from_str(text).map_err(ProjectFileParseError::InvalidJson)
}

#[cfg(test)]
mod tests {
    use super::{
        PROJECT_SCHEMA_VERSION_V1, ProjectFileParseError, ProjectFileV1, parse_project_file_v1,
    };
    use crate::{
        animation::{Animated, BezierEasing, Interpolation, Keyframe},
        domain::{LinearRgba, Vec2},
        ids::{AssetId, EffectId, KeyframeId, ObjectId},
        project::{
            AssetKind, AssetRecord, AssetSource, AudioTrack, BlurEffect, Composition, Effect,
            EffectKind, EllipseObject, FontReference, FontStyle, FontWeight, GlowEffect,
            ImageObject, NoiseEffect, Object, ObjectContent, Project, ProjectSettings,
            RectangleObject, RgbSplitEffect, TextAlignment, TextObject, TintEffect,
            TransformAnimation,
        },
        time::{BpmMicros, GridOffsetNs, MusicalTick, TempoMap, TimeSignature},
    };
    use serde::{Deserialize, Serialize};

    fn project() -> Project {
        Project::new(
            "Schema wrapper test",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        )
    }

    fn transform() -> TransformAnimation {
        TransformAnimation::new(
            Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
            Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
            Animated::new_static(0.0),
            Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
            Animated::new_static(1.0),
        )
    }

    fn schema_project() -> Project {
        let audio_id = AssetId::new(1).expect("audio id");
        let image_id = AssetId::new(2).expect("image id");
        let mut project = Project::new(
            "Serde V1",
            ProjectSettings::default(),
            TempoMap::with_initial_tempo(
                GridOffsetNs::new(-25_000_000),
                BpmMicros::new(120_000_000).expect("tempo"),
                TimeSignature::new(4, 4).expect("meter"),
            ),
        );
        project.assets = vec![
            AssetRecord {
                id: audio_id,
                kind: AssetKind::Audio,
                source: AssetSource::File {
                    path: "audio/beat.flac".to_owned(),
                    relative_to_project: true,
                },
            },
            AssetRecord {
                id: image_id,
                kind: AssetKind::Image,
                source: AssetSource::File {
                    path: "C:/media/card.webp".to_owned(),
                    relative_to_project: false,
                },
            },
        ];
        project.audio_track = Some(AudioTrack {
            asset_id: audio_id,
            gain: 0.75,
        });

        let radius = Animated::with_keyframes(
            4.0,
            vec![
                Keyframe::new(
                    KeyframeId::new(12).expect("key"),
                    MusicalTick::new(0),
                    4.0,
                    Interpolation::Hold,
                ),
                Keyframe::new(
                    KeyframeId::new(13).expect("key"),
                    MusicalTick::new(960),
                    24.0,
                    Interpolation::CubicBezier(
                        BezierEasing::new(0.25, 0.1, 0.75, 0.9).expect("bezier"),
                    ),
                ),
                Keyframe::new(
                    KeyframeId::new(14).expect("key"),
                    MusicalTick::new(1_920),
                    8.0,
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("animated radius");

        let effects = vec![
            Effect {
                id: EffectId::new(7).expect("effect id"),
                enabled: true,
                kind: EffectKind::Blur(BlurEffect { radius_px: radius }),
            },
            Effect {
                id: EffectId::new(8).expect("effect id"),
                enabled: true,
                kind: EffectKind::Glow(GlowEffect {
                    radius_px: Animated::new_static(16.0),
                    intensity: Animated::new_static(1.25),
                    threshold: Animated::new_static(0.3),
                    color: Animated::new_static(
                        LinearRgba::new(1.0, 0.4, 0.1, 1.0).expect("glow color"),
                    ),
                }),
            },
            Effect {
                id: EffectId::new(9).expect("effect id"),
                enabled: false,
                kind: EffectKind::Tint(TintEffect {
                    color: Animated::new_static(
                        LinearRgba::new(0.1, 0.2, 0.8, 1.0).expect("tint color"),
                    ),
                    amount: Animated::new_static(0.5),
                }),
            },
            Effect {
                id: EffectId::new(10).expect("effect id"),
                enabled: true,
                kind: EffectKind::Noise(NoiseEffect {
                    amount: Animated::new_static(0.15),
                    size_px: Animated::new_static(3.0),
                    evolution: Animated::new_static(2.5),
                    seed: 42,
                }),
            },
            Effect {
                id: EffectId::new(11).expect("effect id"),
                enabled: true,
                kind: EffectKind::RgbSplit(RgbSplitEffect {
                    amount_px: Animated::new_static(6.0),
                    angle_degrees: Animated::new_static(30.0),
                }),
            },
        ];

        project.composition = Composition {
            objects: vec![
                Object {
                    id: ObjectId::new(3).expect("object id"),
                    name: "Rectangle".to_owned(),
                    visible: true,
                    locked: false,
                    transform: transform(),
                    content: ObjectContent::Rectangle(RectangleObject {
                        size: Animated::new_static(
                            Vec2::new(640.0, 360.0).expect("rectangle size"),
                        ),
                        fill: Animated::new_static(
                            LinearRgba::new(0.2, 0.3, 0.4, 1.0).expect("rectangle fill"),
                        ),
                        corner_radius: Animated::new_static(12.0),
                    }),
                    effects,
                },
                Object {
                    id: ObjectId::new(4).expect("object id"),
                    name: "Ellipse".to_owned(),
                    visible: true,
                    locked: false,
                    transform: transform(),
                    content: ObjectContent::Ellipse(EllipseObject {
                        size: Animated::new_static(Vec2::new(200.0, 200.0).expect("ellipse size")),
                        fill: Animated::new_static(
                            LinearRgba::new(0.8, 0.2, 0.3, 1.0).expect("ellipse fill"),
                        ),
                    }),
                    effects: Vec::new(),
                },
                Object {
                    id: ObjectId::new(5).expect("object id"),
                    name: "Image".to_owned(),
                    visible: true,
                    locked: false,
                    transform: transform(),
                    content: ObjectContent::Image(ImageObject { asset: image_id }),
                    effects: Vec::new(),
                },
                Object {
                    id: ObjectId::new(6).expect("object id"),
                    name: "Text".to_owned(),
                    visible: true,
                    locked: false,
                    transform: transform(),
                    content: ObjectContent::Text(TextObject {
                        text: "Rhythm Привет".to_owned(),
                        font: FontReference {
                            family: "Inter".to_owned(),
                            weight: FontWeight::Bold,
                            style: FontStyle::Italic,
                        },
                        font_size: 72.0,
                        color: Animated::new_static(
                            LinearRgba::new(1.0, 1.0, 1.0, 1.0).expect("text color"),
                        ),
                        alignment: TextAlignment::Center,
                    }),
                    effects: Vec::new(),
                },
            ],
        };
        project.next_entity_id = 15;
        project.validate().expect("schema project must be valid");
        project
    }

    fn assert_serde<T>()
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
    }

    #[test]
    fn project_file_v1_sets_fixed_schema_version_and_preserves_metadata() {
        let file = ProjectFileV1::new(project(), "0.9.7-test");

        assert_eq!(PROJECT_SCHEMA_VERSION_V1, 1);
        assert_eq!(ProjectFileV1::SCHEMA_VERSION, 1);
        assert_eq!(file.schema_version, 1);
        assert_eq!(file.created_with_version, "0.9.7-test");
        assert_eq!(file.project.metadata.name, "Schema wrapper test");
    }

    #[test]
    fn current_app_wrapper_records_package_version_without_changing_schema_version() {
        let file = ProjectFileV1::for_current_app(project());

        assert_eq!(file.schema_version, 1);
        assert_eq!(file.created_with_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn project_file_v1_is_serde_round_trip_safe_for_semantic_project_graph() {
        assert_serde::<ProjectFileV1>();

        let file = ProjectFileV1::new(schema_project(), "0.1.0-test");
        let json = serde_json::to_string_pretty(&file).expect("serialize ProjectFileV1");
        let decoded: ProjectFileV1 =
            serde_json::from_str(&json).expect("deserialize ProjectFileV1");

        assert_eq!(decoded, file);
        decoded
            .project
            .validate()
            .expect("round-trip project valid");
    }

    #[test]
    fn rhfx_parser_produces_detached_candidate_from_utf8_json() {
        let mut project = schema_project();
        project.metadata.name = "Ритм ♪".to_owned();
        let source = ProjectFileV1::new(project, "0.1.0-test");
        let bytes = serde_json::to_vec(&source).expect("encode project");

        let candidate = parse_project_file_v1(&bytes).expect("parse candidate");

        assert_eq!(candidate, source);
        assert_eq!(candidate.project.metadata.name, "Ритм ♪");
        candidate.project.validate().expect("valid candidate");
    }

    #[test]
    fn rhfx_parser_reports_invalid_utf8_separately_from_json() {
        let result = parse_project_file_v1(&[b'{', 0xff, b'}']);

        assert!(matches!(result, Err(ProjectFileParseError::InvalidUtf8(_))));
    }

    #[test]
    fn rhfx_parser_rejects_truncated_and_wrong_shape_json() {
        for bytes in [
            b"{\"schema_version\":1,\"project\":{".as_slice(),
            b"{}".as_slice(),
            b"{\"schema_version\":1} trailing".as_slice(),
        ] {
            let result = parse_project_file_v1(bytes);
            assert!(matches!(result, Err(ProjectFileParseError::InvalidJson(_))));
        }
    }

    #[test]
    fn rhfx_parser_does_not_prematurely_apply_semantic_validation() {
        let mut project = schema_project();
        project.settings.composition_width = 0;
        let bytes = serde_json::to_vec(&ProjectFileV1::new(project, "0.1.0-test")).expect("encode");

        let candidate = parse_project_file_v1(&bytes).expect("structurally valid candidate");

        assert_eq!(candidate.project.settings.composition_width, 0);
        assert!(candidate.project.validate().is_err());
    }

    #[test]
    fn json_uses_semantic_integer_units_and_root_version_fields() {
        let file = ProjectFileV1::new(schema_project(), "0.1.0-test");
        let value = serde_json::to_value(file).expect("serialize value");

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["created_with_version"], "0.1.0-test");
        assert!(value["project"]["settings"]["duration"].is_u64());
        assert!(value["project"]["composition"]["objects"][0]["id"].is_u64());
        assert!(
            value["project"]["composition"]["objects"][0]["content"]["Rectangle"]["corner_radius"]
                ["keyframes"]
                .is_array()
        );
    }
}
