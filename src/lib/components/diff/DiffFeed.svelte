<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import type {
    ChangeReviewFinding,
    DiffExplanation,
    InlineAnswer,
  } from "$lib/domain/ai";
  import {
    diffAnchorId,
    displayPath,
    type DiffFileSummary,
    type FileDiff,
  } from "$lib/domain/diff";
  import DiffFileSection from "./DiffFileSection.svelte";

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
    onAskInline = () => Promise.reject(new Error("Inline Ask is unavailable.")),
  }: {
    files: DiffFileSummary[];
    diffs: Record<string, FileDiff | undefined>;
    loadingPaths: Record<string, boolean | undefined>;
    errors: Record<string, string | undefined>;
    activePath?: string;
    mode: "split" | "unified";
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
      side: "old" | "new",
      startLine: number,
      endLine: number,
      question: string,
    ) => Promise<InlineAnswer>;
  } = $props();

  let feedElement = $state<HTMLElement>();
  let scroller: HTMLElement | null = null;
  let renderObserver: IntersectionObserver | undefined;
  let activeObserver: IntersectionObserver | undefined;
  let renderedPaths = $state<Record<string, boolean | undefined>>({});
  let requestedPathsVersion = $state(0);
  let lastFilesKey = $state("");
  let collapsed = $state<Record<string, boolean>>({});
  let copiedPath = $state<string>();
  let copyResetTimer: ReturnType<typeof setTimeout> | undefined;
  let renderTimer: ReturnType<typeof setTimeout> | undefined;
  const sectionElements = new SvelteMap<string, HTMLElement>();
  const requestedPaths = new SvelteSet<string>();
  const pendingRenderPaths: string[] = [];
  const pendingRenderPathSet = new SvelteSet<string>();
  const activeSectionPaths = new SvelteSet<string>();
  const INITIAL_RENDERED_FILE_COUNT = 8;
  const LAZY_RENDER_ROOT_MARGIN = "1200px 0px";
  const ACTIVE_FILE_OFFSET = 52;
  const RENDER_INTERVAL_MS = 32;

  onDestroy(() => {
    if (copyResetTimer !== undefined) clearTimeout(copyResetTimer);
    if (renderTimer !== undefined) clearTimeout(renderTimer);
  });

  onMount(() => {
    scroller =
      (feedElement?.closest(".viewer-scroll") as HTMLElement | null) ??
      feedElement ??
      null;
    setupRenderObserver();
    setupActiveObserver();
    window.addEventListener("resize", setupActiveObserver);

    return () => {
      window.removeEventListener("resize", setupActiveObserver);
      renderObserver?.disconnect();
      activeObserver?.disconnect();
    };
  });

  $effect(() => {
    const filesKey = files.map(displayPath).join("\0");
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
      { root: scroller, rootMargin: LAZY_RENDER_ROOT_MARGIN },
    );
    observeDeferredSections();
  }

  function setupActiveObserver() {
    activeObserver?.disconnect();
    activeSectionPaths.clear();
    if (!scroller) return;
    const offset = Math.min(
      ACTIVE_FILE_OFFSET,
      Math.max(0, scroller.clientHeight - 1),
    );
    const bottomMargin = Math.max(0, scroller.clientHeight - offset - 1);
    activeObserver = new IntersectionObserver(updateActiveSections, {
      root: scroller,
      rootMargin: `-${offset}px 0px -${bottomMargin}px 0px`,
    });
    for (const section of sectionElements.values())
      activeObserver.observe(section);
  }

  function updateActiveSections(entries: IntersectionObserverEntry[]) {
    for (const entry of entries) {
      const path = (entry.target as HTMLElement).dataset.diffPath;
      if (!path) continue;
      if (entry.isIntersecting) activeSectionPaths.add(path);
      else activeSectionPaths.delete(path);
    }
    const nextActivePath = files
      .map(displayPath)
      .find((path) => activeSectionPaths.has(path));
    if (nextActivePath && nextActivePath !== activePath)
      onActive(nextActivePath);
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
      },
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
    if (requestedPaths.has(path) && diffs[path] && !collapsed[path])
      renderedPaths[path] = true;
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

    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.append(textarea);
    textarea.select();
    const copied = document.execCommand("copy");
    textarea.remove();
    if (!copied) throw new Error("Copy failed");
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
      aria-current={activePath === path ? "true" : undefined}
      use:observeSection={path}
    >
      <DiffFileSection
        {file}
        {path}
        diff={diffs[path]}
        loading={loadingPaths[path]}
        error={errors[path]}
        active={activePath === path}
        collapsed={collapsed[path]}
        copied={copiedPath === path}
        rendered={renderedPaths[path]}
        {mode}
        {wrap}
        explanation={fileExplanations[path]}
        aiLoading={fileAiLoading[path]}
        aiError={fileAiErrors[path]}
        findings={findings.filter((finding) => finding.path === path)}
        onToggle={() => toggleCollapsed(path)}
        onCopy={() => copyPath(path)}
        onLoad={() => onLoad(path)}
        onExplain={() => onExplainFile(path)}
        onAskInline={(side, startLine, endLine, question) =>
          onAskInline(path, side, startLine, endLine, question)}
      />
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
</style>
