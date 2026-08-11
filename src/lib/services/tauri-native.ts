import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppApi } from "./api";

export const nativeApi: AppApi = {
  async selectRepository() {
    const result = await open({
      directory: true,
      multiple: false,
      title: "Open Git repository",
    });
    return typeof result === "string" ? result : null;
  },
  validateRepository: (path) => invoke("validate_repository", { path }),
  listProjects: () => invoke("list_projects"),
  saveProject: (input) => invoke("save_project", { input }),
  touchProject: (projectId) => invoke("touch_project", { projectId }),
  removeProject: (projectId) => invoke("remove_project", { projectId }),
  getUserPreferences: () => invoke("get_user_preferences"),
  saveUserPreferences: (preferences) =>
    invoke("save_user_preferences", { preferences }),
  getDiffSummary: (projectId, selection) =>
    invoke("get_diff_summary", { projectId, selection }),
  getFileDiffs: (projectId, selection, paths) =>
    invoke("get_file_diffs", { projectId, selection, paths }),
  explainFileChange: (projectId, selection, path) =>
    invoke("explain_file_change", { projectId, selection, path }),
  askInlineQuestion: (projectId, selection, question) =>
    invoke("ask_inline_question", { projectId, selection, question }),
  getChangeReviewAvailability: (projectId, selection) =>
    invoke("get_change_review_availability", { projectId, selection }),
  runChangeReview: (projectId, selection) =>
    invoke("run_change_review", { projectId, selection }),
};
