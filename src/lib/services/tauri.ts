import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { DiffExplanation } from '$lib/domain/ai';
import type { DiffSummary, FileDiff } from '$lib/domain/diff';
import type { ProjectConfig, RepositoryInfo, SaveProjectInput } from '$lib/domain/project';

const nativeApi = {
  async selectRepository(): Promise<string | null> {
    const result = await open({ directory: true, multiple: false, title: 'Open Git repository' });
    return typeof result === 'string' ? result : null;
  },
  validateRepository: (path: string) => invoke<RepositoryInfo>('validate_repository', { path }),
  listProjects: () => invoke<ProjectConfig[]>('list_projects'),
  saveProject: (input: SaveProjectInput) => invoke<ProjectConfig>('save_project', { input }),
  touchProject: (projectId: string) => invoke<ProjectConfig>('touch_project', { projectId }),
  removeProject: (projectId: string) => invoke<void>('remove_project', { projectId }),
  getDiffSummary: (projectId: string) => invoke<DiffSummary>('get_diff_summary', { projectId }),
  getFileDiff: (projectId: string, path: string) =>
    invoke<FileDiff>('get_file_diff', { projectId, path }),
  getFileDiffs: (projectId: string, paths: string[]) =>
    invoke<FileDiff[]>('get_file_diffs', { projectId, paths }),
  explainFileDiff: (projectId: string, path: string) =>
    invoke<DiffExplanation>('explain_file_diff', { projectId, path })
};

const demoProject: ProjectConfig = {
  id: 'readiff-browser-demo',
  name: 'readiff-demo',
  repoPath: '/Users/example/src/readiff-demo',
  baseRef: 'main',
  lastOpenedAt: new Date().toISOString()
};

let mockProjects = [demoProject];
const mockContents: Record<string, string> = {
  'src/services/review.ts': `import { buildContext } from '../lib/context';

export type Review = {
  summary: string;
  evidence: string[];
};

export async function review(diff: string): Promise<Review> {
  const context = await buildContext(diff);
  return { summary: context.summary, evidence: context.references };
}
`,
  'src/lib/context.ts': `export async function buildContext(diff: string) {
  const lines = diff.split('\\n');
  return {
    summary: \`Reviewing \${lines.length} diff lines\`,
    references: lines.filter((line) => line.startsWith('+'))
  };
}
`
};

const mockSummary: DiffSummary = {
  baseRef: 'main',
  headSha: '8f31dc290c98f3ebd149ebf4e9cdb594d7356cb7',
  mergeBaseSha: '2cde30b476ecb745ea84ee1fcbd13a51ea4f3a13',
  totalAdditions: 8,
  totalDeletions: 2,
  files: [
    {
      oldPath: 'src/services/review.ts',
      newPath: 'src/services/review.ts',
      status: 'modified',
      additions: 6,
      deletions: 2
    },
    {
      newPath: 'src/lib/context.ts',
      status: 'added',
      additions: 2,
      deletions: 0
    }
  ]
};

function mockDiff(path: string): FileDiff {
  if (path === 'src/lib/context.ts') {
    const content = mockContents[path];
    return {
      file: mockSummary.files[1],
      newContent: content,
      hunks: [
        `@@ -0,0 +1,8 @@\n+export async function buildContext(diff: string) {\n+  const lines = diff.split('\\n');\n+  return {\n+    summary: \`Reviewing \${lines.length} diff lines\`,\n+    references: lines.filter((line) => line.startsWith('+'))\n+  };\n+}\n+`
      ],
      unifiedDiff: `diff --git a/${path} b/${path}\nnew file mode 100644\n--- /dev/null\n+++ b/${path}\n@@ -0,0 +1,8 @@\n+export async function buildContext(diff: string) {\n+  const lines = diff.split('\\n');\n+  return {\n+    summary: \`Reviewing \${lines.length} diff lines\`,\n+    references: lines.filter((line) => line.startsWith('+'))\n+  };\n+}\n+`,
      truncated: false
    };
  }
  const oldContent = `export async function review(diff: string) {
  return summarize(diff);
}
`;
  const newContent = mockContents['src/services/review.ts'];
  return {
    file: mockSummary.files[0],
    oldContent,
    newContent,
    hunks: [
      `@@ -1,3 +1,12 @@\n+import { buildContext } from '../lib/context';\n+\n+export type Review = {\n+  summary: string;\n+  evidence: string[];\n+};\n+\n export async function review(diff: string) {\n-  return summarize(diff);\n+  const context = await buildContext(diff);\n+  return { summary: context.summary, evidence: context.references };\n }\n`
    ],
    unifiedDiff: `diff --git a/src/services/review.ts b/src/services/review.ts\n--- a/src/services/review.ts\n+++ b/src/services/review.ts\n@@ -1,3 +1,12 @@\n+import { buildContext } from '../lib/context';\n+\n+export type Review = {\n+  summary: string;\n+  evidence: string[];\n+};\n+\n export async function review(diff: string) {\n-  return summarize(diff);\n+  const context = await buildContext(diff);\n+  return { summary: context.summary, evidence: context.references };\n }\n`,
    truncated: false
  };
}

const mockApi: typeof nativeApi = {
  selectRepository: async () => '/Users/example/src/new-repository',
  validateRepository: async (path) => ({
    repoPath: path,
    suggestedName: path.split('/').at(-1) ?? 'repository',
    detectedBaseRef: 'main',
    currentBranch: 'feature/readiff',
    localBranches: ['feature/readiff', 'main']
  }),
  listProjects: async () => mockProjects,
  saveProject: async (input) => {
    const project: ProjectConfig = {
      id: input.id ?? `browser-demo-${mockProjects.length + 1}`,
      name: input.name,
      repoPath: input.repoPath,
      baseRef: input.baseRef,
      lastOpenedAt: new Date().toISOString()
    };
    mockProjects = [project, ...mockProjects.filter((item) => item.id !== project.id)];
    return project;
  },
  touchProject: async (projectId) => {
    const project = mockProjects.find((item) => item.id === projectId) ?? demoProject;
    return { ...project, lastOpenedAt: new Date().toISOString() };
  },
  removeProject: async (projectId) => {
    mockProjects = mockProjects.filter((item) => item.id !== projectId);
  },
  getDiffSummary: async () => mockSummary,
  getFileDiff: async (_projectId, path) => mockDiff(path),
  getFileDiffs: async (_projectId, paths) => paths.map(mockDiff),
  explainFileDiff: async (_projectId, path) => ({
    summary:
      'The change introduces a context-building step before producing a structured review result.',
    inferredIntent:
      'The likely intent is to make review explanations traceable to concrete diff evidence.',
    risk: 'medium',
    concerns: ['Callers may need to handle failures from the new asynchronous context step.'],
    references: [{ path, startLine: 8, endLine: 10, side: 'new' }],
    caveats: ['Intent is inferred from the diff and has not been confirmed by the author.']
  })
};

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export const tauriApi = isTauri ? nativeApi : mockApi;
