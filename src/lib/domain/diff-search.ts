import {
  displayPath,
  type DiffFileSummary,
  type FileDiff,
} from "$lib/domain/diff";

export type DiffSearchMatch = {
  id: string;
  path: string;
  side: "old" | "new" | "both";
  oldLine?: number;
  newLine?: number;
  column: number;
  length: number;
};

const HUNK_HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

export function findDiffSearchMatches(
  files: DiffFileSummary[],
  diffs: Record<string, FileDiff | undefined>,
  query: string,
): DiffSearchMatch[] {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return [];

  const matches: DiffSearchMatch[] = [];

  for (const file of files) {
    if (file.status === "binary") continue;
    const path = displayPath(file);
    const diff = diffs[path];
    if (!diff) continue;

    for (const [hunkIndex, hunk] of diff.hunks.entries()) {
      appendHunkMatches(matches, path, hunk, hunkIndex, normalizedQuery);
    }
  }

  return matches;
}

function appendHunkMatches(
  matches: DiffSearchMatch[],
  path: string,
  hunk: string,
  hunkIndex: number,
  normalizedQuery: string,
) {
  const lines = hunk.split("\n");
  const header = lines[0]?.match(HUNK_HEADER);
  if (!header) return;

  let oldLine = Number(header[1]);
  let newLine = Number(header[2]);

  for (const rawLine of lines.slice(1)) {
    if (!rawLine || rawLine.startsWith("\\ No newline")) continue;
    const prefix = rawLine[0];
    const content = rawLine.slice(1);

    if (prefix === "-") {
      appendLineMatches(matches, {
        path,
        hunkIndex,
        side: "old",
        oldLine,
        content,
        normalizedQuery,
      });
      oldLine += 1;
    } else if (prefix === "+") {
      appendLineMatches(matches, {
        path,
        hunkIndex,
        side: "new",
        newLine,
        content,
        normalizedQuery,
      });
      newLine += 1;
    } else if (prefix === " ") {
      appendLineMatches(matches, {
        path,
        hunkIndex,
        side: "both",
        oldLine,
        newLine,
        content,
        normalizedQuery,
      });
      oldLine += 1;
      newLine += 1;
    }
  }
}

function appendLineMatches(
  matches: DiffSearchMatch[],
  line: {
    path: string;
    hunkIndex: number;
    side: DiffSearchMatch["side"];
    oldLine?: number;
    newLine?: number;
    content: string;
    normalizedQuery: string;
  },
) {
  const normalizedContent = line.content.toLowerCase();
  let column = normalizedContent.indexOf(line.normalizedQuery);

  while (column !== -1) {
    matches.push({
      id: [
        line.path,
        line.hunkIndex,
        line.side,
        line.oldLine ?? "",
        line.newLine ?? "",
        column,
      ].join(":"),
      path: line.path,
      side: line.side,
      oldLine: line.oldLine,
      newLine: line.newLine,
      column,
      length: line.normalizedQuery.length,
    });
    column = normalizedContent.indexOf(
      line.normalizedQuery,
      column + line.normalizedQuery.length,
    );
  }
}
