use std::path::Path;

/// Stable per-project memory session key shared by retrieval (Start) and
/// consolidation (End): the basename of the working directory. It MUST be
/// identical on both sides so recalled memory and written episodes address the
/// same memory-service session. Derived from cwd because that is available at
/// BOTH Start and End (unlike files-accessed metrics, empty at Start).
pub(crate) fn project_session_key(cwd: &Path) -> String {
    cwd.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("default")
        .to_string()
}
