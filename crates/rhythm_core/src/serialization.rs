use crate::project::Project;

pub const PROJECT_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::{PROJECT_SCHEMA_VERSION_V1, ProjectFileV1};
    use crate::{
        project::{Project, ProjectSettings},
        time::{GridOffsetNs, TempoMap},
    };

    fn project() -> Project {
        Project::new(
            "Schema wrapper test",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        )
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
}
