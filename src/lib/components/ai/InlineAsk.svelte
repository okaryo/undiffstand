<script lang="ts">
  import { onMount } from "svelte";
  import { Bot, LoaderCircle, Send, X } from "@lucide/svelte";
  import type { InlineAnswer } from "$lib/domain/ai";
  import { normalizeError } from "$lib/domain/error";

  let {
    side,
    startLine,
    endLine,
    onAsk,
    onClose,
  }: {
    side: "old" | "new";
    startLine: number;
    endLine: number;
    onAsk: (question: string) => Promise<InlineAnswer>;
    onClose: () => void;
  } = $props();

  let question = $state("");
  let answer = $state<InlineAnswer>();
  let loading = $state(false);
  let error = $state<string>();
  let questionInput = $state<HTMLTextAreaElement>();

  onMount(() => questionInput?.focus());

  async function ask() {
    const value = question.trim();
    if (!value || loading || answer) return;
    loading = true;
    error = undefined;
    try {
      answer = await onAsk(value);
    } catch (caught) {
      error = normalizeError(caught).message;
    } finally {
      loading = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && !event.isComposing && !question.trim()) {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      void ask();
    }
  }
</script>

<div class="inline-ask">
  <header>
    <div>
      <Bot size={15} /><strong
        >Ask about {side} lines {startLine}–{endLine}</strong
      >
    </div>
    <button aria-label="Close inline question" title="Close" onclick={onClose}
      ><X size={15} /></button
    >
  </header>
  {#if answer}
    <div class="answer">
      <p>{answer.answer}</p>
      {#if answer.caveats.length}<small>{answer.caveats.join(" ")}</small>{/if}
    </div>
  {:else}
    <textarea
      bind:this={questionInput}
      bind:value={question}
      onkeydown={handleKeydown}
      placeholder="Ask Codex about these changed lines…"
      aria-label="Question about selected lines"
    ></textarea>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="actions">
      <span>⌘↵ to ask · single response</span>
      <button class="ask" disabled={!question.trim() || loading} onclick={ask}>
        {#if loading}<LoaderCircle class="spin" size={14} /> Asking…{:else}<Send
            size={14}
          /> Ask Codex{/if}
      </button>
    </div>
  {/if}
</div>

<style>
  .inline-ask {
    margin: 8px;
    color: var(--text);
    background: #111923;
    border: 1px solid #2c3a48;
    border-radius: 8px;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.22);
    font:
      12px/1.55 Inter,
      ui-sans-serif,
      sans-serif;
  }
  header,
  header div,
  .actions,
  .ask {
    display: flex;
    align-items: center;
  }
  header {
    justify-content: space-between;
    padding: 9px 11px;
    border-bottom: 1px solid var(--border);
  }
  header div {
    gap: 7px;
  }
  header button {
    display: grid;
    place-items: center;
    padding: 3px;
    color: var(--muted);
    background: none;
    border: 0;
    cursor: pointer;
  }
  textarea {
    display: block;
    width: calc(100% - 20px);
    min-height: 86px;
    margin: 10px;
    padding: 10px;
    resize: vertical;
    color: var(--text);
    background: #0a1016;
    border: 1px solid #2c3a48;
    border-radius: 6px;
    outline: none;
    font: 12px/1.5 inherit;
  }
  textarea:focus {
    border-color: var(--accent);
  }
  .actions {
    justify-content: space-between;
    gap: 10px;
    padding: 0 10px 10px;
  }
  .actions span,
  .answer small {
    color: var(--muted);
    font-size: 12px;
  }
  .ask {
    gap: 6px;
    padding: 6px 9px;
    color: #07130e;
    background: var(--accent-bright);
    border: 0;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 650;
    cursor: pointer;
  }
  .ask:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .answer {
    padding: 12px;
  }
  .answer p {
    margin: 0 0 7px;
    white-space: pre-wrap;
  }
  .error {
    margin: 0 10px 9px;
    color: var(--red);
    font-size: 12px;
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
