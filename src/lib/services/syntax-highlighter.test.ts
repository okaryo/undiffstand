import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  SyntaxHighlightRequest,
  SyntaxHighlightResponse,
  SyntaxHighlightSource,
} from "./syntax-highlighter";

type WorkerEvent = "error" | "message" | "messageerror";

class FakeWorker {
  static current: FakeWorker | undefined;

  readonly messages: SyntaxHighlightRequest[] = [];
  readonly terminate = vi.fn();
  private readonly listeners = new Map<
    WorkerEvent,
    Array<(event: never) => void>
  >();

  constructor() {
    FakeWorker.current = this;
  }

  addEventListener(type: WorkerEvent, listener: (event: never) => void) {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  postMessage(message: SyntaxHighlightRequest) {
    this.messages.push(message);
  }

  emitMessage(response: SyntaxHighlightResponse) {
    for (const listener of this.listeners.get("message") ?? []) {
      listener({ data: response } as never);
    }
  }
}

const source: SyntaxHighlightSource = {
  raw: "const value = 1;\n",
  fileName: "src/example.ts",
  language: "ts",
  theme: "dark",
};

describe("syntax highlighter worker client", () => {
  beforeEach(() => {
    vi.resetModules();
    FakeWorker.current = undefined;
    vi.stubGlobal("Worker", FakeWorker);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("adapts a worker result to the synchronous diff highlighter API", async () => {
    const { prepareSyntaxHighlighter } = await import("./syntax-highlighter");
    const prepared = prepareSyntaxHighlighter(
      [source],
      new AbortController().signal,
    );
    const request = FakeWorker.current?.messages[0];
    expect(request).toMatchObject({ type: "highlight", source });

    const result = {
      syntaxFileObject: {},
      syntaxFileLineNumber: 1,
    };
    FakeWorker.current?.emitMessage({
      type: "highlight-result",
      id: request?.id ?? 0,
      result,
      elapsedMs: 12,
    });

    const highlighter = await prepared;
    expect(highlighter).toBeDefined();
    const ast = highlighter?.getAST(
      source.raw,
      source.fileName,
      source.language,
      source.theme,
    );
    expect(ast).toBeDefined();
    expect(highlighter?.processAST(ast!)).toEqual(result);
  });

  it("rejects an obsolete request when it is aborted", async () => {
    const { prepareSyntaxHighlighter } = await import("./syntax-highlighter");
    const controller = new AbortController();
    const prepared = prepareSyntaxHighlighter([source], controller.signal);

    controller.abort();

    await expect(prepared).rejects.toMatchObject({ name: "AbortError" });
  });
});
