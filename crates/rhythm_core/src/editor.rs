use crate::project::{Project, ProjectValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    InvalidProject(ProjectValidationError),
}

#[derive(Debug)]
pub struct ProjectEditor {
    project: Project,
}

impl ProjectEditor {
    pub fn new(project: Project) -> Result<Self, EditError> {
        project.validate().map_err(EditError::InvalidProject)?;
        Ok(Self { project })
    }

    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub fn into_project(self) -> Project {
        self.project
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectEditor;
    use crate::{
        project::{Project, ProjectSettings},
        time::{GridOffsetNs, TempoMap},
    };

    #[test]
    fn project_editor_owns_a_valid_project_and_exposes_read_only_access() {
        let project = Project::new(
            "Untitled",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );

        let editor = ProjectEditor::new(project).expect("valid project");
        assert_eq!(editor.project().metadata.name, "Untitled");
        assert!(editor.project().composition.objects.is_empty());
    }
}
