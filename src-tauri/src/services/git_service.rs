mod command;
mod parser;
mod repository;

use crate::{
    domain::{
        ChangeReviewAvailability, ChangeReviewTarget, ComparisonCommit, DiffComparison,
        DiffFileSummary, DiffScope, DiffSelection, DiffStatus, DiffSummary, DiffWorkspace,
        FileDiff,
    },
    error::{AppError, AppResult},
    services::file_service,
};
use command::{git_output, output_text, successful};
use parser::{parse_name_status, parse_numstat};
pub use repository::{canonical_repository, inspect_repository, validate_base_ref};
use repository::{current_branch, list_local_branches, list_refs};
#[cfg(test)]
use repository::{detect_base_ref, recent_branches, recent_commits};
use std::{collections::HashSet, path::Path, process::Output};

const MAX_DIFF_BYTES: usize = 1_500_000;
const MAX_CONTENT_BYTES: usize = 2_000_000;
const EMPTY_TREE_SHA: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

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

#[derive(Debug, Clone)]
pub struct DiffSnapshot {
    summary: DiffSummary,
    resolved: ResolvedDiff,
    untracked_paths: HashSet<String>,
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

    let from = if base == EMPTY_TREE_SHA {
        EMPTY_TREE_SHA.to_owned()
    } else {
        resolve_commit(repo, base)?
    };
    let from_label = if base == EMPTY_TREE_SHA {
        "empty tree".to_owned()
    } else {
        normalized_ref(base).to_owned()
    };
    if target == "." {
        Ok(ResolvedDiff {
            selection: selection.clone(),
            comparison: DiffComparison {
                from_label: from_label.clone(),
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
                from_label,
                to_label: normalized_ref(target).to_owned(),
                from_sha: Some(from.clone()),
                to_sha: Some(to.clone()),
            },
            range: DiffRange::Revisions { from, to },
        })
    }
}

pub fn comparison_commits(
    repo: &Path,
    selection: &DiffSelection,
) -> AppResult<Vec<ComparisonCommit>> {
    let (base, target) = comparison_commit_range(repo, selection)?;
    comparison_commits_for_range(repo, &base, &target)
}

pub fn comparison_commit_range(
    repo: &Path,
    selection: &DiffSelection,
) -> AppResult<(String, String)> {
    let base = resolve_commit(repo, selection.base.trim())?;
    let target = if selection.target.trim() == "." {
        resolve_commit(repo, "HEAD")?
    } else {
        resolve_commit(repo, selection.target.trim())?
    };
    Ok((base, target))
}

pub fn comparison_commits_for_range(
    repo: &Path,
    base: &str,
    target: &str,
) -> AppResult<Vec<ComparisonCommit>> {
    let range = format!("{base}..{target}");
    let bytes = successful(
        git_output(
            repo,
            [
                "log",
                "--reverse",
                "--topo-order",
                "--format=%H%x1f%h%x1f%s%x1f%an%x1f%aI%x1f%P%x1e",
                &range,
            ],
        )?,
        "UNKNOWN",
        "The commits in this comparison could not be read.",
    )?;

    Ok(bytes
        .split(|byte| *byte == 0x1e)
        .filter_map(|record| {
            let record = record
                .iter()
                .copied()
                .filter(|byte| *byte != b'\n')
                .collect::<Vec<_>>();
            if record.is_empty() {
                return None;
            }
            let mut fields = record.split(|byte| *byte == 0x1f);
            let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
            let sha = text(fields.next()?);
            let short_sha = text(fields.next()?);
            let subject = text(fields.next()?);
            let author_name = text(fields.next()?);
            let authored_at = text(fields.next()?);
            let parents = fields.next().unwrap_or_default();
            Some(ComparisonCommit {
                sha,
                short_sha,
                subject,
                author_name,
                authored_at,
                parent_count: parents
                    .split(|byte| *byte == b' ')
                    .filter(|parent| !parent.is_empty())
                    .count(),
            })
        })
        .collect())
}

