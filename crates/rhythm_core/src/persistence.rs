use crate::project::Project;

/// Schema version written by the first public .rhfx project-file format.
pub const PROJECT_FILE_SCHEMA_V1: u32 = 1;

/// Versioned root envelope for an MVP .rhfx project document.
///
/// Serialization is added separately so the schema wrapper can remain a small
/// semantic type independent from JSON-specific concerns.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFileV1 {
    pub schema_version: u32,
    pub created_with_version: String,
    pub project: Project,
}

impl ProjectFileV1 {
    pub const SCHEMA_VERSION: u32 = PROJECT_FILE_SCHEMA_V1;

    #[must_use]
    pub fn new(project: Project, created_with_version: impl Into<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            created_with_version: created_with_version.into(),
            project,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PROJECT_FILE_SCHEMA_V1, ProjectFileV1};
    use crate::{
        project::{Project, ProjectSettings},
        time::{GridOffsetNs, TempoMap},
    };

    fn project() -> Project {
        Project::new(
            "Envelope",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        )
    }

    #[test]
    fn v1_wrapper_uses_fixed_schema_version_and_preserves_app_version() {
        let file = ProjectFileV1::new(project(), "0.1.0-test");

        assert_eq!(PROJECT_FILE_SCHEMA_V1, 1);
        assert_eq!(ProjectFileV1::SCHEMA_VERSION, 1);
        assert_eq!(file.schema_version, 1);
        assert_eq!(file.created_with_version, "0.1.0-test");
    }

    #[test]
    fn v1_wrapper_preserves_project_semantics() {
        let project = project();
        let file = ProjectFileV1::new(project.clone(), "0.1.0");

        assert_eq!(file.project, project);
    }
}
