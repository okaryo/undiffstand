<script module lang="ts">
  import { SvelteMap } from 'svelte/reactivity';
  import {
    bundledLanguages,
    getDiffViewHighlighter,
    type BundledLanguage,
    type DiffHighlighter
  } from '@git-diff-view/shiki';

  let highlighterPromise: Promise<DiffHighlighter> | undefined;
  const languagePromises = new SvelteMap<string, Promise<void>>();

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
  import { DiffModeEnum, DiffViewWithMultiSelect, SplitSide } from '@git-diff-view/svelte';
  import '@git-diff-view/svelte/styles/diff-view-pure.css';
  import { Binary, FileExclamationPoint } from '@lucide/svelte';
  import type { FileDiff } from '$lib/domain/diff';
  import type { ChangeReviewFinding, InlineAnswer } from '$lib/domain/ai';
  import InlineAsk from '$lib/components/ai/InlineAsk.svelte';
  import ReviewFinding from '$lib/components/ai/ReviewFinding.svelte';
  import EmptyState from '$lib/components/common/EmptyState.svelte';

  let {
    diff,
    mode,
    wrap,
    findings = [],
    onAskInline
  }: {
    diff: FileDiff;
    mode: 'split' | 'unified';
    wrap: boolean;
    findings?: ChangeReviewFinding[];
    onAskInline?: (
      side: 'old' | 'new',
      startLine: number,
      endLine: number,
      question: string
    ) => Promise<InlineAnswer>;
  } = $props();
  let highlighter = $state<DiffHighlighter>();
  let widgetState = $state<{ side: SplitSide; lineNumber: number }>();
  let diffInstance:
    | {
        clearSelection: () => void;
        setPreselectedLines: (lines: { old: number[]; new: number[] }) => void;
      }
    | undefined;
  const language = $derived(
    ((diff.file.newPath ?? diff.file.oldPath ?? '').split('.').at(-1) ?? 'txt').toLowerCase()
  );
  const diffData = $derived.by(() => ({
    oldFile: diff.file.oldPath
      ? { fileName: diff.file.oldPath, fileLang: language, content: diff.oldContent ?? '' }
      : undefined,
    newFile: diff.file.newPath
      ? { fileName: diff.file.newPath, fileLang: language, content: diff.newContent ?? '' }
      : undefined,
    hunks: [diff.unifiedDiff]
  }));
  const extendData = $derived.by(() => {
    const oldFile: Record<string, { data: ChangeReviewFinding[] }> = {};
    const newFile: Record<string, { data: ChangeReviewFinding[] }> = {};
    for (const finding of findings) {
      const target = finding.side === 'old' ? oldFile : newFile;
      const key = String(finding.endLine);
      (target[key] ??= { data: [] }).data.push(finding);
    }
    return { oldFile, newFile };
  });

  function sideName(side: SplitSide): 'old' | 'new' {
    return side === SplitSide.old ? 'old' : 'new';
  }

  function askInline(side: SplitSide, startLine: number, endLine: number, question: string) {
    if (!onAskInline) return Promise.reject(new Error('Inline Ask is unavailable.'));
    return onAskInline(sideName(side), startLine, endLine, question);
  }

  function openInlineForSelection(result: {
    range: { side: 'old' | 'new'; startLineNumber: number; endLineNumber: number };
  }) {
    widgetState = {
      side: result.range.side === 'old' ? SplitSide.old : SplitSide.new,
      lineNumber: Math.max(result.range.startLineNumber, result.range.endLineNumber)
    };
  }

  function closeInline(onClose: () => void) {
    onClose();
    widgetState = undefined;
    diffInstance?.clearSelection();
    diffInstance?.setPreselectedLines({ old: [], new: [] });
  }

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
    icon={FileExclamationPoint}
    title="No text hunks"
    message="This change does not contain a renderable text hunk."
  />
{:else}
  {#if diff.truncated}<div class="warning">
      This large diff was truncated for responsive display.
    </div>{/if}
  <div class="diff-host">
    <DiffViewWithMultiSelect
      data={diffData}
      diffViewMode={mode === 'split' ? DiffModeEnum.Split : DiffModeEnum.Unified}
      diffViewTheme="dark"
      diffViewWrap={wrap}
      diffViewHighlight={highlighter !== undefined}
      registerHighlighter={highlighter}
      diffViewFontSize={12}
      diffViewAddWidget={onAskInline !== undefined}
      enableMultiSelect={onAskInline !== undefined}
      initialWidgetState={widgetState}
      onInstanceCreated={(instance) => (diffInstance = instance)}
      onMultiSelectComplete={openInlineForSelection}
      {extendData}
      renderWidgetLine={InlineWidget}
      renderExtendLine={FindingLine}
    />
  </div>
{/if}

{#snippet InlineWidget({
  lineNumber,
  fromLineNumber,
  side,
  onClose
}: {
  lineNumber: number;
  fromLineNumber: number;
  side: SplitSide;
  onClose: () => void;
})}
  <InlineAsk
    side={sideName(side)}
    startLine={fromLineNumber}
    endLine={lineNumber}
    onAsk={(question) => askInline(side, fromLineNumber, lineNumber, question)}
    onClose={() => closeInline(onClose)}
  />
{/snippet}

{#snippet FindingLine({ data }: { data: ChangeReviewFinding[] })}
  <ReviewFinding findings={data} />
{/snippet}

<style>
  .warning {
    padding: 8px 12px;
    color: #d8b870;
    background: rgba(196, 148, 49, 0.09);
    border-bottom: 1px solid rgba(196, 148, 49, 0.2);
    font-size: 12px;
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
  .diff-host :global(.diff-add-widget) {
    pointer-events: none;
    color: #07130e !important;
    background: var(--accent-bright) !important;
  }
  .diff-host :global(.diff-widget-tooltip::after) {
    font-size: 12px;
  }
  .diff-host :global(.diff-multi-select-active.diff-line-content),
  .diff-host :global(.diff-multi-select-active.diff-line-old-content),
  .diff-host :global(.diff-multi-select-active.diff-line-new-content) {
    box-shadow: inset 3px 0 rgba(99, 198, 154, 0.9);
  }
</style>
