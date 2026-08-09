use crate::{
    domain::{DiffSummary, FileDiff},
    error::AppResult,
    services::{config_service, git_service},
};
use std::path::Path;
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn get_diff_summary<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
) -> AppResult<DiffSummary> {
    let project = config_service::find_project(&app, &project_id)?;
    git_service::diff_summary(Path::new(&project.repo_path), &project.base_ref)
}

#[tauri::command]
pub fn get_file_diff<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    path: String,
) -> AppResult<FileDiff> {
    let project = config_service::find_project(&app, &project_id)?;
    git_service::file_diff(Path::new(&project.repo_path), &project.base_ref, &path)
}

#[tauri::command]
pub fn get_file_diffs<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    paths: Vec<String>,
) -> AppResult<Vec<FileDiff>> {
    let project = config_service::find_project(&app, &project_id)?;
    git_service::file_diffs(Path::new(&project.repo_path), &project.base_ref, &paths)
}
