<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    Check,
    ChevronDown,
    ChevronRight,
    Copy,
    LoaderCircle,
    RotateCw,
    Sparkles
  } from '@lucide/svelte';
  import type { ChangeReviewFinding, DiffExplanation, InlineAnswer } from '$lib/domain/ai';
  import AiAnswer from '$lib/components/ai/AiAnswer.svelte';
  import { diffAnchorId, displayPath, type DiffFileSummary, type FileDiff } from '$lib/domain/diff';
  import DiffFileStatusIcon from './DiffFileStatusIcon.svelte';
  import DiffViewer from './DiffViewer.svelte';

  let {
    files,
    diffs,
    loadingPaths,
    errors,
    activePath,
    mode,
    wrap,
    fileExplanations = {},
    fileAiLoading = {},
    fileAiErrors = {},
    findings = [],
    onLoad,
    onActive,
    onExplainFile = () => {},
    onAskInline = () => Promise.reject(new Error('Inline Ask is unavailable.'))
  }: {
    files: DiffFileSummary[];
    diffs: Record<string, FileDiff | undefined>;
    loadingPaths: Record<string, boolean | undefined>;
    errors: Record<string, string | undefined>;
    activePath?: string;
    mode: 'split' | 'unified';
    wrap: boolean;
    fileExplanations?: Record<string, DiffExplanation | undefined>;
    fileAiLoading?: Record<string, boolean | undefined>;
    fileAiErrors?: Record<string, string | undefined>;
    findings?: ChangeReviewFinding[];
    onLoad: (path: string) => void;
    onActive: (path: string) => void;
    onExplainFile?: (path: string) => void;
    onAskInline?: (
      path: string,
      side: 'old' | 'new',
      startLine: number,
      endLine: number,
      question: string
    ) => Promise<InlineAnswer>;
  } = $props();

  let feedElement = $state<HTMLElement>();
  let scroller: HTMLElement | null = null;
  let renderObserver: IntersectionObserver | undefined;
  let activeObserver: IntersectionObserver | undefined;
  let renderedPaths = $state<Record<string, boolean | undefined>>({});
  let requestedPathsVersion = $state(0);
  let lastFilesKey = $state('');
  let collapsed = $state<Record<string, boolean>>({});
  let copiedPath = $state<string>();
  let copyResetTimer: ReturnType<typeof setTimeout> | undefined;
  let renderTimer: ReturnType<typeof setTimeout> | undefined;
  const sectionElements = new Map<string, HTMLElement>();
  const requestedPaths = new Set<string>();
  const pendingRenderPaths: string[] = [];
  const pendingRenderPathSet = new Set<string>();
  const activeSectionPaths = new Set<string>();
  const INITIAL_RENDERED_FILE_COUNT = 8;
  const LAZY_RENDER_ROOT_MARGIN = '1200px 0px';
  const ACTIVE_FILE_OFFSET = 52;
  const RENDER_INTERVAL_MS = 32;

  onDestroy(() => {
    if (copyResetTimer !== undefined) clearTimeout(copyResetTimer);
    if (renderTimer !== undefined) clearTimeout(renderTimer);
  });

  onMount(() => {
    scroller =
      (feedElement?.closest('.viewer-scroll') as HTMLElement | null) ?? feedElement ?? null;
    setupRenderObserver();
    setupActiveObserver();
    window.addEventListener('resize', setupActiveObserver);

    return () => {
      window.removeEventListener('resize', setupActiveObserver);
      renderObserver?.disconnect();
      activeObserver?.disconnect();
    };
  });

  $effect(() => {
    const filesKey = files.map(displayPath).join('\0');
    if (filesKey === lastFilesKey) return;
    lastFilesKey = filesKey;
    resetDeferredRendering();
    for (const file of files.slice(0, INITIAL_RENDERED_FILE_COUNT)) {
      requestFileRender(displayPath(file));
    }
    if (activePath) requestFileRender(activePath, true);
    observeDeferredSections();
  });

  $effect(() => {
    const path = activePath;
    if (path) requestFileRender(path, true);
  });

  $effect(() => {
    void requestedPathsVersion;
    scheduleReadyRenders();
  });

  function setupRenderObserver() {
    renderObserver?.disconnect();
    if (!scroller) return;
    renderObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const path = (entry.target as HTMLElement).dataset.diffPath;
          if (!entry.isIntersecting || !path) continue;
          renderObserver?.unobserve(entry.target);
          requestFileRender(path);
        }
      },
      { root: scroller, rootMargin: LAZY_RENDER_ROOT_MARGIN }
    );
    observeDeferredSections();
  }

  function setupActiveObserver() {
    activeObserver?.disconnect();
    activeSectionPaths.clear();
    if (!scroller) return;
    const offset = Math.min(ACTIVE_FILE_OFFSET, Math.max(0, scroller.clientHeight - 1));
    const bottomMargin = Math.max(0, scroller.clientHeight - offset - 1);
    activeObserver = new IntersectionObserver(updateActiveSections, {
      root: scroller,
      rootMargin: `-${offset}px 0px -${bottomMargin}px 0px`
    });
    for (const section of sectionElements.values()) activeObserver.observe(section);
  }

  function updateActiveSections(entries: IntersectionObserverEntry[]) {
    for (const entry of entries) {
      const path = (entry.target as HTMLElement).dataset.diffPath;
      if (!path) continue;
      if (entry.isIntersecting) activeSectionPaths.add(path);
      else activeSectionPaths.delete(path);
    }
    const nextActivePath = files.map(displayPath).find((path) => activeSectionPaths.has(path));
    if (nextActivePath && nextActivePath !== activePath) onActive(nextActivePath);
  }

  function observeDeferredSections() {
    if (!renderObserver) return;
    for (const [path, section] of sectionElements) {
      if (!requestedPaths.has(path)) renderObserver.observe(section);
    }
  }

  function observeSection(node: HTMLElement, path: string) {
    sectionElements.set(path, node);
    if (!requestedPaths.has(path)) renderObserver?.observe(node);
    activeObserver?.observe(node);

    return {
      destroy() {
        renderObserver?.unobserve(node);
        activeObserver?.unobserve(node);
        sectionElements.delete(path);
        activeSectionPaths.delete(path);
      }
    };
  }

  function resetDeferredRendering() {
    if (renderTimer !== undefined) clearTimeout(renderTimer);
    renderTimer = undefined;
    requestedPaths.clear();
    pendingRenderPaths.length = 0;
    pendingRenderPathSet.clear();
    renderedPaths = {};
    requestedPathsVersion += 1;
  }

  function requestFileRender(path: string, priority = false) {
    if (!requestedPaths.has(path)) {
      requestedPaths.add(path);
      requestedPathsVersion += 1;
      const section = sectionElements.get(path);
      if (section) renderObserver?.unobserve(section);
    }
    onLoad(path);
    if (diffs[path]) queueReadyRender(path, priority);
  }

  function scheduleReadyRenders() {
    if (activePath && requestedPaths.has(activePath) && diffs[activePath]) {
      queueReadyRender(activePath, true);
    }
    for (const path of requestedPaths) {
      if (diffs[path]) queueReadyRender(path);
    }
  }

  function queueReadyRender(path: string, priority = false) {
    if (renderedPaths[path] || collapsed[path]) return;
    if (pendingRenderPathSet.has(path)) {
      if (priority) {
        const index = pendingRenderPaths.indexOf(path);
        if (index > 0) {
          pendingRenderPaths.splice(index, 1);
          pendingRenderPaths.unshift(path);
        }
      }
      return;
    }
    pendingRenderPathSet.add(path);
    if (priority) pendingRenderPaths.unshift(path);
    else pendingRenderPaths.push(path);
    scheduleNextRender();
  }

  function scheduleNextRender() {
    if (renderTimer !== undefined || pendingRenderPaths.length === 0) return;
    renderTimer = setTimeout(renderNextFile, RENDER_INTERVAL_MS);
  }

  function renderNextFile() {
    renderTimer = undefined;
    const path = pendingRenderPaths.shift();
    if (!path) return;
    pendingRenderPathSet.delete(path);
    if (requestedPaths.has(path) && diffs[path] && !collapsed[path]) renderedPaths[path] = true;
    scheduleNextRender();
  }

  function toggleCollapsed(path: string) {
    collapsed[path] = !collapsed[path];
    if (!collapsed[path] && diffs[path]) queueReadyRender(path, true);
  }

  async function copyPath(path: string) {
    try {
      await writeClipboard(path);
      copiedPath = path;
      if (copyResetTimer !== undefined) clearTimeout(copyResetTimer);
      copyResetTimer = setTimeout(() => {
        copiedPath = undefined;
        copyResetTimer = undefined;
      }, 1500);
    } catch {
      copiedPath = undefined;
    }
  }

  async function writeClipboard(text: string) {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }

    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.append(textarea);
    textarea.select();
    const copied = document.execCommand('copy');
    textarea.remove();
    if (!copied) throw new Error('Copy failed');
  }
