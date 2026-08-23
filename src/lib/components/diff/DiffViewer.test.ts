import { render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FileDiff } from "$lib/domain/diff";
import DiffViewer from "./DiffViewer.svelte";

const shiki = vi.hoisted(() => {
  const loadedLanguages = new Set<string>(["ts"]);
  const loadLanguage = vi.fn(async (language: string) => {
    loadedLanguages.add(language);
  });
  const highlighter = {
    name: "shiki",
    type: "class",
    maxLineToIgnoreSyntax: 2_000,
    ignoreSyntaxHighlightList: [],
    setMaxLineToIgnoreSyntax: vi.fn(),
    setIgnoreSyntaxHighlightList: vi.fn(),
    getAST: vi.fn(() => undefined),
    processAST: vi.fn(() => ({
      syntaxFileObject: {},
      syntaxFileLineNumber: 0,
    })),
    hasRegisteredCurrentLang: vi.fn((language: string) =>
      loadedLanguages.has(language),
    ),
    getHighlighterEngine: () => ({ loadLanguage }),
  };
  return {
    loadedLanguages,
    loadLanguage,
    getDiffViewHighlighter: vi.fn(async () => highlighter),
  };
});

vi.mock("@git-diff-view/shiki", () => ({
  bundledLanguages: { diff: {}, rs: {}, ts: {} },
  getDiffViewHighlighter: shiki.getDiffViewHighlighter,
}));

function diff(path: string): FileDiff {
  return {
    file: {
      oldPath: path,
      newPath: path,
      status: "modified",
      additions: 1,
      deletions: 1,
    },
    oldContent: "const value = 1;\n",
    newContent: "const value = 2;\n",
    unifiedDiff: `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n@@ -1 +1 @@\n-const value = 1;\n+const value = 2;\n`,
    truncated: false,
  };
}

describe("DiffViewer syntax highlighting", () => {
  const originalGetContext = HTMLCanvasElement.prototype.getContext;

  beforeEach(() => {
    vi.useFakeTimers();
    shiki.loadedLanguages.clear();
    shiki.loadedLanguages.add("ts");
    shiki.loadLanguage.mockClear();
    shiki.getDiffViewHighlighter.mockClear();
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
      font: "",
      measureText: () => ({ width: 0 }),
    })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  });

  afterEach(() => {
    vi.useRealTimers();
    HTMLCanvasElement.prototype.getContext = originalGetContext;
  });

  it("initializes one language and loads additional languages on demand", async () => {
    const first = render(DiffViewer, {
      props: { diff: diff("src/example.ts"), mode: "split", wrap: false },
    });
    const second = render(DiffViewer, {
      props: { diff: diff("src/example.rs"), mode: "split", wrap: false },
    });

    await vi.advanceTimersByTimeAsync(100);
    await tick();

    expect(shiki.getDiffViewHighlighter).toHaveBeenCalledTimes(1);
    expect(shiki.getDiffViewHighlighter).toHaveBeenCalledWith(["ts"]);
    expect(shiki.loadLanguage).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(100);
    await tick();

    expect(shiki.loadLanguage).toHaveBeenCalledTimes(1);
    expect(shiki.loadLanguage).toHaveBeenCalledWith("rs");

    first.unmount();
    second.unmount();
  });
});
