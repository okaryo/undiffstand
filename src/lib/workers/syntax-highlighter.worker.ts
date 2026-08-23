import {
  bundledLanguages,
  getDiffViewHighlighter,
  type BundledLanguage,
  type DiffHighlighter,
} from "@git-diff-view/shiki";
import type {
  ProcessedSyntax,
  SyntaxHighlightRequest,
  SyntaxHighlightResponse,
} from "$lib/services/syntax-highlighter";

const MAX_HIGHLIGHT_LINES = 2_000;
const MINIMAL_BOOTSTRAP_LANGUAGE: BundledLanguage = "diff";

let highlighterPromise: Promise<DiffHighlighter> | undefined;
const languagePromises = new Map<string, Promise<void>>();
let languageLoadQueue: Promise<void> = Promise.resolve();

const workerScope = globalThis as unknown as {
  onmessage: ((event: MessageEvent<SyntaxHighlightRequest>) => void) | null;
  postMessage: (response: SyntaxHighlightResponse) => void;
};

function exceedsLineLimit(raw: string) {
  let lineCount = 1;
  for (let index = 0; index < raw.length; index += 1) {
    if (raw.charCodeAt(index) === 10 && ++lineCount > MAX_HIGHLIGHT_LINES) {
      return true;
    }
  }
  return false;
}

async function getHighlighter(language: string) {
  const bundledLanguage =
    language in bundledLanguages
      ? (language as BundledLanguage)
      : MINIMAL_BOOTSTRAP_LANGUAGE;
  highlighterPromise ??= getDiffViewHighlighter([bundledLanguage]);
  const highlighter = await highlighterPromise;

  if (
    language in bundledLanguages &&
    !highlighter.hasRegisteredCurrentLang(language)
  ) {
    let languagePromise = languagePromises.get(language);
    if (!languagePromise) {
      languagePromise = languageLoadQueue.then(() =>
        Promise.resolve(
          highlighter
            .getHighlighterEngine()
            ?.loadLanguage(language as BundledLanguage),
        ).then(() => undefined),
      );
      languageLoadQueue = languagePromise.catch(() => undefined);
      languagePromises.set(language, languagePromise);
    }
    await languagePromise;
  }

  return highlighter;
}

async function highlight(
  request: SyntaxHighlightRequest,
): Promise<ProcessedSyntax | undefined> {
  const { source } = request;
  if (
    !source.raw ||
    exceedsLineLimit(source.raw) ||
    !(source.language in bundledLanguages)
  ) {
    return undefined;
  }

  const highlighter = await getHighlighter(source.language);
  const ast = highlighter.getAST(
    source.raw,
    source.fileName,
    source.language,
    source.theme,
  );
  return ast ? highlighter.processAST(ast) : undefined;
}

workerScope.onmessage = (event) => {
  const request = event.data;
  if (request.type !== "highlight") return;
  const startedAt = performance.now();

  void highlight(request)
    .then((result) => {
      workerScope.postMessage({
        type: "highlight-result",
        id: request.id,
        result,
        elapsedMs: performance.now() - startedAt,
      });
    })
    .catch((error: unknown) => {
      workerScope.postMessage({
        type: "highlight-result",
        id: request.id,
        elapsedMs: performance.now() - startedAt,
        error: error instanceof Error ? error.message : String(error),
      });
    });
};
