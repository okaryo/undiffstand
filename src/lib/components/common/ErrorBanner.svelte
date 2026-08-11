<script lang="ts">
  import { TriangleAlert, X } from '@lucide/svelte';
  import type { AppError } from '$lib/domain/error';

  let { error, onDismiss }: { error: AppError; onDismiss?: () => void } = $props();
</script>

<div class="banner" role="alert">
  <TriangleAlert size={17} />
  <div>
    <strong>{error.message}</strong>
    {#if error.detail}<span>{error.detail}</span>{/if}
  </div>
  {#if onDismiss}
    <button aria-label="Dismiss error" onclick={onDismiss}><X size={15} /></button>
  {/if}
</div>

<style>
  .banner {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: start;
    gap: 10px;
    padding: 11px 13px;
    color: #f3b6a8;
    background: rgba(201, 70, 43, 0.12);
    border: 1px solid rgba(231, 91, 61, 0.28);
    border-radius: 8px;
    font-size: 12px;
  }

  strong,
  span {
    display: block;
  }

  strong {
    color: #ffd5cb;
    font-weight: 550;
  }

  span {
    margin-top: 3px;
    color: #c58f84;
    word-break: break-word;
  }

  button {
    display: grid;
    padding: 1px;
    color: inherit;
    background: none;
    border: 0;
    cursor: pointer;
  }
</style>
