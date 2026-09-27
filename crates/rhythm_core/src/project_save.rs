//! Staging and flush/close phases of explicit Save, without publication or revision changes.
use std::{
    error::Error,
    ffi::OsString,
    fmt, fs,
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    project::Project,
    serialization::{ProjectFileEncodeError, serialize_project_file_v1},
};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
const MAX_TEMP_NAME_ATTEMPTS: usize = 64;

#[derive(Debug)]
pub enum ProjectSaveStageError {
    Encode(ProjectFileEncodeError),
    InvalidDestination,
    Io(io::Error),
    TemporaryNameCollision,
}

impl fmt::Display for ProjectSaveStageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(error) => fmt::Display::fmt(error, formatter),
            Self::InvalidDestination => write!(formatter, "project destination has no filename"),
            Self::Io(error) => write!(formatter, "cannot stage project save: {error}"),
            Self::TemporaryNameCollision => {
                write!(formatter, "cannot allocate a unique sibling temp file")
            }
        }
    }
}

impl Error for ProjectSaveStageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::InvalidDestination | Self::TemporaryNameCollision => None,
        }
    }
}

/// Owns an unpublished sibling temp file. Dropping the stage closes and removes
/// it; only a successfully flushed/closed stage can proceed to publication.
#[derive(Debug)]
pub struct StagedProjectSave {
    destination: PathBuf,
    temp_path: PathBuf,
    file: Option<File>,
}

impl StagedProjectSave {
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    #[must_use]
    pub fn temp_path(&self) -> &Path {
        &self.temp_path
    }

    /// Flush buffered bytes, synchronize the file contents/metadata, then close
    /// its handle. Only the closed stage may be handed to a future publisher.
    /// A failed flush/sync drops the stage, removing the temp and preserving
    /// the original destination. This does not publish or mark a revision saved.
    pub fn flush_and_close(self) -> Result<ClosedProjectSave, ProjectSaveStageError> {
        self.finish_with(|file| {
            file.flush()?;
            file.sync_all()
        })
    }

    fn finish_with(
        mut self,
        finalize: impl FnOnce(&mut File) -> io::Result<()>,
    ) -> Result<ClosedProjectSave, ProjectSaveStageError> {
        finalize(self.file.as_mut().expect("staged file is open"))
            .map_err(ProjectSaveStageError::Io)?;
        // Explicitly drop the OS handle before a possible Windows rename/replace.
        drop(self.file.take());
        let closed = ClosedProjectSave {
            destination: std::mem::take(&mut self.destination),
            temp_path: std::mem::take(&mut self.temp_path),
        };
        Ok(closed)
    }
}

