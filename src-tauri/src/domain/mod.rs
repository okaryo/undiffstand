use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    pub id: String,
    pub name: String,
    pub repo_path: String,
    pub base_ref: String,
    pub last_opened_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub schema_version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_project_id: Option<String>,
    pub projects: Vec<ProjectConfig>,
    #[serde(default)]
    pub preferences: UserPreferences,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            active_project_id: None,
            projects: Vec::new(),
            preferences: UserPreferences::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserPreferences {
    #[serde(default)]
    pub change_detail: ChangeDetailPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeDetailPreferences {
    #[serde(default)]
    pub changed_files_panel: ChangedFilesPanelPreferences,
    #[serde(default)]
    pub ai_panel: AiPanelPreferences,
    #[serde(default)]
    pub diff: DiffPreferences,
}

impl Default for ChangeDetailPreferences {
    fn default() -> Self {
        Self {
            changed_files_panel: ChangedFilesPanelPreferences::default(),
            ai_panel: AiPanelPreferences::default(),
            diff: DiffPreferences::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFilesPanelPreferences {
    #[serde(default = "default_panel_open")]
    pub open: bool,
    #[serde(default = "default_changed_files_panel_width")]
    pub width: u16,
}

impl Default for ChangedFilesPanelPreferences {
    fn default() -> Self {
        Self {
            open: true,
            width: default_changed_files_panel_width(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiPanelPreferences {
    #[serde(default = "default_panel_open")]
    pub open: bool,
    #[serde(default = "default_ai_panel_width")]
    pub width: u16,
}

impl Default for AiPanelPreferences {
    fn default() -> Self {
        Self {
            open: true,
            width: default_ai_panel_width(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffPreferences {
    #[serde(default)]
    pub mode: DiffViewMode,
    #[serde(default)]
    pub wrap_long_lines: bool,
}

impl Default for DiffPreferences {
    fn default() -> Self {
        Self {
            mode: DiffViewMode::Split,
            wrap_long_lines: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DiffViewMode {
    #[default]
    Split,
    Unified,
}

fn default_panel_open() -> bool {
    true
}

fn default_changed_files_panel_width() -> u16 {
    225
}

fn default_ai_panel_width() -> u16 {
    290
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectInput {
    pub id: Option<String>,
    pub name: String,
    pub repo_path: String,
    pub base_ref: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInfo {
    pub repo_path: String,
    pub suggested_name: String,
    pub detected_base_ref: Option<String>,
    pub current_branch: Option<String>,
    pub local_branches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Binary,
    Submodule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffFileSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_path: Option<String>,
    pub status: DiffStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additions: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletions: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSummary {
    pub base_ref: String,
    pub head_sha: String,
    pub merge_base_sha: String,
    pub files: Vec<DiffFileSummary>,
    pub total_additions: u64,
    pub total_deletions: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub file: DiffFileSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_content: Option<String>,
    pub hunks: Vec<String>,
    pub unified_diff: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReference {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffExplanation {
    pub summary: String,
    pub inferred_intent: String,
    pub risk: String,
    pub concerns: Vec<String>,
    pub references: Vec<SourceReference>,
    pub caveats: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn app_config_without_preferences_uses_change_detail_defaults() {
        let config: AppConfig = serde_json::from_value(json!({
            "schemaVersion": 1,
            "projects": []
        }))
        .expect("legacy app config should deserialize");

        assert_eq!(config.schema_version, 1);
        assert_eq!(config.preferences, UserPreferences::default());
    }

    #[test]
    fn partial_preferences_use_field_defaults() {
        let preferences: UserPreferences = serde_json::from_value(json!({
            "changeDetail": {
                "changedFilesPanel": { "width": 300 },
                "diff": { "mode": "unified" }
            }
        }))
        .expect("partial preferences should deserialize");

        assert!(preferences.change_detail.changed_files_panel.open);
        assert_eq!(preferences.change_detail.changed_files_panel.width, 300);
        assert_eq!(
            preferences.change_detail.ai_panel,
            AiPanelPreferences::default()
        );
        assert_eq!(preferences.change_detail.diff.mode, DiffViewMode::Unified);
        assert!(!preferences.change_detail.diff.wrap_long_lines);
    }
}
