<script lang="ts">
  import { FolderGit2, X } from '@lucide/svelte';
  import type { ProjectConfig, RepositoryInfo, SaveProjectInput } from '$lib/domain/project';

  let {
    repository,
    project,
    saving = false,
    deleting = false,
    onSave,
    onDelete,
    onClose
  }: {
    repository?: RepositoryInfo;
    project?: ProjectConfig;
    saving?: boolean;
    deleting?: boolean;
    onSave: (input: SaveProjectInput) => void;
    onDelete?: () => void;
    onClose: () => void;
  } = $props();

  let name = $state('');
  let baseRef = $state('');
  let initialized = false;
  const repoPath = $derived(project?.repoPath ?? repository?.repoPath ?? '');
  const refs = $derived(repository?.localBranches ?? []);

  $effect(() => {
    if (!initialized) {
      name = project?.name ?? repository?.suggestedName ?? '';
      const configuredRef = project?.baseRef;
      const configuredLocalBranch = configuredRef
        ? refs.find((ref) => ref === configuredRef || configuredRef.endsWith(`/${ref}`))
        : undefined;
      baseRef = configuredLocalBranch ?? repository?.detectedBaseRef ?? refs[0] ?? '';
      initialized = true;
    }
  });

  function submit(event: SubmitEvent) {
    event.preventDefault();
    onSave({ id: project?.id, name: name.trim(), repoPath, baseRef: baseRef.trim() });
  }
</script>

<div class="backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onClose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="project-dialog-title">
    <header>
      <div class="icon"><FolderGit2 size={20} /></div>
      <div>
        <h2 id="project-dialog-title">{project ? 'Project settings' : 'Add repository'}</h2>
        <p>{project ? 'Update the display name or comparison ref.' : 'Confirm how ReaDiff should open this repository.'}</p>
      </div>
      <button class="icon-button" aria-label="Close" onclick={onClose}><X size={18} /></button>
    </header>

    <form onsubmit={submit}>
      <label>
        <span>Project name</span>
        <input bind:value={name} required autocomplete="off" />
      </label>
      <label>
        <span>Repository</span>
        <input value={repoPath} readonly />
      </label>
      <label>
        <span>Compare against</span>
        <select bind:value={baseRef} required>
          {#each refs as ref}<option value={ref}>{ref}{ref === repository?.currentBranch ? ' (current)' : ''}</option>{/each}
        </select>
        <small>Local branches only. ReaDiff reviews the merge base through the current working tree.</small>
      </label>
      <footer>
        {#if project && onDelete}
          <button class="danger" type="button" disabled={saving || deleting} onclick={onDelete}>
            {deleting ? 'Removing…' : 'Remove project'}
          </button>
        {/if}
        <button class="secondary" type="button" onclick={onClose}>Cancel</button>
        <button class="primary" type="submit" disabled={saving || deleting || !name.trim() || !baseRef.trim()}>
          {saving ? 'Saving…' : project ? 'Save settings' : 'Add project'}
        </button>
      </footer>
    </form>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    z-index: 50;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 7, 11, 0.72);
    backdrop-filter: blur(6px);
  }

  .dialog {
    width: min(520px, 100%);
    background: #111821;
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.42);
  }

  header {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 13px;
    align-items: start;
    padding: 20px 20px 17px;
    border-bottom: 1px solid var(--border);
  }

  .icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.12);
    border-radius: 9px;
  }

  h2 { margin: 0; font-size: 16px; }
  p { margin: 4px 0 0; color: var(--muted); font-size: 12px; }

  .icon-button {
    display: grid;
    padding: 4px;
    color: var(--muted);
    background: none;
    border: 0;
    cursor: pointer;
  }

  form { display: grid; gap: 17px; padding: 20px; }
  label { display: grid; gap: 7px; }
  label > span { color: #c9d2dc; font-size: 12px; font-weight: 550; }
  input, select {
    width: 100%;
    height: 38px;
    box-sizing: border-box;
    padding: 10px 11px;
    color: var(--text);
    background: #0b1118;
    border: 1px solid var(--border-strong);
    border-radius: 7px;
    outline: none;
    font: inherit;
    font-size: 13px;
  }
  input:focus, select:focus { border-color: var(--accent); box-shadow: 0 0 0 3px rgba(87, 184, 142, 0.1); }
  input[readonly] { color: var(--muted); }
  small { color: var(--muted); font-size: 11px; }
  footer { display: flex; justify-content: flex-end; gap: 9px; padding-top: 3px; }
  button.danger, button.secondary, button.primary {
    padding: 8px 13px;
    border-radius: 7px;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .danger { margin-right: auto; color: var(--red); background: transparent; border: 1px solid rgba(224, 108, 100, 0.28); }
  .danger:hover { background: rgba(224, 108, 100, 0.08); border-color: rgba(224, 108, 100, 0.45); }
  .secondary { color: var(--text); background: transparent; border: 1px solid var(--border-strong); }
  .primary { color: #06110d; background: var(--accent-bright); border: 1px solid var(--accent-bright); }
  button:disabled { opacity: 0.45; cursor: default; }
</style>
