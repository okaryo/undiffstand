<script module lang="ts">
  import {
    bundledLanguages,
    getDiffViewHighlighter,
    type BundledLanguage,
    type DiffHighlighter
  } from '@git-diff-view/shiki';

  let highlighterPromise: Promise<DiffHighlighter> | undefined;
  const languagePromises = new Map<string, Promise<void>>();

  async function loadHighlighter(language: string) {
    highlighterPromise ??= getDiffViewHighlighter();
    const highlighter = await highlighterPromise;

    if (language in bundledLanguages) {
      let languagePromise = languagePromises.get(language);
      if (!languagePromise) {
        languagePromise = highlighter
          .getHighlighterEngine()
          ?.loadLanguage(language as BundledLanguage);
        if (languagePromise) languagePromises.set(language, languagePromise);
      }
      await languagePromise;
    }

    return highlighter;
  }
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { DiffModeEnum, DiffView } from '@git-diff-view/svelte';
  import '@git-diff-view/svelte/styles/diff-view-pure.css';
  import { Binary, FileWarning } from '@lucide/svelte';
  import type { FileDiff } from '$lib/domain/diff';
  import EmptyState from '$lib/components/common/EmptyState.svelte';

  let { diff, mode, wrap }: { diff: FileDiff; mode: 'split' | 'unified'; wrap: boolean } = $props();
  let highlighter = $state<DiffHighlighter>();
  const language = $derived(
    ((diff.file.newPath ?? diff.file.oldPath ?? '').split('.').at(-1) ?? 'txt').toLowerCase()
  );

  onMount(() => {
    let mounted = true;

    void loadHighlighter(language)
      .then((loadedHighlighter) => {
        if (mounted) highlighter = loadedHighlighter;
      })
      .catch((error: unknown) => {
        console.error('Failed to initialize Shiki syntax highlighting.', error);
      });

    return () => {
      mounted = false;
    };
  });
</script>

{#if diff.file.status === 'binary'}
  <EmptyState
    icon={Binary}
    title="Binary file changed"
    message="Text diff is unavailable for this file."
  />
{:else if diff.hunks.length === 0}
  <EmptyState
    icon={FileWarning}
    title="No text hunks"
    message="This change does not contain a renderable text hunk."
  />
{:else}
  {#if diff.truncated}<div class="warning">
      This large diff was truncated for responsive display.
    </div>{/if}
  <div class="diff-host">
    <DiffView
      data={{
        oldFile: diff.file.oldPath
          ? { fileName: diff.file.oldPath, fileLang: language, content: diff.oldContent ?? '' }
          : undefined,
        newFile: diff.file.newPath
          ? { fileName: diff.file.newPath, fileLang: language, content: diff.newContent ?? '' }
          : undefined,
        // The library's `hunks` input expects complete per-file unified diff documents.
        // Keep that package-specific detail inside this adapter component.
        hunks: [diff.unifiedDiff]
      }}
      diffViewMode={mode === 'split' ? DiffModeEnum.Split : DiffModeEnum.Unified}
      diffViewTheme="dark"
      diffViewWrap={wrap}
      diffViewHighlight={highlighter !== undefined}
      registerHighlighter={highlighter}
      diffViewFontSize={12}
    />
  </div>
{/if}

<style>
  .warning {
    padding: 8px 12px;
    color: #d8b870;
    background: rgba(196, 148, 49, 0.09);
    border-bottom: 1px solid rgba(196, 148, 49, 0.2);
    font-size: 11px;
  }
  .diff-host {
    min-width: 720px;
    min-height: 100%;
    --diff-view-font-family: var(--mono);
  }
  .diff-host :global(.diff-view-wrapper) {
    border: 0 !important;
    border-radius: 0 !important;
  }
</style>
