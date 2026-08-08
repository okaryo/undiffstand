use crate::{
    domain::{DiffFileSummary, DiffStatus, DiffSummary, FileDiff, RepositoryInfo},
    error::{AppError, AppResult},
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const MAX_DIFF_BYTES: usize = 1_500_000;
const MAX_CONTENT_BYTES: usize = 2_000_000;

fn git_output<I, S>(repo: &Path, args: I) -> AppResult<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
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
        })
}

fn successful(output: Output, code: &'static str, message: &'static str) -> AppResult<Vec<u8>> {
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(AppError::new(code, message).with_detail(detail))
    }
}

fn output_text(output: Output, code: &'static str, message: &'static str) -> AppResult<String> {
    let bytes = successful(output, code, message)?;
    String::from_utf8(bytes).map_err(AppError::unknown)
}

pub fn canonical_repository(path: &Path) -> AppResult<PathBuf> {
    let selected = path.canonicalize().map_err(|error| {
        AppError::new(
            "NOT_A_GIT_REPOSITORY",
            "The selected folder could not be opened.",
        )
        .with_detail(error.to_string())
    })?;
    let output = git_output(&selected, ["rev-parse", "--show-toplevel"])?;
    let root = output_text(
        output,
        "NOT_A_GIT_REPOSITORY",
        "The selected folder is not inside a Git repository.",
    )?;
    PathBuf::from(root.trim())
        .canonicalize()
        .map_err(AppError::unknown)
}

