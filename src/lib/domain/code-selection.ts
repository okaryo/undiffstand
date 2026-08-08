export type CodeRevision = 'working-tree' | 'head' | 'base';
export type DiffSide = 'old' | 'new';

export type CodeSelection = {
  path: string;
  revision: CodeRevision;
  side?: DiffSide;
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
  text: string;
};

export function createWorkingTreeSelection(
  path: string,
  content: string,
  anchor: number,
  head: number
): CodeSelection | null {
  const from = Math.max(0, Math.min(anchor, head, content.length));
  const to = Math.max(0, Math.min(Math.max(anchor, head), content.length));
  if (from === to) return null;

  const position = (offset: number) => {
    const prefix = content.slice(0, offset);
    const lastNewline = prefix.lastIndexOf('\n');
    return {
      line: prefix.split('\n').length,
      column: offset - lastNewline
    };
  };
  const start = position(from);
  const end = position(to - 1);

  return {
    path,
    revision: 'working-tree',
    startLine: start.line,
    startColumn: start.column,
    endLine: end.line,
    endColumn: end.column + 1,
    text: content.slice(from, to)
  };
}
