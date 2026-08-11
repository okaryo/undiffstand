<script lang="ts">
  import { CircleAlert, Lightbulb } from "@lucide/svelte";
  import type { DiffExplanation, SourceReference } from "$lib/domain/ai";

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
    {#if explanation.keyChanges.length}<section>
        <h4>Key changes</h4>
        <ul>
          {#each explanation.keyChanges as item, index (index)}<li>
              {item}
            </li>{/each}
        </ul>
      </section>{/if}
    {#if explanation.references.length}{@render References(
        explanation.references,
      )}{/if}
    {#if explanation.caveats.length}{@render Caveats(explanation.caveats)}{/if}
  </div>
{/if}

{#snippet References(references: SourceReference[])}
  <section>
    <h4>References</h4>
    <div class="references">
      {#each references as reference, index (index)}
        <div>
          <span>{reference.path}</span><code
            >L{reference.startLine}–{reference.endLine}</code
          >
        </div>
      {/each}
    </div>
  </section>
{/snippet}

{#snippet Caveats(caveats: string[])}
  <section class="caveats">
    <h4><CircleAlert size={13} /> Caveats</h4>
    <ul>
      {#each caveats as caveat, index (index)}<li>{caveat}</li>{/each}
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
    font-size: 12px;
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
    font-size: 12px;
    font-weight: 620;
  }
  ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding-left: 17px;
    color: #aeb8c2;
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
    font: 12px var(--mono);
  }
  .caveats {
    color: #bca976;
  }
</style>
