import { describe, expect, it } from "vitest";
import type { DiffFileSummary, FileDiff } from "$lib/domain/diff";
import { findDiffSearchMatches } from "./diff-search";

const file: DiffFileSummary = {
  oldPath: "src/example.ts",
  newPath: "src/example.ts",
  status: "modified",
};

const diff: FileDiff = {
  file,
  oldContent: "const answer = 41;\nshared answer\n",
  newContent: "const answer = 42;\nshared answer\n",
  unifiedDiff:
    "diff --git a/src/example.ts b/src/example.ts\n@@ -1,2 +1,2 @@\n-const answer = 41;\n+const answer = 42;\n shared answer answer\n",
  truncated: false,
};

describe("findDiffSearchMatches", () => {
  it("finds changed and context lines case-insensitively", () => {
    const matches = findDiffSearchMatches(
      [file],
      { "src/example.ts": diff },
      "ANSWER",
    );

    expect(matches).toMatchObject([
      { side: "old", oldLine: 1, column: 6 },
      { side: "new", newLine: 1, column: 6 },
      { side: "both", oldLine: 2, newLine: 2, column: 7 },
      { side: "both", oldLine: 2, newLine: 2, column: 14 },
    ]);
  });

  it("searches only renderable hunk content", () => {
    const binary: DiffFileSummary = {
      newPath: "asset.bin",
      status: "binary",
    };
    const binaryDiff: FileDiff = {
      file: binary,
      unifiedDiff: "answer in a diff header",
      truncated: false,
    };

    expect(
      findDiffSearchMatches(
        [binary, file],
        { "asset.bin": binaryDiff, "src/example.ts": diff },
        "diff header",
      ),
    ).toEqual([]);
  });
});
