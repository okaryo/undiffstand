<script lang="ts">
  import { AlertTriangle, ShieldAlert } from '@lucide/svelte';
  import type { ChangeReviewFinding } from '$lib/domain/ai';

  let { findings }: { findings: ChangeReviewFinding[] } = $props();
</script>

<div class="finding-list">
  {#each findings as finding}
    <article class="finding {finding.severity}">
      <header>
        {#if finding.severity === 'critical' || finding.severity === 'high'}<ShieldAlert
            size={14}
          />{:else}<AlertTriangle size={14} />{/if}
        <strong>{finding.title}</strong><span>{finding.severity}</span>
      </header>
      <p>{finding.body}</p>
    </article>
  {/each}
</div>

<style>
  .finding-list {
    display: grid;
    gap: 7px;
    padding: 7px;
    font:
      12px/1.55 Inter,
      sans-serif;
  }
  .finding {
    padding: 10px;
    color: var(--text);
    background: #141b24;
    border: 1px solid #35404b;
    border-left: 3px solid #d0a553;
    border-radius: 6px;
  }
  .finding.critical,
  .finding.high {
    border-left-color: var(--red);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  header strong {
    flex: 1;
  }
  header span {
    color: #c9a960;
    font-size: 9px;
    text-transform: uppercase;
  }
  .critical header span,
  .high header span {
    color: var(--red);
  }
  p {
    margin: 6px 0 0;
    color: #b8c2cc;
    white-space: pre-wrap;
  }
</style>
