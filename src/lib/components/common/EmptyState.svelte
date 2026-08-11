<script lang="ts">
  import type { Component } from "svelte";

  let {
    icon,
    title,
    message,
    compact = false,
    fill = false,
  }: {
    icon: Component;
    title: string;
    message: string;
    compact?: boolean;
    fill?: boolean;
  } = $props();
  const Icon = $derived(icon);
</script>

<div class:compact class:fill class="empty-state">
  <Icon size={compact ? 24 : 32} strokeWidth={1.6} />
  <h3>{title}</h3>
  <p>{message}</p>
</div>

<style>
  .empty-state {
    min-height: 260px;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 10px;
    padding: 32px;
    color: var(--muted);
    text-align: center;
  }

  .empty-state.compact {
    min-height: 160px;
    padding: 20px;
  }

  .empty-state.fill {
    height: 100%;
  }

  h3 {
    margin: 4px 0 0;
    color: var(--text);
    font-size: 15px;
    font-weight: 600;
  }

  p {
    max-width: 380px;
    margin: 0;
    font-size: 13px;
    line-height: 1.6;
  }
</style>
