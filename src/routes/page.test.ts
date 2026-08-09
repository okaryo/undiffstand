import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DiffSummary, FileDiff } from '$lib/domain/diff';
import type { ProjectConfig } from '$lib/domain/project';
import Page from './+page.svelte';

const tauriApi = vi.hoisted(() => ({
  listProjects: vi.fn(),
  touchProject: vi.fn(),
  getDiffSummary: vi.fn(),
  getFileDiffs: vi.fn(),
  validateRepository: vi.fn(),
  selectRepository: vi.fn(),
  saveProject: vi.fn(),
  removeProject: vi.fn(),
  explainFileDiff: vi.fn()
}));

vi.mock('$lib/services/tauri', () => ({ tauriApi }));

const project: ProjectConfig = {
  id: 'alpha',
  name: 'Alpha',
  repoPath: '/repos/alpha',
  baseRef: 'main',
  lastOpenedAt: '2026-08-08T10:00:00Z'
};

const summary: DiffSummary = {
  baseRef: 'main',
  headSha: '1234567890abcdef',
  mergeBaseSha: 'abcdef1234567890',
  totalAdditions: 2,
  totalDeletions: 1,
  files: [
    {
      oldPath: 'src/example.ts',
      newPath: 'src/example.ts',
      status: 'modified',
      additions: 2,
      deletions: 1
    }
  ]
};

const fileDiff: FileDiff = {
  file: summary.files[0],
  oldContent: 'const answer = 41;\n',
  newContent: 'const answer = 42;\n',
  hunks: ['@@ -1 +1 @@\n-const answer = 41;\n+const answer = 42;\n'],
  unifiedDiff:
    'diff --git a/src/example.ts b/src/example.ts\n--- a/src/example.ts\n+++ b/src/example.ts\n@@ -1 +1 @@\n-const answer = 41;\n+const answer = 42;\n',
  truncated: false
};

class IntersectionObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const originalScrollIntoView = Element.prototype.scrollIntoView;
const originalGetContext = HTMLCanvasElement.prototype.getContext;

describe('change details auto-refresh', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal('IntersectionObserver', IntersectionObserverStub);
    Element.prototype.scrollIntoView = vi.fn();
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
      font: '',
      measureText: () => ({ width: 0 })
    })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
    history.replaceState(null, '', '/');
    tauriApi.listProjects.mockResolvedValue([project]);
    tauriApi.touchProject.mockResolvedValue(project);
    tauriApi.getDiffSummary.mockResolvedValue(summary);
    tauriApi.getFileDiffs.mockResolvedValue([fileDiff]);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    Element.prototype.scrollIntoView = originalScrollIntoView;
    HTMLCanvasElement.prototype.getContext = originalGetContext;
  });

  it('reloads the open project when window focus returns', async () => {
    history.replaceState(null, '', '/?project=alpha&file=src%2Fexample.ts');
    render(Page);

    await waitFor(() => expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1));
    await fireEvent.focus(window);
    await waitFor(() => expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(2));

    expect(new URLSearchParams(location.search).get('file')).toBe('src/example.ts');
  });

  it('does not reload from the project list', async () => {
    render(Page);
    await waitFor(() => expect(tauriApi.listProjects).toHaveBeenCalledOnce());

    await fireEvent.focus(window);

    expect(tauriApi.getDiffSummary).not.toHaveBeenCalled();
  });

  it('keeps the rendered diff visible while an automatic refresh is pending', async () => {
    history.replaceState(null, '', '/?project=alpha');
    render(Page);
    await waitFor(() => expect(tauriApi.getFileDiffs).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.queryByText(/Loading diff/)).not.toBeInTheDocument());

    let finishRefresh: (diffs: FileDiff[]) => void = () => {};
    tauriApi.getFileDiffs.mockImplementationOnce(
      () =>
        new Promise<FileDiff[]>((resolve) => {
          finishRefresh = resolve;
        })
    );

    await fireEvent.focus(window);
    await waitFor(() => expect(tauriApi.getFileDiffs).toHaveBeenCalledTimes(2));

    expect(screen.queryByText(/Loading diff/)).not.toBeInTheDocument();
    finishRefresh([fileDiff]);
    await waitFor(() => expect(screen.queryByText(/Loading diff/)).not.toBeInTheDocument());
  });

  it('coalesces repeated focus events', async () => {
    history.replaceState(null, '', '/?project=alpha');
    render(Page);
    await waitFor(() => expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(1));

    await fireEvent.focus(window);
    await waitFor(() => expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(2));
    await fireEvent.focus(window);

    expect(tauriApi.getDiffSummary).toHaveBeenCalledTimes(2);
  });
});
