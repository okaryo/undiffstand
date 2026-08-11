<script lang="ts">
  import { Download, LoaderCircle } from '@lucide/svelte';
  import type { UpdateState } from '$lib/domain/update';

  let {
    state,
    onInstall
  }: {
    state: UpdateState;
    onInstall: () => void;
  } = $props();

  const installing = $derived(state === 'installing');
</script>

{#if state === 'available' || installing}
  <button
    class="update-action"
    class:installing
    type="button"
    disabled={installing}
    aria-label={installing ? 'Installing update' : 'Install available update'}
    title={installing ? 'Installing update' : 'Install available update'}
    onclick={onInstall}
  >
    {#if installing}
      <LoaderCircle class="spin" size={13} aria-hidden="true" />
      <span>Installing…</span>
    {:else}
      <Download size={13} aria-hidden="true" />
      <span>Install update</span>
    {/if}
  </button>
{/if}

<style>
  .update-action {
    display: inline-flex;
    flex: 0 0 auto;
    height: 28px;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 0 8px;
    color: #dff8ec;
    background: rgba(87, 184, 142, 0.11);
    border: 1px solid rgba(87, 184, 142, 0.3);
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  .update-action:hover:not(:disabled),
  .update-action:focus-visible {
    color: #f1fff8;
    background: rgba(87, 184, 142, 0.18);
    border-color: rgba(87, 184, 142, 0.48);
    outline: none;
  }

  .update-action:focus-visible {
    box-shadow: 0 0 0 2px rgba(87, 184, 142, 0.16);
  }

  .update-action:disabled {
    cursor: wait;
  }

  .update-action.installing {
    opacity: 0.75;
  }

  :global(.spin) {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
