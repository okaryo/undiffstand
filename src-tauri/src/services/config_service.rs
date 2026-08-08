use crate::{
    domain::{AppConfig, ProjectConfig},
    error::{AppError, AppResult},
};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "projects.json";
const CONFIG_KEY: &str = "config";

pub fn load<R: Runtime>(app: &AppHandle<R>) -> AppResult<AppConfig> {
    let store = app.store(STORE_FILE).map_err(AppError::unknown)?;
    match store.get(CONFIG_KEY) {
        Some(value) => serde_json::from_value(value).map_err(AppError::unknown),
        None => Ok(AppConfig::default()),
    }
}

pub fn save<R: Runtime>(app: &AppHandle<R>, config: &AppConfig) -> AppResult<()> {
    let store = app.store(STORE_FILE).map_err(AppError::unknown)?;
    let value = serde_json::to_value(config).map_err(AppError::unknown)?;
    store.set(CONFIG_KEY, value);
    store.save().map_err(AppError::unknown)
}

pub fn find_project<R: Runtime>(app: &AppHandle<R>, project_id: &str) -> AppResult<ProjectConfig> {
    load(app)?
        .projects
        .into_iter()
        .find(|project| project.id == project_id)
        .ok_or_else(|| {
            AppError::new(
                "PROJECT_NOT_FOUND",
                "The selected project no longer exists.",
            )
        })
}
