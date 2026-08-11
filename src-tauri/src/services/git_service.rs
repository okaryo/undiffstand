use crate::{
    domain::{
        ChangeReviewAvailability, ChangeReviewTarget, DiffComparison, DiffFileSummary,
        DiffSelection, DiffStatus, DiffSummary, FileDiff, GitCommitSummary, RepositoryInfo,
    },
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
        if ref_exists(repo, &reference)? {
            return Ok((Some(reference), local_branches));
        }
    }

    for candidate in ["main", "master"] {
        if local_branches.iter().any(|branch| branch == candidate) {
            return Ok((Some(candidate.to_owned()), local_branches));
        }
    }
    Ok((None, local_branches))
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

fn list_refs(repo: &Path, namespace: &str) -> AppResult<Vec<String>> {
    let output = git_output(
        repo,
        ["for-each-ref", "--format=%(refname:short)", namespace],
    )?;
    let mut refs: Vec<String> = output_text(output, "UNKNOWN", "Git refs could not be listed.")?
        .lines()
        .filter(|reference| !reference.ends_with("/HEAD"))
        .map(ToOwned::to_owned)
        .collect();
    refs.sort();
    refs.dedup();
    Ok(refs)
}

fn recent_branches(repo: &Path, current_branch: Option<&str>) -> AppResult<Vec<String>> {
    let output = git_output(
        repo,
        [
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short)",
            "refs/heads",
        ],
    )?;
    Ok(output_text(
        output,
        "UNKNOWN",
        "Recent Git branches could not be listed.",
    )?
    .lines()
    .filter(|reference| !reference.ends_with("/HEAD"))
    .filter(|reference| current_branch != Some(*reference))
    .take(5)
    .map(ToOwned::to_owned)
    .collect())
}

fn recent_commits(repo: &Path) -> AppResult<Vec<GitCommitSummary>> {
    let output = git_output(
        repo,
        [
            "log",
            "--all",
            "--date-order",
            "-n",
            "10",
            "--format=%H%x09%h%x09%s",
        ],
    )?;
    Ok(
        output_text(output, "UNKNOWN", "Recent commits could not be listed.")?
            .lines()
            .filter_map(|line| {
                let mut fields = line.splitn(3, '\t');
                Some(GitCommitSummary {
                    sha: fields.next()?.to_owned(),
                    short_sha: fields.next()?.to_owned(),
                    subject: fields.next()?.to_owned(),
                })
            })
            .collect(),
    )
}