fn resolve_scoped_diff(
    repo: &Path,
    selection: &DiffSelection,
    scope: &DiffScope,
) -> AppResult<ResolvedDiff> {
    match scope {
        DiffScope::All => resolve_diff(repo, selection),
        DiffScope::Uncommitted => {
            if selection.target.trim() != "." {
                return Err(AppError::new(
                    "INVALID_DIFF_SCOPE",
                    "Uncommitted changes are not part of this comparison.",
                ));
            }
            resolve_diff(repo, &DiffSelection::default())
        }
        DiffScope::Commit { sha } => {
            let commit = comparison_commits(repo, selection)?
                .into_iter()
                .find(|commit| commit.sha == *sha)
                .ok_or_else(|| {
                    AppError::new(
                        "INVALID_DIFF_SCOPE",
                        "The selected commit is not part of this comparison.",
                    )
                })?;
            let parents = commit_parents(repo, &commit.sha)?;
            resolve_diff(
                repo,
                &DiffSelection {
                    base: parents
                        .first()
                        .cloned()
                        .unwrap_or_else(|| EMPTY_TREE_SHA.to_owned()),
                    target: commit.sha,
                },
            )
        }
    }
}

fn commit_parents(repo: &Path, sha: &str) -> AppResult<Vec<String>> {
    let output = git_output(repo, ["show", "-s", "--format=%P", sha])?;
    Ok(output_text(
        output,
        "INVALID_DIFF_TARGET",
        "The selected commit could not be read.",
    )?
    .split_whitespace()
    .map(ToOwned::to_owned)
    .collect())
}

pub fn validate_diff_selection(repo: &Path, selection: &DiffSelection) -> AppResult<()> {
    resolve_diff(repo, selection).map(|_| ())
}

pub fn change_review_availability(
    repo: &Path,
    selection: &DiffSelection,
) -> AppResult<ChangeReviewAvailability> {
    let has_changes = !diff_summary(repo, selection)?.files.is_empty();
    change_review_availability_for(repo, selection, has_changes)
}

pub fn diff_workspace_with_snapshot(
    repo: &Path,
    selection: &DiffSelection,
    scope: &DiffScope,
) -> AppResult<(DiffWorkspace, DiffSnapshot)> {
    let snapshot = diff_snapshot_for_scope(repo, selection, scope)?;
    let summary = snapshot.summary.clone();
    let review_availability =
        change_review_availability_for(repo, &summary.selection, !summary.files.is_empty())?;
    Ok((
        DiffWorkspace {
            summary,
            review_availability,
        },
        snapshot,
    ))
}

