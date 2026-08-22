mod commands;
mod domain;
mod error;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::git::DiffSnapshotCache::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::project::validate_repository,
            commands::project::list_projects,
            commands::project::save_project,
            commands::project::touch_project,
            commands::project::save_project_comparison,
            commands::project::remove_project,
            commands::preferences::get_user_preferences,
            commands::preferences::save_user_preferences,
            commands::git::get_diff_workspace,
            commands::git::get_file_diffs,
            commands::ai::explain_file_change,
            commands::ai::ask_inline_question,
            commands::ai::run_change_review,
        ])
        .run(tauri::generate_context!())
        .expect("error while running undiffstand");
}
