//! Compatibility boundary for the editor recovery subsystem.
//! The canonical Windows application-data layout lives in app_paths.

use crate::app_paths::AppPaths;
use std::{io, path::PathBuf};

/// Resolve/create the shared LocalAppData layout and return only this
/// subsystem's recovery directory. No CWD/relative fallback is permitted.
pub fn ensure_recovery_root() -> io::Result<PathBuf> {
    AppPaths::ensure().map(|paths| paths.recovery().to_path_buf())
}
