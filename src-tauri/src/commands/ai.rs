use crate::{
    domain::DiffExplanation,
    error::{AppError, AppResult},
    services::{
        ai::{AiProvider, CodexProvider},
        config_service, git_service,
    },
};
use std::path::Path;
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub async fn explain_file_diff<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    path: String,
) -> AppResult<DiffExplanation> {
    let project = config_service::find_project(&app, &project_id)?;
    let repo = Path::new(&project.repo_path);
    let summary = git_service::diff_summary(repo, &project.base_ref)?;
    let diff = git_service::file_diff(repo, &project.base_ref, &path)?;
    if diff.truncated {
        return Err(AppError::new(
            "AI_INPUT_TOO_LARGE",
            "This diff is too large for the initial AI analysis flow.",
        ));
    }
    CodexProvider::new()
        .explain_file_diff(&project.base_ref, &summary.head_sha, &diff)
        .await
}