</script>

<div class="diff-feed" bind:this={feedElement}>
  {#each files as file (displayPath(file))}
    {@const path = displayPath(file)}
    <article
      id={diffAnchorId(path)}
      data-diff-path={path}
      class="file-diff"
      class:active={activePath === path}
      aria-current={activePath === path ? 'true' : undefined}
      use:observeSection={path}
    >
      <header class="file-header">
        <button
          class="collapse-button"
          aria-expanded={!collapsed[path]}
          aria-label={`${collapsed[path] ? 'Expand' : 'Collapse'} diff for ${path}`}
          title={collapsed[path] ? 'Expand file diff' : 'Collapse file diff'}
          onclick={() => toggleCollapsed(path)}
        >
          {#if collapsed[path]}<ChevronRight size={14} />{:else}<ChevronDown size={14} />{/if}
        </button>
        <DiffFileStatusIcon status={file.status} />
        <div class="file-path" title={path}>
          <strong>{path}</strong>
          <button
            class="copy-path"
            aria-label={`Copy path ${path}`}
            title={copiedPath === path ? 'Copied' : 'Copy path'}
            onclick={() => copyPath(path)}
          >
            {#if copiedPath === path}<Check size={13} />{:else}<Copy size={13} />{/if}
          </button>
        </div>
        <span class="file-counts">
          {#if file.additions !== undefined}<b>+{file.additions}</b>{/if}
          {#if file.deletions !== undefined}<em>−{file.deletions}</em>{/if}
        </span>
        <button
          class="explain-file"
          disabled={fileAiLoading[path] || file.status === 'binary'}
          aria-label={`Explain changes in ${path}`}
          title="Explain this file's changes"
          onclick={() => onExplainFile(path)}
          >{#if fileAiLoading[path]}<LoaderCircle class="spin" size={13} />{:else}<Sparkles
              size={13}
            />{/if}<span>Explain</span></button
        >
      </header>

      {#if fileExplanations[path] || fileAiLoading[path] || fileAiErrors[path]}
        <div class="file-ai">
          <div class="file-ai-title"><Sparkles size={13} /><strong>Change explanation</strong></div>
          {#if fileAiLoading[path] && !fileExplanations[path]}<div class="file-ai-loading">
              <LoaderCircle class="spin" size={14} />Explaining this file's changes…
            </div>{/if}
          {#if fileAiErrors[path]}<p class="file-ai-error">{fileAiErrors[path]}</p>{/if}
          <AiAnswer explanation={fileExplanations[path]} />
        </div>
      {/if}

      {#if !collapsed[path]}
        <div class="diff-body">
          {#if renderedPaths[path] && diffs[path]}
            <DiffViewer
              diff={diffs[path]}
              {mode}
              {wrap}
              findings={findings.filter((finding) => finding.path === path)}
              onAskInline={(side, startLine, endLine, question) =>
                onAskInline(path, side, startLine, endLine, question)}
            />
          {:else if errors[path]}
            <div class="file-error">
              <span>{errors[path]}</span>
              <button onclick={() => onLoad(path)}><RotateCw size={13} />Retry</button>
            </div>
          {:else}
            <div class="file-loading">
              {#if loadingPaths[path]}<LoaderCircle class="spin" size={16} />Loading diff…{:else if diffs[path]}<LoaderCircle
                  class="spin"
                  size={16}
                />Preparing diff…{:else}Waiting to load…{/if}
            </div>
          {/if}
        </div>
      {/if}
    </article>
  {/each}
</div>

<style>
  .diff-feed {
    min-width: 720px;
    background: #0b1016;
  }
  .file-diff {
    scroll-margin-top: 0;
    border-bottom: 8px solid #080c11;
  }
  .file-diff.active .file-header {
    border-left-color: var(--accent-bright);
  }
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
  .collapse-button {
    display: grid;
    place-items: center;
    width: 22px;
    height: 24px;
    padding: 0;
    color: #75818d;
    background: transparent;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-path strong {
    min-width: 0;
    color: #d2d9df;
    font: 11px var(--mono);
    font-weight: 600;
  }
  .copy-path {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    color: #66727d;
    background: transparent;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
  }
  .copy-path:hover {
    color: #c6ced5;
    background: var(--hover);
  }
  .file-counts {
    display: flex;
    gap: 5px;
    font-size: 10px;
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
    background: rgba(87, 184, 142, 0.06);
    border: 1px solid rgba(87, 184, 142, 0.16);
    border-radius: 5px;
    font-size: 10px;
    cursor: pointer;
  }
  .explain-file:hover:not(:disabled) {
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.11);
  }
  .explain-file:disabled {
    opacity: 0.45;
    cursor: default;
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
    font-size: 11px;
  }
  .file-ai-loading {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 13px;
    color: var(--muted);
    font-size: 11px;
  }
  .file-ai-error {
    margin: 0;
    padding: 12px;
    color: var(--red);
    font-size: 11px;
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
    font-size: 11px;
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
    background: #16202a;
    border: 1px solid #293642;
    border-radius: 5px;
    cursor: pointer;
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
