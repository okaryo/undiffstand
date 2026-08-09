export type DiffStatus =
  'added' | 'modified' | 'deleted' | 'renamed' | 'copied' | 'binary' | 'submodule';

export type DiffFileSummary = {
  oldPath?: string;
  newPath?: string;
  status: DiffStatus;
  additions?: number;
  deletions?: number;
};

export type DiffSummary = {
  baseRef: string;
  headSha: string;
  mergeBaseSha: string;
  files: DiffFileSummary[];
  totalAdditions: number;
  totalDeletions: number;
};

export type FileDiff = {
  file: DiffFileSummary;
  oldContent?: string;
  newContent?: string;
  hunks: string[];
  unifiedDiff: string;
  truncated: boolean;
};

export function displayPath(file: DiffFileSummary): string {
  return file.newPath ?? file.oldPath ?? 'Unknown file';
}

export function diffAnchorId(path: string): string {
  return `diff-${encodeURIComponent(path)}`;
}
