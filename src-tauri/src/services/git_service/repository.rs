use super::command::{git_output, output_text};
use crate::{
    domain::{GitCommitSummary, RepositoryInfo},
    error::{AppError, AppResult},
};
use std::path::{Path, PathBuf};

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

pub(super) fn ref_exists(repo: &Path, reference: &str) -> AppResult<bool> {
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

pub(super) fn current_branch(repo: &Path) -> AppResult<Option<String>> {
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

pub(super) fn list_local_branches(repo: &Path) -> AppResult<Vec<String>> {
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

pub(super) fn list_refs(repo: &Path, namespace: &str) -> AppResult<Vec<String>> {
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

pub(super) fn recent_branches(repo: &Path, current_branch: Option<&str>) -> AppResult<Vec<String>> {
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

pub(super) fn recent_commits(repo: &Path) -> AppResult<Vec<GitCommitSummary>> {
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