impl Drop for StagedProjectSave {
    fn drop(&mut self) {
        // Close before deleting: Windows does not remove an open staged file.
        drop(self.file.take());
        if !self.temp_path.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

/// Fully written and synchronized sibling file, with no open write handle.
/// Still unpublished: dropping it cleans up the temp and leaves the canonical
/// document unchanged. AI-238 will consume this type during safe publication.
#[derive(Debug)]
pub struct ClosedProjectSave {
    destination: PathBuf,
    temp_path: PathBuf,
}

impl ClosedProjectSave {
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    #[must_use]
    pub fn temp_path(&self) -> &Path {
        &self.temp_path
    }
}

impl Drop for ClosedProjectSave {
    fn drop(&mut self) {
        if !self.temp_path.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

/// Validate/encode the snapshot first, then create_new and write a sibling
/// temp file. Never truncate or write the canonical destination. Publication,
/// publication and saved_revision changes belong to later Save stages.
pub fn stage_project_file_save(
    project: &Project,
    destination: &Path,
) -> Result<StagedProjectSave, ProjectSaveStageError> {
    let bytes = serialize_project_file_v1(project).map_err(ProjectSaveStageError::Encode)?;
    let filename = destination
        .file_name()
        .ok_or(ProjectSaveStageError::InvalidDestination)?;
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    for _ in 0..MAX_TEMP_NAME_ATTEMPTS {
        let mut temp_name = OsString::from(".");
        temp_name.push(filename);
        temp_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let temp_path = parent.join(temp_name);

        let file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(ProjectSaveStageError::Io(error)),
        };
        let mut staged = StagedProjectSave {
            destination: destination.to_path_buf(),
            temp_path,
            file: Some(file),
        };
        staged
            .file
            .as_mut()
            .expect("newly created stage owns its file")
            .write_all(&bytes)
            .map_err(ProjectSaveStageError::Io)?;
        return Ok(staged);
    }

    Err(ProjectSaveStageError::TemporaryNameCollision)
}

#[cfg(test)]
mod tests {
    use super::{ProjectSaveStageError, stage_project_file_save};
    use crate::{
        project::{Project, ProjectSettings, ProjectValidationError},
        serialization::{ProjectFileEncodeError, parse_project_file_v1},
        time::{GridOffsetNs, TempoMap},
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rhythm-save-stage-{}-{}",
                std::process::id(),
                NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create isolated test directory");
            Self(path)
        }

        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn project() -> Project {
        Project::new(
            "Staged édit",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        )
    }

    #[test]
    fn staging_writes_parseable_sibling_without_changing_canonical_file() {
        let root = TestDir::new();
        let destination = root.join("creative.rhfx");
        fs::write(&destination, b"existing known-good document").expect("existing canonical");
        let expected = project();

        let temp_path = {
            let staged = stage_project_file_save(&expected, &destination).expect("stage project");
            assert_eq!(staged.destination(), destination.as_path());
            assert_eq!(staged.temp_path().parent(), destination.parent());
            assert_ne!(staged.temp_path(), destination);
            assert!(
                staged
                    .temp_path()
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().ends_with(".tmp"))
            );
            assert_eq!(
                fs::read(&destination).expect("read existing document"),
                b"existing known-good document"
            );
            let bytes = fs::read(staged.temp_path()).expect("read staged JSON");
            let candidate = parse_project_file_v1(&bytes).expect("valid V1 JSON");
            assert_eq!(candidate.project, expected);
            staged.temp_path().to_path_buf()
        };

        assert!(
            !temp_path.exists(),
            "dropping an unpublished stage cleans it"
        );
        assert_eq!(
            fs::read(&destination).expect("read unchanged document"),
            b"existing known-good document"
        );
    }

    #[test]
    fn stages_have_unique_names_and_clean_up_independently() {
        let root = TestDir::new();
        let destination = root.join("new.rhfx");
        let first = stage_project_file_save(&project(), &destination).expect("first");
        let second = stage_project_file_save(&project(), &destination).expect("second");
        assert_ne!(first.temp_path(), second.temp_path());
        let first_path = first.temp_path().to_path_buf();
        let second_path = second.temp_path().to_path_buf();
        assert!(!destination.exists(), "staging must never publish");
        drop(first);
        assert!(!first_path.exists());
        assert!(second_path.exists());
        drop(second);
        assert!(!second_path.exists());
        assert!(!destination.exists());
    }

    #[test]
    fn invalid_snapshot_cannot_touch_destination_or_create_temp() {
        let root = TestDir::new();
        let destination = root.join("existing.rhfx");
        fs::write(&destination, b"original").expect("existing");
        let mut invalid = project();
        invalid.settings.composition_width = 0;

        assert!(matches!(
            stage_project_file_save(&invalid, &destination),
            Err(ProjectSaveStageError::Encode(
                ProjectFileEncodeError::Validation(
                    ProjectValidationError::InvalidCompositionDimensions
                )
            ))
        ));
        assert_eq!(fs::read(&destination).expect("existing"), b"original");
        assert_eq!(fs::read_dir(&root.0).expect("list").count(), 1);
    }

    #[test]
    fn failed_temp_create_preserves_existing_target_and_invalid_path_is_rejected() {
        let root = TestDir::new();
        let blocking_file = root.join("not-a-directory");
        fs::write(&blocking_file, b"do not overwrite").expect("blocking file");
        let impossible = blocking_file.join("target.rhfx");
        assert!(matches!(
            stage_project_file_save(&project(), &impossible),
            Err(ProjectSaveStageError::Io(_))
        ));
        assert_eq!(
            fs::read(&blocking_file).expect("existing"),
            b"do not overwrite"
        );
        assert!(matches!(
            stage_project_file_save(&project(), Path::new("")),
            Err(ProjectSaveStageError::InvalidDestination)
        ));
    }
}
