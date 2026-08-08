<script lang="ts">
  import { FileCode2, FilePlus2, FileX2, Search } from '@lucide/svelte';
  import { displayPath, type DiffFileSummary } from '$lib/domain/diff';

  let {
    files,
    selectedPath,
    onSelect
  }: { files: DiffFileSummary[]; selectedPath?: string; onSelect: (path: string) => void } = $props();
  let query = $state('');
  const filtered = $derived(files.filter((file) => displayPath(file).toLowerCase().includes(query.toLowerCase())));

  function statusLetter(status: DiffFileSummary['status']) {
    return { added: 'A', modified: 'M', deleted: 'D', renamed: 'R', copied: 'C', binary: 'B', submodule: 'S' }[status];
  }
</script>

<div class="search"><Search size={13} /><input bind:value={query} placeholder="Filter changed files" aria-label="Filter changed files" /></div>
<div class="list">
  {#each filtered as file (displayPath(file))}
    {@const path = displayPath(file)}
    <button class:active={selectedPath === path} onclick={() => onSelect(path)} title={path}>
      {#if file.status === 'added'}<FilePlus2 size={14} />{:else if file.status === 'deleted'}<FileX2 size={14} />{:else}<FileCode2 size={14} />{/if}
      <span class="path"><strong>{path.split('/').at(-1)}</strong>{#if path.includes('/')}<small>{path.slice(0, path.lastIndexOf('/'))}</small>{/if}</span>
      <span class="counts"><i class="status {file.status}">{statusLetter(file.status)}</i>{#if file.additions !== undefined}<b>+{file.additions}</b>{/if}{#if file.deletions !== undefined}<em>−{file.deletions}</em>{/if}</span>
    </button>
  {/each}
</div>

<style>
  .search { display: flex; align-items: center; gap: 7px; margin: 10px; padding: 7px 9px; color: var(--muted); background: var(--input); border: 1px solid var(--border); border-radius: 6px; }
  input { min-width: 0; width: 100%; padding: 0; color: var(--text); background: transparent; border: 0; outline: 0; font: inherit; font-size: 11px; }
  .list { padding: 0 6px 10px; }
  button { display: grid; grid-template-columns: auto minmax(0,1fr) auto; align-items: center; gap: 8px; width: 100%; padding: 8px; color: var(--muted); text-align: left; background: transparent; border: 0; border-radius: 6px; cursor: pointer; }
  button:hover { color: var(--text); background: var(--hover); }
  button.active { color: var(--text); background: rgba(87,184,142,.11); }
  .path { display: grid; min-width: 0; gap: 2px; }
  .path strong, .path small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .path strong { color: inherit; font-size: 12px; font-weight: 530; }
  .path small { color: #66717d; font-size: 11px; }
  .counts { display: flex; align-items: center; gap: 4px; font-size: 10px; }
  .counts b { color: var(--green); font-weight: 500; }
  .counts em { color: var(--red); font-style: normal; }
  .status { display: grid; place-items: center; width: 14px; height: 14px; color: #8b97a3; font-style: normal; font-weight: 650; }
  .status.added { color: var(--green); } .status.deleted { color: var(--red); } .status.renamed { color: #c3a5f8; }
</style>
