<script lang="ts">
  import type { DiffSummary } from '$lib/domain/diff';

  let { summary }: { summary: DiffSummary } = $props();
  const filesLabel = $derived(
    `${summary.files.length} changed ${summary.files.length === 1 ? 'file' : 'files'}`
  );
  const additionsLabel = $derived(
    `${summary.totalAdditions} ${summary.totalAdditions === 1 ? 'addition' : 'additions'}`
  );
  const deletionsLabel = $derived(
    `${summary.totalDeletions} ${summary.totalDeletions === 1 ? 'deletion' : 'deletions'}`
  );
</script>

<div class="summary" aria-label={`${filesLabel}, ${additionsLabel}, ${deletionsLabel}`}>
  <strong class="files" title={filesLabel}>{summary.files.length}</strong>
  <strong class="addition" title={additionsLabel}>+{summary.totalAdditions}</strong>
  <strong class="deletion" title={deletionsLabel}>−{summary.totalDeletions}</strong>
</div>

<style>
  .summary {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  strong {
    font-size: 12px;
    font-weight: 650;
    line-height: 1;
    letter-spacing: 0;
    text-transform: none;
    white-space: nowrap;
  }
  .files {
    display: grid;
    place-items: center;
    min-width: 18px;
    height: 17px;
    padding: 0 4px;
    color: #7e8b96;
    background: #18212b;
    border-radius: 8px;
  }
  .addition {
    color: var(--green);
  }
  .deletion {
    color: var(--red);
  }
</style>
