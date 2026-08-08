<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Bot,
    Braces,
    CirclePlus,
    Code2,
    Columns2,
    FileCode2,
    FolderGit2,
    GitBranch,
    LoaderCircle,
    PanelRight,
    Pencil,
    RefreshCw,
    Rows3,
    Settings,
    WrapText,
    X
  } from '@lucide/svelte';
  import AiPanel from '$lib/components/ai/AiPanel.svelte';
  import CodeViewer from '$lib/components/code/CodeViewer.svelte';
  import FileList from '$lib/components/code/FileList.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';
  import ErrorBanner from '$lib/components/common/ErrorBanner.svelte';
  import DiffFileList from '$lib/components/diff/DiffFileList.svelte';
  import DiffSummaryView from '$lib/components/diff/DiffSummary.svelte';
  import DiffViewer from '$lib/components/diff/DiffViewer.svelte';
  import ProjectDialog from '$lib/components/project/ProjectDialog.svelte';
  import ProjectSwitcher from '$lib/components/project/ProjectSwitcher.svelte';
  import type { AiAnswer, DiffExplanation, SourceReference } from '$lib/domain/ai';
  import type { CodeSelection } from '$lib/domain/code-selection';
  import { displayPath, type DiffSummary, type FileDiff } from '$lib/domain/diff';
  import { normalizeError, type AppError } from '$lib/domain/error';
  import type {
    FileContent,
    ProjectConfig,
    RepoFile,
    RepositoryInfo,
    SaveProjectInput
  } from '$lib/domain/project';
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

  let mode = $state<'review' | 'browse'>('review');
  let diffMode = $state<'split' | 'unified'>('split');
  let wrapLines = $state(false);
  let summary = $state<DiffSummary | null>(null);
  let selectedDiffPath = $state<string | undefined>();
  let selectedDiff = $state<FileDiff | null>(null);
  let repoFiles = $state<RepoFile[]>([]);
  let selectedFilePath = $state<string | undefined>();
  let fileContent = $state<FileContent | null>(null);
  let selection = $state<CodeSelection | null>(null);
  let aiAnswer = $state<AiAnswer | undefined>();
  let diffExplanation = $state<DiffExplanation | undefined>();
  let aiLoading = $state(false);
  let contentLoading = $state(false);
  let aiPanelOpen = $state(true);
  let targetLine = $state(0);
  let navigationLineNonce = 0;

  onMount(async () => {
    try {
      projects = await tauriApi.listProjects();
      const params = new URLSearchParams(window.location.search);
      const projectId = params.get('project');
      if (projectId) {
        const project = projects.find((item) => item.id === projectId);
        if (project) {
          mode = params.get('mode') === 'browse' ? 'browse' : 'review';
          await openProject(project, params.get('file') ?? undefined);
        }
      }
    } catch (caught) {
      error = normalizeError(caught);
    } finally {
      loading = false;
    }
  });

  function setUrl(file?: string) {
    if (!activeProject) return;
    const params = new URLSearchParams({ project: activeProject.id, mode });
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
    if (!confirm(`Remove ${project.name} from ReaDiff? The repository will not be changed.`)) return;
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

  async function loadWorkspace(requestedFile?: string) {
    if (!activeProject) return;
    workspaceError = null;
    summary = null;
    selectedDiff = null;
    fileContent = null;
    selection = null;
    aiAnswer = undefined;
    diffExplanation = undefined;
    try {
      const [loadedSummary, loadedFiles] = await Promise.all([
        tauriApi.getDiffSummary(activeProject.id),
        tauriApi.listRepositoryFiles(activeProject.id)
      ]);
      summary = loadedSummary;
      repoFiles = loadedFiles;
      if (mode === 'review') {
        const path = requestedFile && loadedSummary.files.some((file) => displayPath(file) === requestedFile)
          ? requestedFile
          : loadedSummary.files[0] ? displayPath(loadedSummary.files[0]) : undefined;
        if (path) await selectDiff(path);
      } else {
        const path = requestedFile && loadedFiles.some((file) => file.path === requestedFile)
          ? requestedFile
          : loadedFiles[0]?.path;
        if (path) await selectFile(path);
      }
      setUrl(mode === 'review' ? selectedDiffPath : selectedFilePath);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    }
  }

  async function switchMode(next: 'review' | 'browse') {
    if (mode === next) return;
    mode = next;
    selection = null;
    aiAnswer = undefined;
    diffExplanation = undefined;
    workspaceError = null;
    if (next === 'review' && !selectedDiff && summary?.files[0]) {
      await selectDiff(displayPath(summary.files[0]));
    }
    if (next === 'browse' && !fileContent && repoFiles[0]) {
      await selectFile(repoFiles[0].path);
    }
    setUrl(next === 'review' ? selectedDiffPath : selectedFilePath);
  }

  async function selectDiff(path: string) {
    if (!activeProject) return;
    selectedDiffPath = path;
    selectedDiff = null;
    diffExplanation = undefined;
    contentLoading = true;
    workspaceError = null;
    setUrl(path);
    try {
      selectedDiff = await tauriApi.getFileDiff(activeProject.id, path);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    } finally {
      contentLoading = false;
    }
  }

  async function selectFile(path: string, line?: number) {
    if (!activeProject) return;
    selectedFilePath = path;
    fileContent = null;
    selection = null;
    contentLoading = true;
    workspaceError = null;
    setUrl(path);
    try {
      fileContent = await tauriApi.readRepositoryFile(activeProject.id, path);
      if (line) {
        navigationLineNonce += 1;
        targetLine = line + navigationLineNonce * 0.0001;
      }
    } catch (caught) {
      workspaceError = normalizeError(caught);
    } finally {
      contentLoading = false;
    }
  }

  async function askAi(question: string) {
    if (!activeProject || !selection) return;
    aiLoading = true;
    aiAnswer = undefined;
    diffExplanation = undefined;
    workspaceError = null;
    try {
      aiAnswer = await tauriApi.askAboutCode(activeProject.id, selection, question);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    } finally {
      aiLoading = false;
    }
  }

  async function explainDiff() {
    if (!activeProject || !selectedDiffPath) return;
    aiLoading = true;
    diffExplanation = undefined;
    aiAnswer = undefined;
    workspaceError = null;
    try {
      diffExplanation = await tauriApi.explainFileDiff(activeProject.id, selectedDiffPath);
    } catch (caught) {
      workspaceError = normalizeError(caught);
    } finally {
      aiLoading = false;
    }
  }

  async function openReference(reference: SourceReference) {
    if (!repoFiles.some((file) => file.path === reference.path)) {
      workspaceError = { code: 'FILE_NOT_FOUND', message: `Referenced file is not available in the working tree: ${reference.path}` };
      return;
    }
    mode = 'browse';
    await selectFile(reference.path, reference.startLine);
  }

  function goHome() {
    activeProject = null;
    summary = null;
    selectedDiff = null;
    fileContent = null;
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
    return Number.isNaN(date.valueOf()) ? value : new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(date);
  }
</script>

<svelte:head>
  <title>{activeProject ? `${activeProject.name} · ReaDiff` : 'ReaDiff'}</title>
  <meta name="description" content="Understand code. Review changes." />
</svelte:head>

{#if activeProject}
  <main class="app-shell">
    <header class="topbar">
      <button class="brand compact" onclick={goHome} title="Back to projects"><span class="brand-mark"><Braces size={16} /></span><strong>ReaDiff</strong></button>
      <ProjectSwitcher {projects} {activeProject} onSelect={openProject} />
      <nav aria-label="Workspace mode">
        <button class:active={mode === 'review'} onclick={() => switchMode('review')}><GitBranch size={14} />Review</button>
        <button class:active={mode === 'browse'} onclick={() => switchMode('browse')}><Code2 size={14} />Browse</button>
      </nav>
      <div class="top-actions">
        <button onclick={() => loadWorkspace(mode === 'review' ? selectedDiffPath : selectedFilePath)} title="Refresh"><RefreshCw size={14} /></button>
        <button onclick={() => aiPanelOpen = !aiPanelOpen} class:active={aiPanelOpen} title="Toggle AI panel"><PanelRight size={14} /></button>
        <button onclick={editActiveProject} title="Project settings"><Settings size={14} /></button>
      </div>
    </header>

    {#if workspaceError}
      <div class="workspace-error"><ErrorBanner error={workspaceError} onDismiss={() => workspaceError = null} /></div>
    {/if}

    <div class:withoutAi={!aiPanelOpen} class="workspace">
      <aside class="sidebar">
        <div class="pane-title"><span>{mode === 'review' ? 'Changed files' : 'Repository files'}</span><b>{mode === 'review' ? (summary?.files.length ?? 0) : repoFiles.length}</b></div>
        {#if mode === 'review'}
          {#if summary}<DiffFileList files={summary.files} selectedPath={selectedDiffPath} onSelect={selectDiff} />{/if}
        {:else}
          <FileList files={repoFiles} selectedPath={selectedFilePath} onSelect={selectFile} />
        {/if}
      </aside>

      <section class="content-pane">
        {#if mode === 'review'}
          <div class="content-toolbar">
            {#if summary}<DiffSummaryView {summary} />{/if}
            <div class="view-controls">
              <button class:active={diffMode === 'split'} onclick={() => diffMode = 'split'} title="Split diff"><Columns2 size={13} />Split</button>
              <button class:active={diffMode === 'unified'} onclick={() => diffMode = 'unified'} title="Unified diff"><Rows3 size={13} />Unified</button>
              <button class:active={wrapLines} onclick={() => wrapLines = !wrapLines} title="Wrap long lines"><WrapText size={13} /></button>
            </div>
          </div>
          <div class="viewer-scroll">
            {#if contentLoading}<div class="loading-state"><LoaderCircle class="spin" size={20} />Loading diff…</div>
            {:else if selectedDiff}<DiffViewer diff={selectedDiff} mode={diffMode} wrap={wrapLines} />
            {:else if summary && summary.files.length === 0}<EmptyState
                icon={GitBranch}
                title="No changes"
                message={`The working tree has no changes relative to the merge base with ${activeProject.baseRef}.`}
                fill
              />
            {:else}<EmptyState icon={FileCode2} title="Choose a changed file" message="Select a file to inspect its working-tree diff." />{/if}
          </div>
        {:else}
          <div class="content-toolbar code-toolbar">
            <div class="current-file"><FileCode2 size={13} />{selectedFilePath ?? 'No file selected'}{#if fileContent}<span>{fileContent.lineCount} lines</span>{/if}</div>
            {#if selection}<button class="ask-selection" onclick={() => aiPanelOpen = true}><Bot size={13} />Ask AI about lines {selection.startLine}–{selection.endLine}</button>{/if}
          </div>
          <div class="code-host">
            {#if contentLoading}<div class="loading-state"><LoaderCircle class="spin" size={20} />Opening file…</div>
            {:else if fileContent}<CodeViewer path={fileContent.path} content={fileContent.content} language={fileContent.language} {targetLine} onSelection={(value) => selection = value} />
            {:else}<EmptyState icon={Code2} title="Open a tracked file" message="Choose a repository file to read it without modifying your working tree." />{/if}
          </div>
        {/if}
      </section>

      {#if aiPanelOpen}
        <AiPanel {mode} {selection} answer={aiAnswer} explanation={diffExplanation} loading={aiLoading} onAsk={askAi} onExplain={explainDiff} onReference={openReference} />
      {/if}
    </div>
  </main>
{:else}
  <main class="home">
    <header class="home-header">
      <div class="brand"><span class="brand-mark"><Braces size={20} /></span><div><strong>ReaDiff</strong><small>Understand code. Review changes.</small></div></div>
      <button class="settings-button" onclick={() => showSettings = true}><Settings size={14} />Settings</button>
    </header>

    <section class="projects-section">
      {#if error}<ErrorBanner {error} onDismiss={() => error = null} />{/if}
      {#if loading}
        <div class="loading-state"><LoaderCircle class="spin" size={20} />Loading projects…</div>
      {:else if projects.length}
        <div class="section-heading"><h1>Continue reviewing</h1><button onclick={addRepository}><CirclePlus size={14} />Add Project</button></div>
        <div class="project-grid">
          {#each projects as project (project.id)}
            <article>
              <button class="project-main" onclick={() => openProject(project)}>
                <div class="project-icon"><FolderGit2 size={19} /></div>
                <div class="project-info"><h3>{project.name}</h3><p>{project.repoPath}</p><span><GitBranch size={11} />{project.baseRef}</span></div>
              </button>
              <div class="project-meta">
                <button class="edit-project" onclick={() => editProject(project)} title={`Edit ${project.name}`} aria-label={`Edit ${project.name}`}><Pencil size={14} /></button>
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
    onClose={() => showProjectDialog = false}
  />
{/if}

{#if showSettings}
  <div class="settings-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && (showSettings = false)}>
    <div class="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <header><div><Settings size={17} /><h2 id="settings-title">AI settings</h2></div><button onclick={() => showSettings = false}><X size={17} /></button></header>
      <div class="settings-content">
        <div><span>Runtime</span><strong>codex exec</strong><p>ReaDiff invokes the Codex CLI available on <code>PATH</code> in a read-only, isolated temporary directory.</p></div>
        <div><span>Authentication</span><strong>codex login</strong><p>Saved Codex CLI authentication and your local Codex configuration are reused. ReaDiff removes API-key environment variables from the child process.</p></div>
        <div class="privacy"><Bot size={15} /><p>When you use AI, Codex receives the selected code and relevant diff context according to your local Codex configuration. AI descriptions are inferences and may be wrong.</p></div>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(:root) {
    font-family: Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
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
    --mono: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
    background: var(--bg);
  }

  :global(*) { box-sizing: border-box; }
  :global(html, body) { margin: 0; min-width: 900px; min-height: 100%; background: var(--bg); }
  :global(body) { color: var(--text); }
  :global(button) { font-family: inherit; }
  :global(::selection) { background: rgba(87, 184, 142, .3); }

  .home { min-height: 100vh; background:var(--bg); }
  .home-header { display:flex; align-items:center; justify-content:space-between; max-width:900px; height:72px; margin:auto; padding:0 28px; border-bottom:1px solid var(--border); }
  .brand { display:flex; align-items:center; gap:11px; }
  .brand.compact { padding:0; color:var(--text); background:none; border:0; cursor:pointer; }
  .brand-mark { display:grid; place-items:center; width:34px; height:34px; color:#07120e; background:var(--accent-bright); border-radius:8px; box-shadow:0 0 24px rgba(87,184,142,.14); }
  .brand strong { display:block; font-size:15px; letter-spacing:-.01em; }
  .brand small { display:block; margin-top:2px; color:var(--muted); font-size:9px; letter-spacing:.06em; }
  .settings-button, .section-heading button { display:flex; align-items:center; gap:7px; padding:7px 10px; color:#aab4be; background:rgba(16,24,33,.7); border:1px solid var(--border); border-radius:6px; font-size:11px; cursor:pointer; }
  .section-heading button { color:#07120e; background:var(--accent-bright); border-color:var(--accent-bright); font-weight:650; }
  .projects-section { max-width:900px; margin:auto; padding:36px 28px 72px; }
  .section-heading { display:flex; align-items:center; justify-content:space-between; margin-bottom:20px; }
  .section-heading h1 { margin:0; color:#eef3f6; font-size:22px; letter-spacing:-.025em; }
  .project-grid { display:grid; grid-template-columns:minmax(0,1fr); gap:10px; }
  article { display:grid; grid-template-columns:minmax(0,1fr) auto; overflow:hidden; background:#0d141c; border:1px solid var(--border); border-radius:8px; }
  article:hover { border-color:#2c3b48; background:#101821; }
  .project-main { display:grid; grid-template-columns:auto minmax(0,1fr); gap:12px; align-items:center; width:100%; padding:14px; color:var(--text); text-align:left; background:none; border:0; cursor:pointer; }
  .project-icon { display:grid; place-items:center; width:36px; height:36px; color:var(--accent-bright); background:rgba(87,184,142,.08); border-radius:7px; }
  .project-info { min-width:0; } .project-info h3 { margin:0; font-size:12px; } .project-info p { overflow:hidden; margin:4px 0 8px; color:#687581; font:9px var(--mono); text-overflow:ellipsis; white-space:nowrap; }
  .project-info span { display:flex; align-items:center; gap:4px; color:#8b9a95; font-size:9px; }
  .project-meta { display:flex; flex-direction:column; align-items:flex-end; justify-content:center; gap:7px; padding:10px 12px 10px 6px; }
  .project-meta time { color:#697681; font-size:9px; white-space:nowrap; }
  .edit-project { display:grid; place-items:center; width:28px; height:28px; padding:0; color:#687581; background:transparent; border:1px solid transparent; border-radius:6px; cursor:pointer; }
  .edit-project:hover { color:var(--text); background:var(--hover); border-color:var(--border-strong); }
  .empty-projects { display:grid; place-items:center; min-height:420px; padding:64px 40px; color:#596570; text-align:center; }
  .empty-project-icon { display:grid; place-items:center; width:56px; height:56px; color:var(--accent-bright); background:rgba(87,184,142,.08); border:1px solid rgba(87,184,142,.14); border-radius:14px; }
  .empty-projects h1 { margin:18px 0 7px; color:#eef3f6; font-size:22px; letter-spacing:-.025em; }
  .empty-projects p { margin:0; color:var(--muted); font-size:12px; }
  .empty-projects button { display:flex; align-items:center; gap:7px; margin-top:22px; padding:10px 14px; color:#07120e; background:var(--accent-bright); border:1px solid var(--accent-bright); border-radius:7px; font-size:12px; font-weight:650; cursor:pointer; }

  .app-shell { height:100vh; display:grid; grid-template-rows:48px minmax(0,1fr); overflow:hidden; }
  .topbar { display:grid; grid-template-columns:auto minmax(220px,1fr) auto auto; align-items:center; gap:17px; padding:0 11px; background:#0c1219; border-bottom:1px solid var(--border); }
  .topbar .brand-mark { width:27px; height:27px; border-radius:6px; } .topbar .brand { gap:7px; }
  nav { display:flex; align-self:stretch; } nav button { display:flex; align-items:center; gap:6px; padding:0 13px; color:#77838e; background:none; border:0; border-bottom:2px solid transparent; font-size:10px; cursor:pointer; } nav button:hover, nav button.active { color:var(--text); } nav button.active { border-bottom-color:var(--accent-bright); }
  .top-actions { display:flex; align-items:center; gap:2px; } .top-actions button { display:grid; place-items:center; width:28px; height:28px; color:#77838e; background:none; border:0; border-radius:5px; cursor:pointer; } .top-actions button:hover, .top-actions button.active { color:var(--text); background:var(--hover); }
  .workspace-error { position:fixed; z-index:20; top:57px; left:50%; width:min(520px,calc(100% - 40px)); transform:translateX(-50%); }
  .workspace { display:grid; grid-template-columns:225px minmax(500px,1fr) 290px; min-height:0; } .workspace.withoutAi { grid-template-columns:225px minmax(500px,1fr); }
  .sidebar { min-width:0; overflow:auto; background:#0c1219; border-right:1px solid var(--border); }
  .pane-title { display:flex; align-items:center; justify-content:space-between; min-height:38px; padding:0 12px; color:#78848e; border-bottom:1px solid var(--border); font-size:9px; font-weight:650; letter-spacing:.06em; text-transform:uppercase; } .pane-title b { display:grid; place-items:center; min-width:18px; height:17px; padding:0 4px; color:#7e8b96; background:#18212b; border-radius:8px; font-size:8px; }
  .content-pane { display:grid; grid-template-rows:39px minmax(0,1fr); min-width:0; min-height:0; overflow:hidden; background:#0b1016; }
  .content-toolbar { display:flex; align-items:center; justify-content:space-between; min-width:0; padding:0 10px 0 13px; background:#0e151d; border-bottom:1px solid var(--border); }
  .view-controls { display:flex; align-items:center; gap:2px; margin-left:12px; } .view-controls button { display:flex; align-items:center; gap:5px; height:25px; padding:0 7px; color:#6f7b86; background:none; border:1px solid transparent; border-radius:4px; font-size:9px; cursor:pointer; } .view-controls button:hover,.view-controls button.active { color:#bac3cc; background:#16202a; border-color:#24313d; }
  .viewer-scroll { min-height:0; overflow:auto; } .code-host { min-height:0; overflow:hidden; }
  .code-toolbar { gap:10px; } .current-file { display:flex; align-items:center; min-width:0; gap:7px; overflow:hidden; color:#aab5be; font:10px var(--mono); text-overflow:ellipsis; white-space:nowrap; } .current-file span { margin-left:5px; color:#596570; font:9px inherit; }
  .ask-selection { display:flex; align-items:center; gap:5px; padding:5px 8px; color:var(--accent-bright); background:rgba(87,184,142,.08); border:1px solid rgba(87,184,142,.18); border-radius:5px; font-size:9px; cursor:pointer; white-space:nowrap; }
  .loading-state { min-height:190px; display:flex; place-content:center; align-items:center; gap:9px; color:var(--muted); font-size:11px; }

  .settings-backdrop { position:fixed; z-index:50; inset:0; display:grid; place-items:center; padding:24px; background:rgba(4,7,11,.72); backdrop-filter:blur(6px); }
  .settings-dialog { width:min(490px,100%); background:#111821; border:1px solid var(--border-strong); border-radius:12px; box-shadow:0 24px 80px rgba(0,0,0,.4); }
  .settings-dialog header { display:flex; align-items:center; justify-content:space-between; padding:15px 17px; border-bottom:1px solid var(--border); } .settings-dialog header div { display:flex; align-items:center; gap:8px; } .settings-dialog h2 { margin:0; font-size:14px; } .settings-dialog header button { display:grid; padding:3px; color:var(--muted); background:none; border:0; cursor:pointer; }
  .settings-content { display:grid; gap:15px; padding:18px; } .settings-content > div:not(.privacy) { display:grid; grid-template-columns:100px 1fr; gap:3px 12px; padding-bottom:14px; border-bottom:1px solid var(--border); } .settings-content span { grid-row:1/3; color:var(--muted); font-size:10px; } .settings-content strong { font:11px var(--mono); } .settings-content p { margin:4px 0 0; color:var(--muted); font-size:10px; line-height:1.55; } .settings-content code { color:var(--accent-bright); font-family:var(--mono); }
  .privacy { display:flex; align-items:flex-start; gap:8px; padding:10px; color:#c2ad70; background:rgba(183,145,52,.07); border:1px solid rgba(183,145,52,.14); border-radius:6px; } .privacy p { margin:0; color:#a99361; }
  :global(.spin) { animation:spin 1s linear infinite; } @keyframes spin { to { transform:rotate(360deg); } }

  @media (max-width: 1100px) {
    .workspace { grid-template-columns:200px minmax(500px,1fr) 260px; }
  }
</style>
