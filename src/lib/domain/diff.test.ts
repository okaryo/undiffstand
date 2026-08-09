import { describe, expect, it } from 'vitest';
import {
  diffComparisonLabel,
  diffSelectionLabel,
  displayPath,
  revisionDisplayLabel,
  sortDiffFilesByTreeOrder,
  type DiffFileSummary
} from './diff';

describe('diff display labels', () => {
  it('shows the current branch name instead of HEAD', () => {
    expect(revisionDisplayLabel('HEAD', 'feature/readiff')).toBe('feature/readiff');
    expect(diffSelectionLabel({ base: 'HEAD', target: '.' }, 'feature/readiff')).toBe(
      'feature/readiff → working tree'
    );
    expect(diffComparisonLabel({ fromLabel: 'HEAD', toLabel: 'main' }, 'feature/readiff')).toBe(
      'feature/readiff → main'
    );
  });

  it('keeps HEAD as the detached-head fallback', () => {
    expect(revisionDisplayLabel('HEAD', null)).toBe('HEAD');
  });
});

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
