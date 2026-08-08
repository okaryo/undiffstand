<script lang="ts">
  import { AlertCircle, Lightbulb } from '@lucide/svelte';
  import type { DiffExplanation, SourceReference } from '$lib/domain/ai';

  let { explanation }: { explanation?: DiffExplanation } = $props();
</script>

{#if explanation}
  <div class="answer">
    <div class="eyebrow"><Lightbulb size={13} />AI inference</div>
    <p>{explanation.summary}</p>
    <section>
      <h4>Likely intent</h4>
      <p>{explanation.inferredIntent}</p>
    </section>
    <section>
      <h4>Risk <span class="risk {explanation.risk}">{explanation.risk}</span></h4>
      {#if explanation.concerns.length}<ul>
          {#each explanation.concerns as item}<li>{item}</li>{/each}
        </ul>{:else}<p class="muted">No specific concern identified from this diff.</p>{/if}
    </section>
    {#if explanation.references.length}{@render References(explanation.references)}{/if}
    {#if explanation.caveats.length}{@render Caveats(explanation.caveats)}{/if}
  </div>
{/if}

{#snippet References(references: SourceReference[])}
  <section>
    <h4>References</h4>
    <div class="references">
      {#each references as reference}
        <div>
          <span>{reference.path}</span><code>L{reference.startLine}–{reference.endLine}</code>
        </div>
      {/each}
    </div>
  </section>
{/snippet}

{#snippet Caveats(caveats: string[])}
  <section class="caveats">
    <h4><AlertCircle size={13} /> Caveats</h4>
    <ul>
      {#each caveats as caveat}<li>{caveat}</li>{/each}
    </ul>
  </section>
{/snippet}

<style>
  .answer {
    padding: 14px;
    font-size: 12px;
    line-height: 1.65;
  }
  .eyebrow {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 9px;
    color: var(--accent-bright);
    font-size: 10px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  p {
    margin: 0;
    color: #c0cad4;
  }
  section {
    margin-top: 18px;
    padding-top: 15px;
    border-top: 1px solid var(--border);
  }
  h4 {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 0 0 8px;
    color: #e0e6ec;
    font-size: 11px;
    font-weight: 620;
  }
  ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding-left: 17px;
    color: #aeb8c2;
  }
  .muted {
    color: var(--muted);
  }
  .risk {
    padding: 2px 5px;
    border-radius: 4px;
    font-size: 10px;
    text-transform: uppercase;
  }
  .risk.low {
    color: var(--green);
    background: rgba(67, 176, 119, 0.12);
  }
  .risk.medium {
    color: #d7b96c;
    background: rgba(215, 185, 108, 0.12);
  }
  .risk.high {
    color: var(--red);
    background: rgba(222, 101, 92, 0.12);
  }
  .references {
    display: grid;
    gap: 5px;
  }
  .references div {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
    width: 100%;
    padding: 7px 8px;
    color: #aeb8c2;
    background: #0c1219;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .references span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  code {
    color: var(--accent-bright);
    font: 10px var(--mono);
  }
  .caveats {
    color: #bca976;
  }
</style>
