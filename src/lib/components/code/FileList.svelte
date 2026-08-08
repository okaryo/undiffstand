<script lang="ts">
  import { File, Search } from '@lucide/svelte';
  import type { RepoFile } from '$lib/domain/project';

  let { files, selectedPath, onSelect }: { files: RepoFile[]; selectedPath?: string; onSelect: (path: string) => void } = $props();
  let query = $state('');
  const filtered = $derived(files.filter((file) => file.path.toLowerCase().includes(query.toLowerCase())).slice(0, 800));
</script>

<div class="search"><Search size={13} /><input bind:value={query} placeholder="Find tracked file" aria-label="Find tracked file" /></div>
<div class="list">
  {#each filtered as file (file.path)}
    <button class:active={selectedPath === file.path} onclick={() => onSelect(file.path)} title={file.path}>
      <File size={13} />
      <span><strong>{file.path.split('/').at(-1)}</strong>{#if file.path.includes('/')}<small>{file.path.slice(0, file.path.lastIndexOf('/'))}</small>{/if}</span>
    </button>
  {/each}
</div>

<style>
  .search { display:flex; align-items:center; gap:7px; margin:10px; padding:7px 9px; color:var(--muted); background:var(--input); border:1px solid var(--border); border-radius:6px; }
  input { min-width:0; width:100%; padding:0; color:var(--text); background:transparent; border:0; outline:0; font:inherit; font-size:11px; }
  .list { padding:0 6px 10px; }
  button { display:grid; grid-template-columns:auto minmax(0,1fr); gap:8px; align-items:center; width:100%; padding:7px 8px; color:var(--muted); text-align:left; background:transparent; border:0; border-radius:6px; cursor:pointer; }
  button:hover { color:var(--text); background:var(--hover); } button.active { color:var(--text); background:rgba(87,184,142,.11); }
  span { display:grid; min-width:0; gap:2px; } strong,small { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  strong { color:inherit; font-size:11px; font-weight:520; } small { color:#66717d; font-size:9px; }
</style>
