use crate::{
    domain::{DiffFileSummary, DiffStatus, DiffSummary, FileDiff, RepositoryInfo},
    error::{AppError, AppResult},
    services::file_service,
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

fn current_branch(repo: &Path) -> AppResult<Option<String>> {
    let output = git_output(repo, ["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    if !output.status.success() {
        return Ok(None);
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((!branch.is_empty()).then_some(branch))
}

pub fn detect_base_ref(repo: &Path) -> AppResult<(Option<String>, Vec<String>)> {
    let local_branches = list_local_branches(repo)?;
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
        let local_name = reference.strip_prefix("origin/").unwrap_or(&reference);
        if local_branches.iter().any(|branch| branch == local_name) {
            return Ok((Some(local_name.to_owned()), local_branches));
        }
    }

    for candidate in ["main", "master"] {
        if local_branches.iter().any(|branch| branch == candidate) {
            return Ok((Some(candidate.to_owned()), local_branches));
        }
    }
    let detected = current_branch(repo)?
        .filter(|branch| local_branches.contains(branch))
        .or_else(|| local_branches.first().cloned());
    Ok((detected, local_branches))
}

fn list_local_branches(repo: &Path) -> AppResult<Vec<String>> {
    let output = git_output(
        repo,
        ["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )?;
    let mut refs: Vec<String> = output_text(output, "UNKNOWN", "Git refs could not be listed.")?
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    refs.sort();
    refs.dedup();
    Ok(refs)
}

pub fn inspect_repository(path: &Path) -> AppResult<RepositoryInfo> {
    let repo = canonical_repository(path)?;
    let (detected_base_ref, local_branches) = detect_base_ref(&repo)?;
    let suggested_name = repo
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_owned();
    Ok(RepositoryInfo {
        repo_path: repo.to_string_lossy().into_owned(),
        suggested_name,
        detected_base_ref,
        current_branch: current_branch(&repo)?,
        local_branches,
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
                &merge_base_sha,
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
                &merge_base_sha,
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
    files.extend(untracked_files(repo)?);
    files.sort_by(|left, right| {
        let left_path = left
            .new_path
            .as_deref()
            .or(left.old_path.as_deref())
            .unwrap_or("");
        let right_path = right
            .new_path
            .as_deref()
            .or(right.old_path.as_deref())
            .unwrap_or("");
        left_path.cmp(right_path)
    });
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

fn untracked_files(repo: &Path) -> AppResult<Vec<DiffFileSummary>> {
    let bytes = successful(
        git_output(repo, ["ls-files", "--others", "--exclude-standard", "-z"])?,
        "UNKNOWN",
        "Untracked files could not be listed.",
    )?;
    Ok(bytes
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            let path = String::from_utf8_lossy(path).into_owned();
            let additions = file_service::read_file(repo, &path)
                .ok()
                .map(|content| content.lines().count() as u64);
            DiffFileSummary {
                old_path: None,
                new_path: Some(path),
                status: if additions.is_some() {
                    DiffStatus::Added
                } else {
                    DiffStatus::Binary
                },
                additions,
                deletions: additions.map(|_| 0),
            }
        })
        .collect())
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
    file_diff_from_summary(repo, &summary, path)
}

pub fn file_diffs(repo: &Path, base_ref: &str, paths: &[String]) -> AppResult<Vec<FileDiff>> {
    let summary = diff_summary(repo, base_ref)?;
    paths
        .iter()
        .map(|path| file_diff_from_summary(repo, &summary, path))
        .collect()
}

fn file_diff_from_summary(repo: &Path, summary: &DiffSummary, path: &str) -> AppResult<FileDiff> {
    let file = summary
        .files
        .iter()
        .find(|file| {
            file.old_path.as_deref() == Some(path) || file.new_path.as_deref() == Some(path)
        })
        .cloned()
        .ok_or_else(|| {
            AppError::new(
                "FILE_NOT_IN_DIFF",
                "The selected file is not part of this diff.",
            )
        })?;
    let tracked = git_output(repo, ["ls-files", "--error-unmatch", "--", path])?
        .status
        .success();
    let bytes = if tracked {
        successful(
            git_output(
                repo,
                [
                    "diff",
                    "--no-color",
                    "--no-ext-diff",
                    "--find-renames",
                    "--find-copies",
                    &summary.merge_base_sha,
                    "--",
                    path,
                ],
            )?,
            "UNKNOWN",
            "The selected diff could not be read.",
        )?
    } else {
        let output = git_output(
            repo,
            [
                "diff",
                "--no-index",
                "--no-color",
                "--no-ext-diff",
                "--",
                "/dev/null",
                path,
            ],
        )?;
        if output.status.success() || output.status.code() == Some(1) {
            output.stdout
        } else {
            return Err(
                AppError::new("UNKNOWN", "The untracked file diff could not be read.")
                    .with_detail(String::from_utf8_lossy(&output.stderr).trim()),
            );
        }
    };
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
        (Some(new_path), _) => file_service::read_file(repo, new_path).ok(),
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
    fn working_tree_diff_excludes_changes_made_only_on_base() {
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
    fn working_tree_diff_includes_staged_unstaged_and_untracked_files() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "ReaDiff Test"]);
        fs::write(repo.join("base.txt"), "base\n").unwrap();
        git(repo, &["add", "base.txt"]);
        git(repo, &["commit", "-m", "base"]);
        git(repo, &["switch", "-c", "feature"]);
        fs::write(repo.join("committed.txt"), "committed\n").unwrap();
        git(repo, &["add", "committed.txt"]);
        git(repo, &["commit", "-m", "feature"]);

        fs::write(repo.join("committed.txt"), "committed\nunstaged\n").unwrap();
        fs::write(repo.join("staged.txt"), "staged\n").unwrap();
        git(repo, &["add", "staged.txt"]);
        fs::write(repo.join("untracked.txt"), "untracked\n").unwrap();

        let summary = diff_summary(repo, "main").unwrap();
        let paths: Vec<&str> = summary
            .files
            .iter()
            .filter_map(|file| file.new_path.as_deref())
            .collect();
        assert_eq!(paths, ["committed.txt", "staged.txt", "untracked.txt"]);
        assert_eq!(summary.total_additions, 4);

        let diff = file_diff(repo, "main", "committed.txt").unwrap();
        assert_eq!(diff.new_content.as_deref(), Some("committed\nunstaged\n"));
        assert!(diff.unified_diff.contains("+unstaged"));

        let untracked = file_diff(repo, "main", "untracked.txt").unwrap();
        assert_eq!(untracked.new_content.as_deref(), Some("untracked\n"));
        assert!(untracked.unified_diff.contains("+untracked"));

        let diffs = file_diffs(
            repo,
            "main",
            &["committed.txt".to_owned(), "untracked.txt".to_owned()],
        )
        .unwrap();
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].file.new_path.as_deref(), Some("committed.txt"));
        assert_eq!(diffs[1].file.new_path.as_deref(), Some("untracked.txt"));
    }

    #[test]
    fn parses_rename_name_status() {
        let files = parse_name_status(b"R100\0old.rs\0new.rs\0");
        assert_eq!(files[0].status, DiffStatus::Renamed);
        assert_eq!(files[0].old_path.as_deref(), Some("old.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("new.rs"));
    }
}
