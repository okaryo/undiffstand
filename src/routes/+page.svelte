<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import {
    Columns2,
    GitBranch,
    LoaderCircle,
    PanelLeftClose,
    PanelLeftOpen,
    PanelRight,
    RefreshCw,
    Rows3,
    Settings,
    TextWrap,
  } from "@lucide/svelte";
  import AiPanel from "$lib/components/ai/AiPanel.svelte";
  import EmptyState from "$lib/components/common/EmptyState.svelte";
  import ErrorBanner from "$lib/components/common/ErrorBanner.svelte";
  import ResizeHandle from "$lib/components/common/ResizeHandle.svelte";
  import UpdateAction from "$lib/components/common/UpdateAction.svelte";
  import ComparisonDialog from "$lib/components/diff/ComparisonDialog.svelte";
  import DiffFeed from "$lib/components/diff/DiffFeed.svelte";
  import DiffFileList from "$lib/components/diff/DiffFileList.svelte";
  import DiffSearchBar from "$lib/components/diff/DiffSearchBar.svelte";
  import DiffSummaryView from "$lib/components/diff/DiffSummary.svelte";
  import ProjectDialog from "$lib/components/project/ProjectDialog.svelte";
  import ProjectHome from "$lib/components/project/ProjectHome.svelte";
  import ProjectSwitcher from "$lib/components/project/ProjectSwitcher.svelte";
  import AiSettingsDialog from "$lib/components/settings/AiSettingsDialog.svelte";
  import { AiReviewController } from "$lib/controllers/ai-review.svelte";
  import { DiffWorkspaceController } from "$lib/controllers/diff-workspace.svelte";
  import { PreferencesController } from "$lib/controllers/preferences.svelte";
  import type { InlineAnswer } from "$lib/domain/ai";
  import {
    defaultDiffSelection,
    diffComparisonLabel,
    diffSelectionLabel,
    revisionDisplayLabel,
    type DiffSelection,
  } from "$lib/domain/diff";
  import {
    findDiffSearchMatches,
    type DiffSearchMatch,
  } from "$lib/domain/diff-search";
  import { normalizeError, type AppError } from "$lib/domain/error";
  import type {
    DiffViewMode,
    ReviewOutputLanguage,
  } from "$lib/domain/preferences";
  import type {
    ProjectConfig,
    RepositoryInfo,
    SaveProjectInput,
  } from "$lib/domain/project";
  import type { UpdateState } from "$lib/domain/update";
  import { tauriApi } from "$lib/services/tauri";
  import { resolveApplicationShortcut } from "$lib/shortcuts";

  let projects = $state<ProjectConfig[]>([]);
  let activeProject = $state<ProjectConfig | null>(null);
  let repositoryDraft = $state<RepositoryInfo | undefined>();
  let activeRepository = $state<RepositoryInfo | undefined>();
  let editingProject = $state<ProjectConfig | undefined>();
  let showProjectDialog = $state(false);
  let showSettings = $state(false);
  let showComparisonDialog = $state(false);
  let loading = $state(true);
  let saving = $state(false);
  let deleting = $state(false);
  let error = $state<AppError | null>(null);
  let workspaceError = $state<AppError | null>(null);
  let updateState = $state<UpdateState>("unavailable");
  let availableUpdate = $state<Update | null>(null);
  let updateCheckInterval: ReturnType<typeof setInterval> | null = null;
  let unlistenUpdateFocusChange: (() => void) | null = null;
  let updateCheckInFlight = false;
  let lastUpdateCheckAttemptAt = 0;
  const preferences = new PreferencesController(tauriApi, (caught) => {
    workspaceError = normalizeError(caught);
  });
  const aiReview = new AiReviewController(tauriApi, (caught) => {
    workspaceError = caught === null ? null : normalizeError(caught);
  });
  const workspace = new DiffWorkspaceController(
    tauriApi,
    (caught) => {
      workspaceError = caught === null ? null : normalizeError(caught);
    },
    () => aiReview.reset(),
  );

  let aiPanelExpanded = $state(false);
  let diffSearchOpen = $state(false);
  let diffSearchQuery = $state("");
  let activeDiffSearchMatchId = $state<string>();
  let diffSearchFocusVersion = $state(0);
  const diffSearchMatches = $derived(
    workspace.summary
      ? findDiffSearchMatches(
          workspace.summary.files,
          workspace.diffs,
          diffSearchQuery,
        )
      : [],
  );
  const diffSearchMatchCounts = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const match of diffSearchMatches)
      counts[match.path] = (counts[match.path] ?? 0) + 1;
    return counts;
  });
  const activeDiffSearchIndex = $derived(
    activeDiffSearchMatchId
      ? diffSearchMatches.findIndex(
          (match) => match.id === activeDiffSearchMatchId,
        )
      : -1,
  );
  const activeDiffSearchMatch = $derived<DiffSearchMatch | undefined>(
    activeDiffSearchIndex >= 0
      ? diffSearchMatches[activeDiffSearchIndex]
      : undefined,
  );
  const diffSearchPending = $derived(
    Boolean(diffSearchQuery.trim()) &&
      Boolean(
        workspace.summary?.files.some((file) => {
          if (file.status === "binary") return false;
          const path = file.newPath ?? file.oldPath ?? "Unknown file";
          return !workspace.diffs[path] && !workspace.errors[path];
        }),
      ),
  );
  let aiPanelWidthBeforeExpand = 290;
  let activeBaseRef = $derived(
    activeProject?.baseRef && activeProject.baseRef !== "HEAD"
      ? activeProject.baseRef
      : activeRepository?.detectedBaseRef,
  );
  let baseToCurrentIsActive = $derived(
    Boolean(activeBaseRef) &&
      workspace.selection.base === activeBaseRef &&
      workspace.selection.target === "HEAD",
  );
  let currentToWorkingTreeIsActive = $derived(
    workspace.selection.base === "HEAD" && workspace.selection.target === ".",
  );
  let autoRefreshInProgress = false;
  let lastAutoRefreshAt = 0;
  const AUTO_REFRESH_COOLDOWN_MS = 1_000;
  const UPDATE_CHECK_COOLDOWN_MS = 24 * 60 * 60 * 1_000;
  const PANEL_HANDLE_WIDTH = 5;
  const CONTENT_MIN_WIDTH = 500;
  const SIDEBAR_MIN_WIDTH = 160;
  const SIDEBAR_MAX_WIDTH = 420;
  const AI_PANEL_MIN_WIDTH = 240;
  const AI_PANEL_MAX_WIDTH = 520;
  type ResizablePanel = "sidebar" | "ai";

  onMount(() => {
    if (window.innerWidth <= 1100) {
      preferences.sidebarWidth = 200;
      preferences.aiPanelWidth = 260;
    }
    window.addEventListener("focus", refreshWorkspaceOnFocus);
    startUpdateChecks();
    void initialize();

    return () => {
      window.removeEventListener("focus", refreshWorkspaceOnFocus);
      stopUpdateChecks();
    };
  });

  function startUpdateChecks() {
    if (!isTauriRuntime()) {
      updateState = "unavailable";
      return;
    }

    void checkForUpdates({ force: true });
    updateCheckInterval = setInterval(() => {
      void checkForUpdates();
    }, UPDATE_CHECK_COOLDOWN_MS);

    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) void checkForUpdates();
      })
      .then((unlisten) => {
        unlistenUpdateFocusChange = unlisten;
      })
      .catch((caught) => {
        console.warn("Update focus listener setup failed", caught);
      });
  }

  function stopUpdateChecks() {
    if (updateCheckInterval) {
      clearInterval(updateCheckInterval);
      updateCheckInterval = null;
    }

    unlistenUpdateFocusChange?.();
    unlistenUpdateFocusChange = null;
  }

  async function checkForUpdates(options: { force?: boolean } = {}) {
    if (!isTauriRuntime()) {
      updateState = "unavailable";
      return;
    }

    if (shouldSkipUpdateCheck(options.force ?? false)) return;

    updateCheckInFlight = true;
    lastUpdateCheckAttemptAt = Date.now();
    updateState = "checking";

    try {
      const update = await check();
      availableUpdate = update;
      updateState = update ? "available" : "idle";
    } catch (caught) {
      console.warn("Update check failed", caught);
      updateState = "error";
    } finally {
      updateCheckInFlight = false;
    }
  }

  function shouldSkipUpdateCheck(force: boolean) {
    if (
      updateCheckInFlight ||
      updateState === "available" ||
      updateState === "installing"
    )
      return true;

    return (
      !force && Date.now() - lastUpdateCheckAttemptAt < UPDATE_CHECK_COOLDOWN_MS
    );
  }

  async function installUpdate() {
    if (!availableUpdate) return;

    updateState = "installing";

    try {
      await availableUpdate.downloadAndInstall();
      await relaunch();
    } catch (caught) {
      console.warn("Update installation failed", caught);
      updateState = "error";
    }
  }

  function isTauriRuntime() {
    return "__TAURI_INTERNALS__" in window;
  }

  function panelWidthLimits(panel: ResizablePanel, workspaceWidth: number) {
    const min = panel === "sidebar" ? SIDEBAR_MIN_WIDTH : AI_PANEL_MIN_WIDTH;
    const configuredMax =
      panel === "sidebar" ? SIDEBAR_MAX_WIDTH : AI_PANEL_MAX_WIDTH;
    const otherPanelWidth =
      panel === "sidebar"
        ? preferences.aiPanelOpen
          ? preferences.aiPanelWidth
          : 0
        : preferences.sidebarOpen
          ? preferences.sidebarWidth
          : 0;
    const visibleHandleCount =
      Number(preferences.sidebarOpen) + Number(preferences.aiPanelOpen);
    const availableMax =
      workspaceWidth -
      otherPanelWidth -
      CONTENT_MIN_WIDTH -
      visibleHandleCount * PANEL_HANDLE_WIDTH;

    return { min, max: Math.max(min, Math.min(configuredMax, availableMax)) };
  }

  function constrainedPanelWidth(
    panel: ResizablePanel,
    width: number,
    workspaceWidth: number,
  ) {
    const { min, max } = panelWidthLimits(panel, workspaceWidth);
    return Math.round(Math.min(max, Math.max(min, width)));
  }

  function resetPanelWidth(panel: ResizablePanel) {
    if (panel === "sidebar")
      preferences.sidebarWidth = window.innerWidth <= 1100 ? 200 : 225;
    else preferences.aiPanelWidth = window.innerWidth <= 1100 ? 260 : 290;
    preferences.queueSave();
  }

  function toggleAiPanelExpanded() {
    if (aiPanelExpanded) {
      preferences.aiPanelWidth = aiPanelWidthBeforeExpand;
      aiPanelExpanded = false;
    } else {
      aiPanelWidthBeforeExpand = preferences.aiPanelWidth;
      preferences.aiPanelWidth = AI_PANEL_MAX_WIDTH;
      aiPanelExpanded = true;
    }
  }

  function toggleSidebar() {
    preferences.sidebarOpen = !preferences.sidebarOpen;
    preferences.queueSave();
  }

  function toggleAiPanel() {
    preferences.aiPanelOpen = !preferences.aiPanelOpen;
    preferences.queueSave();
  }

  function setDiffMode(mode: DiffViewMode) {
    if (preferences.diffMode === mode) return;
    preferences.diffMode = mode;
    preferences.queueSave();
  }

  function toggleLineWrapping() {
    preferences.wrapLines = !preferences.wrapLines;
    preferences.queueSave();
  }

  function setReviewOutputLanguage(language: ReviewOutputLanguage) {
    if (preferences.reviewOutputLanguage === language) return;
    preferences.reviewOutputLanguage = language;
    aiReview.reset();
    preferences.queueSave();
  }

  async function initialize() {
    try {
      const [loadedProjects, loadedPreferences] = await Promise.all([
        tauriApi.listProjects(),
        tauriApi.getUserPreferences(),
      ]);
      projects = loadedProjects;
      preferences.apply(loadedPreferences);
      const params = new URLSearchParams(window.location.search);
      const projectId = params.get("project");
      if (projectId) {
        const project = projects.find((item) => item.id === projectId);
        if (project) {
          const hasRequestedComparison =
            params.has("base") || params.has("compare") || params.has("target");
          const target = params.get("target") ?? ".";
          const base = params.get("base") ?? params.get("compare") ?? "HEAD";
          await openProject(project, params.get("file") ?? undefined, {
            requestedSelection: hasRequestedComparison
              ? { base, target }
              : undefined,
          });
        }
      }
    } catch (caught) {
      error = normalizeError(caught);
    } finally {
      loading = false;
    }
  }

  function refreshWorkspaceOnFocus() {
    const now = Date.now();
    if (
      !activeProject ||
      showProjectDialog ||
      showSettings ||
      workspace.loading ||
      aiReview.loading ||
      autoRefreshInProgress ||
      now - lastAutoRefreshAt < AUTO_REFRESH_COOLDOWN_MS
    )
      return;

    lastAutoRefreshAt = now;
    autoRefreshInProgress = true;
    const project = activeProject;
    const repository = activeRepository;
    void refreshActiveWorkspace(project, repository).finally(() => {
      autoRefreshInProgress = false;
    });
  }

  async function refreshActiveWorkspace(
    project: ProjectConfig,
    previousRepository: RepositoryInfo | undefined,
  ) {
    try {
      const refreshedRepository = await tauriApi.validateRepository(
        project.repoPath,
      );
      if (activeProject?.id !== project.id) return;

      const selectionHasDeletedBranch = hasDeletedSelectedBranch(
        workspace.selection,
        previousRepository,
        refreshedRepository,
      );
      activeRepository = refreshedRepository;

      if (selectionHasDeletedBranch)
        await applyDiffSelection(defaultDiffSelection());
      else await workspace.load(workspace.selectedPath, { silent: true });
    } catch (caught) {
      if (activeProject?.id === project.id)
        workspaceError = normalizeError(caught);
    }
  }

  function hasDeletedSelectedBranch(
    selection: DiffSelection,
    previousRepository: RepositoryInfo | undefined,
    refreshedRepository: RepositoryInfo,
  ) {
    if (!previousRepository) return false;

    const previousBranches = new Set([
      ...previousRepository.localBranches,
      ...previousRepository.remoteBranches,
    ]);
    const refreshedBranches = new Set([
      ...refreshedRepository.localBranches,
      ...refreshedRepository.remoteBranches,
    ]);

    return [selection.base, selection.target].some(
      (revision) =>
        previousBranches.has(revision) && !refreshedBranches.has(revision),
    );
  }

  async function addRepository() {
    error = null;
    try {
      const path = await tauriApi.selectRepository();
      if (!path) return;
      repositoryDraft = await tauriApi.validateRepository(path);
      editingProject = undefined;
      showProjectDialog = true;
    } catch (caught) {
      error = normalizeError(caught);
    }
  }

  async function saveProject(input: SaveProjectInput) {
    saving = true;
    error = null;
    try {
      const project = await tauriApi.saveProject(input);
      projects = [
        project,
        ...projects.filter((item) => item.id !== project.id),
      ];
      showProjectDialog = false;
      repositoryDraft = undefined;
      editingProject = undefined;
      if (activeProject?.id === project.id) {
        activeProject = project;
        await workspace.load();
      } else {
        await openProject(project);
      }
    } catch (caught) {
      error = normalizeError(caught);
    } finally {
      saving = false;
    }
  }

  async function removeProject(project: ProjectConfig) {
    if (
      !confirm(
        `Remove ${project.name} from undiffstand? The repository will not be changed.`,
      )
    )
      return;
    deleting = true;
    try {
      await tauriApi.removeProject(project.id);
      projects = projects.filter((item) => item.id !== project.id);
      showProjectDialog = false;
      repositoryDraft = undefined;
      editingProject = undefined;
      if (activeProject?.id === project.id) goHome();
    } catch (caught) {
      const normalized = normalizeError(caught);
      if (activeProject?.id === project.id) workspaceError = normalized;
      else error = normalized;
    } finally {
      deleting = false;
    }
  }

  async function openProject(
    project: ProjectConfig,
    requestedFile?: string,
    options: { requestedSelection?: DiffSelection } = {},
  ) {
    resetDiffSearch();
    loading = true;
    workspaceError = null;
    try {
      const [openedProject, repository] = await Promise.all([
        tauriApi.touchProject(project.id, options.requestedSelection),
        tauriApi.validateRepository(project.repoPath),
      ]);
      activeProject = openedProject;
      activeRepository = repository;
      workspace.activate(openedProject.id, openedProject.comparison);
      projects = [
        activeProject,
        ...projects.filter((item) => item.id !== activeProject?.id),
      ];
      await workspace.load(requestedFile);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    } finally {
      loading = false;
    }
  }

  async function applyDiffSelection(selection: DiffSelection) {
    showComparisonDialog = false;
    resetDiffSearch();
    if (!activeProject) return;
    try {
      const updatedProject = await tauriApi.saveProjectComparison(
        activeProject.id,
        selection,
      );
      activeProject = updatedProject;
      projects = projects.map((project) =>
        project.id === updatedProject.id ? updatedProject : project,
      );
      await workspace.applySelection(selection);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    }
  }

  async function configureBaseBranch() {
    showComparisonDialog = false;
    await editActiveProject();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    const shortcut = resolveApplicationShortcut(event);

    if (shortcut === "dismiss-dialogs") {
      if (diffSearchOpen) {
        event.preventDefault();
        closeDiffSearch();
        return;
      }
      if (showProjectDialog || showSettings || showComparisonDialog)
        event.preventDefault();
      showProjectDialog = false;
      showSettings = false;
      if (!workspace.loading) showComparisonDialog = false;
    } else if (shortcut === "open-settings") {
      event.preventDefault();
      showSettings = true;
    } else if (
      shortcut === "find-in-changes" &&
      activeProject &&
      !showProjectDialog &&
      !showSettings &&
      !showComparisonDialog
    ) {
      event.preventDefault();
      openDiffSearch();
    } else if (shortcut === "refresh-change-detail" && activeProject) {
      event.preventDefault();
      void workspace.load(workspace.selectedPath);
    } else if (shortcut === "toggle-changed-files" && activeProject) {
      event.preventDefault();
      toggleSidebar();
    } else if (shortcut === "toggle-ai-panel" && activeProject) {
      event.preventDefault();
      toggleAiPanel();
    }
  }

  function openDiffSearch() {
    diffSearchOpen = true;
    diffSearchFocusVersion += 1;
  }

  function closeDiffSearch() {
    diffSearchOpen = false;
    activeDiffSearchMatchId = undefined;
  }

  function resetDiffSearch() {
    diffSearchOpen = false;
    diffSearchQuery = "";
    activeDiffSearchMatchId = undefined;
  }

  function updateDiffSearchQuery(query: string) {
    diffSearchQuery = query;
    activeDiffSearchMatchId = undefined;
    if (!query.trim() || !workspace.summary) return;
    for (const file of workspace.summary.files) {
      if (file.status !== "binary")
        workspace.queue(file.newPath ?? file.oldPath ?? "Unknown file");
    }
  }

  function moveDiffSearch(direction: 1 | -1) {
    if (diffSearchMatches.length === 0) return;
    const nextIndex =
      activeDiffSearchIndex === -1
        ? direction === 1
          ? 0
          : diffSearchMatches.length - 1
        : (activeDiffSearchIndex + direction + diffSearchMatches.length) %
          diffSearchMatches.length;
    activeDiffSearchMatchId = diffSearchMatches[nextIndex]?.id;
  }

  async function explainFileChange(path: string) {
    if (activeProject)
      await aiReview.explainFile(activeProject.id, workspace.selection, path);
  }

  async function askInline(
    path: string,
    side: "old" | "new",
    startLine: number,
    endLine: number,
    question: string,
  ): Promise<InlineAnswer> {
    if (!activeProject) return Promise.reject(new Error("No project is open."));
    return aiReview.askInline(
      activeProject.id,
      workspace.selection,
      path,
      side,
      startLine,
      endLine,
      question,
    );
  }

  async function runChangeReview() {
    if (activeProject) {
      await aiReview.review(
        activeProject.id,
        workspace.selection,
        workspace.reviewAvailability,
      );
    }
  }

  function goHome() {
    resetDiffSearch();
    activeProject = null;
    activeRepository = undefined;
    workspace.reset();
    workspaceError = null;
    history.replaceState(null, "", window.location.pathname);
  }

  async function editProject(project: ProjectConfig) {
    if (activeProject?.id === project.id) workspaceError = null;
    else error = null;
    try {
      repositoryDraft = await tauriApi.validateRepository(project.repoPath);
      editingProject = project;
      showProjectDialog = true;
    } catch (caught) {
      const normalized = normalizeError(caught);
      if (activeProject?.id === project.id) workspaceError = normalized;
      else error = normalized;
    }
  }

  async function editActiveProject() {
    if (activeProject) await editProject(activeProject);
  }
