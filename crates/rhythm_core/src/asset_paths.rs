use std::path::{Component, Path};

use crate::project::AssetSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPathError {
    ProjectPathNotAbsolute,
    AssetPathNotAbsolute,
    ProjectPathHasNoDirectory,
    UnnormalizedAssetPath,
    NonUtf8Path,
}

pub fn asset_source_for_saved_project(
    saved_project_path: &Path,
    asset_path: &Path,
) -> Result<AssetSource, AssetPathError> {
    if let Some(relative) = relative_asset_source(saved_project_path, asset_path)? {
        return Ok(relative);
    }

    if asset_path
        .components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(AssetPathError::UnnormalizedAssetPath);
    }

    let path = asset_path
        .to_str()
        .ok_or(AssetPathError::NonUtf8Path)?
        .to_owned();
    Ok(AssetSource::File {
        path,
        relative_to_project: false,
    })
}

pub fn relative_asset_source(
    saved_project_path: &Path,
    asset_path: &Path,
) -> Result<Option<AssetSource>, AssetPathError> {
    if !saved_project_path.is_absolute() {
        return Err(AssetPathError::ProjectPathNotAbsolute);
    }
    if !asset_path.is_absolute() {
        return Err(AssetPathError::AssetPathNotAbsolute);
    }

    let project_directory = saved_project_path
        .parent()
        .ok_or(AssetPathError::ProjectPathHasNoDirectory)?;
    let Ok(relative) = asset_path.strip_prefix(project_directory) else {
        return Ok(None);
    };

    if relative.as_os_str().is_empty()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Ok(None);
    }

    let path = relative
        .to_str()
        .ok_or(AssetPathError::NonUtf8Path)?
        .to_owned();

    Ok(Some(AssetSource::File {
        path,
        relative_to_project: true,
    }))
}

/// Re-express an asset against the published Save As location without touching
/// the filesystem. Missing assets retain their identity, and ineligible paths
/// (non-absolute origin/destination, parent traversal, non-UTF-8) are unchanged.
///
/// Existing project-relative paths are anchored to the *old* project directory
/// before their representation is chosen for the *new* directory. In-project
/// absolute paths may become relative on first Save As.
#[must_use]
pub fn rebase_asset_source_for_save_as(
    source: &AssetSource,
    previous_project_path: Option<&Path>,
    destination: &Path,
) -> AssetSource {
    let AssetSource::File {
        path,
        relative_to_project,
    } = source;
    if !destination.is_absolute() {
        return source.clone();
    }

    let stored = Path::new(path);
    let absolute = if *relative_to_project {
        let Some(previous) = previous_project_path.filter(|path| path.is_absolute()) else {
            return source.clone();
        };
        if stored.as_os_str().is_empty()
            || stored.is_absolute()
            || stored.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::CurDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return source.clone();
        }
        let Some(parent) = previous.parent() else {
            return source.clone();
        };
        parent.join(stored)
    } else {
        if !stored.is_absolute() {
            return source.clone();
        }
        stored.to_path_buf()
    };

    asset_source_for_saved_project(destination, &absolute).unwrap_or_else(|_| source.clone())
}

