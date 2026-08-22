import type { ChangeReviewAvailability } from "$lib/domain/ai";
import type { DiffSelection, DiffSummary, FileDiff } from "$lib/domain/diff";
import { defaultUserPreferences } from "$lib/domain/preferences";
import type { ProjectConfig } from "$lib/domain/project";
import type { AppApi } from "./api";

const demoProject: ProjectConfig = {
  id: "undiffstand-browser-demo",
  name: "undiffstand-demo",
  repoPath: "/Users/example/src/undiffstand-demo",
  baseRef: "main",
  comparison: { base: "HEAD", target: "." },
  lastOpenedAt: new Date().toISOString(),
};

let projects = [demoProject];
let userPreferences = defaultUserPreferences();
const contents: Record<string, string> = {
  "src/services/review.ts": `import { buildContext } from '../lib/context';

export type Review = {
  summary: string;
  evidence: string[];
};

export async function review(diff: string): Promise<Review> {
  const context = await buildContext(diff);
  return { summary: context.summary, evidence: context.references };
}
`,
  "src/lib/context.ts": `export async function buildContext(diff: string) {
  const lines = diff.split('\\n');
  return {
    summary: \`Reviewing \${lines.length} diff lines\`,
    references: lines.filter((line) => line.startsWith('+'))
  };
}
`,
};

const summary: DiffSummary = {
  selection: { base: "HEAD", target: "." },
  comparison: {
    fromLabel: "HEAD",
    toLabel: "working tree",
    fromSha: "8f31dc290c98f3ebd149ebf4e9cdb594d7356cb7",
  },
  totalAdditions: 8,
  totalDeletions: 2,
  files: [
    {
      oldPath: "src/services/review.ts",
      newPath: "src/services/review.ts",
      status: "modified",
      additions: 6,
      deletions: 2,
    },
    {
      newPath: "src/lib/context.ts",
      status: "added",
      additions: 2,
      deletions: 0,
    },
  ],
};

function summaryFor(selection: DiffSelection): DiffSummary {
  return {
    ...summary,
    selection,
    comparison: {
      fromLabel: selection.base,
      toLabel: selection.target === "." ? "working tree" : selection.target,
      fromSha: summary.comparison.fromSha,
      toSha: selection.target === "." ? undefined : summary.comparison.fromSha,
    },
  };
}

function reviewAvailabilityFor(
  selection: DiffSelection,
): ChangeReviewAvailability {
  if (selection.base === "HEAD" && selection.target === ".") {
    return {
      available: true,
      target: { kind: "uncommitted" },
      scopeLabel: "feature/undiffstand → working tree",
    };
  }
  if (
    selection.base === "main" &&
    ["HEAD", "feature/undiffstand"].includes(selection.target)
  ) {
    return {
      available: true,
      target: { kind: "base", baseBranch: "main" },
      scopeLabel: `main → ${selection.target}`,
    };
  }
  return {
    available: false,
    reason: "Change Review requires the target to be the current branch.",
    scopeLabel: `${selection.base} → ${selection.target}`,
  };
}

function diffFor(path: string): FileDiff {
  if (path === "src/lib/context.ts") {
    const content = contents[path];
    return {
      file: summary.files[1],
      newContent: content,
      hunks: [
        `@@ -0,0 +1,8 @@\n+export async function buildContext(diff: string) {\n+  const lines = diff.split('\\n');\n+  return {\n+    summary: \`Reviewing \${lines.length} diff lines\`,\n+    references: lines.filter((line) => line.startsWith('+'))\n+  };\n+}\n+`,
      ],
      unifiedDiff: `diff --git a/${path} b/${path}\nnew file mode 100644\n--- /dev/null\n+++ b/${path}\n@@ -0,0 +1,8 @@\n+export async function buildContext(diff: string) {\n+  const lines = diff.split('\\n');\n+  return {\n+    summary: \`Reviewing \${lines.length} diff lines\`,\n+    references: lines.filter((line) => line.startsWith('+'))\n+  };\n+}\n+`,
      truncated: false,
    };
  }
  const oldContent = `export async function review(diff: string) {
  return summarize(diff);
}
`;
  const newContent = contents["src/services/review.ts"];
  return {
    file: summary.files[0],
    oldContent,
    newContent,
    hunks: [
      `@@ -1,3 +1,12 @@\n+import { buildContext } from '../lib/context';\n+\n+export type Review = {\n+  summary: string;\n+  evidence: string[];\n+};\n+\n export async function review(diff: string) {\n-  return summarize(diff);\n+  const context = await buildContext(diff);\n+  return { summary: context.summary, evidence: context.references };\n }\n`,
    ],
    unifiedDiff: `diff --git a/src/services/review.ts b/src/services/review.ts\n--- a/src/services/review.ts\n+++ b/src/services/review.ts\n@@ -1,3 +1,12 @@\n+import { buildContext } from '../lib/context';\n+\n+export type Review = {\n+  summary: string;\n+  evidence: string[];\n+};\n+\n export async function review(diff: string) {\n-  return summarize(diff);\n+  const context = await buildContext(diff);\n+  return { summary: context.summary, evidence: context.references };\n }\n`,
    truncated: false,
  };
}

