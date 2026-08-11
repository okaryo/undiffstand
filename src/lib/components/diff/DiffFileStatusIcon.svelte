<script lang="ts">
  import { File, FileDiff, FilePlus, FileX } from "@lucide/svelte";
  import type { DiffStatus } from "$lib/domain/diff";

  let { status, size = 14 }: { status: DiffStatus; size?: number } = $props();
</script>

<span class="file-icon {status}" aria-hidden="true">
  {#if status === "added"}
    <FilePlus {size} />
  {:else if status === "modified"}
    <FileDiff {size} />
  {:else if status === "deleted"}
    <FileX {size} />
  {:else}
    <File {size} />
  {/if}
</span>

<style>
  .file-icon {
    display: grid;
    place-items: center;
    color: #82909c;
  }
  .file-icon.added {
    color: var(--green);
  }
  .file-icon.modified {
    color: #6f8593;
  }
  .file-icon.deleted {
    color: var(--red);
  }
  .file-icon.renamed {
    color: #c3a5f8;
  }
  .file-icon.copied {
    color: #6dbeb9;
  }
  .file-icon.binary {
    color: #c5a66b;
  }
</style>
