export type SourceReference = {
  path: string;
  startLine: number;
  endLine: number;
  side?: "old" | "new";
};

export type DiffExplanation = {
  summary: string;
  inferredIntent: string;
  keyChanges: string[];
  references: SourceReference[];
  caveats: string[];
};

export type InlineQuestion = {
  path: string;
  side: "old" | "new";
  startLine: number;
  endLine: number;
  question: string;
};

export type InlineAnswer = {
  answer: string;
  references: SourceReference[];
  caveats: string[];
};

export type ChangeReviewTarget =
  { kind: "uncommitted" } | { kind: "base"; baseBranch: string };

export type ChangeReviewAvailability = {
  available: boolean;
  target?: ChangeReviewTarget;
  reason?: string;
  scopeLabel: string;
};

export type ChangeReviewGroup = {
  id: string;
  title: string;
  summary: string;
  files: string[];
  keyPoints: string[];
};

export type ChangeReviewFinding = {
  title: string;
  body: string;
  severity: "critical" | "high" | "medium" | "low";
  path: string;
  startLine: number;
  endLine: number;
  side?: "old" | "new";
};

export type ChangeReviewReport = {
  summary: string;
  inferredIntent: string;
  groups: ChangeReviewGroup[];
  findings: ChangeReviewFinding[];
  caveats: string[];
};
