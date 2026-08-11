<script lang="ts">
  import {
    CirclePlus,
    FolderGit2,
    GitBranch,
    LoaderCircle,
    Pencil,
    Settings
  } from '@lucide/svelte';
  import ErrorBanner from '$lib/components/common/ErrorBanner.svelte';
  import type { AppError } from '$lib/domain/error';
  import type { ProjectConfig } from '$lib/domain/project';

  let {
    projects,
    loading = false,
    error = null,
    onAdd,
    onOpen,
    onEdit,
    onOpenSettings,
    onDismissError
  }: {
    projects: ProjectConfig[];
    loading?: boolean;
    error?: AppError | null;
    onAdd: () => void | Promise<void>;
    onOpen: (project: ProjectConfig) => void | Promise<void>;
    onEdit: (project: ProjectConfig) => void | Promise<void>;
    onOpenSettings: () => void;
    onDismissError: () => void;
  } = $props();

  function formatDate(value: string) {
    const date = new Date(value);
    return Number.isNaN(date.valueOf())
      ? value
      : new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
          date
        );
  }
</script>

<main class="home">
  <header>
    <div class="brand">
      <span class="brand-mark"><img src="/undiffstand-icon.png" alt="" /></span>
      <div>
        <strong>undiffstand</strong><small
          >AI-assisted diff understanding for human reviewers.</small
        >
      </div>
    </div>
    <button class="settings-button" onclick={onOpenSettings}><Settings size={14} />Settings</button>
  </header>

  <section class="projects-section">
    {#if error}<ErrorBanner {error} onDismiss={onDismissError} />{/if}
    {#if loading}
      <div class="loading-state"><LoaderCircle class="spin" size={20} />Loading projects…</div>
    {:else if projects.length}
      <div class="section-heading">
        <h1>Continue reviewing</h1>
        <button onclick={onAdd}><CirclePlus size={14} />Add Project</button>
      </div>
      <div class="project-grid">
        {#each projects as project (project.id)}
          <article>
            <button class="project-main" onclick={() => onOpen(project)}>
              <div class="project-icon"><FolderGit2 size={19} /></div>
              <div class="project-info">
                <h3>{project.name}</h3>
                <p>{project.repoPath}</p>
                <span><GitBranch size={11} />Current branch → working tree</span>
              </div>
            </button>
            <div class="project-meta">
              <button
                class="edit-project"
                onclick={() => onEdit(project)}
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
        <button onclick={onAdd}><CirclePlus size={16} />Add Project</button>
      </div>
    {/if}
  </section>
</main>

<style>
  .home {
    min-height: 100vh;
    background: var(--bg);
  }
  header {
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
  .brand-mark {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    overflow: hidden;
    border-radius: 8px;
    box-shadow: 0 0 24px rgba(87, 184, 142, 0.14);
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
  .brand small {
    display: block;
    margin-top: 2px;
    color: var(--muted);
    font-size: 12px;
    letter-spacing: 0.03em;
  }
  .settings-button,
  .section-heading button {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 10px;
    color: #aab4be;
    cursor: pointer;
    background: rgba(16, 24, 33, 0.7);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
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
    background: #101821;
    border-color: #2c3b48;
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
    cursor: pointer;
    background: none;
    border: 0;
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
    font: 12px/1.35 var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .project-info span {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #8b9a95;
    font-size: 12px;
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
    font-size: 12px;
    white-space: nowrap;
  }
  .edit-project {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    color: #687581;
    cursor: pointer;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
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
    color: #596570;
    text-align: center;
  }
  .loading-state {
    display: flex;
    min-height: 190px;
    place-content: center;
    align-items: center;
    gap: 9px;
    color: var(--muted);
    font-size: 12px;
  }
  .empty-projects {
    padding: 64px 40px;
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
    cursor: pointer;
    background: var(--accent-bright);
    border: 1px solid var(--accent-bright);
    border-radius: 7px;
    font-size: 12px;
    font-weight: 650;
  }
</style>