pub fn inspect_repository(path: &Path) -> AppResult<RepositoryInfo> {
    let repo = canonical_repository(path)?;
    let (detected_base_ref, local_branches) = detect_base_ref(&repo)?;
    let current_branch = current_branch(&repo)?;
    let recent_branches = recent_branches(&repo, current_branch.as_deref())?;
    let suggested_name = repo
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_owned();
    Ok(RepositoryInfo {
        repo_path: repo.to_string_lossy().into_owned(),
        suggested_name,
        detected_base_ref,
        current_branch,
        recent_branches,
        local_branches,
        remote_branches: list_refs(&repo, "refs/remotes")?,
        recent_commits: recent_commits(&repo)?,
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

#[derive(Debug, Clone)]
enum DiffRange {
    Revisions { from: String, to: String },
    RevisionToWorkingTree { from: String },
}

#[derive(Debug, Clone)]
struct ResolvedDiff {
    selection: DiffSelection,
    comparison: DiffComparison,
    range: DiffRange,
}

fn normalized_ref(reference: &str) -> &str {
    if reference == "@" {
        "HEAD"
    } else {
        reference
    }
}

fn resolve_commit(repo: &Path, reference: &str) -> AppResult<String> {
    let reference = normalized_ref(reference.trim());
    if reference.is_empty() || reference.starts_with('-') {
        return Err(AppError::new(
            "INVALID_DIFF_TARGET",
            "The comparison ref is empty or invalid.",
        ));
    }
    let revision = format!("{reference}^{{commit}}");
    let output = git_output(
        repo,
        ["rev-parse", "--verify", "--end-of-options", &revision],
    )?;
    output_text(
        output,
        "INVALID_DIFF_TARGET",
        "The requested commit or branch could not be resolved.",
    )
    .map(|sha| sha.trim().to_owned())
}

fn resolve_diff(repo: &Path, selection: &DiffSelection) -> AppResult<ResolvedDiff> {
    let base = selection.base.trim();
    let target = selection.target.trim();

    if base.is_empty() || target.is_empty() {
        return Err(AppError::new(
            "INVALID_DIFF_TARGET",
            "Choose both a base and a target to review.",
        ));
    }

    let from = resolve_commit(repo, base)?;
    if target == "." {
        Ok(ResolvedDiff {
            selection: selection.clone(),
            comparison: DiffComparison {
                from_label: normalized_ref(base).to_owned(),
                to_label: "working tree".to_owned(),
                from_sha: Some(from.clone()),
                to_sha: None,
            },
            range: DiffRange::RevisionToWorkingTree { from },
        })
    } else {
        let to = resolve_commit(repo, target)?;
        Ok(ResolvedDiff {
            selection: selection.clone(),
            comparison: DiffComparison {
                from_label: normalized_ref(base).to_owned(),
                to_label: normalized_ref(target).to_owned(),
                from_sha: Some(from.clone()),
                to_sha: Some(to.clone()),
            },
            range: DiffRange::Revisions { from, to },
        })
    }
}

pub fn change_review_availability(
    repo: &Path,
    selection: &DiffSelection,
) -> AppResult<ChangeReviewAvailability> {
    let base = normalized_ref(selection.base.trim());
    let target = selection.target.trim();
    let current_branch = current_branch(repo)?;
    let display_ref = |reference: &str| {
        if reference == "HEAD" {
            current_branch.as_deref().unwrap_or("HEAD").to_owned()
        } else {
            reference.to_owned()
        }
    };
    let target_label = if target == "." {
        "working tree".to_owned()
    } else {
        display_ref(normalized_ref(target))
    };
    let scope_label = format!("{} → {}", display_ref(base), target_label);

    if diff_summary(repo, selection)?.files.is_empty() {
        return Ok(unavailable_review(
            scope_label,
            "Change Review is unavailable because this comparison has no changes.",
        ));
    }

    if base == "HEAD" && target == "." {
        return Ok(ChangeReviewAvailability {
            available: true,
            target: Some(ChangeReviewTarget::Uncommitted),
            reason: None,
            scope_label,
        });
    }

    if target == "." {
        return Ok(unavailable_review(
            scope_label,
            "Change Review supports the working tree only when the comparison starts at HEAD.",
        ));
    }

    let Some(branch) = current_branch else {
        return Ok(unavailable_review(
            scope_label,
            "Change Review is unavailable while HEAD is detached.",
        ));
    };
    let target = normalized_ref(target);
    if target != "HEAD" && target != branch {
        return Ok(unavailable_review(
            scope_label,
            "Change Review requires the comparison target to be the current branch.",
        ));
    }

    let mut branches = list_local_branches(repo)?;
    branches.extend(list_refs(repo, "refs/remotes")?);
    if !branches.iter().any(|candidate| candidate == base) {
        return Ok(unavailable_review(
            scope_label,
            "Change Review requires the comparison base to be a branch.",
        ));
    }

    Ok(ChangeReviewAvailability {
        available: true,
        target: Some(ChangeReviewTarget::Base {
            base_branch: base.to_owned(),
        }),
        reason: None,
        scope_label,
    })
}

fn unavailable_review(scope_label: String, reason: &str) -> ChangeReviewAvailability {
    ChangeReviewAvailability {
        available: false,
        target: None,
        reason: Some(reason.to_owned()),
        scope_label,
    }
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

fn range_output(
    repo: &Path,
    resolved: &ResolvedDiff,
    flags: &[&str],
    path: Option<&str>,
) -> AppResult<Output> {
    let mut args: Vec<String> = Vec::new();
    match &resolved.range {
        DiffRange::Revisions { from, to } => {
            args.push("diff".to_owned());
            args.extend(flags.iter().map(|value| (*value).to_owned()));
            args.push(from.clone());
            args.push(to.clone());
        }
        DiffRange::RevisionToWorkingTree { from } => {
            args.push("diff".to_owned());
            args.extend(flags.iter().map(|value| (*value).to_owned()));
            args.push(from.clone());
        }
    }
    args.push("--".to_owned());
    if let Some(path) = path {
        args.push(path.to_owned());
    }
    git_output(repo, args)
}

pub fn diff_summary(repo: &Path, selection: &DiffSelection) -> AppResult<DiffSummary> {
    let resolved = resolve_diff(repo, selection)?;
    let name_bytes = successful(
        range_output(
            repo,
            &resolved,
            &[
                "--no-color",
                "--name-status",
                "-z",
                "--find-renames",
                "--find-copies",
            ],
            None,
        )?,
        "UNKNOWN",
        "The changed file list could not be read.",
    )?;
    let stats_bytes = successful(
        range_output(
            repo,
            &resolved,
            &[
                "--no-color",
                "--numstat",
                "-z",
                "--find-renames",
                "--find-copies",
            ],
            None,
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
    if matches!(resolved.range, DiffRange::RevisionToWorkingTree { .. }) {
        files.extend(untracked_files(repo)?);
    }
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
        selection: resolved.selection,
        comparison: resolved.comparison,
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

enum ContentSource<'a> {
    Commit(&'a str),
    WorkingTree,
}

fn content_from_source(
    repo: &Path,
    source: ContentSource<'_>,
    path: &str,
) -> AppResult<Option<String>> {
    match source {
        ContentSource::Commit(revision) => git_show(repo, revision, path),
        ContentSource::WorkingTree => Ok(file_service::read_file(repo, path).ok()),
    }
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

pub fn file_diff(repo: &Path, selection: &DiffSelection, path: &str) -> AppResult<FileDiff> {
    let summary = diff_summary(repo, selection)?;
    let resolved = resolve_diff(repo, selection)?;
    file_diff_from_summary(repo, &summary, &resolved, path)
}

pub fn file_diffs(
    repo: &Path,
    selection: &DiffSelection,
    paths: &[String],
) -> AppResult<Vec<FileDiff>> {
    let summary = diff_summary(repo, selection)?;
    let resolved = resolve_diff(repo, selection)?;
    paths
        .iter()
        .map(|path| file_diff_from_summary(repo, &summary, &resolved, path))
        .collect()
}

fn file_diff_from_summary(
    repo: &Path,
    summary: &DiffSummary,
    resolved: &ResolvedDiff,
    path: &str,
) -> AppResult<FileDiff> {
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
    let untracked_working_tree_file =
        matches!(resolved.range, DiffRange::RevisionToWorkingTree { .. })
            && !git_output(repo, ["ls-files", "--error-unmatch", "--", path])?
                .status
                .success();
    let bytes = if !untracked_working_tree_file {
        successful(
            range_output(
                repo,
                resolved,
                &[
                    "--no-color",
                    "--no-ext-diff",
                    "--find-renames",
                    "--find-copies",
                ],
                Some(path),
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
    let (old_source, new_source) = match &resolved.range {
        DiffRange::Revisions { from, to } => {
            (ContentSource::Commit(from), ContentSource::Commit(to))
        }
        DiffRange::RevisionToWorkingTree { from } => {
            (ContentSource::Commit(from), ContentSource::WorkingTree)
        }
    };
    let old_content = match (&file.old_path, &file.status) {
        (Some(_), DiffStatus::Binary) => None,
        (Some(old_path), _) => content_from_source(repo, old_source, old_path)?,
        _ => None,
    };
    let new_content = match (&file.new_path, &file.status) {
        (Some(_), DiffStatus::Binary) => None,
        (Some(new_path), _) => content_from_source(repo, new_source, new_path)?,
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

    fn git_with_date(repo: &Path, args: &[&str], date: &str) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: {args:?}");
    }

    fn initialize_repository(repo: &Path, branch: &str) {
        git(repo, &["init", "-b", branch]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
        fs::write(repo.join("README.md"), "initial\n").unwrap();
        git(repo, &["add", "README.md"]);
        git(repo, &["commit", "-m", "initial"]);
    }

    #[test]
    fn base_ref_detection_prefers_origin_head() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        initialize_repository(repo, "main");
        git(repo, &["branch", "develop"]);
        git(
            repo,
            &["update-ref", "refs/remotes/origin/develop", "develop"],
        );
        git(
            repo,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/develop",
            ],
        );

        let (detected, _) = detect_base_ref(repo).unwrap();

        assert_eq!(detected.as_deref(), Some("develop"));
    }

    #[test]
    fn base_ref_detection_prefers_main_then_master() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        initialize_repository(repo, "master");
        git(repo, &["branch", "main"]);

        let (with_main, _) = detect_base_ref(repo).unwrap();
        assert_eq!(with_main.as_deref(), Some("main"));

        git(repo, &["branch", "-D", "main"]);
        let (with_master, _) = detect_base_ref(repo).unwrap();
        assert_eq!(with_master.as_deref(), Some("master"));
    }

    #[test]
    fn base_ref_detection_does_not_guess_an_arbitrary_branch() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        initialize_repository(repo, "feature");

        let (detected, _) = detect_base_ref(repo).unwrap();

        assert_eq!(detected, None);
    }

    #[test]
    fn recent_repository_lists_are_sorted_and_capped() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
        fs::write(repo.join("history.txt"), "base\n").unwrap();
        git(repo, &["add", "history.txt"]);
        git_with_date(repo, &["commit", "-m", "base"], "2026-01-01T00:00:00Z");

        for index in 1..=11 {
            fs::write(repo.join("history.txt"), format!("commit {index}\n")).unwrap();
            git(repo, &["add", "history.txt"]);
            let subject = format!("commit-{index}");
            let date = format!("2026-01-01T00:00:{index:02}Z");
            git_with_date(repo, &["commit", "-m", &subject], &date);
            if index <= 6 {
                let branch = format!("branch-{index}");
                git(repo, &["branch", &branch]);
            }
        }
        git(repo, &["update-ref", "refs/remotes/origin/recent", "HEAD"]);

        let branches = recent_branches(repo, Some("main")).unwrap();
        assert_eq!(
            branches,
            ["branch-6", "branch-5", "branch-4", "branch-3", "branch-2"]
        );
        assert!(branches.iter().all(|branch| !branch.starts_with("origin/")));

        let commits = recent_commits(repo).unwrap();
        assert_eq!(commits.len(), 10);
        assert_eq!(commits.first().unwrap().subject, "commit-11");
        assert_eq!(commits.last().unwrap().subject, "commit-2");
    }

    #[test]
    fn default_working_tree_diff_uses_head_as_its_base() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
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

        let summary = diff_summary(repo, &DiffSelection::default()).unwrap();
        assert!(summary.files.is_empty());
        assert_eq!(summary.comparison.from_label, "HEAD");
        assert_eq!(summary.comparison.to_label, "working tree");
    }

    #[test]
    fn change_review_only_accepts_native_codex_review_targets() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
        fs::write(repo.join("base.txt"), "base\n").unwrap();
        git(repo, &["add", "base.txt"]);
        git(repo, &["commit", "-m", "base"]);
        git(repo, &["switch", "-c", "feature"]);
        fs::write(repo.join("feature.txt"), "feature\n").unwrap();
        git(repo, &["add", "feature.txt"]);
        git(repo, &["commit", "-m", "feature"]);
        fs::write(repo.join("working.txt"), "working\n").unwrap();

        let uncommitted = change_review_availability(repo, &DiffSelection::default()).unwrap();
        assert!(uncommitted.available);
        assert_eq!(uncommitted.target, Some(ChangeReviewTarget::Uncommitted));

        let branch = change_review_availability(
            repo,
            &DiffSelection {
                base: "main".to_owned(),
                target: "feature".to_owned(),
            },
        )
        .unwrap();
        assert!(branch.available);
        assert_eq!(
            branch.target,
            Some(ChangeReviewTarget::Base {
                base_branch: "main".to_owned()
            })
        );

        let unsupported = change_review_availability(
            repo,
            &DiffSelection {
                base: "main".to_owned(),
                target: ".".to_owned(),
            },
        )
        .unwrap();
        assert!(!unsupported.available);
        assert!(unsupported.reason.unwrap().contains("starts at HEAD"));
    }

    #[test]
    fn change_review_is_unavailable_without_changes() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
        fs::write(repo.join("base.txt"), "base\n").unwrap();
        git(repo, &["add", "base.txt"]);
        git(repo, &["commit", "-m", "base"]);

        let availability = change_review_availability(repo, &DiffSelection::default()).unwrap();

        assert!(!availability.available);
        assert!(availability
            .reason
            .unwrap()
            .contains("comparison has no changes"));
    }

    #[test]
    fn working_tree_diff_includes_staged_unstaged_and_untracked_files() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
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

        let selection = DiffSelection::default();
        let summary = diff_summary(repo, &selection).unwrap();
        let paths: Vec<&str> = summary
            .files
            .iter()
            .filter_map(|file| file.new_path.as_deref())
            .collect();
        assert_eq!(paths, ["committed.txt", "staged.txt", "untracked.txt"]);
        assert_eq!(summary.total_additions, 3);

        let diff = file_diff(repo, &selection, "committed.txt").unwrap();
        assert_eq!(diff.new_content.as_deref(), Some("committed\nunstaged\n"));
        assert!(diff.unified_diff.contains("+unstaged"));

        let untracked = file_diff(repo, &selection, "untracked.txt").unwrap();
        assert_eq!(untracked.new_content.as_deref(), Some("untracked\n"));
        assert!(untracked.unified_diff.contains("+untracked"));

        let diffs = file_diffs(
            repo,
            &selection,
            &["committed.txt".to_owned(), "untracked.txt".to_owned()],
        )
        .unwrap();
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].file.new_path.as_deref(), Some("committed.txt"));
        assert_eq!(diffs[1].file.new_path.as_deref(), Some("untracked.txt"));
    }

    #[test]
    fn explicit_commits_are_compared_as_two_endpoints() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
        fs::write(repo.join("base.txt"), "base\n").unwrap();
        git(repo, &["add", "base.txt"]);
        git(repo, &["commit", "-m", "base"]);
        fs::write(repo.join("latest.txt"), "latest\n").unwrap();
        git(repo, &["add", "latest.txt"]);
        git(repo, &["commit", "-m", "latest"]);

        let selection = DiffSelection {
            base: "HEAD~1".to_owned(),
            target: "HEAD".to_owned(),
        };
        let summary = diff_summary(repo, &selection).unwrap();
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].new_path.as_deref(), Some("latest.txt"));
        let diff = file_diff(repo, &selection, "latest.txt").unwrap();
        assert!(diff.unified_diff.contains("+latest"));
    }

    #[test]
    fn explicit_refs_are_compared_directly() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
        fs::write(repo.join("base.txt"), "base\n").unwrap();
        git(repo, &["add", "base.txt"]);
        git(repo, &["commit", "-m", "base"]);
        git(repo, &["switch", "-c", "feature"]);
        fs::write(repo.join("feature.txt"), "feature\n").unwrap();
        git(repo, &["add", "feature.txt"]);
        git(repo, &["commit", "-m", "feature"]);
        git(repo, &["switch", "main"]);
        fs::write(repo.join("main.txt"), "main\n").unwrap();
        git(repo, &["add", "main.txt"]);
        git(repo, &["commit", "-m", "main"]);

        let selection = DiffSelection {
            base: "main".to_owned(),
            target: "feature".to_owned(),
        };
        let summary = diff_summary(repo, &selection).unwrap();
        let paths: Vec<&str> = summary
            .files
            .iter()
            .filter_map(|file| file.new_path.as_deref().or(file.old_path.as_deref()))
            .collect();
        assert_eq!(paths, ["feature.txt", "main.txt"]);

        let feature = file_diff(repo, &selection, "feature.txt").unwrap();
        assert_eq!(feature.new_content.as_deref(), Some("feature\n"));
    }

    #[test]
    fn parses_rename_name_status() {
        let files = parse_name_status(b"R100\0old.rs\0new.rs\0");
        assert_eq!(files[0].status, DiffStatus::Renamed);
        assert_eq!(files[0].old_path.as_deref(), Some("old.rs"));
        assert_eq!(files[0].new_path.as_deref(), Some("new.rs"));
    }
}
