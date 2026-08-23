import { render } from "@testing-library/svelte";
import { tick } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FileDiff } from "$lib/domain/diff";
import DiffViewer from "./DiffViewer.svelte";

const syntaxHighlighter = vi.hoisted(() => {
  const highlighter = {
    name: "shiki-worker-test",
    type: "class",
    maxLineToIgnoreSyntax: 2_000,
    ignoreSyntaxHighlightList: [],
    setMaxLineToIgnoreSyntax: vi.fn(),
    setIgnoreSyntaxHighlightList: vi.fn(),
    getAST: vi.fn(() => ({ type: "root", children: [] })),
    processAST: vi.fn(() => ({
      syntaxFileObject: {},
      syntaxFileLineNumber: 0,
    })),
    hasRegisteredCurrentLang: vi.fn(() => true),
    getHighlighterEngine: () => null,
  };
  return {
    highlighter,
    prepare: vi.fn(async () => highlighter),
  };
});

vi.mock("$lib/services/syntax-highlighter", () => ({
  prepareSyntaxHighlighter: syntaxHighlighter.prepare,
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
    syntaxHighlighter.prepare.mockClear();
    syntaxHighlighter.highlighter.getAST.mockClear();
    syntaxHighlighter.highlighter.processAST.mockClear();
    HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
      font: "",
      measureText: () => ({ width: 0 }),
    })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  });

  afterEach(() => {
    vi.useRealTimers();
    HTMLCanvasElement.prototype.getContext = originalGetContext;
  });

  it("prepares syntax off-thread and activates one viewer per idle slot", async () => {
    const first = render(DiffViewer, {
      props: { diff: diff("src/example.ts"), mode: "split", wrap: false },
    });
    const second = render(DiffViewer, {
      props: { diff: diff("src/example.rs"), mode: "split", wrap: false },
    });

    await tick();

    expect(syntaxHighlighter.prepare).toHaveBeenCalledTimes(2);
    expect(syntaxHighlighter.prepare).toHaveBeenNthCalledWith(
      1,
      [
        {
          raw: "const value = 1;\n",
          fileName: "src/example.ts",
          language: "ts",
          theme: "dark",
        },
        {
          raw: "const value = 2;\n",
          fileName: "src/example.ts",
          language: "ts",
          theme: "dark",
        },
      ],
      expect.any(AbortSignal),
    );
    expect(syntaxHighlighter.highlighter.getAST).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(100);
    await tick();

    expect(syntaxHighlighter.highlighter.getAST).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(100);
    await tick();

    expect(syntaxHighlighter.highlighter.getAST).toHaveBeenCalledTimes(4);

    first.unmount();
    second.unmount();
  });
});
