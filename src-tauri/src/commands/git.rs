use crate::{
    domain::{ComparisonCommit, DiffScope, DiffSelection, DiffWorkspace, FileDiff},
    error::AppResult,
    services::{config_service, git_service},
};
use std::{path::Path, sync::Mutex};
use tauri::{AppHandle, Runtime, State};

const MAX_CACHED_DIFF_SNAPSHOTS: usize = 4;
const MAX_CACHED_COMMIT_LISTS: usize = 4;

struct CachedDiffSnapshot {
    project_id: String,
    repo_path: String,
    selection: DiffSelection,
    snapshot: git_service::DiffSnapshot,
}

#[derive(Default)]
pub struct DiffSnapshotCache(Mutex<Vec<CachedDiffSnapshot>>);

struct CachedComparisonCommits {
    project_id: String,
    repo_path: String,
    base_sha: String,
    target_sha: String,
    commits: Vec<ComparisonCommit>,
}

#[derive(Default)]
pub struct ComparisonCommitCache(Mutex<Vec<CachedComparisonCommits>>);

impl DiffSnapshotCache {
    fn store(
        &self,
        project_id: String,
        repo_path: String,
        selection: DiffSelection,
        snapshot: git_service::DiffSnapshot,
    ) {
        let mut snapshots = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        snapshots.retain(|cached| {
            cached.project_id != project_id
                || cached.repo_path != repo_path
                || cached.selection != selection
        });
        snapshots.push(CachedDiffSnapshot {
            project_id,
            repo_path,
            selection,
            snapshot,
        });
        if snapshots.len() > MAX_CACHED_DIFF_SNAPSHOTS {
            snapshots.remove(0);
        }
    }

    fn get(
        &self,
        project_id: &str,
        repo_path: &str,
        selection: &DiffSelection,
    ) -> Option<git_service::DiffSnapshot> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .rev()
            .find(|cached| {
                cached.project_id == project_id
                    && cached.repo_path == repo_path
                    && cached.selection == *selection
            })
            .map(|cached| cached.snapshot.clone())
    }
}

impl ComparisonCommitCache {
    fn store(
        &self,
        project_id: String,
        repo_path: String,
        base_sha: String,
        target_sha: String,
        commits: Vec<ComparisonCommit>,
    ) {
        let mut cached_lists = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cached_lists.retain(|cached| {
            cached.project_id != project_id
                || cached.repo_path != repo_path
                || cached.base_sha != base_sha
                || cached.target_sha != target_sha
        });
        cached_lists.push(CachedComparisonCommits {
            project_id,
            repo_path,
            base_sha,
            target_sha,
            commits,
        });
        if cached_lists.len() > MAX_CACHED_COMMIT_LISTS {
            cached_lists.remove(0);
        }
    }

    fn get(
        &self,
        project_id: &str,
        repo_path: &str,
        base_sha: &str,
        target_sha: &str,
    ) -> Option<Vec<ComparisonCommit>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .rev()
            .find(|cached| {
                cached.project_id == project_id
                    && cached.repo_path == repo_path
                    && cached.base_sha == base_sha
                    && cached.target_sha == target_sha
            })
            .map(|cached| cached.commits.clone())
    }
}

#[tauri::command(async)]
pub fn get_diff_workspace<R: Runtime>(
    app: AppHandle<R>,
    cache: State<'_, DiffSnapshotCache>,
    project_id: String,
    selection: DiffSelection,
    scope: DiffScope,
) -> AppResult<DiffWorkspace> {
    let project = config_service::find_project(&app, &project_id)?;
    let (workspace, snapshot) = git_service::diff_workspace_with_snapshot(
        Path::new(&project.repo_path),
        &selection,
        &scope,
    )?;
    cache.store(
        project_id,
        project.repo_path,
        workspace.summary.selection.clone(),
        snapshot,
    );
    Ok(workspace)
}

#[tauri::command(async)]
pub fn get_comparison_commits<R: Runtime>(
    app: AppHandle<R>,
    cache: State<'_, ComparisonCommitCache>,
    project_id: String,
    selection: DiffSelection,
) -> AppResult<Vec<ComparisonCommit>> {
    let project = config_service::find_project(&app, &project_id)?;
    let repo = Path::new(&project.repo_path);
    let (base_sha, target_sha) = git_service::comparison_commit_range(repo, &selection)?;
    if let Some(commits) = cache.get(&project_id, &project.repo_path, &base_sha, &target_sha) {
        return Ok(commits);
    }
    let commits = git_service::comparison_commits_for_range(repo, &base_sha, &target_sha)?;
    cache.store(
        project_id,
        project.repo_path,
        base_sha,
        target_sha,
        commits.clone(),
    );
    Ok(commits)
}

#[tauri::command(async)]
pub fn get_file_diffs<R: Runtime>(
    app: AppHandle<R>,
    cache: State<'_, DiffSnapshotCache>,
    project_id: String,
    selection: DiffSelection,
    paths: Vec<String>,
) -> AppResult<Vec<FileDiff>> {
    let project = config_service::find_project(&app, &project_id)?;
    let repo = Path::new(&project.repo_path);
    if let Some(snapshot) = cache.get(&project_id, &project.repo_path, &selection) {
        git_service::file_diffs_from_snapshot(repo, &snapshot, &paths)
    } else {
        git_service::file_diffs(repo, &selection, &paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(sha: &str) -> ComparisonCommit {
        ComparisonCommit {
            sha: sha.to_owned(),
            short_sha: sha.chars().take(7).collect(),
            subject: "subject".to_owned(),
            author_name: "author".to_owned(),
            authored_at: "2026-08-23T00:00:00Z".to_owned(),
            parent_count: 1,
        }
    }

    #[test]
    fn comparison_commit_cache_uses_resolved_range_and_replaces_entries() {
        let cache = ComparisonCommitCache::default();
        cache.store(
            "project".to_owned(),
            "/repo".to_owned(),
            "base".to_owned(),
            "target".to_owned(),
            vec![commit("1111111")],
        );
        cache.store(
            "project".to_owned(),
            "/repo".to_owned(),
            "base".to_owned(),
            "target".to_owned(),
            vec![commit("2222222")],
        );

        assert_eq!(
            cache.get("project", "/repo", "base", "target").unwrap()[0].sha,
            "2222222"
        );
        assert!(cache
            .get("project", "/repo", "base", "other-target")
            .is_none());
    }
}