</script>

<svelte:head>
  <title
    >{activeProject
      ? `${activeProject.name} · undiffstand`
      : "undiffstand"}</title
  >
  <meta
    name="description"
    content="AI-assisted diff understanding for human reviewers."
  />
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} />

{#if activeProject}
  <main class="app-shell">
    <header class="topbar">
      <button class="brand compact" onclick={goHome} title="Back to projects"
        ><span class="brand-mark"
          ><img src="/undiffstand-icon.png" alt="" /></span
        ><strong>undiffstand</strong></button
      >
      <button
        class="sidebar-toggle"
        type="button"
        aria-controls="changed-files-sidebar"
        aria-expanded={preferences.sidebarOpen}
        aria-label={preferences.sidebarOpen
          ? "Hide changed files sidebar"
          : "Show changed files sidebar"}
        title={preferences.sidebarOpen
          ? "Hide changed files"
          : "Show changed files"}
        onclick={toggleSidebar}
      >
        {#if preferences.sidebarOpen}<PanelLeftClose
            size={15}
          />{:else}<PanelLeftOpen size={15} />{/if}
      </button>
      <ProjectSwitcher
        {projects}
        {activeProject}
        comparisonLabel={workspace.summary
          ? diffComparisonLabel(
              workspace.summary.comparison,
              activeRepository?.currentBranch,
            )
          : diffSelectionLabel(
              workspace.selection,
              activeRepository?.currentBranch,
            )}
        onEditComparison={() => (showComparisonDialog = true)}
        onEditProject={editActiveProject}
        onSelect={openProject}
      />
      <div class="view-controls">
        <div class="view-mode-switch" role="group" aria-label="Diff layout">
          <button
            class:active={preferences.diffMode === "split"}
            aria-pressed={preferences.diffMode === "split"}
            onclick={() => setDiffMode("split")}
            title="Split diff"><Columns2 size={13} />Split</button
          >
          <button
            class:active={preferences.diffMode === "unified"}
            aria-pressed={preferences.diffMode === "unified"}
            onclick={() => setDiffMode("unified")}
            title="Unified diff"><Rows3 size={13} />Unified</button
          >
        </div>
        <button
          class="wrap-control"
          class:active={preferences.wrapLines}
          aria-pressed={preferences.wrapLines}
          onclick={toggleLineWrapping}
          title="Wrap long lines"><TextWrap size={13} /></button
        >
      </div>
      <div class="top-actions">
        <button
          onclick={() => workspace.load(workspace.selectedPath)}
          title="Refresh"><RefreshCw size={14} /></button
        >
        <button
          onclick={toggleAiPanel}
          class:active={preferences.aiPanelOpen}
          title="Toggle AI panel"><PanelRight size={14} /></button
        >
        <button onclick={() => (showSettings = true)} title="AI settings"
          ><Settings size={14} /></button
        >
      </div>
      <UpdateAction
        state={updateState}
        onInstall={() => void installUpdate()}
      />
    </header>

    {#if workspaceError}
      <div class="workspace-error">
        <ErrorBanner
          error={workspaceError}
          onDismiss={() => (workspaceError = null)}
        />
      </div>
    {/if}

    <div
      class:withoutAi={!preferences.aiPanelOpen}
      class:withoutSidebar={!preferences.sidebarOpen}
      class="workspace"
      style:--sidebar-width={`${preferences.sidebarWidth}px`}
      style:--ai-panel-width={`${preferences.aiPanelWidth}px`}
    >
      {#if preferences.sidebarOpen}
        <aside id="changed-files-sidebar" class="sidebar">
          <div class="pane-title">
            <span>Changed files</span>
            {#if workspace.summary}<DiffSummaryView
                summary={workspace.summary}
              />{:else}<b>0</b>{/if}
          </div>
          {#if workspace.summary}<DiffFileList
              files={workspace.summary.files}
              selectedPath={workspace.selectedPath}
              matchCounts={diffSearchOpen ? diffSearchMatchCounts : {}}
              onSelect={(path) => workspace.select(path)}
            />{/if}
        </aside>
        <ResizeHandle
          label="Resize changed files sidebar"
          value={preferences.sidebarWidth}
          minimum={SIDEBAR_MIN_WIDTH}
          maximum={SIDEBAR_MAX_WIDTH}
          cssProperty="--sidebar-width"
          constrain={(width, workspaceWidth) =>
            constrainedPanelWidth("sidebar", width, workspaceWidth)}
          onChange={(width) => (preferences.sidebarWidth = width)}
          onCommit={() => preferences.queueSave()}
          onReset={() => resetPanelWidth("sidebar")}
        />
      {/if}

      <section class="content-pane">
        {#if diffSearchOpen}
          <DiffSearchBar
            query={diffSearchQuery}
            current={activeDiffSearchIndex + 1}
            total={diffSearchMatches.length}
            pending={diffSearchPending}
            focusVersion={diffSearchFocusVersion}
            onQuery={updateDiffSearchQuery}
            onNext={() => moveDiffSearch(1)}
            onPrevious={() => moveDiffSearch(-1)}
            onClose={closeDiffSearch}
          />
        {/if}
        <div class="viewer-scroll">
          {#if workspace.loading}<div class="loading-state">
              <LoaderCircle class="spin" size={20} />Loading changes…
            </div>
          {:else if workspace.summary && workspace.summary.files.length > 0}<DiffFeed
              files={workspace.summary.files}
              diffs={workspace.diffs}
              loadingPaths={workspace.loadingPaths}
              errors={workspace.errors}
              activePath={workspace.selectedPath}
              mode={preferences.diffMode}
              wrap={preferences.wrapLines}
              fileExplanations={aiReview.fileExplanations}
              fileAiLoading={aiReview.fileLoading}
              fileAiErrors={aiReview.fileErrors}
              findings={aiReview.report?.findings ?? []}
              searchQuery={diffSearchOpen ? diffSearchQuery.trim() : ""}
              searchMatch={diffSearchOpen ? activeDiffSearchMatch : undefined}
              onLoad={(path) => workspace.queue(path)}
              onActive={(path) => workspace.setActive(path)}
              onExplainFile={explainFileChange}
              onAskInline={askInline}
            />
          {:else if workspace.summary}<EmptyState
              icon={GitBranch}
              title="No changes"
              message={`No changes found from ${revisionDisplayLabel(workspace.summary.comparison.fromLabel, activeRepository?.currentBranch)} to ${revisionDisplayLabel(workspace.summary.comparison.toLabel, activeRepository?.currentBranch)}.`}
              fill
            />
          {/if}
        </div>
      </section>

      {#if preferences.aiPanelOpen}
        <ResizeHandle
          label="Resize AI panel"
          value={preferences.aiPanelWidth}
          minimum={AI_PANEL_MIN_WIDTH}
          maximum={AI_PANEL_MAX_WIDTH}
          direction={-1}
          cssProperty="--ai-panel-width"
          constrain={(width, workspaceWidth) =>
            constrainedPanelWidth("ai", width, workspaceWidth)}
          onChange={(width) => (preferences.aiPanelWidth = width)}
          onCommit={() => preferences.queueSave()}
          onStart={() => (aiPanelExpanded = false)}
          onReset={() => resetPanelWidth("ai")}
        />
        <AiPanel
          availability={workspace.reviewAvailability}
          report={aiReview.report}
          loading={aiReview.loading}
          expanded={aiPanelExpanded}
          onReview={runChangeReview}
          onToggleExpanded={toggleAiPanelExpanded}
        />
      {/if}
    </div>
  </main>
{:else}
  <ProjectHome
    {projects}
    {loading}
    {error}
    onAdd={addRepository}
    onOpen={openProject}
    onEdit={editProject}
    {updateState}
    onInstallUpdate={() => void installUpdate()}
    onOpenSettings={() => (showSettings = true)}
    onDismissError={() => (error = null)}
  />
{/if}

{#if showProjectDialog}
  <ProjectDialog
    repository={repositoryDraft}
    project={editingProject}
    {saving}
    {deleting}
    onSave={saveProject}
    onDelete={editingProject ? () => removeProject(editingProject!) : undefined}
    onClose={() => (showProjectDialog = false)}
  />
{/if}

{#if activeProject && activeRepository && showComparisonDialog}
  <ComparisonDialog
    selection={workspace.selection}
    repository={activeRepository}
    {activeBaseRef}
    loading={workspace.loading}
    {baseToCurrentIsActive}
    {currentToWorkingTreeIsActive}
    onApply={applyDiffSelection}
    onConfigureBase={configureBaseBranch}
    onClose={() => (showComparisonDialog = false)}
  />
{/if}

{#if showSettings}
  <AiSettingsDialog
    outputLanguage={preferences.reviewOutputLanguage}
    onOutputLanguageChange={setReviewOutputLanguage}
    onClose={() => (showSettings = false)}
  />
{/if}

<style>
  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
  }
  .brand.compact {
    padding: 0;
    color: var(--text);
    background: none;
    border: 0;
    cursor: pointer;
  }
  .brand-mark {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 8px;
    box-shadow: 0 0 24px rgba(87, 184, 142, 0.14);
    overflow: hidden;
  }
  .brand-mark img {
    display: block;
    width: 100%;
    height: 100%;
  }
  .brand strong {
    display: block;
    font-size: 15px;
    letter-spacing: -0.01em;
  }
  .app-shell {
    height: 100vh;
    display: grid;
    grid-template-rows: 48px minmax(0, 1fr);
    overflow: hidden;
  }
  .topbar {
    display: grid;
    grid-template-columns: auto auto minmax(220px, 1fr) auto auto auto;
    align-items: center;
    gap: 8px;
    padding: 0 11px;
    background: #0c1219;
    border-bottom: 1px solid var(--border);
  }
  .topbar .brand-mark {
    width: 27px;
    height: 27px;
    border-radius: 6px;
  }
  .topbar .brand {
    gap: 7px;
  }
  .top-actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .sidebar-toggle,
  .top-actions button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    color: #77838e;
    background: none;
    border: 0;
    border-radius: 5px;
    cursor: pointer;
  }
  .sidebar-toggle:hover,
  .top-actions button:hover,
  .top-actions button.active {
    color: var(--text);
    background: var(--hover);
  }
  .workspace-error {
    position: fixed;
    z-index: 20;
    top: 57px;
    left: 50%;
    width: min(520px, calc(100% - 40px));
    transform: translateX(-50%);
  }
  .workspace {
    display: grid;
    grid-template-columns:
      var(--sidebar-width) var(--panel-handle-width) minmax(500px, 1fr)
      var(--panel-handle-width) var(--ai-panel-width);
    min-height: 0;
    overflow: hidden;
    --panel-handle-width: 5px;
  }
  .workspace.withoutAi {
    grid-template-columns:
      var(--sidebar-width) var(--panel-handle-width)
      minmax(500px, 1fr);
  }
  .workspace.withoutSidebar {
    grid-template-columns: minmax(500px, 1fr) var(--panel-handle-width) var(
        --ai-panel-width
      );
  }
  .workspace.withoutSidebar.withoutAi {
    grid-template-columns: minmax(500px, 1fr);
  }
  .sidebar {
    min-width: 0;
    overflow: auto;
    background: #0c1219;
    border-right: 1px solid var(--border);
  }
  .pane-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 38px;
    padding: 0 12px;
    color: #78848e;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .pane-title b {
    display: grid;
    place-items: center;
    min-width: 18px;
    height: 17px;
    padding: 0 4px;
    color: #7e8b96;
    background: #18212b;
    border-radius: 8px;
    font-size: 12px;
  }
  .content-pane {
    position: relative;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: #0b1016;
  }
  .view-controls {
    display: flex;
    align-items: center;
    gap: 7px;
    padding-right: 6px;
    border-right: 1px solid var(--border);
  }
  .view-mode-switch {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    background: #111922;
    border: 1px solid #293541;
    border-radius: 7px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.015);
  }
  .view-mode-switch button,
  .wrap-control {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 25px;
    padding: 0 7px;
    color: #6f7b86;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  .view-mode-switch button:hover {
    color: #aeb8c1;
    background: rgba(255, 255, 255, 0.035);
  }
  .view-mode-switch button.active {
    color: #e5eaee;
    background: #1d2731;
    border-color: #303c48;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.28);
  }
  .wrap-control {
    width: 27px;
    justify-content: center;
    padding: 0;
  }
  .wrap-control:hover,
  .wrap-control.active {
    color: #bac3cc;
    background: #16202a;
    border-color: #24313d;
  }
  .viewer-scroll {
    min-height: 0;
    overflow: auto;
  }
  .loading-state {
    min-height: 190px;
    display: flex;
    place-content: center;
    align-items: center;
    gap: 9px;
    color: var(--muted);
    font-size: 12px;
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
