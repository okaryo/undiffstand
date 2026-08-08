import { describe, expect, it } from 'vitest';
import { createWorkingTreeSelection } from './code-selection';

describe('createWorkingTreeSelection', () => {
  it('creates one-based line and column coordinates for a multiline selection', () => {
    const content = 'const one = 1;\nconst two = 2;\nreturn one + two;\n';
    const from = content.indexOf('two');
    const to = from + 3;

    expect(createWorkingTreeSelection('src/example.ts', content, from, to)).toEqual({
      path: 'src/example.ts',
      revision: 'working-tree',
      startLine: 2,
      startColumn: 7,
      endLine: 2,
      endColumn: 10,
      text: 'two'
    });
  });

  it('returns null for an empty selection', () => {
    expect(createWorkingTreeSelection('a.ts', 'hello', 2, 2)).toBeNull();
  });
});
