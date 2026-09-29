//! All-or-nothing preflight before export frame zero.
//! The caller supplies the captured project identity; no editor state is read.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

use rhythm_core::{
    ids::{AssetId, ObjectId},
    project::{AssetKind, AssetSource, ObjectContent, Project},
};

use crate::{
    audio::{AudioProbe, probe_audio_file},
    export_job::ExportJobSnapshot,
    image_decode::{DecodedImage, ImageDecodeError, decode_image_file},
    text::{ExportFontReadiness, preflight_export_fonts},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportResourceIssue {
    InvalidSourcePath { asset_id: AssetId, reason: &'static str },
    MissingAssetRecord { asset_id: AssetId },
    WrongAssetKind { asset_id: AssetId, expected: AssetKind },
    Image { asset_id: AssetId, path: PathBuf, error: ImageDecodeError },
    Audio { asset_id: AssetId, path: PathBuf, diagnostic: String },
    MissingFontGlyphs {
        object_id: ObjectId,
        resolved_family: String,
        count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportResourceErrors {
    /// Collected in deterministic project order so a UI can present all
    /// missing/broken dependencies in one actionable dialog.
    pub issues: Vec<ExportResourceIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedExportImage {
    pub path: PathBuf,
    pub decoded: DecodedImage,
}

#[derive(Debug)]
pub struct PreparedExportResources {
    pub images: BTreeMap<AssetId, PreparedExportImage>,
    pub audio: Option<AudioProbe>,
    /// Includes requested-family -> bundled Inter fallback decisions.
    pub fonts: Vec<ExportFontReadiness>,
}

fn resolve_source_path(
    source: &AssetSource,
    project_path: Option<&Path>,
) -> Result<PathBuf, &'static str> {
    let AssetSource::File {
        path,
        relative_to_project,
    } = source;
    let stored = Path::new(path);
    if stored.as_os_str().is_empty() {
        return Err("asset path is empty");
    }
    if stored
        .components()
        .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err("asset path is not normalized");
    }
    if *relative_to_project {
        if stored.is_absolute()
            || stored
                .components()
                .any(|part| matches!(part, Component::RootDir | Component::Prefix(_)))
        {
            return Err("project-relative asset must be a relative path");
        }
        let project = project_path.ok_or("project-relative asset requires a saved project path")?;
        if !project.is_absolute()
            || project
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err("canonical project path must be normalized and absolute");
        }
        let parent = project.parent().ok_or("project path has no parent directory")?;
        Ok(parent.join(stored))
    } else if stored.is_absolute() {
        Ok(stored.to_path_buf())
    } else {
        Err("absolute asset source must be an absolute path")
    }
}

impl ExportJobSnapshot {
    /// Decode visible images, probe the primary audio file and resolve/shape
    /// visible Text fonts before the first frame. Missing referenced resources
    /// abort the job rather than silently rendering placeholders. Unreferenced
    /// assets and invisible objects do not block an otherwise valid export.
    pub fn prepare_resources(
        &self,
        canonical_project_path: Option<&Path>,
    ) -> Result<PreparedExportResources, ExportResourceErrors> {
        prepare_project_resources(self.project(), canonical_project_path)
    }
}

fn prepare_project_resources(
    project: &Project,
    canonical_project_path: Option<&Path>,
) -> Result<PreparedExportResources, ExportResourceErrors> {
    let mut image_ids = BTreeSet::new();
    for object in &project.composition.objects {
        if object.visible
            && let ObjectContent::Image(image) = &object.content
        {
            image_ids.insert(image.asset);
        }
    }

    let mut issues = Vec::new();
    let mut images = BTreeMap::new();
    for asset_id in image_ids {
        let Some(record) = project.assets.iter().find(|asset| asset.id == asset_id) else {
            issues.push(ExportResourceIssue::MissingAssetRecord { asset_id });
            continue;
        };
        if record.kind != AssetKind::Image {
            issues.push(ExportResourceIssue::WrongAssetKind {
                asset_id,
                expected: AssetKind::Image,
            });
            continue;
        }
        match resolve_source_path(&record.source, canonical_project_path) {
            Ok(path) => match decode_image_file(&path) {
                Ok(decoded) => {
                    images.insert(asset_id, PreparedExportImage { path, decoded });
                }
                Err(error) => issues.push(ExportResourceIssue::Image {
                    asset_id,
                    path,
                    error,
                }),
            },
            Err(reason) => issues.push(ExportResourceIssue::InvalidSourcePath {
                asset_id,
                reason,
            }),
        }
    }

    let mut audio = None;
    if let Some(track) = &project.audio_track {
        let asset_id = track.asset_id;
        if let Some(record) = project.assets.iter().find(|asset| asset.id == asset_id) {
            if record.kind != AssetKind::Audio {
                issues.push(ExportResourceIssue::WrongAssetKind {
                    asset_id,
                    expected: AssetKind::Audio,
                });
            } else {
                match resolve_source_path(&record.source, canonical_project_path) {
                    Ok(path) => match probe_audio_file(&path) {
                        Ok(probe) => audio = Some(probe),
                        Err(error) => issues.push(ExportResourceIssue::Audio {
                            asset_id,
                            path,
                            diagnostic: format!("{error:?}"),
                        }),
                    },
                    Err(reason) => issues.push(ExportResourceIssue::InvalidSourcePath {
                        asset_id,
                        reason,
                    }),
                }
            }
        } else {
            issues.push(ExportResourceIssue::MissingAssetRecord { asset_id });
        }
    }

    let fonts = preflight_export_fonts(project);
    issues.extend(fonts.iter().filter(|font| font.missing_glyphs > 0).map(|font| {
        ExportResourceIssue::MissingFontGlyphs {
            object_id: font.object_id,
            resolved_family: font.resolved_family.clone(),
            count: font.missing_glyphs,
        }
    }));

    if issues.is_empty() {
        Ok(PreparedExportResources {
            images,
            audio,
            fonts,
        })
    } else {
        Err(ExportResourceErrors { issues })
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportResourceIssue, resolve_source_path};
    use crate::export_job::ExportJobSnapshot;
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
    use rhythm_core::{
        animation::Animated,
        domain::{LinearRgba, Vec2},
        editor::ProjectEditor,
        ids::{AssetId, ObjectId},
        project::{
            AssetKind, AssetRecord, AssetSource, AudioTrack, FontReference, FontStyle,
            FontWeight, ImageObject, Object, ObjectContent, Project, ProjectSettings,
            TextAlignment, TextObject, TransformAnimation,
        },
        time::{GridOffsetNs, TempoMap},
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "rhythm-export-resource-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).expect("create test directory");
            Self(dir)
        }
        fn join(&self, path: &str) -> PathBuf {
            self.0.join(path)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn make_project() -> Project {
        Project::new(
            "Resource preflight",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        )
    }

    fn transform() -> TransformAnimation {
        TransformAnimation::new(
            Animated::new_static(Vec2::new(320.0, 240.0).expect("position")),
            Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
            Animated::new_static(0.0),
            Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
            Animated::new_static(1.0),
        )
    }

    fn image_project(source: AssetSource) -> Project {
        let mut project = make_project();
        let asset_id = AssetId::new(1).expect("asset id");
        project.assets.push(AssetRecord {
            id: asset_id,
            kind: AssetKind::Image,
            source,
        });
        project.composition.objects.push(Object {
            id: ObjectId::new(2).expect("object id"),
            name: "image".into(),
            visible: true,
            locked: false,
            transform: transform(),
            content: ObjectContent::Image(ImageObject { asset: asset_id }),
            effects: Vec::new(),
        });
        project.next_entity_id = 3;
        project
    }

    fn capture(project: Project) -> ExportJobSnapshot {
        ExportJobSnapshot::capture(&ProjectEditor::new(project).expect("valid editor"))
            .expect("capture project")
    }

    #[test]
    fn absolute_and_project_relative_sources_require_safe_context() {
        let absolute = std::env::temp_dir().join("resource.png");
        let project = std::env::temp_dir().join("project.rhfx");
        let source = AssetSource::File {
            path: "media/image.png".into(),
            relative_to_project: true,
        };
        assert!(resolve_source_path(&source, None).is_err());
        assert_eq!(
            resolve_source_path(&source, Some(&project)).expect("relative source"),
            project.parent().expect("project directory").join("media/image.png")
        );
        assert!(resolve_source_path(
            &AssetSource::File {
                path: "../outside.png".into(),
                relative_to_project: true
            },
            Some(&project)
        )
        .is_err());
        assert_eq!(
            resolve_source_path(
                &AssetSource::File {
                    path: absolute.to_string_lossy().into_owned(),
                    relative_to_project: false
                },
                None
            ),
            Ok(absolute)
        );
    }

    #[test]
    fn image_preflight_decodes_required_source_before_frame_zero() {
        let root = TestDir::new();
        let path = root.join("small.png");
        let mut encoded = Vec::new();
        PngEncoder::new(&mut encoded)
            .write_image(&[255, 128, 0, 192], 1, 1, ExtendedColorType::Rgba8)
            .expect("encode");
        fs::write(&path, encoded).expect("write");
        let source = AssetSource::File {
            path: "small.png".into(),
            relative_to_project: true,
        };
        let job = capture(image_project(source));
        let project_path = root.join("saved.rhfx");
        let prepared = job.prepare_resources(Some(&project_path)).expect("preflight");
        let decoded = &prepared.images[&AssetId::new(1).expect("id")].decoded;
        assert_eq!((decoded.width, decoded.height), (1, 1));
        assert_eq!(&decoded.rgba8, &[255, 128, 0, 192]);
        assert!(prepared.audio.is_none());
    }

    #[test]
    fn aggregated_errors_block_missing_image_and_audio_together() {
        let root = TestDir::new();
        let mut project = image_project(AssetSource::File {
            path: "missing.png".into(),
            relative_to_project: true,
        });
        let audio_id = AssetId::new(3).expect("id");
        project.assets.push(AssetRecord {
            id: audio_id,
            kind: AssetKind::Audio,
            source: AssetSource::File {
                path: root.join("missing.wav").to_string_lossy().into_owned(),
                relative_to_project: false,
            },
        });
        project.audio_track = Some(AudioTrack::new(audio_id));
        project.next_entity_id = 4;
        let errors = capture(project)
            .prepare_resources(Some(&root.join("saved.rhfx")))
            .expect_err("unready assets block export");
        assert_eq!(errors.issues.len(), 2);
        assert!(matches!(errors.issues[0], ExportResourceIssue::Image { .. }));
        assert!(matches!(errors.issues[1], ExportResourceIssue::Audio { .. }));
    }

    #[test]
    fn invisible_image_and_unreferenced_asset_do_not_block_export() {
        let mut project = image_project(AssetSource::File {
            path: "missing.png".into(),
            relative_to_project: true,
        });
        project.composition.objects[0].visible = false;
        let resources = capture(project).prepare_resources(None).expect("unused media skipped");
        assert!(resources.images.is_empty());
        assert!(resources.fonts.is_empty());
    }

    #[test]
    fn missing_requested_font_uses_known_bundled_fallback() {
        let mut project = make_project();
        project.composition.objects.push(Object {
            id: ObjectId::new(1).expect("id"),
            name: "text".into(),
            visible: true,
            locked: false,
            transform: transform(),
            content: ObjectContent::Text(TextObject {
                text: "Hello Привет".into(),
                font: FontReference {
                    family: "RhythmEffects-No-Such-Font-987654".into(),
                    weight: FontWeight::Normal,
                    style: FontStyle::Normal,
                },
                font_size: 24.0,
                color: Animated::new_static(LinearRgba::black_opaque()),
                alignment: TextAlignment::Left,
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 2;
        let prepared = capture(project).prepare_resources(None).expect("bundled font fallback");
        assert_eq!(prepared.fonts.len(), 1);
        assert!(prepared.fonts[0].fallback_used);
        assert_eq!(prepared.fonts[0].resolved_family, "Inter");
        assert_eq!(prepared.fonts[0].missing_glyphs, 0);
    }
}