fn ref_exists(repo: &Path, reference: &str) -> AppResult<bool> {
    Ok(git_output(
        repo,
        [
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
    )?
    .status
    .success())
}

pub fn detect_base_ref(repo: &Path) -> AppResult<(Option<String>, Vec<String>)> {
    let remote_head = git_output(
        repo,
        [
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )?;
    if remote_head.status.success() {
        let reference = String::from_utf8_lossy(&remote_head.stdout)
            .trim()
            .to_owned();
        if !reference.is_empty() && ref_exists(repo, &reference)? {
            return Ok((Some(reference), list_refs(repo)?));
        }
    }

    for candidate in ["origin/main", "origin/master", "main", "master"] {
        if ref_exists(repo, candidate)? {
            return Ok((Some(candidate.to_owned()), list_refs(repo)?));
        }
    }
    Ok((None, list_refs(repo)?))
}

fn list_refs(repo: &Path) -> AppResult<Vec<String>> {
    let output = git_output(
        repo,
        [
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;
    let mut refs: Vec<String> = output_text(output, "UNKNOWN", "Git refs could not be listed.")?
        .lines()
        .filter(|line| !line.ends_with("/HEAD"))
        .map(ToOwned::to_owned)
        .collect();
    refs.sort();
    refs.dedup();
    Ok(refs)
}

pub fn inspect_repository(path: &Path) -> AppResult<RepositoryInfo> {
    let repo = canonical_repository(path)?;
    let (detected_base_ref, available_refs) = detect_base_ref(&repo)?;
    let suggested_name = repo
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_owned();
    Ok(RepositoryInfo {
        repo_path: repo.to_string_lossy().into_owned(),
        suggested_name,
        detected_base_ref,
        available_refs,
    })
}

pub fn validate_base_ref(repo: &Path, base_ref: &str) -> AppResult<()> {
    if base_ref.trim().is_empty() || !ref_exists(repo, base_ref)? {
        return Err(AppError::new(
            "INVALID_BASE_REF",
            "The comparison ref does not exist in this repository.",
        ));
    }
    Ok(())
}

fn revision_info(repo: &Path, base_ref: &str) -> AppResult<(String, String)> {
    validate_base_ref(repo, base_ref)?;
    let head = output_text(
        git_output(repo, ["rev-parse", "HEAD"])?,
        "UNKNOWN",
        "HEAD could not be resolved.",
    )?;
    let merge_base_output = git_output(repo, ["merge-base", base_ref, "HEAD"])?;
    if !merge_base_output.status.success() {
        return Err(AppError::new(
            "NO_MERGE_BASE",
            "The comparison ref and HEAD do not have a common ancestor.",
        ));
    }
    let merge_base = String::from_utf8_lossy(&merge_base_output.stdout)
        .trim()
        .to_owned();
    Ok((head.trim().to_owned(), merge_base))
}

fn parse_name_status(bytes: &[u8]) -> Vec<DiffFileSummary> {
    let fields: Vec<&[u8]> = bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect();
    let mut files = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let status_text = String::from_utf8_lossy(fields[index]);
        index += 1;
        let status_code = status_text.chars().next().unwrap_or('M');
        let path = fields
            .get(index)
            .map(|field| String::from_utf8_lossy(field).into_owned());
        index += 1;
        let (old_path, new_path, status) = match status_code {
            'A' => (None, path, DiffStatus::Added),
            'D' => (path, None, DiffStatus::Deleted),
            'R' | 'C' => {
                let new_path = fields
                    .get(index)
                    .map(|field| String::from_utf8_lossy(field).into_owned());
                index += 1;
                (
                    path,
                    new_path,
                    if status_code == 'R' {
                        DiffStatus::Renamed
                    } else {
                        DiffStatus::Copied
                    },
                )
            }
            _ => (path.clone(), path, DiffStatus::Modified),
        };
        files.push(DiffFileSummary {
            old_path,
            new_path,
            status,
            additions: None,
            deletions: None,
        });
    }
    files
}

fn parse_numstat(bytes: &[u8]) -> Vec<(Option<u64>, Option<u64>)> {
    let fields: Vec<&[u8]> = bytes.split(|byte| *byte == 0).collect();
    let mut stats = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        index += 1;
        if field.is_empty() {
            continue;
        }
        let mut parts = field.splitn(3, |byte| *byte == b'\t');
        let additions = parts.next().unwrap_or_default();
        let deletions = parts.next().unwrap_or_default();
        let path = parts.next().unwrap_or_default();
        if path.is_empty() {
            index = (index + 2).min(fields.len());
        }
        let parse_count = |value: &[u8]| String::from_utf8_lossy(value).parse::<u64>().ok();
        stats.push((parse_count(additions), parse_count(deletions)));
    }
    stats
}

pub fn diff_summary(repo: &Path, base_ref: &str) -> AppResult<DiffSummary> {
    let (head_sha, merge_base_sha) = revision_info(repo, base_ref)?;
    let range = format!("{base_ref}...HEAD");
    let name_bytes = successful(
        git_output(
            repo,
            [
                "diff",
                "--no-color",
                "--name-status",
                "-z",
                "--find-renames",
                "--find-copies",
                &range,
                "--",
            ],
        )?,
        "UNKNOWN",
        "The changed file list could not be read.",
    )?;
    let stats_bytes = successful(
        git_output(
            repo,
            [
                "diff",
                "--no-color",
                "--numstat",
                "-z",
                "--find-renames",
                "--find-copies",
                &range,
                "--",
            ],
        )?,
        "UNKNOWN",
        "Diff statistics could not be read.",
    )?;
    let mut files = parse_name_status(&name_bytes);
    let stats = parse_numstat(&stats_bytes);
    for (file, (additions, deletions)) in files.iter_mut().zip(stats) {
        file.additions = additions;
        file.deletions = deletions;
        if additions.is_none() && deletions.is_none() {
            file.status = DiffStatus::Binary;
        }
    }
    let total_additions = files.iter().filter_map(|file| file.additions).sum();
    let total_deletions = files.iter().filter_map(|file| file.deletions).sum();
    Ok(DiffSummary {
        base_ref: base_ref.to_owned(),
        head_sha,
        merge_base_sha,
        files,
        total_additions,
        total_deletions,
    })
}

fn git_show(repo: &Path, revision: &str, path: &str) -> AppResult<Option<String>> {
    let object = format!("{revision}:{path}");
    let output = git_output(repo, ["show", &object])?;
    if !output.status.success() {
        return Ok(None);
    }
    if output.stdout.len() > MAX_CONTENT_BYTES || output.stdout.contains(&0) {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8(output.stdout).map_err(AppError::unknown)?,
    ))
}

fn split_hunks(diff: &str) -> Vec<String> {
    let mut hunks = Vec::new();
    let mut current = String::new();
    for line in diff.lines() {
        if line.starts_with("@@ ") {
            if !current.is_empty() {
                hunks.push(std::mem::take(&mut current));
            }
            current.push_str(line);
            current.push('\n');
        } else if !current.is_empty() {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.is_empty() {
        hunks.push(current);
    }
    hunks
}

pub fn file_diff(repo: &Path, base_ref: &str, path: &str) -> AppResult<FileDiff> {
    let summary = diff_summary(repo, base_ref)?;
    let file = summary
        .files
        .into_iter()
        .find(|file| {
            file.old_path.as_deref() == Some(path) || file.new_path.as_deref() == Some(path)
        })
        .ok_or_else(|| {
            AppError::new(
                "FILE_NOT_IN_DIFF",
                "The selected file is not part of this diff.",
            )
        })?;
    let range = format!("{base_ref}...HEAD");
    let output = git_output(
        repo,
        [
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--find-renames",
            "--find-copies",
            &range,
            "--",
            path,
        ],
    )?;
    let bytes = successful(output, "UNKNOWN", "The selected diff could not be read.")?;
    let truncated = bytes.len() > MAX_DIFF_BYTES;
    let visible = if truncated {
        &bytes[..MAX_DIFF_BYTES]
    } else {
        &bytes
    };
    let unified_diff = String::from_utf8_lossy(visible).into_owned();
    let old_content = match (&file.old_path, &file.status) {
        (Some(_), DiffStatus::Binary) => None,
        (Some(old_path), _) => git_show(repo, &summary.merge_base_sha, old_path)?,
        _ => None,
    };
    let new_content = match (&file.new_path, &file.status) {
        (Some(_), DiffStatus::Binary) => None,
        (Some(new_path), _) => git_show(repo, "HEAD", new_path)?,
        _ => None,
    };
    let hunks = split_hunks(&unified_diff);
    Ok(FileDiff {
        file,
        old_content,
        new_content,
        hunks,
        unified_diff,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process::Command};

    fn git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: {args:?}");
    }

    #[test]
    fn three_dot_diff_excludes_changes_made_only_on_base() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "ReaDiff Test"]);
        fs::write(repo.join("shared.txt"), "base\n").unwrap();
        git(repo, &["add", "shared.txt"]);
        git(repo, &["commit", "-m", "base"]);
        git(repo, &["switch", "-c", "feature"]);
        fs::write(repo.join("feature.txt"), "feature\n").unwrap();
        git(repo, &["add", "feature.txt"]);
        git(repo, &["commit", "-m", "feature"]);
        git(repo, &["switch", "main"]);
        fs::write(repo.join("base-only.txt"), "base only\n").unwrap();
        git(repo, &["add", "base-only.txt"]);
        git(repo, &["commit", "-m", "base only"]);
        git(repo, &["switch", "feature"]);

        let summary = diff_summary(repo, "main").unwrap();
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].new_path.as_deref(), Some("feature.txt"));
        assert_eq!(summary.total_additions, 1);
    }

    #[test]
    fn parses_rename_name_status() {
        let files = parse_name_status(b"R100\0old.rs\0new.rs\0");
        assert_eq!(files[0].status, DiffStatus::Renamed);
        assert_eq!(files[0].old_path.as_deref(), Some("old.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("new.rs"));
    }
}
