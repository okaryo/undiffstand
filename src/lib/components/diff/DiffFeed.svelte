<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { Check, ChevronDown, ChevronRight, Copy, LoaderCircle, RotateCw } from '@lucide/svelte';
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
    onLoad,
    onActive
  }: {
    files: DiffFileSummary[];
    diffs: Record<string, FileDiff | undefined>;
    loadingPaths: Record<string, boolean | undefined>;
    errors: Record<string, string | undefined>;
    activePath?: string;
    mode: 'split' | 'unified';
    wrap: boolean;
    onLoad: (path: string) => void;
    onActive: (path: string) => void;
  } = $props();

  let feedElement = $state<HTMLElement>();
  let scroller: HTMLElement | null = null;
  let loadObserver: IntersectionObserver | undefined;
  let activeFrame: number | undefined;
  let collapsed = $state<Record<string, boolean>>({});
  let copiedPath = $state<string>();
  let copyResetTimer: ReturnType<typeof setTimeout> | undefined;
  const sectionElements = new Map<string, HTMLElement>();

  onDestroy(() => {
    if (copyResetTimer !== undefined) clearTimeout(copyResetTimer);
  });

  onMount(() => {
    scroller = feedElement?.closest('.viewer-scroll') as HTMLElement | null;
    loadObserver = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          const path = (entry.target as HTMLElement).dataset.diffPath;
          if (entry.isIntersecting && path) onLoad(path);
        }
      },
      { root: scroller, rootMargin: '900px 0px' }
    );
    for (const section of sectionElements.values()) loadObserver.observe(section);
    scroller?.addEventListener('scroll', scheduleActiveSync, { passive: true });
    window.addEventListener('resize', scheduleActiveSync);
    scheduleActiveSync();

    return () => {
      scroller?.removeEventListener('scroll', scheduleActiveSync);
      window.removeEventListener('resize', scheduleActiveSync);
      loadObserver?.disconnect();
      if (activeFrame !== undefined) cancelAnimationFrame(activeFrame);
    };
  });

  function observeSection(node: HTMLElement, path: string) {
    sectionElements.set(path, node);
    loadObserver?.observe(node);

    return {
      destroy() {
        loadObserver?.unobserve(node);
        sectionElements.delete(path);
      }
    };
  }

  function scheduleActiveSync() {
    if (activeFrame !== undefined) cancelAnimationFrame(activeFrame);
    activeFrame = requestAnimationFrame(() => {
      activeFrame = undefined;
      syncActivePath();
    });
  }

  function syncActivePath() {
    if (!scroller || files.length === 0) return;
    if (scroller.scrollHeight <= scroller.clientHeight + 2) return;
    const anchor = scroller.getBoundingClientRect().top + 52;
    let candidate = displayPath(files[0]);

    if (scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 2) {
      candidate = displayPath(files.at(-1) ?? files[0]);
      if (candidate !== activePath) onActive(candidate);
      return;
    }

    for (const file of files) {
      const path = displayPath(file);
      const section = sectionElements.get(path);
      if (!section) continue;
      if (section.getBoundingClientRect().top <= anchor) candidate = path;
      else break;
    }

    if (candidate !== activePath) onActive(candidate);
  }

  function toggleCollapsed(path: string) {
    collapsed[path] = !collapsed[path];
    scheduleActiveSync();
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
      </header>

      {#if !collapsed[path]}
        <div class="diff-body">
          {#if diffs[path]}
            <DiffViewer diff={diffs[path]} {mode} {wrap} />
          {:else if errors[path]}
            <div class="file-error">
              <span>{errors[path]}</span>
              <button onclick={() => onLoad(path)}><RotateCw size={13} />Retry</button>
            </div>
          {:else}
            <div class="file-loading">
              {#if loadingPaths[path]}<LoaderCircle class="spin" size={16} />Loading diff…{:else}Waiting
                to load…{/if}
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
    grid-template-columns: 22px 14px minmax(0, 1fr) auto;
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
