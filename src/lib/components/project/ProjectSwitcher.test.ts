import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { ProjectConfig } from '$lib/domain/project';
import ProjectSwitcher from './ProjectSwitcher.svelte';

const projects: ProjectConfig[] = [
  {
    id: 'alpha',
    name: 'Alpha',
    repoPath: '/repos/alpha',
    baseRef: 'main',
    lastOpenedAt: '2026-08-08T10:00:00Z'
  },
  {
    id: 'beta',
    name: 'Beta',
    repoPath: '/repos/beta',
    baseRef: 'develop',
    lastOpenedAt: '2026-08-07T10:00:00Z'
  }
];

describe('ProjectSwitcher', () => {
  it('shows registered projects and selects a different project', async () => {
    const onSelect = vi.fn();
    render(ProjectSwitcher, {
      projects,
      activeProject: projects[0],
      comparisonLabel: 'main → working tree',
      onSelect
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Switch project. Current: Alpha' }));

    expect(screen.getByRole('menu', { name: 'Projects' })).toBeInTheDocument();
    expect(screen.getByRole('menuitemradio', { name: /Alpha/ })).toHaveAttribute(
      'aria-checked',
      'true'
    );

    await fireEvent.click(screen.getByRole('menuitemradio', { name: /Beta/ }));

    expect(onSelect).toHaveBeenCalledWith(projects[1]);
    expect(screen.queryByRole('menu', { name: 'Projects' })).not.toBeInTheDocument();
  });

  it('closes the project list with Escape', async () => {
    render(ProjectSwitcher, {
      projects,
      activeProject: projects[0],
      comparisonLabel: 'main → working tree',
      onSelect: vi.fn()
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Switch project. Current: Alpha' }));
    await fireEvent.keyDown(window, { key: 'Escape' });

    expect(screen.queryByRole('menu', { name: 'Projects' })).not.toBeInTheDocument();
  });

  it('closes the project list when clicking outside', async () => {
    render(ProjectSwitcher, {
      projects,
      activeProject: projects[0],
      comparisonLabel: 'main → working tree',
      onSelect: vi.fn()
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Switch project. Current: Alpha' }));
    await fireEvent.click(document.body);

    expect(screen.queryByRole('menu', { name: 'Projects' })).not.toBeInTheDocument();
  });

  it('closes the project list when clicking another part of the header', async () => {
    render(ProjectSwitcher, {
      projects,
      activeProject: projects[0],
      comparisonLabel: 'main → working tree',
      onSelect: vi.fn()
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Switch project. Current: Alpha' }));
    await fireEvent.click(screen.getByText('main → working tree'));

    expect(screen.queryByRole('menu', { name: 'Projects' })).not.toBeInTheDocument();
  });
});
