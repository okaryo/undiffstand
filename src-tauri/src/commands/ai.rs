use crate::{
    domain::{
        ChangeReviewAvailability, ChangeReviewReport, DiffExplanation, DiffSelection, InlineAnswer,
        InlineQuestion, ReviewOutputLanguage,
    },
    error::{AppError, AppResult},
    services::{
        ai::{AiProvider, CodexProvider},
        config_service, git_service,
    },
};
use std::path::Path;
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub async fn explain_file_change<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: DiffSelection,
    path: String,
) -> AppResult<DiffExplanation> {
    let project = config_service::find_project(&app, &project_id)?;
    let language = output_language(&app)?;
    let repo = Path::new(&project.repo_path);
    let (summary, diff) = git_service::file_diff_with_summary(repo, &selection, &path)?;
    ensure_ai_ready_diff(diff.truncated)?;
    CodexProvider::new()
        .explain_file_change(
            &summary.comparison.from_label,
            &summary.comparison.to_label,
            &diff,
            language,
        )
        .await
}

#[tauri::command]
pub async fn ask_inline_question<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: DiffSelection,
    question: InlineQuestion,
) -> AppResult<InlineAnswer> {
    validate_inline_question(&question)?;
    let project = config_service::find_project(&app, &project_id)?;
    let language = output_language(&app)?;
    let repo = Path::new(&project.repo_path);
    let (summary, diff) = git_service::file_diff_with_summary(repo, &selection, &question.path)?;
    ensure_ai_ready_diff(diff.truncated)?;
    let content = match question.side.as_str() {
        "old" => diff.old_content.as_deref(),
        "new" => diff.new_content.as_deref(),
        _ => None,
    }
    .ok_or_else(|| {
        AppError::new(
            "INVALID_INLINE_SELECTION",
            "The selected side does not contain reviewable text.",
        )
    })?;
    let selected_source = selected_source(content, question.start_line, question.end_line)?;
    CodexProvider::new()
        .answer_inline_question(
            &summary.comparison.from_label,
            &summary.comparison.to_label,
            &diff,
            &question,
            &selected_source,
            language,
        )
        .await
}

#[tauri::command]
pub fn get_change_review_availability<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: DiffSelection,
) -> AppResult<ChangeReviewAvailability> {
    let project = config_service::find_project(&app, &project_id)?;
    git_service::change_review_availability(Path::new(&project.repo_path), &selection)
}

#[tauri::command]
pub async fn run_change_review<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    selection: DiffSelection,
) -> AppResult<ChangeReviewReport> {
    let project = config_service::find_project(&app, &project_id)?;
    let language = output_language(&app)?;
    let repo = Path::new(&project.repo_path);
    let availability = git_service::change_review_availability(repo, &selection)?;
    let target = availability.target.ok_or_else(|| {
        AppError::new(
            "CHANGE_REVIEW_UNAVAILABLE",
            availability
                .reason
                .unwrap_or_else(|| "Change Review cannot use the selected comparison.".to_owned()),
        )
    })?;
    CodexProvider::new()
        .review_changes(repo, &target, &availability.scope_label, language)
        .await
}

fn output_language<R: Runtime>(app: &AppHandle<R>) -> AppResult<ReviewOutputLanguage> {
    Ok(config_service::load(app)?.preferences.ai.output_language)
}

fn ensure_ai_ready_diff(truncated: bool) -> AppResult<()> {
    if truncated {
        Err(AppError::new(
            "AI_INPUT_TOO_LARGE",
            "This file diff is too large for inline AI analysis.",
        ))
    } else {
        Ok(())
    }
}

fn validate_inline_question(question: &InlineQuestion) -> AppResult<()> {
    if !matches!(question.side.as_str(), "old" | "new")
        || question.start_line == 0
        || question.end_line < question.start_line
    {
        return Err(AppError::new(
            "INVALID_INLINE_SELECTION",
            "Choose a valid changed-line range on one side of the diff.",
        ));
    }
    let length = question.question.trim().chars().count();
    if length == 0 || length > 4_000 {
        return Err(AppError::new(
            "INVALID_INLINE_QUESTION",
            "Enter a question between 1 and 4,000 characters.",
        ));
    }
    Ok(())
}

fn selected_source(content: &str, start_line: usize, end_line: usize) -> AppResult<String> {
    let lines: Vec<&str> = content.lines().collect();
    if start_line == 0 || end_line < start_line || end_line > lines.len() {
        return Err(AppError::new(
            "INVALID_INLINE_SELECTION",
            "The selected lines are outside the current file content.",
        ));
    }
    Ok(lines[start_line - 1..end_line]
        .iter()
        .enumerate()
        .map(|(index, line)| format!("{:>6} | {line}", start_line + index))
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_source_keeps_original_line_numbers() {
        let selected = selected_source("one\ntwo\nthree\n", 2, 3).unwrap();
        assert_eq!(selected, "     2 | two\n     3 | three");
    }
}
