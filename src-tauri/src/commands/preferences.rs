use crate::{domain::UserPreferences, error::AppResult, services::config_service};
use tauri::{AppHandle, Runtime};

const CHANGED_FILES_PANEL_MIN_WIDTH: u16 = 160;
const CHANGED_FILES_PANEL_MAX_WIDTH: u16 = 420;
const AI_PANEL_MIN_WIDTH: u16 = 240;
const AI_PANEL_MAX_WIDTH: u16 = 520;

#[tauri::command]
pub fn get_user_preferences<R: Runtime>(app: AppHandle<R>) -> AppResult<UserPreferences> {
    Ok(config_service::load(&app)?.preferences)
}

#[tauri::command]
pub fn save_user_preferences<R: Runtime>(
    app: AppHandle<R>,
    mut preferences: UserPreferences,
) -> AppResult<UserPreferences> {
    normalize(&mut preferences);
    let mut config = config_service::load(&app)?;
    config.preferences = preferences.clone();
    config_service::save(&app, &config)?;
    Ok(preferences)
}

fn normalize(preferences: &mut UserPreferences) {
    let detail = &mut preferences.change_detail;
    detail.changed_files_panel.width = detail
        .changed_files_panel
        .width
        .clamp(CHANGED_FILES_PANEL_MIN_WIDTH, CHANGED_FILES_PANEL_MAX_WIDTH);
    detail.ai_panel.width = detail
        .ai_panel
        .width
        .clamp(AI_PANEL_MIN_WIDTH, AI_PANEL_MAX_WIDTH);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_panel_widths_to_supported_ranges() {
        let mut preferences = UserPreferences::default();
        preferences.change_detail.changed_files_panel.width = 10;
        preferences.change_detail.ai_panel.width = 999;

        normalize(&mut preferences);

        assert_eq!(preferences.change_detail.changed_files_panel.width, 160);
        assert_eq!(preferences.change_detail.ai_panel.width, 520);
    }
}