fn change_review_availability_for(
    repo: &Path,
    selection: &DiffSelection,
    has_changes: bool,
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

    if !has_changes {
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

    if let Some(commit) = commit_review_target(repo, base, target)? {
        let commit_scope_label = match &commit {
            ChangeReviewTarget::Commit { sha, title } => {
                format!("commit {} · {title}", &sha[..7])
            }
            _ => scope_label.clone(),
        };
        return Ok(ChangeReviewAvailability {
            available: true,
            target: Some(commit),
            reason: None,
            scope_label: commit_scope_label,
        });
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

fn commit_review_target(
    repo: &Path,
    base: &str,
    target: &str,
) -> AppResult<Option<ChangeReviewTarget>> {
    let looks_like_sha =
        |value: &str| value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !looks_like_sha(base) || !looks_like_sha(target) {
        return Ok(None);
    }
    let parents = commit_parents(repo, target)?;
    let matches_parent = parents.first().map(String::as_str) == Some(base)
        || (parents.is_empty() && base == EMPTY_TREE_SHA);
    if !matches_parent {
        return Ok(None);
    }
    let title = output_text(
        git_output(repo, ["show", "-s", "--format=%s", target])?,
        "INVALID_DIFF_TARGET",
        "The selected commit could not be read.",
    )?
    .trim()
    .to_owned();
    Ok(Some(ChangeReviewTarget::Commit {
        sha: target.to_owned(),
        title,
    }))
}

fn unavailable_review(scope_label: String, reason: &str) -> ChangeReviewAvailability {
    ChangeReviewAvailability {
        available: false,
        target: None,
        reason: Some(reason.to_owned()),
        scope_label,
    }
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
    diff_snapshot(repo, selection).map(|snapshot| snapshot.summary)
}

fn diff_snapshot(repo: &Path, selection: &DiffSelection) -> AppResult<DiffSnapshot> {
    diff_snapshot_for_scope(repo, selection, &DiffScope::All)
}

fn diff_snapshot_for_scope(
    repo: &Path,
    selection: &DiffSelection,
    scope: &DiffScope,
) -> AppResult<DiffSnapshot> {
    let resolved = resolve_scoped_diff(repo, selection, scope)?;
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
    let mut untracked_paths = HashSet::new();
    if matches!(resolved.range, DiffRange::RevisionToWorkingTree { .. }) {
        let untracked = untracked_files(repo)?;
        untracked_paths.extend(
            untracked
                .iter()
                .filter_map(|file| file.new_path.as_ref().cloned()),
        );
        files.extend(untracked);
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
    Ok(DiffSnapshot {
        summary: DiffSummary {
            selection: resolved.selection.clone(),
            comparison: resolved.comparison.clone(),
            files,
            total_additions,
            total_deletions,
        },
        resolved,
        untracked_paths,
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

#[cfg(test)]
pub fn file_diff(repo: &Path, selection: &DiffSelection, path: &str) -> AppResult<FileDiff> {
    file_diff_with_summary(repo, selection, path).map(|(_, diff)| diff)
}

pub fn file_diff_with_summary(
    repo: &Path,
    selection: &DiffSelection,
    path: &str,
) -> AppResult<(DiffSummary, FileDiff)> {
    let snapshot = diff_snapshot(repo, selection)?;
    let diff = file_diff_from_snapshot(repo, &snapshot, path)?;
    Ok((snapshot.summary, diff))
}

pub fn file_diffs(
    repo: &Path,
    selection: &DiffSelection,
    paths: &[String],
) -> AppResult<Vec<FileDiff>> {
    let snapshot = diff_snapshot(repo, selection)?;
    file_diffs_from_snapshot(repo, &snapshot, paths)
}

pub fn file_diffs_from_snapshot(
    repo: &Path,
    snapshot: &DiffSnapshot,
    paths: &[String],
) -> AppResult<Vec<FileDiff>> {
    paths
        .iter()
        .map(|path| file_diff_from_snapshot(repo, snapshot, path))
        .collect()
}

fn file_diff_from_snapshot(
    repo: &Path,
    snapshot: &DiffSnapshot,
    path: &str,
) -> AppResult<FileDiff> {
    let file = snapshot
        .summary
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
    let untracked_working_tree_file = snapshot.untracked_paths.contains(path);
    let bytes = if !untracked_working_tree_file {
        successful(
            range_output(
                repo,
                &snapshot.resolved,
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
    let (old_source, new_source) = match &snapshot.resolved.range {
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
    Ok(FileDiff {
        file,
        old_content,
        new_content,
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
    fn diff_selection_validation_rejects_a_deleted_ref() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        git(repo, &["init", "-b", "main"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "undiffstand test"]);
        fs::write(repo.join("base.txt"), "base\n").unwrap();
        git(repo, &["add", "base.txt"]);
        git(repo, &["commit", "-m", "base"]);

        let error = validate_diff_selection(
            repo,
            &DiffSelection {
                base: "deleted-branch".to_owned(),
                target: "HEAD".to_owned(),
            },
        )
        .expect_err("a deleted ref should be rejected");

        assert_eq!(error.code, "INVALID_DIFF_TARGET");
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
    fn comparison_commits_include_merged_history_in_topological_order() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        initialize_repository(repo, "main");
        let base = resolve_commit(repo, "HEAD").unwrap();

        git(repo, &["switch", "-c", "feature"]);
        fs::write(repo.join("feature.txt"), "feature\n").unwrap();
        git(repo, &["add", "feature.txt"]);
        git(repo, &["commit", "-m", "feature change"]);
        git(repo, &["switch", "-c", "side", &base]);
        fs::write(repo.join("side.txt"), "side\n").unwrap();
        git(repo, &["add", "side.txt"]);
        git(repo, &["commit", "-m", "side change"]);
        git(repo, &["switch", "feature"]);
        git(repo, &["merge", "--no-ff", "side", "-m", "merge side"]);

        let commits = comparison_commits(
            repo,
            &DiffSelection {
                base,
                target: "feature".to_owned(),
            },
        )
        .unwrap();
        let subjects: Vec<&str> = commits
            .iter()
            .map(|commit| commit.subject.as_str())
            .collect();

        assert_eq!(subjects.len(), 3);
        assert!(subjects.contains(&"feature change"));
        assert!(subjects.contains(&"side change"));
        assert_eq!(subjects.last(), Some(&"merge side"));
        assert_eq!(commits.last().unwrap().parent_count, 2);
    }

    #[test]
    fn merge_commit_scope_uses_the_first_parent_and_supports_change_review() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        initialize_repository(repo, "main");
        let base = resolve_commit(repo, "HEAD").unwrap();

        git(repo, &["switch", "-c", "feature"]);
        fs::write(repo.join("feature.txt"), "feature\n").unwrap();
        git(repo, &["add", "feature.txt"]);
        git(repo, &["commit", "-m", "feature change"]);
        git(repo, &["switch", "-c", "side", &base]);
        fs::write(repo.join("side.txt"), "side\n").unwrap();
        git(repo, &["add", "side.txt"]);
        git(repo, &["commit", "-m", "side change"]);
        git(repo, &["switch", "feature"]);
        git(repo, &["merge", "--no-ff", "side", "-m", "merge side"]);
        let merge_sha = resolve_commit(repo, "HEAD").unwrap();

        let (workspace, _) = diff_workspace_with_snapshot(
            repo,
            &DiffSelection {
                base,
                target: "feature".to_owned(),
            },
            &DiffScope::Commit {
                sha: merge_sha.clone(),
            },
        )
        .unwrap();
        let paths: Vec<&str> = workspace
            .summary
            .files
            .iter()
            .filter_map(|file| file.new_path.as_deref())
            .collect();

        assert_eq!(paths, ["side.txt"]);
        assert_eq!(workspace.summary.selection.target, merge_sha);
        assert_eq!(
            workspace.review_availability.target,
            Some(ChangeReviewTarget::Commit {
                sha: workspace.summary.selection.target.clone(),
                title: "merge side".to_owned(),
            })
        );
    }

    #[test]
    fn uncommitted_scope_excludes_committed_changes_from_a_working_tree_comparison() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        initialize_repository(repo, "main");
        let base = resolve_commit(repo, "HEAD").unwrap();
        fs::write(repo.join("committed.txt"), "committed\n").unwrap();
        git(repo, &["add", "committed.txt"]);
        git(repo, &["commit", "-m", "committed change"]);
        fs::write(repo.join("working.txt"), "working\n").unwrap();

        let (workspace, _) = diff_workspace_with_snapshot(
            repo,
            &DiffSelection {
                base,
                target: ".".to_owned(),
            },
            &DiffScope::Uncommitted,
        )
        .unwrap();
        let paths: Vec<&str> = workspace
            .summary
            .files
            .iter()
            .filter_map(|file| file.new_path.as_deref())
            .collect();

        assert_eq!(paths, ["working.txt"]);
        assert_eq!(workspace.summary.selection, DiffSelection::default());
    }

    #[test]
    fn root_commit_scope_uses_the_empty_tree_as_its_parent() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        initialize_repository(repo, "main");
        let base = resolve_commit(repo, "main").unwrap();
        git(repo, &["switch", "--orphan", "unrelated"]);
        fs::write(repo.join("root.txt"), "root\n").unwrap();
        git(repo, &["add", "root.txt"]);
        git(repo, &["commit", "-m", "unrelated root"]);
        let root_sha = resolve_commit(repo, "HEAD").unwrap();

        let (workspace, _) = diff_workspace_with_snapshot(
            repo,
            &DiffSelection {
                base,
                target: "unrelated".to_owned(),
            },
            &DiffScope::Commit {
                sha: root_sha.clone(),
            },
        )
        .unwrap();

        assert_eq!(workspace.summary.selection.base, EMPTY_TREE_SHA);
        assert_eq!(workspace.summary.selection.target, root_sha);
        assert_eq!(workspace.summary.files.len(), 1);
        assert_eq!(
            workspace.summary.files[0].new_path.as_deref(),
            Some("root.txt")
        );
        assert!(matches!(
            workspace.review_availability.target,
            Some(ChangeReviewTarget::Commit { .. })
        ));
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
