<script lang="ts">
  import { ArrowRight, GitCompareArrows } from '@lucide/svelte';
  import type { DiffSelection } from '$lib/domain/diff';
  import type { GitCommitSummary } from '$lib/domain/project';
  import RevisionSelect from './RevisionSelect.svelte';

  let {
    selection,
    recentBranches = [],
    localBranches = [],
    remoteBranches = [],
    recentCommits = [],
    currentBranch,
    loading = false,
    onApply
  }: {
    selection: DiffSelection;
    recentBranches?: string[];
    localBranches?: string[];
    remoteBranches?: string[];
    recentCommits?: GitCommitSummary[];
    currentBranch?: string | null;
    loading?: boolean;
    onApply: (selection: DiffSelection) => void | Promise<void>;
  } = $props();

  let base = $state('HEAD');
  let target = $state('.');
  let initializedKey = '';

  $effect(() => {
    const key = `${selection.base}\0${selection.target}`;
    if (key === initializedKey) return;
    initializedKey = key;
    base = selection.base;
    target = selection.target;
  });

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (loading || !base || !target) return;
    void onApply({ base, target });
  }
</script>

<form class="selector" aria-label="Choose changes to review" onsubmit={submit}>
  <GitCompareArrows size={15} />
  <span class="prefix">Changes from</span>
  <RevisionSelect
    value={base}
    label="From"
    {currentBranch}
    {recentBranches}
    {localBranches}
    {remoteBranches}
    {recentCommits}
    disabled={loading}
    onChange={(value) => (base = value)}
  />
  <ArrowRight size={14} />
  <span class="prefix">to</span>
  <RevisionSelect
    value={target}
    label="To"
    allowWorkingTree
    {currentBranch}
    {recentBranches}
    {localBranches}
    {remoteBranches}
    {recentCommits}
    disabled={loading}
    onChange={(value) => (target = value)}
  />
  <button type="submit" disabled={loading || !base || !target}
    >{loading ? 'Applying…' : 'Apply'}</button
  >
</form>

<style>
  .selector {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: #73808b;
  }
  .selector :global(svg) {
    flex: 0 0 auto;
  }
  .prefix {
    color: #7c8994;
    font-size: 11px;
    white-space: nowrap;
  }
  button {
    flex: 0 0 auto;
    height: 29px;
    padding: 0 10px;
    color: #07120e;
    background: var(--accent-bright);
    border: 1px solid var(--accent-bright);
    border-radius: 6px;
    font-size: 11px;
    font-weight: 650;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
