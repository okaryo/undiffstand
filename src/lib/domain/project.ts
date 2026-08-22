import type { DiffSelection } from "$lib/domain/diff";

export type ProjectConfig = {
  id: string;
  name: string;
  repoPath: string;
  baseRef: string;
  comparison: DiffSelection;
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
  recentBranches: string[];
  localBranches: string[];
  remoteBranches: string[];
  recentCommits: GitCommitSummary[];
};

export type GitCommitSummary = {
  sha: string;
  shortSha: string;
  subject: string;
};
