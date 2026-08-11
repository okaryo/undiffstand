export type DiffViewMode = 'split' | 'unified';
export type ReviewOutputLanguage = 'english' | 'japanese';

export const reviewOutputLanguageOptions = [
  { value: 'english', label: 'English' },
  { value: 'japanese', label: '日本語' }
] as const;

export type UserPreferences = {
  ai?: {
    outputLanguage: ReviewOutputLanguage;
  };
  changeDetail: {
    changedFilesPanel: {
      open: boolean;
      width: number;
    };
    aiPanel: {
      open: boolean;
      width: number;
    };
    diff: {
      mode: DiffViewMode;
      wrapLongLines: boolean;
    };
  };
};

export function defaultUserPreferences(): UserPreferences {
  return {
    ai: { outputLanguage: 'english' },
    changeDetail: {
      changedFilesPanel: { open: true, width: 225 },
      aiPanel: { open: true, width: 290 },
      diff: { mode: 'split', wrapLongLines: false }
    }
  };
}
