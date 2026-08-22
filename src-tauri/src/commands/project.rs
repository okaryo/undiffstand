use crate::{
    domain::{DiffSelection, ProjectConfig, RepositoryInfo, SaveProjectInput},
    error::{AppError, AppResult},
    services::{config_service, git_service},
};
use chrono::Utc;
use std::path::Path;
use tauri::{AppHandle, Runtime};
use uuid::Uuid;

#[tauri::command]
pub fn validate_repository(path: String) -> AppResult<RepositoryInfo> {
    git_service::inspect_repository(Path::new(&path))
}

#[tauri::command]
pub fn list_projects<R: Runtime>(app: AppHandle<R>) -> AppResult<Vec<ProjectConfig>> {
    let mut projects = config_service::load(&app)?.projects;
    projects.sort_by(|left, right| right.last_opened_at.cmp(&left.last_opened_at));
    Ok(projects)
}

#[tauri::command]
pub fn save_project<R: Runtime>(
    app: AppHandle<R>,
    input: SaveProjectInput,
) -> AppResult<ProjectConfig> {
    let repo = git_service::canonical_repository(Path::new(&input.repo_path))?;
    git_service::validate_base_ref(&repo, &input.base_ref)?;
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "INVALID_PROJECT",
            "Project name cannot be empty.",
        ));
    }
    let mut config = config_service::load(&app)?;
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let comparison = config
        .projects
        .iter()
        .find(|project| project.id == id)
        .map(|project| project.comparison.clone())
        .unwrap_or_default();
    let project = ProjectConfig {
        id: id.clone(),
        name: name.to_owned(),
        repo_path: repo.to_string_lossy().into_owned(),
        base_ref: input.base_ref.trim().to_owned(),
        comparison,
        last_opened_at: Utc::now().to_rfc3339(),
    };
    if let Some(index) = config.projects.iter().position(|item| item.id == id) {
        config.projects[index] = project.clone();
    } else {
        config.projects.push(project.clone());
    }
    config.active_project_id = Some(id);
    config_service::save(&app, &config)?;
    Ok(project)
}

#[tauri::command]
pub fn touch_project<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: Option<DiffSelection>,
) -> AppResult<ProjectConfig> {
    let mut config = config_service::load(&app)?;
    let project = config
        .projects
        .iter_mut()
        .find(|project| project.id == project_id)
        .ok_or_else(|| {
            AppError::new(
                "PROJECT_NOT_FOUND",
                "The selected project no longer exists.",
            )
        })?;
    let requested_selection = selection.unwrap_or_else(|| project.comparison.clone());
    let repo = git_service::canonical_repository(Path::new(&project.repo_path))?;
    if let Err(error) = git_service::validate_diff_selection(&repo, &requested_selection) {
        if error.code == "INVALID_DIFF_TARGET" {
            project.comparison = DiffSelection::default();
        } else {
            return Err(error);
        }
    } else {
        project.comparison = requested_selection;
    }
    project.last_opened_at = Utc::now().to_rfc3339();
    let result = project.clone();
    config.active_project_id = Some(project_id);
    config_service::save(&app, &config)?;
    Ok(result)
}

#[tauri::command]
pub fn save_project_comparison<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: DiffSelection,
) -> AppResult<ProjectConfig> {
    let mut config = config_service::load(&app)?;
    let project = config
        .projects
        .iter_mut()
        .find(|project| project.id == project_id)
        .ok_or_else(|| {
            AppError::new(
                "PROJECT_NOT_FOUND",
                "The selected project no longer exists.",
            )
        })?;
    let repo = git_service::canonical_repository(Path::new(&project.repo_path))?;
    git_service::validate_diff_selection(&repo, &selection)?;
    project.comparison = selection;
    let result = project.clone();
    config_service::save(&app, &config)?;
    Ok(result)
}

#[tauri::command]
pub fn remove_project<R: Runtime>(app: AppHandle<R>, project_id: String) -> AppResult<()> {
    let mut config = config_service::load(&app)?;
    config.projects.retain(|project| project.id != project_id);
    if config.active_project_id.as_deref() == Some(&project_id) {
        config.active_project_id = None;
    }
    config_service::save(&app, &config)
}
