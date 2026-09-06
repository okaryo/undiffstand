import { tick } from "svelte";
import { SvelteSet, SvelteURLSearchParams } from "svelte/reactivity";
import {
  defaultDiffSelection,
  defaultDiffScope,
  diffAnchorId,
  displayPath,
  sameDiffScope,
  sortDiffFilesByTreeOrder,
  type ComparisonCommit,
  type DiffScope,
  type DiffSelection,
  type DiffSummary,
  type FileDiff,
} from "$lib/domain/diff";
import type { ChangeReviewAvailability } from "$lib/domain/ai";
import { normalizeError } from "$lib/domain/error";
import type { AppApi } from "$lib/services/api";

export class DiffWorkspaceController {
  summary = $state<DiffSummary | null>(null);
  selection = $state<DiffSelection>(defaultDiffSelection());
  scope = $state<DiffScope>(defaultDiffScope());
  commits = $state<ComparisonCommit[]>([]);
  commitsLoading = $state(false);
  selectedPath = $state<string>();
  diffs = $state<Record<string, FileDiff | undefined>>({});
  loadingPaths = $state<Record<string, boolean | undefined>>({});
  errors = $state<Record<string, string | undefined>>({});
  reviewedPaths = new SvelteSet<string>();
  reviewAvailability = $state<ChangeReviewAvailability>();
  loading = $state(false);

  private projectId?: string;
  private pendingPaths = new SvelteSet<string>();
  private batchTimer: ReturnType<typeof setTimeout> | undefined;
  private generation = 0;
  private loadGeneration = 0;
  private commitLoadGeneration = 0;

  constructor(
    private readonly api: Pick<
      AppApi,
      "getDiffWorkspace" | "getComparisonCommits" | "getFileDiffs"
    >,
    private readonly onError: (error: unknown) => void,
    private readonly onResetAi: () => void,
  ) {}

  get activeSelection(): DiffSelection {
    return this.summary?.selection ?? this.selection;
  }

  activate(
    projectId: string,
    selection: DiffSelection = defaultDiffSelection(),
  ) {
    this.projectId = projectId;
    this.selection = { ...selection };
    this.scope = defaultDiffScope();
    this.reviewedPaths.clear();
    this.clearCommits();
  }

  async load(
    requestedFile?: string,
    options: { silent?: boolean; refreshCommits?: boolean } = {},
  ) {
    if (!this.projectId) return;
    const projectId = this.projectId;
    const selection = { ...this.selection };
    const scope = cloneScope(this.scope);
    const loadGeneration = ++this.loadGeneration;
    const silent = options.silent ?? false;
    const refreshCommits = options.refreshCommits ?? true;
    this.onError(null);
    if (!silent) {
      this.summary = null;
      this.loading = true;
      this.clearDiffs();
    }

    try {
      const { summary: loadedSummary, reviewAvailability } =
        await this.api.getDiffWorkspace(projectId, selection, scope);
      if (!this.isCurrentLoad(projectId, selection, scope, loadGeneration))
        return;

      const orderedSummary = {
        ...loadedSummary,
        files: sortDiffFilesByTreeOrder(loadedSummary.files),
      };
      this.reviewAvailability = reviewAvailability;
      const path =
        requestedFile &&
        orderedSummary.files.some((file) => displayPath(file) === requestedFile)
          ? requestedFile
          : orderedSummary.files[0]
            ? displayPath(orderedSummary.files[0])
            : undefined;

      if (silent)
        await this.refreshLoadedDiffs(
          projectId,
          selection,
          scope,
          loadedSummary.selection,
          orderedSummary,
          path,
          loadGeneration,
        );
      if (!this.isCurrentLoad(projectId, selection, scope, loadGeneration))
        return;

      this.summary = orderedSummary;
      if (!silent) this.loading = false;
      this.selectedPath = path;
      this.syncUrl(path);
      if (refreshCommits) void this.loadCommits(projectId, selection);
      if (path) {
        this.queue(path);
        await tick();
        if (
          !silent &&
          this.isCurrentLoad(projectId, selection, scope, loadGeneration)
        )
          this.scrollTo(path, false);
      }
    } catch (error) {
      if (this.isCurrentLoad(projectId, selection, scope, loadGeneration))
        this.onError(error);
    } finally {
      if (
        !silent &&
        this.isCurrentLoad(projectId, selection, scope, loadGeneration)
      )
        this.loading = false;
    }
  }

