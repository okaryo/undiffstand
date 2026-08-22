import { render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DiffFileSummary, FileDiff } from "$lib/domain/diff";
import DiffFeed from "./DiffFeed.svelte";

class IntersectionObserverMock {
  static instances: IntersectionObserverMock[] = [];

  readonly observed = new Set<Element>();

  constructor(
    private readonly callback: IntersectionObserverCallback,
    readonly options?: IntersectionObserverInit,
  ) {
    IntersectionObserverMock.instances.push(this);
  }

  observe(target: Element) {
    this.observed.add(target);
  }

  unobserve(target: Element) {
    this.observed.delete(target);
  }

  disconnect() {
    this.observed.clear();
  }

  takeRecords() {
    return [];
  }

  trigger(target: Element, isIntersecting: boolean) {
    this.callback(
      [{ target, isIntersecting } as IntersectionObserverEntry],
      this as unknown as IntersectionObserver,
    );
  }
}

function createFiles(count: number): DiffFileSummary[] {
  return Array.from({ length: count }, (_, index) => ({
    oldPath: `src/file-${index + 1}.ts`,
    newPath: `src/file-${index + 1}.ts`,
    status: "modified" as const,
    additions: 1,
    deletions: 1,
  }));
}

function createDiff(file: DiffFileSummary): FileDiff {
  return {
    file,
    oldContent: "const value = 1;\n",
    newContent: "const value = 2;\n",
    hunks: ["@@ -1 +1 @@\n-const value = 1;\n+const value = 2;\n"],
    unifiedDiff: `diff --git a/${file.oldPath} b/${file.newPath}\n--- a/${file.oldPath}\n+++ b/${file.newPath}\n@@ -1 +1 @@\n-const value = 1;\n+const value = 2;\n`,
    truncated: false,
  };
}

function createBooleanDiff(file: DiffFileSummary): FileDiff {
  return {
    file,
    oldContent: "const truth = false;\n",
    newContent: "const truth = true;\n",
    hunks: ["@@ -1 +1 @@\n-const truth = false;\n+const truth = true;\n"],
    unifiedDiff: `diff --git a/${file.oldPath} b/${file.newPath}\n--- a/${file.oldPath}\n+++ b/${file.newPath}\n@@ -1 +1 @@\n-const truth = false;\n+const truth = true;\n`,
    truncated: false,
  };
}

