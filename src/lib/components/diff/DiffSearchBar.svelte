<script lang="ts">
  import { tick } from "svelte";
  import {
    ChevronDown,
    ChevronUp,
    LoaderCircle,
    Search,
    X,
  } from "@lucide/svelte";

  let {
    query,
    current,
    total,
    pending = false,
    focusVersion,
    onQuery,
    onNext,
    onPrevious,
    onClose,
  }: {
    query: string;
    current: number;
    total: number;
    pending?: boolean;
    focusVersion: number;
    onQuery: (query: string) => void;
    onNext: () => void;
    onPrevious: () => void;
    onClose: () => void;
  } = $props();

  let inputElement = $state<HTMLInputElement>();

  $effect(() => {
    void focusVersion;
    void tick().then(() => {
      inputElement?.focus();
      inputElement?.select();
    });
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      if (event.shiftKey) onPrevious();
      else onNext();
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    }
  }
</script>

<div class="diff-search" role="search" aria-label="Find in changes">
  <Search size={14} />
  <input
    bind:this={inputElement}
    type="search"
    value={query}
    aria-label="Find in changes"
    placeholder="Find in changes"
    autocomplete="off"
    spellcheck="false"
    oninput={(event) => onQuery(event.currentTarget.value)}
    onkeydown={handleKeydown}
  />
  <span class="result-count" aria-live="polite">
    {#if pending}<LoaderCircle
        class="spin"
        size={13}
      />{:else if query.trim()}{current} / {total}{:else}0 / 0{/if}
  </span>
  <button
    type="button"
    aria-label="Previous match"
    title="Previous match (Shift+Enter)"
    disabled={total === 0}
    onclick={onPrevious}
  >
    <ChevronUp size={14} />
  </button>
  <button
    type="button"
    aria-label="Next match"
    title="Next match (Enter)"
    disabled={total === 0}
    onclick={onNext}
  >
    <ChevronDown size={14} />
  </button>
  <button
    type="button"
    aria-label="Close search"
    title="Close (Escape)"
    onclick={onClose}
  >
    <X size={14} />
  </button>
</div>

<style>
  .diff-search {
    position: absolute;
    z-index: 20;
    top: 8px;
    right: 18px;
    display: grid;
    grid-template-columns: 14px minmax(150px, 230px) auto 26px 26px 26px;
    align-items: center;
    gap: 5px;
    min-height: 34px;
    padding: 4px 5px 4px 9px;
    color: var(--muted);
    background: #131c25;
    border: 1px solid #34414d;
    border-radius: 7px;
    box-shadow: 0 5px 18px rgba(0, 0, 0, 0.38);
  }
  input {
    min-width: 0;
    padding: 4px 2px;
    color: var(--text);
    background: transparent;
    border: 0;
    outline: 0;
    font: 12px var(--mono);
  }
  input::-webkit-search-cancel-button {
    display: none;
  }
  .result-count {
    display: flex;
    min-width: 48px;
    align-items: center;
    justify-content: flex-end;
    color: #87939e;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  button {
    display: grid;
    width: 26px;
    height: 26px;
    place-items: center;
    padding: 0;
    color: #9aa5ae;
    cursor: pointer;
    background: transparent;
    border: 0;
    border-radius: 4px;
  }
  button:hover:not(:disabled) {
    color: var(--text);
    background: var(--hover);
  }
  button:disabled {
    cursor: default;
    opacity: 0.35;
  }
</style>
