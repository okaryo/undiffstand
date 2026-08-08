<script lang="ts">
  import { DiffModeEnum, DiffView } from '@git-diff-view/svelte';
  import '@git-diff-view/svelte/styles/diff-view-pure.css';
  import { Binary, FileWarning } from '@lucide/svelte';
  import type { FileDiff } from '$lib/domain/diff';
  import EmptyState from '$lib/components/common/EmptyState.svelte';

  let { diff, mode, wrap }: { diff: FileDiff; mode: 'split' | 'unified'; wrap: boolean } = $props();
  const language = $derived((diff.file.newPath ?? diff.file.oldPath ?? '').split('.').at(-1) ?? 'text');
</script>

{#if diff.file.status === 'binary'}
  <EmptyState icon={Binary} title="Binary file changed" message="Text diff is unavailable for this file." />
{:else if diff.hunks.length === 0}
  <EmptyState icon={FileWarning} title="No text hunks" message="This change does not contain a renderable text hunk." />
{:else}
  {#if diff.truncated}<div class="warning">This large diff was truncated for responsive display.</div>{/if}
  <div class="diff-host">
    <DiffView
      data={{
        oldFile: diff.file.oldPath ? { fileName: diff.file.oldPath, fileLang: language, content: diff.oldContent ?? '' } : undefined,
        newFile: diff.file.newPath ? { fileName: diff.file.newPath, fileLang: language, content: diff.newContent ?? '' } : undefined,
        // The library's `hunks` input expects complete per-file unified diff documents.
        // Keep that package-specific detail inside this adapter component.
        hunks: [diff.unifiedDiff]
      }}
      diffViewMode={mode === 'split' ? DiffModeEnum.Split : DiffModeEnum.Unified}
      diffViewTheme="dark"
      diffViewWrap={wrap}
      diffViewHighlight={true}
      diffViewFontSize={12}
    />
  </div>
{/if}

<style>
  .warning { padding: 8px 12px; color: #d8b870; background: rgba(196,148,49,.09); border-bottom: 1px solid rgba(196,148,49,.2); font-size: 11px; }
  .diff-host { min-width: 720px; min-height: 100%; --diff-view-font-family: var(--mono); }
  .diff-host :global(.diff-view-wrapper) { border: 0 !important; border-radius: 0 !important; }
</style>
