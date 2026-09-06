import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  SyntaxHighlightRequest,
  SyntaxHighlightResponse,
} from "$lib/services/syntax-highlighter";

describe("syntax highlighter worker", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  it("loads Dart after the worker was initialized for Markdown", async () => {
    const responses: SyntaxHighlightResponse[] = [];
    vi.stubGlobal("postMessage", (response: SyntaxHighlightResponse) => {
      responses.push(response);
    });

    await import("./syntax-highlighter.worker");

    const request = (id: number, language: string, raw: string) => {
      const message: SyntaxHighlightRequest = {
        type: "highlight",
        id,
        source: {
          raw,
          fileName: `example.${language}`,
          language,
          theme: "dark",
        },
      };
      const workerScope = globalThis as unknown as {
        onmessage?: (event: MessageEvent<SyntaxHighlightRequest>) => void;
      };
      workerScope.onmessage?.(new MessageEvent("message", { data: message }));
    };

    request(1, "md", "# Heading\n");
    await vi.waitFor(() => expect(responses).toHaveLength(1));
    expect(responses[0]).not.toHaveProperty("error");

    request(2, "dart", "class Counter {\n  int value = 0;\n}\n");
    await vi.waitFor(() => expect(responses).toHaveLength(2));
    expect(responses[1]).not.toHaveProperty("error");
    expect(responses[1]?.result?.syntaxFileLineNumber).toBeGreaterThan(0);
    expect(
      responses[1]?.result?.syntaxFileObject[1]?.nodeList.some(
        ({ wrapper }) => wrapper?.properties?.style !== undefined,
      ),
    ).toBe(true);
  });
});
