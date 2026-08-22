import { tick } from "svelte";
import { SvelteSet, SvelteURLSearchParams } from "svelte/reactivity";
import {
  defaultDiffSelection,
  diffAnchorId,
  displayPath,
  sortDiffFilesByTreeOrder,
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
  selectedPath = $state<string>();
  diffs = $state<Record<string, FileDiff | undefined>>({});
  loadingPaths = $state<Record<string, boolean | undefined>>({});
  errors = $state<Record<string, string | undefined>>({});
  reviewAvailability = $state<ChangeReviewAvailability>();
  loading = $state(false);

  private projectId?: string;
  private pendingPaths = new SvelteSet<string>();
  private batchTimer: ReturnType<typeof setTimeout> | undefined;
  private generation = 0;
  private loadGeneration = 0;

  constructor(
    private readonly api: Pick<AppApi, "getDiffWorkspace" | "getFileDiffs">,
    private readonly onError: (error: unknown) => void,
    private readonly onResetAi: () => void,
  ) {}

  activate(
    projectId: string,
    selection: DiffSelection = defaultDiffSelection(),
  ) {
    this.projectId = projectId;
    this.selection = { ...selection };
  }

  async load(requestedFile?: string, options: { silent?: boolean } = {}) {
    if (!this.projectId) return;
    const projectId = this.projectId;
    const selection = { ...this.selection };
    const loadGeneration = ++this.loadGeneration;
    const silent = options.silent ?? false;
    this.onError(null);
    if (!silent) {
      this.summary = null;
      this.loading = true;
      this.clearDiffs();
    }

    try {
      const { summary: loadedSummary, reviewAvailability } =
        await this.api.getDiffWorkspace(projectId, selection);
      if (!this.isCurrentLoad(projectId, selection, loadGeneration)) return;

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
          orderedSummary,
          path,
          loadGeneration,
        );
      if (!this.isCurrentLoad(projectId, selection, loadGeneration)) return;

      this.summary = orderedSummary;
      if (!silent) this.loading = false;
      this.selectedPath = path;
      this.syncUrl(path);
      if (path) {
        this.queue(path);
        await tick();
        if (!silent && this.isCurrentLoad(projectId, selection, loadGeneration))
          this.scrollTo(path, false);
      }
    } catch (error) {
      if (this.isCurrentLoad(projectId, selection, loadGeneration))
        this.onError(error);
    } finally {
      if (!silent && this.isCurrentLoad(projectId, selection, loadGeneration))
        this.loading = false;
    }
  }

  async applySelection(selection: DiffSelection) {
    this.selection = { ...selection };
    this.selectedPath = undefined;
    await this.load();
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
    this.selectedPath = undefined;
    this.clearPending();
    this.diffs = {};
    this.loadingPaths = {};
    this.errors = {};
    this.reviewAvailability = undefined;
    this.loadGeneration += 1;
    this.onResetAi();
  }

  private async refreshLoadedDiffs(
    projectId: string,
    selection: DiffSelection,
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
        ? await this.api.getFileDiffs(projectId, selection, paths)
        : [];
    if (!this.isCurrentLoad(projectId, selection, loadGeneration)) return;

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
    const selection = { ...this.selection };
    this.pendingPaths.clear();

    try {
      const loadedDiffs = await this.api.getFileDiffs(
        projectId,
        selection,
        paths,
      );
      if (
        !this.isCurrent(projectId, selection) ||
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
        this.isCurrent(projectId, selection) &&
        this.generation === generation
      ) {
        const message = normalizeError(error).message;
        for (const path of paths) this.errors[path] = message;
      }
    } finally {
      if (
        this.isCurrent(projectId, selection) &&
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

  private isCurrent(projectId: string, selection: DiffSelection) {
    return (
      this.projectId === projectId &&
      this.selection.base === selection.base &&
      this.selection.target === selection.target
    );
  }

  private isCurrentLoad(
    projectId: string,
    selection: DiffSelection,
    loadGeneration: number,
  ) {
    return (
      this.loadGeneration === loadGeneration &&
      this.isCurrent(projectId, selection)
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
