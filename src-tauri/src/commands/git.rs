use crate::{
    domain::{DiffSelection, DiffWorkspace, FileDiff},
    error::AppResult,
    services::{config_service, git_service},
};
use std::path::Path;
use tauri::{AppHandle, Runtime};

#[tauri::command(async)]
pub fn get_diff_workspace<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: DiffSelection,
) -> AppResult<DiffWorkspace> {
    let project = config_service::find_project(&app, &project_id)?;
    git_service::diff_workspace(Path::new(&project.repo_path), &selection)
}

#[tauri::command(async)]
pub fn get_file_diffs<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: DiffSelection,
    paths: Vec<String>,
) -> AppResult<Vec<FileDiff>> {
    let project = config_service::find_project(&app, &project_id)?;
    git_service::file_diffs(Path::new(&project.repo_path), &selection, &paths)
}
