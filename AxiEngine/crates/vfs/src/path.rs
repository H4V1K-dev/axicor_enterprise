use crate::error::VfsError;

/// Validates that a path meets all archive namespace path rules.
///
/// # Errors
///
/// Returns [`VfsError::InvalidPath`] or [`VfsError::PathTooLong`] if the path violates the rules.
pub fn validate_archive_path(path: &str) -> Result<(), VfsError> {
    if path.is_empty() {
        return Err(VfsError::InvalidPath);
    }
    if path.len() > 255 {
        return Err(VfsError::PathTooLong);
    }
    if path.contains('\\') {
        return Err(VfsError::InvalidPath);
    }
    if path.contains(':') {
        return Err(VfsError::InvalidPath);
    }
    if path.starts_with('/') {
        return Err(VfsError::InvalidPath);
    }
    if path.contains('\0') {
        return Err(VfsError::InvalidPath);
    }

    for segment in path.split('/') {
        if segment.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        if segment == "." || segment == ".." {
            return Err(VfsError::InvalidPath);
        }
    }

    Ok(())
}
