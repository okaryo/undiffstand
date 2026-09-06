<script lang="ts">
  import { Check, File, FileDiff, FilePlus, FileX } from "@lucide/svelte";
  import type { DiffStatus } from "$lib/domain/diff";

  let {
    status,
    size = 14,
    reviewed = false,
  }: { status: DiffStatus; size?: number; reviewed?: boolean } = $props();
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
  {#if reviewed}
    <span class="review-check"><Check size={7} strokeWidth={4} /></span>
  {/if}
</span>

<style>
  .file-icon {
    position: relative;
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
  .review-check {
    position: absolute;
    right: -3px;
    bottom: -3px;
    display: grid;
    width: 9px;
    height: 9px;
    place-items: center;
    color: #07120e;
    background: var(--accent-bright);
    border: 1px solid #111923;
    border-radius: 50%;
  }
</style>