describe("DiffFeed deferred rendering", () => {
  const originalGetContext = HTMLCanvasElement.prototype.getContext;

  beforeEach(() => {
    vi.useFakeTimers();
    IntersectionObserverMock.instances = [];
    vi.stubGlobal("IntersectionObserver", IntersectionObserverMock);
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
      font: "",
      measureText: () => ({ width: 0 }),
    })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    HTMLCanvasElement.prototype.getContext = originalGetContext;
  });

  it("loads the selected file first and prefetches only two adjacent files", async () => {
    const files = createFiles(10);
    const diffs = Object.fromEntries(
      files.map((file) => [file.newPath as string, createDiff(file)]),
    );
    const onLoad = vi.fn();
    const { container, unmount } = render(DiffFeed, {
      props: {
        files,
        diffs,
        loadingPaths: {},
        errors: {},
        activePath: "src/file-5.ts",
        mode: "split",
        wrap: false,
        onLoad,
        onActive: vi.fn(),
      },
    });

    expect(onLoad.mock.calls.map(([path]) => path)).toEqual(["src/file-5.ts"]);
    expect(container.querySelectorAll(".diff-host")).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(32);
    await tick();
    expect(container.querySelectorAll(".diff-host")).toHaveLength(1);

    await vi.advanceTimersByTimeAsync(32);
    await tick();
    expect(onLoad.mock.calls.map(([path]) => path)).toEqual([
      "src/file-5.ts",
      "src/file-6.ts",
      "src/file-4.ts",
    ]);
    expect(container.querySelectorAll(".diff-host")).toHaveLength(1);

    await vi.advanceTimersByTimeAsync(32 * 2);
    await tick();
    expect(container.querySelectorAll(".diff-host")).toHaveLength(3);
    unmount();
  });

  it("uses observers to request nearby files and update the active file", async () => {
    const files = createFiles(9);
    const onLoad = vi.fn();
    const onActive = vi.fn();
    const { container, unmount } = render(DiffFeed, {
      props: {
        files,
        diffs: {},
        loadingPaths: {},
        errors: {},
        mode: "split",
        wrap: false,
        onLoad,
        onActive,
      },
    });
    const ninthSection = container.querySelector<HTMLElement>(
      '[data-diff-path="src/file-9.ts"]',
    );
    expect(ninthSection).not.toBeNull();

    const renderObserver = IntersectionObserverMock.instances.find(
      (observer) => observer.options?.rootMargin === "400px 0px",
    );
    const activeObserver = IntersectionObserverMock.instances.find(
      (observer) => observer !== renderObserver,
    );
    expect(renderObserver).toBeDefined();
    expect(activeObserver).toBeDefined();

    renderObserver?.trigger(ninthSection as HTMLElement, true);
    expect(onLoad).toHaveBeenCalledWith("src/file-9.ts");

    activeObserver?.trigger(ninthSection as HTMLElement, true);
    expect(onActive).toHaveBeenCalledWith("src/file-9.ts");
    unmount();
  });

  it("prioritizes and activates a search match in a deferred file", async () => {
    const files = createFiles(9);
    const ninthPath = files[8].newPath as string;
    const onLoad = vi.fn();
    const onActive = vi.fn();
    const { container, unmount } = render(DiffFeed, {
      props: {
        files,
        diffs: { [ninthPath]: createDiff(files[8]) },
        loadingPaths: {},
        errors: {},
        mode: "split",
        wrap: false,
        searchQuery: "value",
        searchMatch: {
          id: `${ninthPath}:0:new::1:6`,
          path: ninthPath,
          side: "new",
          newLine: 1,
          column: 6,
          length: 5,
        },
        onLoad,
        onActive,
      },
    });

    expect(onLoad).toHaveBeenCalledWith(ninthPath);
    expect(onActive).toHaveBeenCalledWith(ninthPath);

    await vi.advanceTimersByTimeAsync(32);
    await tick();
    expect(
      container.querySelector(`[data-diff-path="${ninthPath}"] .diff-host`),
    ).toBeInTheDocument();
    unmount();
  });

  it("removes prefix highlights as the search query changes", async () => {
    class HighlightMock extends Set<AbstractRange> {
      constructor(...ranges: AbstractRange[]) {
        super(ranges);
      }
    }
    const highlights = new Map<string, HighlightMock>();
    vi.stubGlobal("Highlight", HighlightMock);
    vi.stubGlobal("CSS", { highlights });
    const files = createFiles(2);
    const diffs = Object.fromEntries(
      files.map((file) => [file.newPath as string, createBooleanDiff(file)]),
    );
    const props = {
      files,
      diffs,
      loadingPaths: {},
      errors: {},
      mode: "split" as const,
      wrap: false,
      searchQuery: "t",
      onLoad: vi.fn(),
      onActive: vi.fn(),
    };
    const { rerender, unmount } = render(DiffFeed, { props });

    await vi.advanceTimersByTimeAsync(128);
    await tick();
    const prefixHighlights = highlights.get("undiffstand-diff-search-match");
    expect(
      [...(prefixHighlights ?? [])].some((range) => range.toString() === "t"),
    ).toBe(true);
    await rerender({ ...props, searchQuery: "tr" });
    await rerender({ ...props, searchQuery: "true" });
    await tick();

    expect(prefixHighlights?.size).toBe(0);
    const renderedHighlights = highlights.get("undiffstand-diff-search-match");
    expect(renderedHighlights).toBeDefined();
    expect(
      [...(renderedHighlights ?? [])].map((range) => range.toString()),
    ).toEqual(["true", "true"]);
    unmount();
  });
});
