export type ProjectConfig = {
  id: string;
  name: string;
  repoPath: string;
  baseRef: string;
  lastOpenedAt: string;
};

export type SaveProjectInput = {
  id?: string;
  name: string;
  repoPath: string;
  baseRef: string;
};

export type RepositoryInfo = {
  repoPath: string;
  suggestedName: string;
  detectedBaseRef: string | null;
  currentBranch: string | null;
  localBranches: string[];
};

export type RepoFile = { path: string };

export type FileContent = {
  path: string;
  content: string;
  language: string;
  lineCount: number;
};
