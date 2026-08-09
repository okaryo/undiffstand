<script lang="ts">
  import { Check, ChevronDown, ChevronRight } from '@lucide/svelte';
  import type { GitCommitSummary } from '$lib/domain/project';

  let {
    value,
    label,
    allowWorkingTree = false,
    currentBranch,
    recentBranches = [],
    localBranches = [],
    remoteBranches = [],
    recentCommits = [],
    disabled = false,
    onChange
  }: {
    value: string;
    label: string;
    allowWorkingTree?: boolean;
    currentBranch?: string | null;
    recentBranches?: string[];
    localBranches?: string[];
    remoteBranches?: string[];
    recentCommits?: GitCommitSummary[];
    disabled?: boolean;
    onChange: (value: string) => void;
  } = $props();

  let open = $state(false);
  let localOpen = $state(false);
  let remoteOpen = $state(false);
  let trigger: HTMLButtonElement;
  let menu = $state<HTMLDivElement>();

  let menuId = $derived(`revision-select-${label.toLowerCase()}-options`);
  let visibleRecentBranches = $derived(recentBranches.slice(0, 5));
  let visibleRecentCommits = $derived(recentCommits.slice(0, 10));

  function isKnownRevision(candidate: string) {
    return (
      candidate === 'HEAD' ||
      visibleRecentBranches.includes(candidate) ||
      localBranches.includes(candidate) ||
      remoteBranches.includes(candidate) ||
      visibleRecentCommits.some((commit) => commit.sha === candidate)
    );
  }

  function displayValue(candidate: string) {
    if (candidate === '.') return 'Working tree';
    if (candidate === 'HEAD') return `HEAD${currentBranch ? ` — ${currentBranch}` : ''}`;
    const commit = visibleRecentCommits.find((item) => item.sha === candidate);
    return commit ? `${commit.shortSha} — ${commit.subject}` : candidate;
  }

  function select(candidate: string) {
    onChange(candidate);
    open = false;
    trigger.focus();
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
    if (event.key === 'Escape' && open) {
      open = false;
      trigger.focus();
    }
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

{#snippet option(optionValue: string, optionLabel: string)}
  <button
    type="button"
    role="option"
    aria-selected={value === optionValue}
    class:active={value === optionValue}
    onclick={() => select(optionValue)}
  >
    <span>{optionLabel}</span>
    {#if value === optionValue}<Check size={13} />{/if}
  </button>
{/snippet}

<div class="revision-select">
  <button
    bind:this={trigger}
    type="button"
    class="trigger"
    class:open
    role="combobox"
    aria-label={label}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={menuId}
    {disabled}
    onclick={() => (open = !open)}
  >
    <span>{displayValue(value)}</span>
    <ChevronDown size={13} />
  </button>

  {#if open}
    <div bind:this={menu} id={menuId} class="menu" role="listbox" aria-label={`${label} revisions`}>
      {#if allowWorkingTree}
        <section aria-label="Workspace">
          <h3>Workspace</h3>
          {@render option('.', 'Working tree')}
        </section>
      {/if}

      {#if !isKnownRevision(value) && value !== '.'}
        <section aria-label="Selected">
          <h3>Selected</h3>
          {@render option(value, value)}
        </section>
      {/if}

      <section aria-label="Current">
        <h3>Current</h3>
        {@render option('HEAD', displayValue('HEAD'))}
      </section>

      {#if visibleRecentBranches.length}
        <section aria-label="Recent branches">
          <h3>Recent branches</h3>
          {#each visibleRecentBranches as branch (branch)}
            {@render option(branch, branch)}
          {/each}
        </section>
      {/if}

      {#if localBranches.length}
        <section aria-label="Local branches">
          <button
            type="button"
            class="group-toggle"
            aria-expanded={localOpen}
            onclick={() => (localOpen = !localOpen)}
          >
            {#if localOpen}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
            <span>Local branches</span>
            <small>{localBranches.length}</small>
          </button>
          {#if localOpen}
            {#each localBranches as branch (branch)}
              {@render option(branch, branch)}
            {/each}
          {/if}
        </section>
      {/if}

      {#if remoteBranches.length}
        <section aria-label="Remote branches">
          <button
            type="button"
            class="group-toggle"
            aria-expanded={remoteOpen}
            onclick={() => (remoteOpen = !remoteOpen)}
          >
            {#if remoteOpen}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
            <span>Remote branches</span>
            <small>{remoteBranches.length}</small>
          </button>
          {#if remoteOpen}
            {#each remoteBranches as branch (branch)}
              {@render option(branch, branch)}
            {/each}
          {/if}
        </section>
      {/if}

      {#if visibleRecentCommits.length}
        <section aria-label="Recent commits">
          <h3>Recent commits</h3>
          {#each visibleRecentCommits as commit (commit.sha)}
            {@render option(commit.sha, `${commit.shortSha} — ${commit.subject}`)}
          {/each}
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .revision-select {
    position: relative;
    min-width: 0;
  }

  .trigger {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    width: 245px;
    height: 29px;
    gap: 7px;
    padding: 0 8px;
    color: #cbd3da;
    text-align: left;
    background: #0a1016;
    border: 1px solid #293541;
    border-radius: 6px;
    outline: none;
    font: 11px var(--mono);
    cursor: pointer;
  }

  .trigger span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger.open,
  .trigger:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px rgba(87, 184, 142, 0.08);
  }

  .trigger:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .menu {
    position: absolute;
    z-index: 45;
    top: calc(100% + 6px);
    left: 0;
    width: 300px;
    max-height: min(460px, calc(100vh - 125px));
    padding: 6px;
    overflow-y: auto;
    background: #111923;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 14px 36px rgba(0, 0, 0, 0.42);
  }

  section + section {
    padding-top: 5px;
    margin-top: 5px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  h3 {
    margin: 0;
    padding: 5px 8px;
    color: #66737f;
    font-size: 10px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .menu button {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 15px;
    align-items: center;
    width: 100%;
    min-height: 29px;
    gap: 7px;
    padding: 5px 8px;
    color: #95a1ab;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: 5px;
    font: 11px var(--mono);
    cursor: pointer;
  }

  .menu button span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu button:hover,
  .menu button:focus-visible,
  .menu button.active {
    color: var(--text);
    background: var(--hover);
    outline: none;
  }

  .menu button.active {
    background: rgba(87, 184, 142, 0.1);
  }

  .menu .group-toggle {
    grid-template-columns: 14px minmax(0, 1fr) auto;
    color: #74818c;
    font-family: inherit;
    font-weight: 650;
  }

  .group-toggle small {
    color: #55616c;
    font: 10px var(--mono);
  }
</style>
