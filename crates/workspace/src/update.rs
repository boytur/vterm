//! Update staging helpers extracted from `Workspace`.
//!
//! macOS stages a `.app` directory, Windows stages a single `.exe` file, so
//! cleanup must handle both shapes.

/// Removes a staged update artifact regardless of shape.
pub(crate) fn remove_staged_path(path: &std::path::Path) {
    if path.is_file() {
        let _ = std::fs::remove_file(path);
    } else {
        let _ = std::fs::remove_dir_all(path);
    }
}
