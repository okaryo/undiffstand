import type { DiffHighlighter } from "@git-diff-view/shiki";

export type SyntaxHighlightTheme = "light" | "dark";

export type SyntaxHighlightSource = {
  raw: string;
  fileName: string;
  language: string;
  theme: SyntaxHighlightTheme;
};

export type ProcessedSyntax = ReturnType<DiffHighlighter["processAST"]>;

export type SyntaxHighlightRequest = {
  type: "highlight";
  id: number;
  source: SyntaxHighlightSource;
};

export type SyntaxHighlightResponse = {
  type: "highlight-result";
  id: number;
  result?: ProcessedSyntax;
  elapsedMs: number;
  error?: string;
};

type PendingRequest = {
  resolve: (result: ProcessedSyntax | undefined) => void;
  reject: (error: Error) => void;
  removeAbortListener: () => void;
};

type PrecomputedSyntax = {
  source: SyntaxHighlightSource;
  result: ProcessedSyntax;
};

type PrecomputedAst = {
  result: ProcessedSyntax;
};

let worker: Worker | undefined;
let nextRequestId = 1;
let nextHighlighterId = 1;
const pendingRequests = new Map<number, PendingRequest>();

function abortError() {
  return new DOMException("Syntax highlighting was cancelled.", "AbortError");
}

function rejectPendingRequests(error: Error) {
  for (const pending of pendingRequests.values()) {
    pending.removeAbortListener();
    pending.reject(error);
  }
  pendingRequests.clear();
}

function resetWorker(error: Error) {
  worker?.terminate();
  worker = undefined;
  rejectPendingRequests(error);
}

function getWorker() {
  if (worker) return worker;

  worker = new Worker(
    new URL("../workers/syntax-highlighter.worker.ts", import.meta.url),
    { type: "module" },
  );
  worker.addEventListener(
    "message",
    (event: MessageEvent<SyntaxHighlightResponse>) => {
      const response = event.data;
      if (response.type !== "highlight-result") return;
      const pending = pendingRequests.get(response.id);
      if (!pending) return;

      pendingRequests.delete(response.id);
      pending.removeAbortListener();
      if (response.error) {
        pending.reject(new Error(response.error));
      } else {
        pending.resolve(response.result);
      }
    },
  );
  worker.addEventListener("error", () => {
    resetWorker(new Error("The syntax highlighting worker failed."));
  });
  worker.addEventListener("messageerror", () => {
    resetWorker(
      new Error("The syntax highlighting worker returned invalid data."),
    );
  });

  return worker;
}

function requestSyntaxHighlight(
  source: SyntaxHighlightSource,
  signal: AbortSignal,
) {
  if (signal.aborted) {
    return Promise.reject<ProcessedSyntax | undefined>(abortError());
  }

  return new Promise<ProcessedSyntax | undefined>((resolve, reject) => {
    const id = nextRequestId++;
    const onAbort = () => {
      const pending = pendingRequests.get(id);
      if (!pending) return;
      pendingRequests.delete(id);
      pending.removeAbortListener();
      reject(abortError());
    };
    const removeAbortListener = () =>
      signal.removeEventListener("abort", onAbort);

    pendingRequests.set(id, { resolve, reject, removeAbortListener });
    signal.addEventListener("abort", onAbort, { once: true });

    try {
      const request: SyntaxHighlightRequest = {
        type: "highlight",
        id,
        source,
      };
      getWorker().postMessage(request);
    } catch (error) {
      pendingRequests.delete(id);
      removeAbortListener();
      reject(error instanceof Error ? error : new Error(String(error)));
    }
  });
}

function sameSource(
  source: SyntaxHighlightSource,
  raw: string,
  fileName?: string,
  language?: string,
  theme?: SyntaxHighlightTheme,
) {
  return (
    source.raw === raw &&
    source.fileName === fileName &&
    source.language === language &&
    source.theme === theme
  );
}

function createPrecomputedHighlighter(
  entries: PrecomputedSyntax[],
): DiffHighlighter {
  const name = `shiki-worker-${nextHighlighterId++}`;
  const highlighter = {
    name,
    type: "class",
    maxLineToIgnoreSyntax: 2_000,
    ignoreSyntaxHighlightList: [],
    setMaxLineToIgnoreSyntax: () => undefined,
    setIgnoreSyntaxHighlightList: () => undefined,
    getAST: (
      raw: string,
      fileName?: string,
      language?: string,
      theme?: SyntaxHighlightTheme,
    ) => {
      const entry = entries.find(({ source }) =>
        sameSource(source, raw, fileName, language, theme),
      );
      return entry
        ? ({ result: entry.result } as unknown as ReturnType<
            DiffHighlighter["getAST"]
          >)
        : undefined;
    },
    processAST: (ast: unknown) => (ast as PrecomputedAst).result,
    hasRegisteredCurrentLang: (language: string) =>
      entries.some(({ source }) => source.language === language),
    getHighlighterEngine: () => null,
  };

  return highlighter as unknown as DiffHighlighter;
}

export async function prepareSyntaxHighlighter(
  sources: SyntaxHighlightSource[],
  signal: AbortSignal,
) {
  if (sources.length === 0 || typeof Worker === "undefined") return undefined;

  const results = await Promise.all(
    sources.map(async (source) => ({
      source,
      result: await requestSyntaxHighlight(source, signal),
    })),
  );
  const entries = results.filter(
    (entry): entry is PrecomputedSyntax => entry.result !== undefined,
  );

  return entries.length > 0 ? createPrecomputedHighlighter(entries) : undefined;
}
