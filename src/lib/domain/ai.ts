export type SourceReference = {
  path: string;
  startLine: number;
  endLine: number;
  side?: 'old' | 'new';
};

export type AiAnswer = {
  answer: string;
  references: SourceReference[];
  caveats: string[];
};

export type DiffExplanation = {
  summary: string;
  inferredIntent: string;
  risk: 'low' | 'medium' | 'high';
  concerns: string[];
  references: SourceReference[];
  caveats: string[];
};