  async applySelection(selection: DiffSelection) {
    const viewChanged =
      this.selection.base !== selection.base ||
      this.selection.target !== selection.target ||
      this.scope.kind !== "all";
    this.selection = { ...selection };
    this.scope = defaultDiffScope();
    if (viewChanged) this.reviewedPaths.clear();
    this.clearCommits();
    this.selectedPath = undefined;
    await this.load();
  }

  async applyScope(scope: DiffScope) {
    if (sameDiffScope(this.scope, scope)) return;
    this.scope = cloneScope(scope);
    this.reviewedPaths.clear();
    this.selectedPath = undefined;
    await this.load(undefined, { refreshCommits: false });
  }

  setReviewed(path: string, reviewed: boolean) {
    if (reviewed) this.reviewedPaths.add(path);
    else this.reviewedPaths.delete(path);
  }

  select(path: string) {
    if (!this.projectId) return;
    this.selectedPath = path;
    this.syncUrl(path);
    this.queue(path);
    this.scrollTo(path, true);
  }

  setActive(path: string) {
    if (this.selectedPath === path) return;
    this.selectedPath = path;
    this.syncUrl(path);
  }

  queue(path: string) {
    if (!this.projectId || this.diffs[path] || this.loadingPaths[path]) return;
    this.errors[path] = undefined;
    this.loadingPaths[path] = true;
    this.pendingPaths.add(path);
    this.batchTimer ??= setTimeout(() => void this.loadPending(), 16);
  }

  reset() {
    this.projectId = undefined;
    this.summary = null;
    this.selection = defaultDiffSelection();
    this.scope = defaultDiffScope();
    this.clearCommits();
    this.selectedPath = undefined;
    this.clearPending();
    this.diffs = {};
    this.loadingPaths = {};
    this.errors = {};
    this.reviewedPaths.clear();
    this.reviewAvailability = undefined;
    this.loadGeneration += 1;
    this.onResetAi();
  }

  private async loadCommits(projectId: string, selection: DiffSelection) {
    const commitLoadGeneration = ++this.commitLoadGeneration;
    this.commitsLoading = true;
    try {
      const commits = await this.api.getComparisonCommits(projectId, selection);
      if (!this.isCurrentCommitLoad(projectId, selection, commitLoadGeneration))
        return;
      this.commits = commits;
    } catch (error) {
      if (this.isCurrentCommitLoad(projectId, selection, commitLoadGeneration))
        this.onError(error);
    } finally {
      if (this.isCurrentCommitLoad(projectId, selection, commitLoadGeneration))
        this.commitsLoading = false;
    }
  }

  private clearCommits() {
    this.commits = [];
    this.commitsLoading = false;
    this.commitLoadGeneration += 1;
  }

  private async refreshLoadedDiffs(
    projectId: string,
    rootSelection: DiffSelection,
    scope: DiffScope,
    activeSelection: DiffSelection,
    summary: DiffSummary,
    selectedPath: string | undefined,
    loadGeneration: number,
  ) {
    const availablePaths = new SvelteSet(summary.files.map(displayPath));
    const refreshPaths = new SvelteSet([
      ...Object.keys(this.diffs),
      ...Object.keys(this.errors),
      ...Object.entries(this.loadingPaths)
        .filter(([, isLoading]) => isLoading)
        .map(([path]) => path),
    ]);
    if (selectedPath) refreshPaths.add(selectedPath);
    const paths = [...refreshPaths].filter((path) => availablePaths.has(path));
    const refreshedDiffs =
      paths.length > 0
        ? await this.api.getFileDiffs(projectId, activeSelection, paths)
        : [];
    if (!this.isCurrentLoad(projectId, rootSelection, scope, loadGeneration))
      return;

    this.clearPending();
    this.diffs = Object.fromEntries(
      refreshedDiffs.map((diff) => [displayPath(diff.file), diff]),
    );
    this.loadingPaths = {};
    this.errors = {};
  }