/// Transform the detached Save As snapshot, without changing current editor
/// state. The caller commits the corresponding in-memory rebasing only after
/// its publication operation succeeds.
pub fn rebase_project_assets_for_save_as(
    project: &mut crate::project::Project,
    previous_project_path: Option<&Path>,
    destination: &Path,
) {
    for asset in &mut project.assets {
        asset.source = rebase_asset_source_for_save_as(
            &asset.source,
            previous_project_path,
            destination,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AssetPathError, asset_source_for_saved_project, rebase_asset_source_for_save_as,
        relative_asset_source,
    };
    use crate::project::AssetSource;
    use std::path::PathBuf;

    fn saved_project_path() -> PathBuf {
        std::env::temp_dir()
            .join("rhythm-effects-relative-assets")
            .join("project.rhfx")
    }

    #[test]
    fn file_inside_saved_project_directory_becomes_relative_source() {
        let project = saved_project_path();
        let asset = project
            .parent()
            .expect("project directory")
            .join("assets")
            .join("image.png");

        assert_eq!(
            relative_asset_source(&project, &asset),
            Ok(Some(AssetSource::File {
                path: PathBuf::from("assets")
                    .join("image.png")
                    .to_string_lossy()
                    .into_owned(),
                relative_to_project: true,
            }))
        );
    }

    #[test]
    fn sibling_or_parent_file_is_not_classified_as_project_relative() {
        let project = saved_project_path();
        let project_directory = project.parent().expect("project directory");
        let outside = project_directory
            .parent()
            .expect("parent directory")
            .join("outside.png");

        assert_eq!(relative_asset_source(&project, &outside), Ok(None));
    }

    #[test]
    fn external_file_is_persisted_as_absolute_source() {
        let project = saved_project_path();
        let project_directory = project.parent().expect("project directory");
        let outside = project_directory
            .parent()
            .expect("parent directory")
            .join("outside.png");

        assert_eq!(
            asset_source_for_saved_project(&project, &outside),
            Ok(AssetSource::File {
                path: outside.to_string_lossy().into_owned(),
                relative_to_project: false,
            })
        );
    }

    #[test]
    fn combined_policy_keeps_in_project_files_relative() {
        let project = saved_project_path();
        let asset = project
            .parent()
            .expect("project directory")
            .join("assets")
            .join("image.png");

        assert_eq!(
            asset_source_for_saved_project(&project, &asset),
            relative_asset_source(&project, &asset).map(Option::unwrap)
        );
    }

    #[test]
    fn combined_policy_rejects_unnormalized_external_path() {
        let project = saved_project_path();
        let asset = project
            .parent()
            .expect("project directory")
            .join("..")
            .join("outside.png");

        assert_eq!(
            asset_source_for_saved_project(&project, &asset),
            Err(AssetPathError::UnnormalizedAssetPath)
        );
    }

    #[test]
    fn lexical_parent_traversal_is_not_accepted_as_project_relative() {
        let project = saved_project_path();
        let asset = project
            .parent()
            .expect("project directory")
            .join("assets")
            .join("..")
            .join("..")
            .join("outside.png");

        assert_eq!(relative_asset_source(&project, &asset), Ok(None));
    }

    #[test]
    fn save_as_keeps_resource_identity_across_new_project_directory() {
        let old_project = saved_project_path();
        let old_parent = old_project.parent().expect("old project dir");
        let destination = old_parent.parent().expect("root").join("new").join("saved.rhfx");
        let original = AssetSource::File {
            path: "assets/image.png".into(),
            relative_to_project: true,
        };
        assert_eq!(
            rebase_asset_source_for_save_as(&original, Some(&old_project), &destination),
            AssetSource::File {
                path: old_parent
                    .join("assets/image.png")
                    .to_str()
                    .expect("UTF-8 test path")
                    .into(),
                relative_to_project: false,
            }
        );

        let nested_destination = old_parent.join("assets").join("saved.rhfx");
        assert_eq!(
            rebase_asset_source_for_save_as(&original, Some(&old_project), &nested_destination),
            AssetSource::File {
                path: old_parent
                    .join("assets/image.png")
                    .to_str()
                    .expect("UTF-8 test path")
                    .into(),
                relative_to_project: false,
            }
        );
    }

    #[test]
    fn save_as_relativizes_absolute_resource_under_new_directory() {
        let new_project = saved_project_path();
        let absolute_asset = new_project.parent().expect("project directory").join("missing.png");
        let source = AssetSource::File {
            path: absolute_asset.to_str().expect("UTF-8 test path").into(),
            relative_to_project: false,
        };
        assert_eq!(
            rebase_asset_source_for_save_as(&source, None, &new_project),
            AssetSource::File {
                path: "missing.png".into(),
                relative_to_project: true,
            }
        );
    }

    #[test]
    fn save_as_keeps_ineligible_asset_sources_unchanged() {
        let old_project = saved_project_path();
        let new_project = old_project.parent().expect("parent").join("new.rhfx");
        let relative = AssetSource::File {
            path: "../outside.png".into(),
            relative_to_project: true,
        };
        let bare = AssetSource::File {
            path: "relative-without-origin.png".into(),
            relative_to_project: false,
        };
        assert_eq!(
            rebase_asset_source_for_save_as(&relative, Some(&old_project), &new_project),
            relative
        );
        assert_eq!(
            rebase_asset_source_for_save_as(&relative, None, &new_project),
            relative
        );
        assert_eq!(rebase_asset_source_for_save_as(&bare, None, &new_project), bare);
    }

    #[test]
    fn relative_inputs_are_rejected() {
        let project = saved_project_path();

        assert_eq!(
            relative_asset_source(PathBuf::from("project.rhfx").as_path(), &project),
            Err(AssetPathError::ProjectPathNotAbsolute)
        );
        assert_eq!(
            relative_asset_source(&project, PathBuf::from("image.png").as_path()),
            Err(AssetPathError::AssetPathNotAbsolute)
        );
    }
}
