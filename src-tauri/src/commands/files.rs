use crate::{
    domain::{FileContent, RepoFile},
    error::AppResult,
    services::{config_service, file_service},
};
use std::path::Path;
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn list_repository_files<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
) -> AppResult<Vec<RepoFile>> {
    let project = config_service::find_project(&app, &project_id)?;
    file_service::list_files(Path::new(&project.repo_path))
}

#[tauri::command]
pub fn read_repository_file<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    path: String,
) -> AppResult<FileContent> {
    let project = config_service::find_project(&app, &project_id)?;
    file_service::read_file(Path::new(&project.repo_path), &path)
}
