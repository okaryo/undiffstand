mod commands;
mod domain;
mod error;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::project::validate_repository,
            commands::project::list_projects,
            commands::project::save_project,
            commands::project::touch_project,
            commands::project::remove_project,
            commands::preferences::get_user_preferences,
            commands::preferences::save_user_preferences,
            commands::git::get_diff_summary,
            commands::git::get_file_diff,
            commands::git::get_file_diffs,
            commands::ai::explain_file_diff,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ReaDiff");
}
