export type DiffViewMode = 'split' | 'unified';

export type UserPreferences = {
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
    changeDetail: {
      changedFilesPanel: { open: true, width: 225 },
      aiPanel: { open: true, width: 290 },
      diff: { mode: 'split', wrapLongLines: false }
    }
  };
}
