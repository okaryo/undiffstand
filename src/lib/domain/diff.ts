export type DiffStatus =
  'added' | 'modified' | 'deleted' | 'renamed' | 'copied' | 'binary' | 'submodule';

export type DiffFileSummary = {
  oldPath?: string;
  newPath?: string;
  status: DiffStatus;
  additions?: number;
  deletions?: number;
};

export type DiffSelection = {
  base: string;
  target: string;
};

export type DiffComparison = {
  fromLabel: string;
  toLabel: string;
  fromSha?: string;
  toSha?: string;
};

export type DiffSummary = {
  selection: DiffSelection;
  comparison: DiffComparison;
  files: DiffFileSummary[];
  totalAdditions: number;
  totalDeletions: number;
};

export const defaultDiffSelection = (): DiffSelection => ({ base: 'HEAD', target: '.' });

export function diffSelectionLabel(selection: DiffSelection): string {
  const target = selection.target === '.' ? 'working tree' : selection.target;
  return `${selection.base} → ${target}`;
}

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

type DiffFileTreeNode = {
  directories: Map<string, DiffFileTreeNode>;
  files: DiffFileSummary[];
};

export function sortDiffFilesByTreeOrder(files: DiffFileSummary[]): DiffFileSummary[] {
  const root: DiffFileTreeNode = { directories: new Map(), files: [] };

  for (const file of files) {
    const parts = displayPath(file).split('/');
    parts.pop();
    let directory = root;

    for (const part of parts) {
      let child = directory.directories.get(part);
      if (!child) {
        child = { directories: new Map(), files: [] };
        directory.directories.set(part, child);
      }
      directory = child;
    }

    directory.files.push(file);
  }

  const sortedFiles: DiffFileSummary[] = [];

  function visit(directory: DiffFileTreeNode) {
    const directories = [...directory.directories.entries()].sort(([left], [right]) =>
      left.localeCompare(right)
    );
    for (const [, child] of directories) visit(child);

    sortedFiles.push(
      ...[...directory.files].sort((left, right) =>
        displayPath(left).localeCompare(displayPath(right))
      )
    );
  }

  visit(root);
  return sortedFiles;
}

export function diffAnchorId(path: string): string {
  return `diff-${encodeURIComponent(path)}`;
}