  private clearDiffs() {
    this.clearPending();
    this.diffs = {};
    this.loadingPaths = {};
    this.errors = {};
    this.reviewAvailability = undefined;
    this.onResetAi();
  }

  private async loadPending() {
    this.batchTimer = undefined;
    if (!this.projectId || this.pendingPaths.size === 0) return;
    const projectId = this.projectId;
    const generation = this.generation;
    const paths = [...this.pendingPaths];
    const selection = { ...this.activeSelection };
    this.pendingPaths.clear();

    try {
      const loadedDiffs = await this.api.getFileDiffs(
        projectId,
        selection,
        paths,
      );
      if (
        !this.isCurrentView(projectId, selection) ||
        this.generation !== generation
      )
        return;
      for (const diff of loadedDiffs) this.diffs[displayPath(diff.file)] = diff;
      if (this.selectedPath && paths.includes(this.selectedPath)) {
        await tick();
        if (this.generation === generation)
          this.scrollTo(this.selectedPath, false);
      }
    } catch (error) {
      if (
        this.isCurrentView(projectId, selection) &&
        this.generation === generation
      ) {
        const message = normalizeError(error).message;
        for (const path of paths) this.errors[path] = message;
      }
    } finally {
      if (
        this.isCurrentView(projectId, selection) &&
        this.generation === generation
      ) {
        for (const path of paths) this.loadingPaths[path] = false;
      }
    }
  }

  private clearPending() {
    if (this.batchTimer !== undefined) clearTimeout(this.batchTimer);
    this.batchTimer = undefined;
    this.pendingPaths.clear();
    this.generation += 1;
  }

  private isCurrentView(projectId: string, selection: DiffSelection) {
    const activeSelection = this.summary?.selection;
    return (
      this.projectId === projectId &&
      activeSelection?.base === selection.base &&
      activeSelection.target === selection.target
    );
  }

  private isCurrentLoad(
    projectId: string,
    selection: DiffSelection,
    scope: DiffScope,
    loadGeneration: number,
  ) {
    return (
      this.loadGeneration === loadGeneration &&
      this.projectId === projectId &&
      this.selection.base === selection.base &&
      this.selection.target === selection.target &&
      sameDiffScope(this.scope, scope)
    );
  }

  private isCurrentCommitLoad(
    projectId: string,
    selection: DiffSelection,
    commitLoadGeneration: number,
  ) {
    return (
      this.commitLoadGeneration === commitLoadGeneration &&
      this.projectId === projectId &&
      this.selection.base === selection.base &&
      this.selection.target === selection.target
    );
  }

  private syncUrl(file?: string) {
    if (!this.projectId) return;
    const params = new SvelteURLSearchParams({ project: this.projectId });
    if (file) params.set("file", file);
    if (this.selection.base !== "HEAD") params.set("base", this.selection.base);
    if (this.selection.target !== ".")
      params.set("target", this.selection.target);
    history.replaceState(null, "", `?${params.toString()}`);
  }

  private scrollTo(path: string, smooth: boolean) {
    document.getElementById(diffAnchorId(path))?.scrollIntoView({
      behavior: smooth ? "smooth" : "auto",
      block: "start",
    });
  }
}

function cloneScope(scope: DiffScope): DiffScope {
  return scope.kind === "commit" ? { ...scope } : { kind: scope.kind };
}
