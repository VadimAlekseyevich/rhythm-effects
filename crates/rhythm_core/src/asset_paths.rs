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

#[cfg(test)]
mod tests {
    use super::{AssetPathError, asset_source_for_saved_project, relative_asset_source};
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
