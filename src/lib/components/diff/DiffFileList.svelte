<script lang="ts">
  import {
    ChevronDown,
    ChevronRight,
    Folder,
    FolderOpen,
    Search,
  } from "@lucide/svelte";
  import {
    displayPath,
    sortDiffFilesByTreeOrder,
    type DiffFileSummary,
  } from "$lib/domain/diff";
  import DiffFileStatusIcon from "./DiffFileStatusIcon.svelte";

  let {
    files,
    selectedPath,
    matchCounts = {},
    onSelect,
  }: {
    files: DiffFileSummary[];
    selectedPath?: string;
    matchCounts?: Record<string, number | undefined>;
    onSelect: (path: string) => void;
  } = $props();

  type DirectoryNode = {
    name: string;
    path: string;
    directories: Map<string, DirectoryNode>;
    files: DiffFileSummary[];
  };

  type TreeRow =
    | {
        kind: "directory";
        name: string;
        path: string;
        depth: number;
        expanded: boolean;
      }
    | {
        kind: "file";
        name: string;
        path: string;
        depth: number;
        file: DiffFileSummary;
      };

  let query = $state("");
  let expandedDirectories = $state<Record<string, boolean>>({});

  const filteredFiles = $derived(
    sortDiffFilesByTreeOrder(
      files.filter((file) =>
        displayPath(file).toLowerCase().includes(query.trim().toLowerCase()),
      ),
    ),
  );
  const rows = $derived(
    buildRows(filteredFiles, expandedDirectories, query.trim().length > 0),
  );

  function buildRows(
    sourceFiles: DiffFileSummary[],
    expanded: Record<string, boolean>,
    expandAll: boolean,
  ): TreeRow[] {
    const root: DirectoryNode = {
      name: "",
      path: "",
      directories: new Map(),
      files: [],
    };

    for (const file of sourceFiles) {
      const path = displayPath(file);
      const parts = path.split("/");
      parts.pop();
      let directory = root;

      for (const part of parts) {
        const directoryPath = directory.path
          ? `${directory.path}/${part}`
          : part;
        let child = directory.directories.get(part);
        if (!child) {
          child = {
            name: part,
            path: directoryPath,
            directories: new Map(),
            files: [],
          };
          directory.directories.set(part, child);
        }
        directory = child;
      }

      directory.files.push(file);
    }

    const result: TreeRow[] = [];

    function visit(directory: DirectoryNode, depth: number) {
      for (const child of directory.directories.values()) {
        const isExpanded = expandAll || (expanded[child.path] ?? true);
        result.push({
          kind: "directory",
          name: child.name,
          path: child.path,
          depth,
          expanded: isExpanded,
        });
        if (isExpanded) visit(child, depth + 1);
      }

      for (const file of directory.files) {
        const path = displayPath(file);
        result.push({
          kind: "file",
          name: path.split("/").at(-1) ?? path,
          path,
          depth,
          file,
        });
      }
    }

    visit(root, 0);
    return result;
  }

  function toggleDirectory(path: string) {
    expandedDirectories[path] = !(expandedDirectories[path] ?? true);
  }
</script>

<div class="search">
  <Search size={13} />
  <input
    bind:value={query}
    placeholder="Filter changed files"
    aria-label="Filter changed files"
  />
</div>
<nav class="tree" aria-label="Changed files">
  {#each rows as row (row.kind === "directory" ? `directory:${row.path}` : `file:${row.path}`)}
    {#if row.kind === "directory"}
      <button
        class="directory-row"
        aria-expanded={row.expanded}
        aria-label={`${row.expanded ? "Collapse" : "Expand"} ${row.path}`}
        style={`--depth: ${row.depth}`}
        onclick={() => toggleDirectory(row.path)}
        title={row.path}
      >
        <span class="chevron">
          {#if row.expanded}<ChevronDown size={13} />{:else}<ChevronRight
              size={13}
            />{/if}
        </span>
        {#if row.expanded}<FolderOpen size={14} />{:else}<Folder
            size={14}
          />{/if}
        <strong>{row.name}</strong>
      </button>
    {:else}
      <button
        class="file-row"
        class:active={selectedPath === row.path}
        aria-current={selectedPath === row.path ? "true" : undefined}
        style={`--depth: ${row.depth}`}
        onclick={() => onSelect(row.path)}
        title={row.path}
      >
        <DiffFileStatusIcon status={row.file.status} />
        <strong>{row.name}</strong>
        {#if matchCounts[row.path]}
          <span class="match-count" title={`${matchCounts[row.path]} matches`}
            >{matchCounts[row.path]}</span
          >
        {/if}
      </button>
    {/if}
  {/each}
</nav>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 10px;
    padding: 7px 9px;
    color: var(--muted);
    background: var(--input);
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  input {
    width: 100%;
    min-width: 0;
    padding: 0;
    color: var(--text);
    background: transparent;
    border: 0;
    outline: 0;
    font: inherit;
    font-size: 12px;
  }
  .tree {
    padding: 0 6px 10px;
  }
  button {
    display: grid;
    align-items: center;
    width: 100%;
    min-width: 0;
    padding: 7px 7px 7px calc(7px + var(--depth) * 13px);
    color: var(--muted);
    text-align: left;
    background: transparent;
    border: 0;
    border-radius: 5px;
    cursor: pointer;
  }
  button:hover {
    color: var(--text);
    background: var(--hover);
  }
  .directory-row {
    grid-template-columns: 13px 14px minmax(0, 1fr);
    gap: 5px;
  }
  .directory-row > :global(svg) {
    color: #6f8593;
  }
  .chevron {
    display: grid;
    place-items: center;
  }
  .directory-row strong,
  .file-row strong {
    overflow: hidden;
    color: inherit;
    font-size: 12px;
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-row {
    grid-template-columns: 14px minmax(0, 1fr) auto;
    gap: 7px;
    padding-left: calc(16px + var(--depth) * 13px);
  }
  .file-row.active {
    color: var(--text);
    background: rgba(87, 184, 142, 0.11);
  }
  .match-count {
    min-width: 17px;
    padding: 1px 5px;
    color: #d5b76a;
    text-align: center;
    background: rgba(218, 177, 62, 0.12);
    border-radius: 8px;
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }
</style>
