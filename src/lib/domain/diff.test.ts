import { describe, expect, it } from 'vitest';
import { displayPath, sortDiffFilesByTreeOrder, type DiffFileSummary } from './diff';

describe('sortDiffFilesByTreeOrder', () => {
  it('orders directory contents before files at the same level', () => {
    const files: DiffFileSummary[] = [
      { newPath: 'CHANGELOG.md', status: 'added' },
      { newPath: 'src/zeta.ts', status: 'modified' },
      { newPath: 'docs/overview.md', status: 'modified' },
      { newPath: 'src/lib/alpha.ts', status: 'added' },
      { newPath: 'README.md', status: 'modified' },
      { newPath: 'docs/guide/start.md', status: 'added' },
      { newPath: 'src/alpha.ts', status: 'modified' }
    ];

    expect(sortDiffFilesByTreeOrder(files).map(displayPath)).toEqual([
      'docs/guide/start.md',
      'docs/overview.md',
      'src/lib/alpha.ts',
      'src/alpha.ts',
      'src/zeta.ts',
      'CHANGELOG.md',
      'README.md'
    ]);
    expect(files.map(displayPath)).toEqual([
      'CHANGELOG.md',
      'src/zeta.ts',
      'docs/overview.md',
      'src/lib/alpha.ts',
      'README.md',
      'docs/guide/start.md',
      'src/alpha.ts'
    ]);
  });
});
