<script lang="ts">
  import { Check, GitBranch, X } from "@lucide/svelte";
  import type { DiffSelection } from "$lib/domain/diff";
  import type { RepositoryInfo } from "$lib/domain/project";
  import DiffSelector from "./DiffSelector.svelte";

  let {
    selection,
    repository,
    activeBaseRef,
    baseToCurrentIsActive = false,
    currentToWorkingTreeIsActive = false,
    onApply,
    onConfigureBase,
    onClose,
  }: {
    selection: DiffSelection;
    repository: RepositoryInfo;
    activeBaseRef?: string | null;
    baseToCurrentIsActive?: boolean;
    currentToWorkingTreeIsActive?: boolean;
    onApply: (selection: DiffSelection) => void | Promise<void>;
    onConfigureBase: () => void | Promise<void>;
    onClose: () => void;
  } = $props();
</script>

<div
  class="backdrop"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && onClose()}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="comparison-dialog-title"
  >
    <header>
      <div>
        <GitBranch size={17} />
        <h2 id="comparison-dialog-title">Change comparison</h2>
      </div>
      <button aria-label="Close comparison dialog" onclick={onClose}
        ><X size={17} /></button
      >
    </header>
    <div class="content">
      <section class="editor" aria-label="Comparison">
        <DiffSelector
          {selection}
          recentBranches={repository.recentBranches}
          localBranches={repository.localBranches}
          remoteBranches={repository.remoteBranches}
          recentCommits={repository.recentCommits}
          currentBranch={repository.currentBranch}
          {onApply}
        />
      </section>
      <section
        class="quick-comparisons"
        aria-labelledby="quick-comparisons-title"
      >
        <div class="section-heading">
          <div>
            <h3 id="quick-comparisons-title">Quick comparisons</h3>
            <p>Apply a common comparison immediately.</p>
          </div>
          {#if !activeBaseRef}
            <button
              type="button"
              class="settings-link"
              onclick={onConfigureBase}
              >Set base branch in Project settings</button
            >
          {/if}
        </div>
        <div class="quick-comparison-list">
          <div class="quick-comparison-row">
            <div>
              <strong
                >{activeBaseRef ?? "Base branch"} → {repository.currentBranch ??
                  "Current branch"}</strong
              >
              <span>Base branch → Current branch</span>
            </div>
            {#if baseToCurrentIsActive}
              <span
                class="current-comparison"
                role="status"
                aria-label="Current comparison"><Check size={13} />Current</span
              >
            {:else if activeBaseRef && activeBaseRef === repository.currentBranch}
              <span class="comparison-unavailable">Same branch</span>
            {:else}
              <button
                type="button"
                class="compare-button"
                aria-label={`Compare ${activeBaseRef ?? "base branch"} → ${repository.currentBranch ?? "current branch"}`}
                disabled={!activeBaseRef || !repository.currentBranch}
                title={!activeBaseRef
                  ? "Set a base branch in Project settings."
                  : undefined}
                onclick={() =>
                  activeBaseRef &&
                  onApply({ base: activeBaseRef, target: "HEAD" })}
                >Compare</button
              >
            {/if}
          </div>
          <div class="quick-comparison-row">
            <div>
              <strong
                >{repository.currentBranch ?? "HEAD"} → Working tree</strong
              >
              <span>Current branch → Working tree</span>
            </div>
            {#if currentToWorkingTreeIsActive}
              <span
                class="current-comparison"
                role="status"
                aria-label="Current comparison"><Check size={13} />Current</span
              >
            {:else}
              <button
                type="button"
                class="compare-button"
                aria-label={`Compare ${repository.currentBranch ?? "HEAD"} → Working tree`}
                onclick={() => onApply({ base: "HEAD", target: "." })}
                >Compare</button
              >
            {/if}
          </div>
        </div>
      </section>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    z-index: 60;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 7, 11, 0.72);
    backdrop-filter: blur(6px);
  }
  .dialog {
    width: min(760px, 100%);
    background: #111821;
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 15px 17px;
    border-bottom: 1px solid var(--border);
  }
  header div {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
  }
  header button {
    display: grid;
    padding: 3px;
    color: var(--muted);
    background: none;
    border: 0;
    cursor: pointer;
  }
  header button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .content {
    display: grid;
    gap: 20px;
    padding: 20px;
  }
  .content h3 {
    margin: 0;
    color: #87939e;
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }
  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .section-heading > div {
    display: grid;
    gap: 4px;
  }
  .section-heading p {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
  }
  .quick-comparisons {
    display: grid;
    gap: 10px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
  .quick-comparison-list {
    overflow: hidden;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
  }
  .quick-comparison-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    min-height: 54px;
    padding: 9px 11px;
    background: #0b1118;
  }
  .quick-comparison-row + .quick-comparison-row {
    border-top: 1px solid var(--border);
  }
  .quick-comparison-row > div {
    display: grid;
    min-width: 0;
    gap: 4px;
  }
  .quick-comparison-row strong {
    overflow: hidden;
    color: #c3ccd4;
    font: 12px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .quick-comparison-row > div > span {
    color: var(--muted);
    font-size: 11px;
  }
  .compare-button {
    min-width: 70px;
    height: 29px;
    padding: 0 10px;
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.08);
    border: 1px solid rgba(87, 184, 142, 0.35);
    border-radius: 6px;
    font-size: 11px;
    font-weight: 650;
    cursor: pointer;
  }
  .compare-button:hover {
    color: #07120e;
    background: var(--accent-bright);
    border-color: var(--accent-bright);
  }
  .compare-button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .current-comparison {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 7px;
    color: #8fb7d9;
    background: rgba(91, 137, 176, 0.14);
    border-radius: 5px;
    font-size: 11px;
    font-weight: 650;
  }
  .comparison-unavailable {
    color: var(--muted);
    font-size: 11px;
  }
  .settings-link {
    padding: 0;
    color: var(--accent-bright);
    background: none;
    border: 0;
    font-size: 11px;
    cursor: pointer;
  }
  .content :global(.selector) {
    justify-content: center;
  }
  @media (max-width: 680px) {
    .content :global(.selector) {
      flex-wrap: wrap;
    }
  }
</style>
