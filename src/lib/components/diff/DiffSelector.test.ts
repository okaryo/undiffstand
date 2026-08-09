import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import DiffSelector from './DiffSelector.svelte';

const repositoryOptions = {
  recentBranches: ['hotfix', 'topic', 'release', 'maintenance', 'develop', 'not-shown'],
  localBranches: ['feature', 'main', 'hotfix', 'topic', 'release', 'maintenance'],
  remoteBranches: ['origin/main', 'origin/release'],
  recentCommits: Array.from({ length: 11 }, (_, index) => ({
    sha: `${index}`.padStart(40, '0'),
    shortSha: `${index}`.padStart(7, '0'),
    subject: `Change ${index}`
  })),
  currentBranch: 'feature'
};

describe('DiffSelector', () => {
  it('shows the current branch name for the default review', async () => {
    const onApply = vi.fn();
    render(DiffSelector, {
      selection: { base: 'HEAD', target: '.' },
      ...repositoryOptions,
      onApply
    });

    expect(screen.getByRole('combobox', { name: 'From' })).toHaveTextContent('feature');
    expect(screen.getByRole('combobox', { name: 'From' })).not.toHaveTextContent('HEAD');
    expect(screen.getByRole('combobox', { name: 'To' })).toHaveTextContent('Working tree');
    await fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    expect(onApply).toHaveBeenCalledWith({ base: 'HEAD', target: '.' });
  });

  it('shows capped recent lists and expands local and remote branches on demand', async () => {
    render(DiffSelector, {
      selection: { base: 'HEAD', target: '.' },
      ...repositoryOptions,
      onApply: vi.fn()
    });

    await fireEvent.click(screen.getByRole('combobox', { name: 'From' }));
    const menu = screen.getByRole('listbox', { name: 'From revisions' });
    const recentBranches = within(menu).getByRole('region', { name: 'Recent branches' });
    const recentCommits = within(menu).getByRole('region', { name: 'Recent commits' });

    expect(within(recentBranches).getAllByRole('option')).toHaveLength(5);
    expect(
      within(recentBranches).queryByRole('option', { name: 'not-shown' })
    ).not.toBeInTheDocument();
    expect(within(recentCommits).getAllByRole('option')).toHaveLength(10);
    expect(within(menu).queryByRole('option', { name: 'main' })).not.toBeInTheDocument();
    expect(within(menu).queryByRole('option', { name: 'origin/release' })).not.toBeInTheDocument();

    await fireEvent.click(within(menu).getByRole('button', { name: /Local branches/ }));
    expect(within(menu).getByRole('option', { name: 'main' })).toBeInTheDocument();
    await fireEvent.click(within(menu).getByRole('button', { name: /Remote branches/ }));
    expect(within(menu).getByRole('option', { name: 'origin\/release' })).toBeInTheDocument();
  });

  it('builds a branch-to-branch comparison', async () => {
    const onApply = vi.fn();
    render(DiffSelector, {
      selection: { base: 'HEAD', target: '.' },
      ...repositoryOptions,
      onApply
    });

    await fireEvent.click(screen.getByRole('combobox', { name: 'From' }));
    let menu = screen.getByRole('listbox', { name: 'From revisions' });
    await fireEvent.click(within(menu).getByRole('button', { name: /Local branches/ }));
    await fireEvent.click(within(menu).getByRole('option', { name: 'main' }));

    await fireEvent.click(screen.getByRole('combobox', { name: 'To' }));
    menu = screen.getByRole('listbox', { name: 'To revisions' });
    const localBranches = within(menu).getByRole('region', { name: 'Local branches' });
    await fireEvent.click(within(localBranches).getByRole('button', { name: /Local branches/ }));
    await fireEvent.click(within(localBranches).getByRole('option', { name: 'feature' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Review' }));

    expect(onApply).toHaveBeenCalledWith({ base: 'main', target: 'feature' });
  });
});
