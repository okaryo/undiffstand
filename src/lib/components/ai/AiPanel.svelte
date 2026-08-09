<script lang="ts">
  import { Bot, LoaderCircle, Sparkles } from '@lucide/svelte';
  import type { DiffExplanation } from '$lib/domain/ai';
  import AiAnswerView from './AiAnswer.svelte';

  let {
    explanation,
    loading = false,
    onExplain
  }: {
    explanation?: DiffExplanation;
    loading?: boolean;
    onExplain: () => void;
  } = $props();
</script>

<aside class="panel">
  <header>
    <div><Bot size={15} /><strong>ReaDiff AI</strong></div>
    <span>Inferences, not facts</span>
  </header>
  <div class="action-card">
    <Sparkles size={17} />
    <div>
      <strong>Explain this change</strong>
      <p>Summarize intent, risk, and evidence for the selected file.</p>
    </div>
    <button onclick={onExplain} disabled={loading}
      >{#if loading}<LoaderCircle class="spin" size={16} /> Analyzing…{:else}Explain changes{/if}</button
    >
  </div>
  <div class="results">
    {#if loading && !explanation}<div class="thinking">
        <LoaderCircle class="spin" size={16} /><span>Reviewing the available evidence…</span>
      </div>{/if}
    <AiAnswerView {explanation} />
  </div>
  <footer>
    AI analysis runs through your local Codex CLI in a read-only sandbox. ReaDiff does not read an
    API key.
  </footer>
</aside>

<style>
  .panel {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    min-width: 0;
    height: 100%;
    background: #0e141c;
    border-left: 1px solid var(--border);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 47px;
    padding: 0 13px;
    border-bottom: 1px solid var(--border);
  }
  header div {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  header strong {
    font-size: 11px;
    font-weight: 620;
  }
  header span {
    color: #68737e;
    font-size: 10px;
  }
  .action-card {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 9px;
    margin: 12px;
    padding: 12px;
    color: var(--accent-bright);
    background: rgba(87, 184, 142, 0.07);
    border: 1px solid rgba(87, 184, 142, 0.18);
    border-radius: 8px;
  }
  .action-card strong {
    color: var(--text);
    font-size: 11px;
  }
  .action-card p {
    margin: 3px 0 9px;
    color: var(--muted);
    font-size: 10px;
    line-height: 1.45;
  }
  .action-card button {
    grid-column: 2;
    justify-self: start;
    padding: 6px 9px;
    color: #07130e;
    background: var(--accent-bright);
    border: 0;
    border-radius: 5px;
    font: 600 10px inherit;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .results {
    min-height: 0;
    overflow: auto;
    border-top: 1px solid var(--border);
  }
  .thinking {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 15px;
    color: var(--muted);
    font-size: 11px;
  }
  footer {
    padding: 9px 12px;
    color: #59636e;
    border-top: 1px solid var(--border);
    font-size: 10px;
    line-height: 1.45;
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
