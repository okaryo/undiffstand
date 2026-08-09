<script lang="ts">
  import { Files, GitCommitHorizontal } from '@lucide/svelte';
  import type { DiffSummary } from '$lib/domain/diff';

  let { summary }: { summary: DiffSummary } = $props();
  const visibleSha = $derived(summary.comparison.toSha ?? summary.comparison.fromSha);
</script>

<div class="summary">
  <div class="metric">
    <Files size={14} /><strong>{summary.files.length}</strong><span>files</span>
  </div>
  <div class="metric"><strong class="addition">+{summary.totalAdditions}</strong></div>
  <div class="metric"><strong class="deletion">−{summary.totalDeletions}</strong></div>
  {#if visibleSha}
    <div class="commit" title={visibleSha}>
      <GitCommitHorizontal size={14} />
      <code>{visibleSha.slice(0, 8)}</code>
    </div>
  {/if}
</div>

<style>
  .summary {
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
  }
  .metric,
  .commit {
    display: flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
  }
  .metric {
    color: var(--muted);
    font-size: 11px;
  }
  strong {
    color: var(--text);
    font-size: 12px;
  }
  .addition {
    color: var(--green);
  }
  .deletion {
    color: var(--red);
  }
  .commit {
    margin-left: auto;
    color: var(--muted);
    font-size: 11px;
  }
  code {
    font-family: var(--mono);
  }
</style>
