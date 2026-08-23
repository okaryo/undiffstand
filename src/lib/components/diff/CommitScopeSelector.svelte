<script lang="ts">
  import {
    Check,
    ChevronDown,
    GitCommitHorizontal,
    GitMerge,
    ListTree,
    PencilLine,
  } from "@lucide/svelte";
  import type { ComparisonCommit, DiffScope } from "$lib/domain/diff";

  let {
    commits,
    scope,
    includeUncommitted,
    loading = false,
    onSelect,
  }: {
    commits: ComparisonCommit[];
    scope: DiffScope;
    includeUncommitted: boolean;
    loading?: boolean;
    onSelect: (scope: DiffScope) => void | Promise<void>;
  } = $props();

  let open = $state(false);
  let trigger: HTMLButtonElement;
  let menu = $state<HTMLDivElement>();
  let menuPosition = $state({
    top: 0,
    left: 0,
    width: 390,
    maxHeight: 440,
  });

  const selectedCommit = $derived(
    scope.kind === "commit"
      ? commits.find((commit) => commit.sha === scope.sha)
      : undefined,
  );
  const currentLabel = $derived(
    scope.kind === "all"
      ? "All changes"
      : scope.kind === "uncommitted"
        ? "Uncommitted changes"
        : selectedCommit
          ? `${selectedCommit.shortSha} ${selectedCommit.subject}`
          : scope.sha.slice(0, 7),
  );

  function select(nextScope: DiffScope) {
    open = false;
    onSelect(nextScope);
  }

  function positionMenu() {
    const viewportGutter = 8;
    const triggerRect = trigger.getBoundingClientRect();
    const width = Math.min(390, window.innerWidth - viewportGutter * 2);
    const left = Math.min(
      Math.max(viewportGutter, triggerRect.left),
      window.innerWidth - width - viewportGutter,
    );
    const top = triggerRect.bottom + 1;

    menuPosition = {
      top,
      left,
      width,
      maxHeight: Math.max(
        80,
        Math.min(440, window.innerHeight - top - viewportGutter),
      ),
    };
  }

  function toggleMenu() {
    if (open) {
      open = false;
      return;
    }
    positionMenu();
    open = true;
  }

  function handleViewportResize() {
    if (open) positionMenu();
  }

  function isSelected(candidate: DiffScope) {
    return (
      scope.kind === candidate.kind &&
      (scope.kind !== "commit" ||
        (candidate.kind === "commit" && scope.sha === candidate.sha))
    );
  }

  function handleWindowClick(event: MouseEvent) {
    if (
      open &&
      event.target instanceof Node &&
      !trigger.contains(event.target) &&
      !menu?.contains(event.target)
    )
      open = false;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && open) {
      event.preventDefault();
      open = false;
      trigger.focus();
    }
  }

  function formattedDate(value: string) {
    const date = new Date(value);
    return Number.isNaN(date.valueOf())
      ? value
      : new Intl.DateTimeFormat(undefined, {
          month: "short",
          day: "numeric",
          year: "numeric",
        }).format(date);
  }
</script>

<svelte:window
  onclick={handleWindowClick}
  onkeydown={handleWindowKeydown}
  onresize={handleViewportResize}
/>

<div class="commit-scope-selector">
  <button
    bind:this={trigger}
    class:open
    class="trigger"
    type="button"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={`Choose changes to view. Current: ${currentLabel}`}
    onclick={toggleMenu}
  >
    {#if scope.kind === "commit" && selectedCommit?.parentCount !== 1}
      <GitMerge size={14} />
    {:else if scope.kind === "commit"}
      <GitCommitHorizontal size={14} />
    {:else if scope.kind === "uncommitted"}
      <PencilLine size={14} />
    {:else}
      <ListTree size={14} />
    {/if}
    <span>{currentLabel}</span>
    <ChevronDown size={13} />
  </button>

  {#if open}
    <div
      bind:this={menu}
      class="menu"
      role="menu"
      aria-label="Comparison commits"
      style:top={`${menuPosition.top}px`}
      style:left={`${menuPosition.left}px`}
      style:width={`${menuPosition.width}px`}
      style:max-height={`${menuPosition.maxHeight}px`}
    >
      <button
        type="button"
        role="menuitemradio"
        aria-checked={isSelected({ kind: "all" })}
        class:active={isSelected({ kind: "all" })}
        onclick={() => select({ kind: "all" })}
      >
        <span class="icon"><ListTree size={15} /></span>
        <span class="details">
          <strong>All changes</strong>
          <small
            >{#if loading}Loading commits…{:else}{commits.length}
              {commits.length === 1 ? "commit" : "commits"}{/if}</small
          >
        </span>
        {#if isSelected({ kind: "all" })}<Check size={14} />{/if}
      </button>

      {#each commits as commit (commit.sha)}
        <button
          type="button"
          role="menuitemradio"
          aria-checked={isSelected({ kind: "commit", sha: commit.sha })}
          class:active={isSelected({ kind: "commit", sha: commit.sha })}
          onclick={() => select({ kind: "commit", sha: commit.sha })}
        >
          <span class="icon">
            {#if commit.parentCount !== 1}<GitMerge
                size={15}
              />{:else}<GitCommitHorizontal size={15} />{/if}
          </span>
          <span class="details">
            <strong>{commit.subject}</strong>
            <small
              >{commit.shortSha} · {commit.authorName} · {formattedDate(
                commit.authoredAt,
              )}</small
            >
          </span>
          {#if isSelected({ kind: "commit", sha: commit.sha })}<Check
              size={14}
            />{/if}
        </button>
      {/each}

      {#if includeUncommitted}
        <button
          type="button"
          role="menuitemradio"
          aria-checked={isSelected({ kind: "uncommitted" })}
          class:active={isSelected({ kind: "uncommitted" })}
          onclick={() => select({ kind: "uncommitted" })}
        >
          <span class="icon"><PencilLine size={15} /></span>
          <span class="details">
            <strong>Uncommitted changes</strong>
            <small>HEAD → working tree</small>
          </span>
          {#if isSelected({ kind: "uncommitted" })}<Check size={14} />{/if}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .commit-scope-selector {
    position: relative;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }

  .trigger {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    width: 100%;
    height: 31px;
    gap: 7px;
    padding: 0 8px;
    color: #aeb8c1;
    text-align: left;
    background: #111922;
    border: 1px solid #293541;
    border-radius: 6px;
    cursor: pointer;
  }

  .trigger span,
  .details strong,
  .details small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trigger:focus-visible,
  .trigger.open {
    border-color: var(--accent);
    outline: none;
    box-shadow: 0 0 0 2px rgba(87, 184, 142, 0.08);
  }

  .menu {
    position: fixed;
    z-index: 70;
    padding: 6px;
    overflow: auto;
    background: #111923;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 14px 36px rgba(0, 0, 0, 0.42);
  }

  .menu button {
    display: grid;
    grid-template-columns: 27px minmax(0, 1fr) 16px;
    align-items: center;
    width: 100%;
    gap: 9px;
    padding: 7px 8px;
    color: #82909c;
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: 6px;
    cursor: pointer;
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

  .icon {
    display: grid;
    place-items: center;
    width: 27px;
    height: 27px;
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.08);
    border-radius: 5px;
  }

  .details {
    display: grid;
    min-width: 0;
    gap: 3px;
  }

  .details strong {
    font-size: 12px;
  }

  .details small {
    color: #66737f;
    font: 11px var(--mono);
  }
</style>
