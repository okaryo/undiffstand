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
