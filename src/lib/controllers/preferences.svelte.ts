import type { UserPreferences } from "$lib/domain/preferences";
import type { AppApi } from "$lib/services/api";

export class PreferencesController {
  sidebarOpen = $state(true);
  sidebarWidth = $state(225);
  aiPanelOpen = $state(true);
  aiPanelWidth = $state(290);
  diffMode = $state<"split" | "unified">("split");
  wrapLines = $state(false);
  reviewOutputLanguage = $state<"english" | "japanese">("english");

  private loaded = false;
  private saving = false;
  private saveRequested = false;

  constructor(
    private readonly api: Pick<AppApi, "saveUserPreferences">,
    private readonly onError: (error: unknown) => void,
  ) {}

  apply(preferences: UserPreferences) {
    const detail = preferences.changeDetail;
    this.sidebarOpen = detail.changedFilesPanel.open;
    this.sidebarWidth = detail.changedFilesPanel.width;
    this.aiPanelOpen = detail.aiPanel.open;
    this.aiPanelWidth = detail.aiPanel.width;
    this.diffMode = detail.diff.mode;
    this.wrapLines = detail.diff.wrapLongLines;
    this.reviewOutputLanguage = preferences.ai?.outputLanguage ?? "english";
    this.loaded = true;
  }

  snapshot(): UserPreferences {
    return {
      ai: { outputLanguage: this.reviewOutputLanguage },
      changeDetail: {
        changedFilesPanel: { open: this.sidebarOpen, width: this.sidebarWidth },
        aiPanel: { open: this.aiPanelOpen, width: this.aiPanelWidth },
        diff: { mode: this.diffMode, wrapLongLines: this.wrapLines },
      },
    };
  }

  queueSave() {
    if (!this.loaded) return;
    this.saveRequested = true;
    if (!this.saving) void this.flushSave();
  }

  private async flushSave() {
    this.saving = true;
    while (this.saveRequested) {
      this.saveRequested = false;
      try {
        await this.api.saveUserPreferences(this.snapshot());
      } catch (error) {
        this.onError(error);
        this.saveRequested = false;
      }
    }
    this.saving = false;
  }
}
