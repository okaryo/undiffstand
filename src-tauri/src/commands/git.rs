use crate::{
    domain::{DiffSelection, DiffWorkspace, FileDiff},
    error::AppResult,
    services::{config_service, git_service},
};
use std::{path::Path, sync::Mutex};
use tauri::{AppHandle, Runtime, State};

const MAX_CACHED_DIFF_SNAPSHOTS: usize = 4;

struct CachedDiffSnapshot {
    project_id: String,
    repo_path: String,
    selection: DiffSelection,
    snapshot: git_service::DiffSnapshot,
}

#[derive(Default)]
pub struct DiffSnapshotCache(Mutex<Vec<CachedDiffSnapshot>>);

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

#[tauri::command(async)]
pub fn get_diff_workspace<R: Runtime>(
    app: AppHandle<R>,
    cache: State<'_, DiffSnapshotCache>,
    project_id: String,
    selection: DiffSelection,
) -> AppResult<DiffWorkspace> {
    let project = config_service::find_project(&app, &project_id)?;
    let (workspace, snapshot) =
        git_service::diff_workspace_with_snapshot(Path::new(&project.repo_path), &selection)?;
    cache.store(project_id, project.repo_path, selection, snapshot);
    Ok(workspace)
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