export const demoApi: AppApi = {
  selectRepository: async () => "/Users/example/src/new-repository",
  validateRepository: async (path) => ({
    repoPath: path,
    suggestedName: path.split("/").at(-1) ?? "repository",
    detectedBaseRef: "main",
    currentBranch: "feature/undiffstand",
    recentBranches: ["main"],
    localBranches: ["feature/undiffstand", "main"],
    remoteBranches: ["origin/main"],
    recentCommits: [
      {
        sha: "8f31dc290c98f3ebd149ebf4e9cdb594d7356cb7",
        shortSha: "8f31dc2",
        subject: "Improve review context",
      },
    ],
  }),
  listProjects: async () => projects,
  saveProject: async (input) => {
    const project: ProjectConfig = {
      id: input.id ?? `browser-demo-${projects.length + 1}`,
      name: input.name,
      repoPath: input.repoPath,
      baseRef: input.baseRef,
      comparison:
        projects.find((item) => item.id === input.id)?.comparison ??
        ({ base: "HEAD", target: "." } as const),
      lastOpenedAt: new Date().toISOString(),
    };
    projects = [project, ...projects.filter((item) => item.id !== project.id)];
    return project;
  },
  touchProject: async (projectId, selection) => {
    const project =
      projects.find((item) => item.id === projectId) ?? demoProject;
    const updated = {
      ...project,
      comparison: selection ?? project.comparison,
      lastOpenedAt: new Date().toISOString(),
    };
    projects = projects.map((item) => (item.id === projectId ? updated : item));
    return updated;
  },
  saveProjectComparison: async (projectId, selection) => {
    const project = projects.find((item) => item.id === projectId);
    if (!project) throw new Error("The selected project no longer exists.");
    const updated = { ...project, comparison: { ...selection } };
    projects = projects.map((item) => (item.id === projectId ? updated : item));
    return updated;
  },
  removeProject: async (projectId) => {
    projects = projects.filter((item) => item.id !== projectId);
  },
  getUserPreferences: async () => structuredClone(userPreferences),
  saveUserPreferences: async (preferences) => {
    userPreferences = structuredClone(preferences);
    return structuredClone(userPreferences);
  },
  getDiffWorkspace: async (_projectId, selection) => ({
    summary: summaryFor(selection),
    reviewAvailability: reviewAvailabilityFor(selection),
  }),
  getFileDiffs: async (_projectId, _selection, paths) => paths.map(diffFor),
  explainFileChange: async (_projectId, _selection, path) => ({
    summary:
      "The change introduces a context-building step before producing a structured review result.",
    inferredIntent:
      "The likely intent is to make review explanations traceable to concrete diff evidence.",
    keyChanges: [
      "Builds review context before creating the result.",
      "Returns concrete evidence together with the summary.",
    ],
    references: [{ path, startLine: 8, endLine: 10, side: "new" }],
    caveats: [
      "Intent is inferred from the diff and has not been confirmed by the author.",
    ],
  }),
  askInlineQuestion: async (_projectId, _selection, question) => ({
    answer: `These lines change how review evidence is assembled before it is returned. The selected ${question.side}-side range is treated as part of the current comparison.`,
    references: [
      {
        path: question.path,
        startLine: question.startLine,
        endLine: question.endLine,
        side: question.side,
      },
    ],
    caveats: ["This answer is inferred from the displayed diff."],
  }),
  runChangeReview: async () => ({
    summary:
      "The change introduces evidence-aware review output and wires it into the existing review flow.",
    inferredIntent:
      "The likely intent is to make review results easier to verify against the changed code.",
    groups: [
      {
        id: "review-context",
        title: "Review context",
        summary: "Builds and returns structured evidence for a review.",
        files: ["src/services/review.ts", "src/lib/context.ts"],
        keyPoints: [
          "Context building is asynchronous.",
          "Evidence is now part of the result.",
        ],
      },
    ],
    findings: [
      {
        title: "Handle context failures",
        body: "The new asynchronous context step can fail, but no recovery behavior is visible here.",
        severity: "medium",
        path: "src/services/review.ts",
        startLine: 8,
        endLine: 9,
        side: "new",
      },
    ],
    caveats: ["Tests were not executed as part of this mock review."],
  }),
};
