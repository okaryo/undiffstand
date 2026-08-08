<script lang="ts">
  import { Bot, LoaderCircle, Send, Sparkles } from '@lucide/svelte';
  import type { AiAnswer, DiffExplanation, SourceReference } from '$lib/domain/ai';
  import type { CodeSelection } from '$lib/domain/code-selection';
  import AiAnswerView from './AiAnswer.svelte';

  let {
    mode,
    selection,
    answer,
    explanation,
    loading = false,
    onAsk,
    onExplain,
    onReference
  }: {
    mode: 'review' | 'browse';
    selection?: CodeSelection | null;
    answer?: AiAnswer;
    explanation?: DiffExplanation;
    loading?: boolean;
    onAsk: (question: string) => void;
    onExplain: () => void;
    onReference: (reference: SourceReference) => void;
  } = $props();

  let question = $state('');
  const presets = ['What does this code do?', 'Explain the control flow', 'What edge cases matter?'];

  function submit() {
    if (question.trim() && selection && !loading) onAsk(question.trim());
  }
</script>

<aside class="panel">
  <header><div><Bot size={15} /><strong>ReaDiff AI</strong></div><span>Inferences, not facts</span></header>
  {#if mode === 'review'}
    <div class="action-card">
      <Sparkles size={17} />
      <div><strong>Explain this change</strong><p>Summarize intent, risk, and evidence for the selected file.</p></div>
      <button onclick={onExplain} disabled={loading}>{#if loading}<LoaderCircle class="spin" size={14} /> Analyzing…{:else}Explain changes{/if}</button>
    </div>
  {:else}
    <div class="selection" class:ready={selection}>
      {#if selection}<strong>{selection.path}</strong><span>Lines {selection.startLine}–{selection.endLine} selected</span>{:else}<strong>Select some code</strong><span>Drag across one or more lines in the viewer.</span>{/if}
    </div>
    <div class="presets">{#each presets as preset}<button onclick={() => question = preset} disabled={!selection}>{preset}</button>{/each}</div>
    <div class="composer">
      <textarea bind:value={question} placeholder="Ask about the selected code…" disabled={!selection || loading} onkeydown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') submit(); }}></textarea>
      <button class="send" aria-label="Ask AI" onclick={submit} disabled={!selection || !question.trim() || loading}>{#if loading}<LoaderCircle class="spin" size={15} />{:else}<Send size={15} />{/if}</button>
    </div>
  {/if}
  <div class="results">
    {#if loading && !answer && !explanation}<div class="thinking"><LoaderCircle class="spin" size={17} /><span>Reviewing the available evidence…</span></div>{/if}
    <AiAnswerView {answer} {explanation} {onReference} />
  </div>
  <footer>Selected code is sent to the OpenAI API. ReaDiff never sends or stores your API key in project settings.</footer>
</aside>

<style>
  .panel { display:grid; grid-template-rows:auto auto auto auto minmax(0,1fr) auto; min-width:0; height:100%; background:#0e141c; border-left:1px solid var(--border); }
  header { display:flex; align-items:center; justify-content:space-between; min-height:47px; padding:0 13px; border-bottom:1px solid var(--border); }
  header div { display:flex; align-items:center; gap:7px; } header strong { font-size:11px; font-weight:620; } header span { color:#68737e; font-size:9px; }
  .action-card { display:grid; grid-template-columns:auto 1fr; gap:9px; margin:12px; padding:12px; color:var(--accent-bright); background:rgba(87,184,142,.07); border:1px solid rgba(87,184,142,.18); border-radius:8px; }
  .action-card strong { color:var(--text); font-size:11px; } .action-card p { margin:3px 0 9px; color:var(--muted); font-size:10px; line-height:1.45; }
  .action-card button { grid-column:2; justify-self:start; padding:6px 9px; color:#07130e; background:var(--accent-bright); border:0; border-radius:5px; font:600 10px inherit; cursor:pointer; }
  .selection { display:grid; gap:2px; margin:11px 12px 5px; padding:9px 10px; color:var(--muted); background:#0a1016; border:1px dashed var(--border-strong); border-radius:6px; }
  .selection.ready { border-style:solid; border-color:rgba(87,184,142,.22); } .selection strong { overflow:hidden; color:#b8c2cc; font-size:10px; text-overflow:ellipsis; white-space:nowrap; } .selection span { font-size:9px; }
  .presets { display:flex; flex-wrap:wrap; gap:5px; padding:5px 12px 8px; }
  .presets button { padding:4px 6px; color:#87929d; background:#111923; border:1px solid var(--border); border-radius:4px; font:9px inherit; cursor:pointer; }
  .presets button:disabled { opacity:.35; }
  .composer { position:relative; margin:0 12px 10px; }
  textarea { width:100%; min-height:70px; box-sizing:border-box; resize:vertical; padding:9px 36px 9px 9px; color:var(--text); background:#090f15; border:1px solid var(--border-strong); border-radius:7px; outline:0; font:11px/1.5 inherit; }
  textarea:focus { border-color:var(--accent); }
  .send { position:absolute; right:7px; bottom:7px; display:grid; place-items:center; width:25px; height:25px; color:#07130e; background:var(--accent-bright); border:0; border-radius:5px; cursor:pointer; }
  button:disabled { opacity:.45; cursor:default; }
  .results { min-height:0; overflow:auto; border-top:1px solid var(--border); }
  .thinking { display:flex; align-items:center; gap:8px; padding:15px; color:var(--muted); font-size:11px; }
  footer { padding:9px 12px; color:#59636e; border-top:1px solid var(--border); font-size:8.5px; line-height:1.45; }
  :global(.spin) { animation:spin 1s linear infinite; } @keyframes spin { to { transform:rotate(360deg); } }
</style>
