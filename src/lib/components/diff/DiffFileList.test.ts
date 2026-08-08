import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import DiffFileList from './DiffFileList.svelte';

describe('DiffFileList', () => {
  it('selects the requested changed file', async () => {
    const onSelect = vi.fn();
    render(DiffFileList, {
      files: [
        {
          oldPath: 'src/review.ts',
          newPath: 'src/review.ts',
          status: 'modified',
          additions: 3,
          deletions: 1
        }
      ],
      onSelect
    });

    await fireEvent.click(screen.getByRole('button', { name: /review\.ts/i }));
    expect(onSelect).toHaveBeenCalledWith('src/review.ts');
  });

  it('filters by the complete repository path', async () => {
    render(DiffFileList, {
      files: [
        { newPath: 'src/alpha.ts', status: 'added', additions: 1, deletions: 0 },
        { newPath: 'tests/beta.ts', status: 'added', additions: 1, deletions: 0 }
      ],
      onSelect: vi.fn()
    });

    await fireEvent.input(screen.getByLabelText('Filter changed files'), {
      target: { value: 'tests/' }
    });
    expect(screen.queryByText('alpha.ts')).not.toBeInTheDocument();
    expect(screen.getByText('beta.ts')).toBeInTheDocument();
  });
});
