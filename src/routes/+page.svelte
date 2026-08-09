<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    Bot,
    Braces,
    CirclePlus,
    Columns2,
    FolderGit2,
    GitBranch,
    LoaderCircle,
    PanelLeftClose,
    PanelLeftOpen,
    PanelRight,
    Pencil,
    RefreshCw,
    Rows3,
    Settings,
    WrapText,
    X
  } from '@lucide/svelte';
  import AiPanel from '$lib/components/ai/AiPanel.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import ErrorBanner from '$lib/components/common/ErrorBanner.svelte';
  import DiffFeed from '$lib/components/diff/DiffFeed.svelte';
  import DiffFileList from '$lib/components/diff/DiffFileList.svelte';
  import DiffSummaryView from '$lib/components/diff/DiffSummary.svelte';
  import ProjectDialog from '$lib/components/project/ProjectDialog.svelte';
  import ProjectSwitcher from '$lib/components/project/ProjectSwitcher.svelte';
  import type { DiffExplanation } from '$lib/domain/ai';
  import { diffAnchorId, displayPath, type DiffSummary, type FileDiff } from '$lib/domain/diff';
  import { normalizeError, type AppError } from '$lib/domain/error';
  import type { ProjectConfig, RepositoryInfo, SaveProjectInput } from '$lib/domain/project';
  import { tauriApi } from '$lib/services/tauri';

  let projects = $state<ProjectConfig[]>([]);
  let activeProject = $state<ProjectConfig | null>(null);
  let repositoryDraft = $state<RepositoryInfo | undefined>();
  let editingProject = $state<ProjectConfig | undefined>();
  let showProjectDialog = $state(false);
  let showSettings = $state(false);
  let loading = $state(true);
  let saving = $state(false);
  let deleting = $state(false);
  let error = $state<AppError | null>(null);
  let workspaceError = $state<AppError | null>(null);

  let diffMode = $state<'split' | 'unified'>('split');
  let wrapLines = $state(false);
  let summary = $state<DiffSummary | null>(null);
  let selectedDiffPath = $state<string | undefined>();
  let diffsByPath = $state<Record<string, FileDiff | undefined>>({});
  let diffLoadingPaths = $state<Record<string, boolean | undefined>>({});
  let diffErrors = $state<Record<string, string | undefined>>({});
  let diffExplanation = $state<DiffExplanation | undefined>();
  let aiLoading = $state(false);
  let contentLoading = $state(false);
  let sidebarOpen = $state(true);
  let aiPanelOpen = $state(true);
  let sidebarWidth = $state(225);
  let aiPanelWidth = $state(290);
  let pendingDiffPaths = new Set<string>();
  let diffBatchTimer: ReturnType<typeof setTimeout> | undefined;
  let diffLoadGeneration = 0;
  let autoRefreshInProgress = false;
  let lastAutoRefreshAt = 0;
  const AUTO_REFRESH_COOLDOWN_MS = 1_000;
  const PANEL_HANDLE_WIDTH = 5;
  const CONTENT_MIN_WIDTH = 500;
  const SIDEBAR_MIN_WIDTH = 160;
  const SIDEBAR_MAX_WIDTH = 420;
  const AI_PANEL_MIN_WIDTH = 240;
  const AI_PANEL_MAX_WIDTH = 520;
  const PANEL_RESIZE_STEP = 10;
  type ResizablePanel = 'sidebar' | 'ai';
  let activeResize = $state<
    | {
        panel: ResizablePanel;
        startX: number;
        startWidth: number;
        currentWidth: number;
        workspaceWidth: number;
        workspace: HTMLElement;
      }
    | undefined
  >(undefined);
  let pendingResizeClientX: number | undefined;
  let resizeAnimationFrame: number | undefined;
  let previousCursor = '';
  let previousUserSelect = '';

  onMount(() => {
    if (window.innerWidth <= 1100) {
      sidebarWidth = 200;
      aiPanelWidth = 260;
    }
    window.addEventListener('focus', refreshWorkspaceOnFocus);
    void initialize();

    return () => {
      window.removeEventListener('focus', refreshWorkspaceOnFocus);
      stopPanelResize();
    };
  });

  function panelWidthLimits(panel: ResizablePanel, workspaceWidth: number) {
    const min = panel === 'sidebar' ? SIDEBAR_MIN_WIDTH : AI_PANEL_MIN_WIDTH;
    const configuredMax = panel === 'sidebar' ? SIDEBAR_MAX_WIDTH : AI_PANEL_MAX_WIDTH;
    const otherPanelWidth =
      panel === 'sidebar' ? (aiPanelOpen ? aiPanelWidth : 0) : sidebarOpen ? sidebarWidth : 0;
    const visibleHandleCount = Number(sidebarOpen) + Number(aiPanelOpen);
    const availableMax =
      workspaceWidth -
      otherPanelWidth -
      CONTENT_MIN_WIDTH -
      visibleHandleCount * PANEL_HANDLE_WIDTH;

    return { min, max: Math.max(min, Math.min(configuredMax, availableMax)) };
  }

  function setPanelWidth(panel: ResizablePanel, width: number, workspaceWidth: number) {
    const nextWidth = constrainedPanelWidth(panel, width, workspaceWidth);
    if (panel === 'sidebar') sidebarWidth = nextWidth;
    else aiPanelWidth = nextWidth;
  }

  function constrainedPanelWidth(panel: ResizablePanel, width: number, workspaceWidth: number) {
    const { min, max } = panelWidthLimits(panel, workspaceWidth);
    return Math.round(Math.min(max, Math.max(min, width)));
  }

  function startPanelResize(event: PointerEvent, panel: ResizablePanel) {
    if (event.button !== 0) return;
    const handle = event.currentTarget as HTMLElement;
    const workspace = handle.parentElement;
    if (!workspace) return;

    event.preventDefault();
    activeResize = {
      panel,
      startX: event.clientX,
      startWidth: panel === 'sidebar' ? sidebarWidth : aiPanelWidth,
      currentWidth: panel === 'sidebar' ? sidebarWidth : aiPanelWidth,
      workspaceWidth: workspace.getBoundingClientRect().width,
      workspace
    };
    previousCursor = document.body.style.cursor;
    previousUserSelect = document.body.style.userSelect;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('pointermove', resizePanel);
    window.addEventListener('pointerup', stopPanelResize);
    window.addEventListener('pointercancel', stopPanelResize);
  }

  function resizePanel(event: PointerEvent) {
    if (!activeResize) return;
    pendingResizeClientX = event.clientX;
    if (resizeAnimationFrame !== undefined) return;
    resizeAnimationFrame = requestAnimationFrame(flushPanelResize);
  }

  function flushPanelResize() {
    resizeAnimationFrame = undefined;
    if (!activeResize || pendingResizeClientX === undefined) return;
    const direction = activeResize.panel === 'sidebar' ? 1 : -1;
    const requestedWidth =
      activeResize.startWidth + (pendingResizeClientX - activeResize.startX) * direction;
    pendingResizeClientX = undefined;
    activeResize.currentWidth = constrainedPanelWidth(
      activeResize.panel,
      requestedWidth,
      activeResize.workspaceWidth
    );
    const property = activeResize.panel === 'sidebar' ? '--sidebar-width' : '--ai-panel-width';
    activeResize.workspace.style.setProperty(property, `${activeResize.currentWidth}px`);
  }

  function stopPanelResize() {
    if (!activeResize) return;
    if (resizeAnimationFrame !== undefined) {
      cancelAnimationFrame(resizeAnimationFrame);
      resizeAnimationFrame = undefined;
    }
    flushPanelResize();
    const { panel, currentWidth } = activeResize;
    if (panel === 'sidebar') sidebarWidth = currentWidth;
    else aiPanelWidth = currentWidth;
    activeResize = undefined;
    pendingResizeClientX = undefined;
    document.body.style.cursor = previousCursor;
    document.body.style.userSelect = previousUserSelect;
    window.removeEventListener('pointermove', resizePanel);
    window.removeEventListener('pointerup', stopPanelResize);
    window.removeEventListener('pointercancel', stopPanelResize);
  }

  function resizePanelWithKeyboard(event: KeyboardEvent, panel: ResizablePanel) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    const workspace = (event.currentTarget as HTMLElement).parentElement;
    if (!workspace) return;

    event.preventDefault();
    const workspaceWidth = workspace.getBoundingClientRect().width;
    const currentWidth = panel === 'sidebar' ? sidebarWidth : aiPanelWidth;
    const direction = panel === 'sidebar' ? 1 : -1;
    const { min, max } = panelWidthLimits(panel, workspaceWidth);
    if (event.key === 'Home') setPanelWidth(panel, min, workspaceWidth);
    else if (event.key === 'End') setPanelWidth(panel, max, workspaceWidth);
    else {
      const movement = event.key === 'ArrowRight' ? PANEL_RESIZE_STEP : -PANEL_RESIZE_STEP;
      setPanelWidth(panel, currentWidth + movement * direction, workspaceWidth);
    }
  }

  function resetPanelWidth(panel: ResizablePanel) {
    if (panel === 'sidebar') sidebarWidth = window.innerWidth <= 1100 ? 200 : 225;
    else aiPanelWidth = window.innerWidth <= 1100 ? 260 : 290;
  }

  async function initialize() {
    try {
      projects = await tauriApi.listProjects();
      const params = new URLSearchParams(window.location.search);
      const projectId = params.get('project');
      if (projectId) {
        const project = projects.find((item) => item.id === projectId);
        if (project) {
          await openProject(project, params.get('file') ?? undefined);
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
      contentLoading ||
      aiLoading ||
      autoRefreshInProgress ||
      now - lastAutoRefreshAt < AUTO_REFRESH_COOLDOWN_MS
    )
      return;

    lastAutoRefreshAt = now;
    autoRefreshInProgress = true;
    void loadWorkspace(selectedDiffPath, { silent: true }).finally(() => {
      autoRefreshInProgress = false;
    });
  }

  function setUrl(file?: string) {
    if (!activeProject) return;
    const params = new URLSearchParams({ project: activeProject.id });
    if (file) params.set('file', file);
    history.replaceState(null, '', `?${params.toString()}`);
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
      projects = [project, ...projects.filter((item) => item.id !== project.id)];
      showProjectDialog = false;
      repositoryDraft = undefined;
      editingProject = undefined;
      if (activeProject?.id === project.id) {
        activeProject = project;
        await loadWorkspace();
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
    if (!confirm(`Remove ${project.name} from ReaDiff? The repository will not be changed.`))
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

  async function openProject(project: ProjectConfig, requestedFile?: string) {
    loading = true;
    workspaceError = null;
    try {
      activeProject = await tauriApi.touchProject(project.id);
      projects = [activeProject, ...projects.filter((item) => item.id !== activeProject?.id)];
      await loadWorkspace(requestedFile);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    } finally {
      loading = false;
    }
  }

  async function loadWorkspace(requestedFile?: string, options: { silent?: boolean } = {}) {
    if (!activeProject) return;
    const projectId = activeProject.id;
    const silent = options.silent ?? false;
    workspaceError = null;
    if (!silent) {
      summary = null;
      contentLoading = true;
      clearWorkspaceDiffs();
    }
    try {
      const loadedSummary = await tauriApi.getDiffSummary(projectId);
      if (activeProject?.id !== projectId) return;
      const path =
        requestedFile && loadedSummary.files.some((file) => displayPath(file) === requestedFile)
          ? requestedFile
          : loadedSummary.files[0]
            ? displayPath(loadedSummary.files[0])
            : undefined;

      if (silent) {
        const availablePaths = new Set(loadedSummary.files.map(displayPath));
        const refreshPaths = new Set([
          ...Object.keys(diffsByPath),
          ...Object.keys(diffErrors),
          ...Object.entries(diffLoadingPaths)
            .filter(([, isLoading]) => isLoading)
            .map(([loadingPath]) => loadingPath)
        ]);
        if (path) refreshPaths.add(path);

        const pathsToRefresh = [...refreshPaths].filter((refreshPath) =>
          availablePaths.has(refreshPath)
        );
        const refreshedDiffs =
          pathsToRefresh.length > 0 ? await tauriApi.getFileDiffs(projectId, pathsToRefresh) : [];
        if (activeProject?.id !== projectId) return;

        clearPendingDiffs();
        diffsByPath = Object.fromEntries(
          refreshedDiffs.map((diff) => [displayPath(diff.file), diff])
        );
        diffLoadingPaths = {};
        diffErrors = {};
        diffExplanation = undefined;
      }

      summary = loadedSummary;
      if (!silent) contentLoading = false;
      selectedDiffPath = path;
      setUrl(path);
      if (path) {
        queueDiff(path);
        await tick();
        if (activeProject?.id === projectId) scrollToDiff(path, false);
      }
    } catch (caught) {
      if (activeProject?.id === projectId) workspaceError = normalizeError(caught);
    } finally {
      if (!silent && activeProject?.id === projectId) contentLoading = false;
    }
  }

  function clearWorkspaceDiffs() {
    clearPendingDiffs();
    diffsByPath = {};
    diffLoadingPaths = {};
    diffErrors = {};
    diffExplanation = undefined;
  }

  function selectDiff(path: string) {
    if (!activeProject) return;
    selectedDiffPath = path;
    diffExplanation = undefined;
    setUrl(path);
    queueDiff(path);
    scrollToDiff(path, true);
  }

  function scrollToDiff(path: string, smooth: boolean) {
    document
      .getElementById(diffAnchorId(path))
      ?.scrollIntoView({ behavior: smooth ? 'smooth' : 'auto', block: 'start' });
  }

  function setActiveDiff(path: string) {
    if (selectedDiffPath === path) return;
    selectedDiffPath = path;
    diffExplanation = undefined;
    setUrl(path);
  }

  function queueDiff(path: string) {
    if (!activeProject || diffsByPath[path] || diffLoadingPaths[path]) return;
    diffErrors[path] = undefined;
    diffLoadingPaths[path] = true;
    pendingDiffPaths.add(path);
    diffBatchTimer ??= setTimeout(loadPendingDiffs, 16);
  }

  async function loadPendingDiffs() {
    diffBatchTimer = undefined;
    if (!activeProject || pendingDiffPaths.size === 0) return;
    const projectId = activeProject.id;
    const generation = diffLoadGeneration;
    const paths = [...pendingDiffPaths];
    pendingDiffPaths.clear();

    try {
      const loadedDiffs = await tauriApi.getFileDiffs(projectId, paths);
      if (activeProject?.id !== projectId || diffLoadGeneration !== generation) return;
      for (const diff of loadedDiffs) {
        diffsByPath[displayPath(diff.file)] = diff;
      }
      if (selectedDiffPath && paths.includes(selectedDiffPath)) {
        await tick();
        if (diffLoadGeneration === generation) scrollToDiff(selectedDiffPath, false);
      }
    } catch (caught) {
      const normalized = normalizeError(caught);
      if (activeProject?.id === projectId && diffLoadGeneration === generation) {
        for (const path of paths) diffErrors[path] = normalized.message;
      }
    } finally {
      if (activeProject?.id === projectId && diffLoadGeneration === generation) {
        for (const path of paths) diffLoadingPaths[path] = false;
      }
    }
  }

  function clearPendingDiffs() {
    if (diffBatchTimer !== undefined) clearTimeout(diffBatchTimer);
    diffBatchTimer = undefined;
    pendingDiffPaths.clear();
    diffLoadGeneration += 1;
  }

  async function explainDiff() {
    if (!activeProject || !selectedDiffPath) return;
    aiLoading = true;
    diffExplanation = undefined;
    workspaceError = null;
    try {
      diffExplanation = await tauriApi.explainFileDiff(activeProject.id, selectedDiffPath);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    } finally {
      aiLoading = false;
    }
  }

  function goHome() {
    activeProject = null;
    summary = null;
    selectedDiffPath = undefined;
    clearPendingDiffs();
    diffsByPath = {};
    diffLoadingPaths = {};
    diffErrors = {};
    workspaceError = null;
    history.replaceState(null, '', window.location.pathname);
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

  function formatDate(value: string) {
    const date = new Date(value);
    return Number.isNaN(date.valueOf())
      ? value
      : new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
          date
        );
  }
</script>

<svelte:head>
  <title>{activeProject ? `${activeProject.name} · ReaDiff` : 'ReaDiff'}</title>
  <meta name="description" content="AI-assisted diff understanding for human reviewers." />
</svelte:head>

{#if activeProject}
  <main class="app-shell">
    <header class="topbar">
      <button class="brand compact" onclick={goHome} title="Back to projects"
        ><span class="brand-mark"><Braces size={16} /></span><strong>ReaDiff</strong></button
      >
      <button
        class="sidebar-toggle"
        type="button"
        aria-controls="changed-files-sidebar"
        aria-expanded={sidebarOpen}
        aria-label={sidebarOpen ? 'Hide changed files sidebar' : 'Show changed files sidebar'}
        title={sidebarOpen ? 'Hide changed files' : 'Show changed files'}
        onclick={() => (sidebarOpen = !sidebarOpen)}
      >
        {#if sidebarOpen}<PanelLeftClose size={15} />{:else}<PanelLeftOpen size={15} />{/if}
      </button>
      <ProjectSwitcher {projects} {activeProject} onSelect={openProject} />
      <div class="top-actions">
        <button onclick={() => loadWorkspace(selectedDiffPath)} title="Refresh"
          ><RefreshCw size={14} /></button
        >
        <button
          onclick={() => (aiPanelOpen = !aiPanelOpen)}
          class:active={aiPanelOpen}
          title="Toggle AI panel"><PanelRight size={14} /></button
        >
        <button onclick={editActiveProject} title="Project settings"><Settings size={14} /></button>
      </div>
    </header>

    {#if workspaceError}
      <div class="workspace-error">
        <ErrorBanner error={workspaceError} onDismiss={() => (workspaceError = null)} />
      </div>
    {/if}

    <div
      class:withoutAi={!aiPanelOpen}
      class:withoutSidebar={!sidebarOpen}
      class="workspace"
      style:--sidebar-width={`${sidebarWidth}px`}
      style:--ai-panel-width={`${aiPanelWidth}px`}
    >
      {#if sidebarOpen}
        <aside id="changed-files-sidebar" class="sidebar">
          <div class="pane-title">
            <span>Changed files</span><b>{summary?.files.length ?? 0}</b>
          </div>
          {#if summary}<DiffFileList
              files={summary.files}
              selectedPath={selectedDiffPath}
              onSelect={selectDiff}
            />{/if}
        </aside>
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex (separator is an interactive window splitter) -->
        <div
          class="resize-handle"
          class:resizing={activeResize?.panel === 'sidebar'}
          role="separator"
          aria-label="Resize changed files sidebar"
          aria-orientation="vertical"
          aria-valuemin={SIDEBAR_MIN_WIDTH}
          aria-valuemax={SIDEBAR_MAX_WIDTH}
          aria-valuenow={sidebarWidth}
          tabindex="0"
          onpointerdown={(event) => startPanelResize(event, 'sidebar')}
          onkeydown={(event) => resizePanelWithKeyboard(event, 'sidebar')}
          ondblclick={() => resetPanelWidth('sidebar')}
        ></div>
      {/if}

      <section class="content-pane">
        <div class="content-toolbar">
          {#if summary}<DiffSummaryView {summary} />{/if}
          <div class="view-controls">
            <div class="view-mode-switch" role="group" aria-label="Diff layout">
              <button
                class:active={diffMode === 'split'}
                aria-pressed={diffMode === 'split'}
                onclick={() => (diffMode = 'split')}
                title="Split diff"><Columns2 size={13} />Split</button
              >
              <button
                class:active={diffMode === 'unified'}
                aria-pressed={diffMode === 'unified'}
                onclick={() => (diffMode = 'unified')}
                title="Unified diff"><Rows3 size={13} />Unified</button
              >
            </div>
            <button
              class="wrap-control"
              class:active={wrapLines}
              aria-pressed={wrapLines}
              onclick={() => (wrapLines = !wrapLines)}
              title="Wrap long lines"><WrapText size={13} /></button
            >
          </div>
        </div>
        <div class="viewer-scroll">
          {#if contentLoading}<div class="loading-state">
              <LoaderCircle class="spin" size={20} />Loading changes…
            </div>
          {:else if summary && summary.files.length > 0}<DiffFeed
              files={summary.files}
              diffs={diffsByPath}
              loadingPaths={diffLoadingPaths}
              errors={diffErrors}
              activePath={selectedDiffPath}
              mode={diffMode}
              wrap={wrapLines}
              onLoad={queueDiff}
              onActive={setActiveDiff}
            />
          {:else if summary}<EmptyState
              icon={GitBranch}
              title="No changes"
              message={`The working tree has no changes relative to the merge base with ${activeProject.baseRef}.`}
              fill
            />
          {/if}
        </div>
      </section>

      {#if aiPanelOpen}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex (separator is an interactive window splitter) -->
        <div
          class="resize-handle"
          class:resizing={activeResize?.panel === 'ai'}
          role="separator"
          aria-label="Resize AI panel"
          aria-orientation="vertical"
          aria-valuemin={AI_PANEL_MIN_WIDTH}
          aria-valuemax={AI_PANEL_MAX_WIDTH}
          aria-valuenow={aiPanelWidth}
          tabindex="0"
          onpointerdown={(event) => startPanelResize(event, 'ai')}
          onkeydown={(event) => resizePanelWithKeyboard(event, 'ai')}
          ondblclick={() => resetPanelWidth('ai')}
        ></div>
        <AiPanel explanation={diffExplanation} loading={aiLoading} onExplain={explainDiff} />
      {/if}
    </div>
  </main>
{:else}
  <main class="home">
    <header class="home-header">
      <div class="brand">
        <span class="brand-mark"><Braces size={20} /></span>
        <div>
          <strong>ReaDiff</strong><small>AI-assisted diff understanding for human reviewers.</small>
        </div>
      </div>
      <button class="settings-button" onclick={() => (showSettings = true)}
        ><Settings size={14} />Settings</button
      >
    </header>

    <section class="projects-section">
      {#if error}<ErrorBanner {error} onDismiss={() => (error = null)} />{/if}
      {#if loading}
        <div class="loading-state"><LoaderCircle class="spin" size={20} />Loading projects…</div>
      {:else if projects.length}
        <div class="section-heading">
          <h1>Continue reviewing</h1>
          <button onclick={addRepository}><CirclePlus size={14} />Add Project</button>
        </div>
        <div class="project-grid">
          {#each projects as project (project.id)}
            <article>
              <button class="project-main" onclick={() => openProject(project)}>
                <div class="project-icon"><FolderGit2 size={19} /></div>
                <div class="project-info">
                  <h3>{project.name}</h3>
                  <p>{project.repoPath}</p>
                  <span><GitBranch size={11} />{project.baseRef}</span>
                </div>
              </button>
              <div class="project-meta">
                <button
                  class="edit-project"
                  onclick={() => editProject(project)}
                  title={`Edit ${project.name}`}
                  aria-label={`Edit ${project.name}`}><Pencil size={14} /></button
                >
                <time>{formatDate(project.lastOpenedAt)}</time>
              </div>
            </article>
          {/each}
        </div>
      {:else}
        <div class="empty-projects">
          <div class="empty-project-icon"><FolderGit2 size={28} strokeWidth={1.5} /></div>
          <h1>Add your first project</h1>
          <p>Choose a local Git repository to start reviewing its changes.</p>
          <button onclick={addRepository}><CirclePlus size={16} />Add Project</button>
        </div>
      {/if}
    </section>
  </main>
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

{#if showSettings}
  <div
    class="settings-backdrop"
    role="presentation"
    onclick={(event) => event.target === event.currentTarget && (showSettings = false)}
  >
    <div class="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <header>
        <div>
          <Settings size={17} />
          <h2 id="settings-title">AI settings</h2>
        </div>
        <button onclick={() => (showSettings = false)}><X size={17} /></button>
      </header>
      <div class="settings-content">
        <div>
          <span>Runtime</span><strong>codex exec</strong>
          <p>
            ReaDiff invokes the Codex CLI available on <code>PATH</code> in a read-only, isolated temporary
            directory.
          </p>
        </div>
        <div>
          <span>Authentication</span><strong>codex login</strong>
          <p>
            Saved Codex CLI authentication and your local Codex configuration are reused. ReaDiff
            removes API-key environment variables from the child process.
          </p>
        </div>
        <div class="privacy">
          <Bot size={15} />
          <p>
            When you use AI, Codex receives the selected diff according to your local Codex
            configuration. AI descriptions are inferences and may be wrong.
          </p>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(:root) {
    font-family:
      Inter,
      ui-sans-serif,
      -apple-system,
      BlinkMacSystemFont,
      'Segoe UI',
      sans-serif;
    color-scheme: dark;
    --bg: #090e14;
    --panel: #0e141c;
    --panel-raised: #111923;
    --input: #0a1016;
    --hover: rgba(137, 158, 178, 0.07);
    --border: #1b2530;
    --border-strong: #2a3642;
    --text: #dbe2e8;
    --muted: #7d8995;
    --accent: #3f9471;
    --accent-bright: #63c69a;
    --green: #55bd83;
    --red: #e06c64;
    --mono: 'SFMono-Regular', Consolas, 'Liberation Mono', monospace;
    background: var(--bg);
  }

  :global(*) {
    box-sizing: border-box;
  }
  :global(html, body) {
    margin: 0;
    min-width: 900px;
    min-height: 100%;
    background: var(--bg);
  }
  :global(body) {
    color: var(--text);
  }
  :global(button) {
    font-family: inherit;
  }
  :global(::selection) {
    background: rgba(87, 184, 142, 0.3);
  }

  .home {
    min-height: 100vh;
    background: var(--bg);
  }
  .home-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    max-width: 900px;
    height: 72px;
    margin: auto;
    padding: 0 28px;
    border-bottom: 1px solid var(--border);
  }
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
    color: #07120e;
    background: var(--accent-bright);
    border-radius: 8px;
    box-shadow: 0 0 24px rgba(87, 184, 142, 0.14);
  }
  .brand strong {
    display: block;
    font-size: 15px;
    letter-spacing: -0.01em;
  }
  .brand small {
    display: block;
    margin-top: 2px;
    color: var(--muted);
    font-size: 11px;
    letter-spacing: 0.03em;
  }
  .settings-button,
  .section-heading button {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 10px;
    color: #aab4be;
    background: rgba(16, 24, 33, 0.7);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 11px;
    cursor: pointer;
  }
  .section-heading button {
    color: #07120e;
    background: var(--accent-bright);
    border-color: var(--accent-bright);
    font-weight: 650;
  }
  .projects-section {
    max-width: 900px;
    margin: auto;
    padding: 36px 28px 72px;
  }
  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
  }
  .section-heading h1 {
    margin: 0;
    color: #eef3f6;
    font-size: 22px;
    letter-spacing: -0.025em;
  }
  .project-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 10px;
  }
  article {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    overflow: hidden;
    background: #0d141c;
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  article:hover {
    border-color: #2c3b48;
    background: #101821;
  }
  .project-main {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 12px;
    align-items: center;
    width: 100%;
    padding: 14px;
    color: var(--text);
    text-align: left;
    background: none;
    border: 0;
    cursor: pointer;
  }
  .project-icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.08);
    border-radius: 7px;
  }
  .project-info {
    min-width: 0;
  }
  .project-info h3 {
    margin: 0;
    font-size: 14px;
  }
  .project-info p {
    overflow: hidden;
    margin: 4px 0 7px;
    color: #687581;
    font: 11px/1.35 var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .project-info span {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #8b9a95;
    font-size: 11px;
  }
  .project-meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: center;
    gap: 7px;
    padding: 10px 12px 10px 6px;
  }
  .project-meta time {
    color: #697681;
    font-size: 11px;
    white-space: nowrap;
  }
  .edit-project {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    color: #687581;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    cursor: pointer;
  }
  .edit-project:hover {
    color: var(--text);
    background: var(--hover);
    border-color: var(--border-strong);
  }
  .empty-projects {
    display: grid;
    place-items: center;
    min-height: 420px;
    padding: 64px 40px;
    color: #596570;
    text-align: center;
  }
  .empty-project-icon {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.08);
    border: 1px solid rgba(87, 184, 142, 0.14);
    border-radius: 14px;
  }
  .empty-projects h1 {
    margin: 18px 0 7px;
    color: #eef3f6;
    font-size: 22px;
    letter-spacing: -0.025em;
  }
  .empty-projects p {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .empty-projects button {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 22px;
    padding: 10px 14px;
    color: #07120e;
    background: var(--accent-bright);
    border: 1px solid var(--accent-bright);
    border-radius: 7px;
    font-size: 12px;
    font-weight: 650;
    cursor: pointer;
  }

  .app-shell {
    height: 100vh;
    display: grid;
    grid-template-rows: 48px minmax(0, 1fr);
    overflow: hidden;
  }
  .topbar {
    display: grid;
    grid-template-columns: auto auto minmax(220px, 1fr) auto;
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
    --panel-handle-width: 5px;
  }
  .workspace.withoutAi {
    grid-template-columns: var(--sidebar-width) var(--panel-handle-width) minmax(500px, 1fr);
  }
  .workspace.withoutSidebar {
    grid-template-columns: minmax(500px, 1fr) var(--panel-handle-width) var(--ai-panel-width);
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
  .resize-handle {
    position: relative;
    z-index: 2;
    min-width: var(--panel-handle-width);
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 0;
    cursor: col-resize;
    touch-action: none;
  }
  .resize-handle::after {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
    background: transparent;
    content: '';
    transform: translateX(-50%);
    transition: background 120ms ease;
  }
  .resize-handle:hover::after,
  .resize-handle:focus-visible::after,
  .resize-handle.resizing::after {
    background: var(--accent-bright);
  }
  .resize-handle:focus-visible {
    outline: 1px solid var(--accent-bright);
    outline-offset: -1px;
  }
  .pane-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 38px;
    padding: 0 12px;
    color: #78848e;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
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
    font-size: 10px;
  }
  .content-pane {
    display: grid;
    grid-template-rows: 39px minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: #0b1016;
  }
  .content-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-width: 0;
    padding: 0 10px 0 13px;
    background: #0e151d;
    border-bottom: 1px solid var(--border);
  }
  .view-controls {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-left: 12px;
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
    font-size: 11px;
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
    font-size: 11px;
  }

  .settings-backdrop {
    position: fixed;
    z-index: 50;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 7, 11, 0.72);
    backdrop-filter: blur(6px);
  }
  .settings-dialog {
    width: min(490px, 100%);
    background: #111821;
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.4);
  }
  .settings-dialog header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 15px 17px;
    border-bottom: 1px solid var(--border);
  }
  .settings-dialog header div {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .settings-dialog h2 {
    margin: 0;
    font-size: 14px;
  }
  .settings-dialog header button {
    display: grid;
    padding: 3px;
    color: var(--muted);
    background: none;
    border: 0;
    cursor: pointer;
  }
  .settings-content {
    display: grid;
    gap: 15px;
    padding: 18px;
  }
  .settings-content > div:not(.privacy) {
    display: grid;
    grid-template-columns: 100px 1fr;
    gap: 3px 12px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
  }
  .settings-content span {
    grid-row: 1/3;
    color: var(--muted);
    font-size: 10px;
  }
  .settings-content strong {
    font: 11px var(--mono);
  }
  .settings-content p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 10px;
    line-height: 1.55;
  }
  .settings-content code {
    color: var(--accent-bright);
    font-family: var(--mono);
  }
  .privacy {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px;
    color: #c2ad70;
    background: rgba(183, 145, 52, 0.07);
    border: 1px solid rgba(183, 145, 52, 0.14);
    border-radius: 6px;
  }
  .privacy p {
    margin: 0;
    color: #a99361;
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
