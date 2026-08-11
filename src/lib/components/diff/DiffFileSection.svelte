<script lang="ts">
  import {
    Bot,
    Check,
    ChevronDown,
    ChevronRight,
    Copy,
    LoaderCircle,
    RotateCw,
    Sparkles
  } from '@lucide/svelte';
  import AiAnswer from '$lib/components/ai/AiAnswer.svelte';
  import type { ChangeReviewFinding, DiffExplanation, InlineAnswer } from '$lib/domain/ai';
  import type { DiffFileSummary, FileDiff } from '$lib/domain/diff';
  import DiffFileStatusIcon from './DiffFileStatusIcon.svelte';
  import DiffViewer from './DiffViewer.svelte';

  let {
    file,
    path,
    diff,
    loading = false,
    error,
    active = false,
    collapsed = false,
    copied = false,
    rendered = false,
    mode,
    wrap,
    explanation,
    aiLoading = false,
    aiError,
    findings = [],
    onToggle,
    onCopy,
    onLoad,
    onExplain,
    onAskInline
  }: {
    file: DiffFileSummary;
    path: string;
    diff?: FileDiff;
    loading?: boolean;
    error?: string;
    active?: boolean;
    collapsed?: boolean;
    copied?: boolean;
    rendered?: boolean;
    mode: 'split' | 'unified';
    wrap: boolean;
    explanation?: DiffExplanation;
    aiLoading?: boolean;
    aiError?: string;
    findings?: ChangeReviewFinding[];
    onToggle: () => void;
    onCopy: () => void;
    onLoad: () => void;
    onExplain: () => void;
    onAskInline: (
      side: 'old' | 'new',
      startLine: number,
      endLine: number,
      question: string
    ) => Promise<InlineAnswer>;
  } = $props();
</script>

<header class="file-header" class:active>
  <button
    class="collapse-button"
    aria-expanded={!collapsed}
    aria-label={`${collapsed ? 'Expand' : 'Collapse'} diff for ${path}`}
    title={collapsed ? 'Expand file diff' : 'Collapse file diff'}
    onclick={onToggle}
  >
    {#if collapsed}<ChevronRight size={14} />{:else}<ChevronDown size={14} />{/if}
  </button>
  <DiffFileStatusIcon status={file.status} />
  <div class="file-path" title={path}>
    <strong>{path}</strong>
    <button
      class="copy-path"
      aria-label={`Copy path ${path}`}
      title={copied ? 'Copied' : 'Copy path'}
      onclick={onCopy}
    >
      {#if copied}<Check size={13} />{:else}<Copy size={13} />{/if}
    </button>
  </div>
  <span class="file-counts">
    {#if file.additions !== undefined}<b>+{file.additions}</b>{/if}
    {#if file.deletions !== undefined}<em>−{file.deletions}</em>{/if}
  </span>
  <button
    class="explain-file"
    disabled={aiLoading || file.status === 'binary'}
    aria-label={`Explain changes in ${path}`}
    title="Explain this file's changes"
    onclick={onExplain}
    >{#if aiLoading}<LoaderCircle class="spin" size={13} />{:else}<Bot size={13} />{/if}<span
      >Explain</span
    ></button
  >
</header>

{#if explanation || aiLoading || aiError}
  <div class="file-ai">
    <div class="file-ai-title"><Sparkles size={13} /><strong>Change explanation</strong></div>
    {#if aiLoading && !explanation}<div class="file-ai-loading">
        <LoaderCircle class="spin" size={14} />Explaining this file's changes…
      </div>{/if}
    {#if aiError}<p class="file-ai-error">{aiError}</p>{/if}
    <AiAnswer {explanation} />
  </div>
{/if}

{#if !collapsed}
  <div class="diff-body">
    {#if rendered && diff}
      <DiffViewer {diff} {mode} {wrap} {findings} {onAskInline} />
    {:else if error}
      <div class="file-error">
        <span>{error}</span>
        <button onclick={onLoad}><RotateCw size={13} />Retry</button>
      </div>
    {:else}
      <div class="file-loading">
        {#if loading}<LoaderCircle class="spin" size={16} />Loading diff…{:else if diff}<LoaderCircle
            class="spin"
            size={16}
          />Preparing diff…{:else}Waiting to load…{/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .file-header {
    position: sticky;
    z-index: 5;
    top: 0;
    display: grid;
    grid-template-columns: 22px 14px minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 7px;
    min-height: 36px;
    padding: 0 12px 0 7px;
    background: #111923;
    border-top: 1px solid #202b36;
    border-bottom: 1px solid #202b36;
    border-left: 2px solid transparent;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.2);
  }
  .file-header.active {
    border-left-color: var(--accent-bright);
  }
  .collapse-button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 24px;
    padding: 0;
    color: #75818d;
    cursor: pointer;
    background: transparent;
    border: 0;
    border-radius: 4px;
  }
  .collapse-button:hover {
    color: var(--text);
    background: var(--hover);
  }
  .file-path {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 5px;
  }
  .file-path strong {
    min-width: 0;
    overflow: hidden;
    color: #d2d9df;
    font: 600 12px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .copy-path {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    color: #66727d;
    cursor: pointer;
    background: transparent;
    border: 0;
    border-radius: 4px;
  }
  .copy-path:hover {
    color: #c6ced5;
    background: var(--hover);
  }
  .file-counts {
    display: flex;
    gap: 5px;
    font-size: 12px;
  }
  .file-counts b {
    color: var(--green);
    font-weight: 500;
  }
  .file-counts em {
    color: var(--red);
    font-style: normal;
  }
  .explain-file {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 7px;
    color: #91aa9f;
    cursor: pointer;
    background: rgba(87, 184, 142, 0.06);
    border: 1px solid rgba(87, 184, 142, 0.16);
    border-radius: 5px;
    font-size: 12px;
  }
  .explain-file:hover:not(:disabled) {
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.11);
  }
  .explain-file:disabled {
    cursor: default;
    opacity: 0.45;
  }
  .file-ai {
    margin: 10px;
    color: var(--text);
    background: #101821;
    border: 1px solid #263440;
    border-left: 3px solid var(--accent);
    border-radius: 7px;
  }
  .file-ai-title {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 9px 12px;
    color: var(--accent-bright);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }
  .file-ai-loading {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 13px;
    color: var(--muted);
    font-size: 12px;
  }
  .file-ai-error {
    margin: 0;
    padding: 12px;
    color: var(--red);
    font-size: 12px;
  }
  .diff-body {
    min-height: 150px;
  }
  .file-loading,
  .file-error {
    display: flex;
    min-height: 150px;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #687581;
    font-size: 12px;
  }
  .file-error {
    color: #c7867c;
  }
  .file-error button {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 8px;
    color: #aeb8c1;
    cursor: pointer;
    background: #16202a;
    border: 1px solid #293642;
    border-radius: 5px;
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
