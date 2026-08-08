use crate::{
    domain::{AiAnswer, CodeSelection, DiffExplanation},
    error::{AppError, AppResult},
    services::{
        ai::{AiProvider, OpenAiProvider},
        config_service, file_service, git_service,
    },
};
use std::path::Path;
use tauri::{AppHandle, Runtime};

fn numbered_context(content: &str, start_line: usize, end_line: usize) -> String {
    let first = start_line.saturating_sub(80).max(1);
    let last = end_line.saturating_add(80);
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_number = index + 1;
            (line_number >= first && line_number <= last)
                .then(|| format!("{line_number:>6} | {line}"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tauri::command]
pub async fn ask_about_code<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: CodeSelection,
    question: String,
) -> AppResult<AiAnswer> {
    if question.trim().is_empty() {
        return Err(AppError::new(
            "INVALID_QUESTION",
            "Enter a question about the selected code.",
        ));
    }
    if selection.text.chars().count() > 40_000 {
        return Err(AppError::new(
            "AI_INPUT_TOO_LARGE",
            "The selected code is too large. Select a smaller range.",
        ));
    }
    let project = config_service::find_project(&app, &project_id)?;
    let file = file_service::read_file(Path::new(&project.repo_path), &selection.path)?;
    let context = numbered_context(&file.content, selection.start_line, selection.end_line);
    OpenAiProvider::from_environment()?
        .ask_about_code(&selection, question.trim(), &context)
        .await
}

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
    OpenAiProvider::from_environment()?
        .explain_file_diff(&project.base_ref, &summary.head_sha, &diff)
        .await
}
