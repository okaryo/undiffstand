use crate::error::{AppError, AppResult};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

const MAX_FILE_BYTES: u64 = 2_000_000;

pub fn resolve_safe_path(repo: &Path, relative: &str) -> AppResult<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AppError::new(
            "PATH_OUTSIDE_REPOSITORY",
            "The requested path is outside the selected repository.",
        ));
    }
    let root = repo.canonicalize().map_err(AppError::unknown)?;
    let resolved = root.join(path).canonicalize().map_err(|error| {
        AppError::new("FILE_NOT_FOUND", "The requested file could not be opened.")
            .with_detail(error.to_string())
    })?;
    if !resolved.starts_with(&root) {
        return Err(AppError::new(
            "PATH_OUTSIDE_REPOSITORY",
            "The requested path is outside the selected repository.",
        ));
    }
    Ok(resolved)
}

pub fn read_file(repo: &Path, relative: &str) -> AppResult<String> {
    let resolved = resolve_safe_path(repo, relative)?;
    let metadata = fs::metadata(&resolved).map_err(AppError::unknown)?;
    if !metadata.is_file() {
        return Err(AppError::new(
            "FILE_NOT_FOUND",
            "The requested path is not a file.",
        ));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(AppError::new(
            "FILE_TOO_LARGE",
            "This file is too large to review safely.",
        ));
    }
    let bytes = fs::read(&resolved).map_err(AppError::unknown)?;
    if bytes.contains(&0) {
        return Err(AppError::new(
            "BINARY_FILE",
            "Binary files cannot be reviewed as text.",
        ));
    }
    String::from_utf8(bytes).map_err(|_| {
        AppError::new(
            "BINARY_FILE",
            "This file is not valid UTF-8 text and cannot be reviewed.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_directory_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let error = resolve_safe_path(temp.path(), "../secret.txt").unwrap_err();
        assert_eq!(error.code, "PATH_OUTSIDE_REPOSITORY");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_that_leaves_repository() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        symlink("/etc/hosts", temp.path().join("outside")).unwrap();
        let error = resolve_safe_path(temp.path(), "outside").unwrap_err();
        assert_eq!(error.code, "PATH_OUTSIDE_REPOSITORY");
    }
}
