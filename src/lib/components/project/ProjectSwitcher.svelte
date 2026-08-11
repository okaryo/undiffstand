<script lang="ts">
  import {
    Check,
    ChevronDown,
    ChevronRight,
    FolderGit2,
    GitCompareArrows,
    Settings
  } from '@lucide/svelte';
  import type { ProjectConfig } from '$lib/domain/project';

  let {
    projects,
    activeProject,
    comparisonLabel,
    onEditComparison,
    onEditProject,
    onSelect
  }: {
    projects: ProjectConfig[];
    activeProject: ProjectConfig;
    comparisonLabel: string;
    onEditComparison: () => void;
    onEditProject: () => void | Promise<void>;
    onSelect: (project: ProjectConfig) => void | Promise<void>;
  } = $props();

  let open = $state(false);
  let trigger: HTMLButtonElement;
  let menu = $state<HTMLDivElement>();

  function selectProject(project: ProjectConfig) {
    open = false;
    if (project.id !== activeProject.id) onSelect(project);
  }

  function editProject() {
    open = false;
    onEditProject();
  }

  function handleWindowClick(event: MouseEvent) {
    if (
      open &&
      event.target instanceof Node &&
      !trigger.contains(event.target) &&
      !menu?.contains(event.target)
    ) {
      open = false;
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') open = false;
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<div class="project-switcher">
  <span class="separator"><ChevronRight size={13} /></span>
  <button
    class:open
    class="trigger"
    type="button"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={`Switch project. Current: ${activeProject.name}`}
    onclick={() => (open = !open)}
    bind:this={trigger}
  >
    <FolderGit2 size={14} />
    <strong>{activeProject.name}</strong>
    <span class="chevron"><ChevronDown size={13} /></span>
  </button>
  <button
    class="comparison-trigger"
    type="button"
    aria-label={`Change comparison. Current: ${comparisonLabel}`}
    title="Change comparison"
    onclick={onEditComparison}
  >
    <span class="comparison">{comparisonLabel}</span>
    <GitCompareArrows size={13} />
  </button>

  {#if open}
    <div class="menu" role="menu" aria-label="Projects" bind:this={menu}>
      <p>Switch project</p>
      {#each projects as project (project.id)}
        <button
          type="button"
          role="menuitemradio"
          aria-checked={project.id === activeProject.id}
          class:active={project.id === activeProject.id}
          onclick={() => selectProject(project)}
        >
          <span class="project-icon"><FolderGit2 size={15} /></span>
          <span class="project-details">
            <strong>{project.name}</strong>
            <small>{project.repoPath}</small>
          </span>
          {#if project.id === activeProject.id}<span class="selected-check"
              ><Check size={14} /></span
            >{/if}
        </button>
      {/each}
      <div class="menu-actions">
        <button type="button" role="menuitem" onclick={editProject}>
          <span class="project-icon"><Settings size={15} /></span>
          <span class="project-details"><strong>Project settings</strong></span>
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .project-switcher {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    gap: 5px;
    color: #5b6772;
    font-size: 12px;
  }

  .separator,
  .chevron,
  .selected-check {
    display: grid;
    place-items: center;
  }

  .separator {
    flex: 0 0 auto;
  }

  .trigger {
    display: flex;
    align-items: center;
    min-width: 0;
    gap: 6px;
    padding: 5px 6px;
    color: #aeb8c1;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 5px;
    cursor: pointer;
  }

  .trigger:hover,
  .trigger.open {
    color: var(--text);
    background: var(--hover);
    border-color: var(--border);
  }

  .trigger strong {
    overflow: hidden;
    max-width: 210px;
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    flex: 0 0 auto;
  }

  .comparison {
    overflow: hidden;
    font: 12px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .comparison-trigger {
    display: flex;
    align-items: center;
    min-width: 0;
    height: 24px;
    gap: 5px;
    padding: 0 5px;
    color: #647079;
    background: transparent;
    border: 0;
    border-radius: 5px;
    cursor: pointer;
  }

  .comparison-trigger :global(svg) {
    flex: 0 0 auto;
  }

  .comparison-trigger:hover {
    color: var(--text);
    background: var(--hover);
  }

  .menu {
    position: absolute;
    z-index: 40;
    top: calc(100% + 8px);
    left: 18px;
    width: min(340px, calc(100vw - 42px));
    padding: 6px;
    background: #111923;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 14px 36px rgba(0, 0, 0, 0.42);
  }

  .menu p {
    margin: 0;
    padding: 6px 8px 7px;
    color: #66737f;
    font-size: 12px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .menu button {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) 16px;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 8px;
    color: #82909c;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: 6px;
    cursor: pointer;
  }

  .menu button:hover,
  .menu button.active {
    color: var(--text);
    background: var(--hover);
  }

  .menu button.active {
    background: rgba(87, 184, 142, 0.1);
  }
  .menu-actions {
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
  .selected-check {
    color: var(--accent-bright);
  }

  .project-icon {
    display: grid;
    place-items: center;
    width: 27px;
    height: 27px;
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.08);
    border-radius: 5px;
  }

  .project-details {
    display: grid;
    min-width: 0;
    gap: 3px;
  }

  .project-details strong,
  .project-details small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .project-details strong {
    font-size: 12px;
  }
  .project-details small {
    color: #66737f;
    font: 12px var(--mono);
  }
</style>
