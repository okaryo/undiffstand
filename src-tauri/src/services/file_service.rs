use crate::{
    domain::{FileContent, RepoFile},
    error::{AppError, AppResult},
};
use std::{
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

const MAX_FILE_BYTES: u64 = 2_000_000;

pub fn list_files(repo: &Path) -> AppResult<Vec<RepoFile>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::new(
                    "GIT_NOT_FOUND",
                    "Git is not installed or is not available on PATH.",
                )
            } else {
                AppError::unknown(error)
            }
        })?;
    if !output.status.success() {
        return Err(AppError::new(
            "UNKNOWN",
            "Repository files could not be listed.",
        ));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| RepoFile {
            path: String::from_utf8_lossy(path).into_owned(),
        })
        .collect())
}

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

pub fn read_file(repo: &Path, relative: &str) -> AppResult<FileContent> {
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
            "This file is too large to display safely.",
        ));
    }
    let bytes = fs::read(&resolved).map_err(AppError::unknown)?;
    if bytes.contains(&0) {
        return Err(AppError::new(
            "BINARY_FILE",
            "Binary files cannot be displayed as code.",
        ));
    }
    let content = String::from_utf8(bytes).map_err(|_| {
        AppError::new(
            "BINARY_FILE",
            "This file is not valid UTF-8 text and cannot be displayed.",
        )
    })?;
    let line_count = content.lines().count().max(1);
    Ok(FileContent {
        path: relative.to_owned(),
        content,
        language: language_for_path(Path::new(relative)),
        line_count,
    })
}

fn language_for_path(path: &Path) -> String {
    match path.extension().and_then(OsStr::to_str).unwrap_or_default() {
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "json" => "json",
        "html" | "svelte" | "vue" => "html",
        "css" | "scss" | "sass" | "less" => "css",
        "md" | "mdx" => "markdown",
        "py" => "python",
        "rs" => "rust",
        "sh" | "bash" | "zsh" => "shell",
        "toml" => "toml",
        "yml" | "yaml" => "yaml",
        "go" => "go",
        _ => "text",
    }
    .to_owned()
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
