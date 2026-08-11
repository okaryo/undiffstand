<script lang="ts">
  import {
    AlertTriangle,
    Bot,
    ChevronDown,
    ChevronRight,
    LoaderCircle,
    Maximize2,
    Minimize2,
    ShieldAlert,
    Sparkles
  } from '@lucide/svelte';
  import SelectMenu from '$lib/components/common/SelectMenu.svelte';
  import type { ChangeReviewAvailability, ChangeReviewReport } from '$lib/domain/ai';
  import { reviewOutputLanguageOptions, type ReviewOutputLanguage } from '$lib/domain/preferences';

  let {
    availability,
    report,
    loading = false,
    expanded = false,
    outputLanguage = 'english',
    onReview,
    onToggleExpanded,
    onOutputLanguageChange
  }: {
    availability?: ChangeReviewAvailability;
    report?: ChangeReviewReport;
    loading?: boolean;
    expanded?: boolean;
    outputLanguage?: ReviewOutputLanguage;
    onReview: () => void;
    onToggleExpanded: () => void;
    onOutputLanguageChange: (language: ReviewOutputLanguage) => void;
  } = $props();

  let openGroups = $state<Record<string, boolean>>({});
  function toggleGroup(id: string) {
    openGroups[id] = !(openGroups[id] ?? true);
  }
</script>

<aside class="panel">
  <header class="panel-header">
    <div><Bot size={15} /><strong>Change Review</strong></div>
    <button
      aria-label={expanded ? 'Restore review panel width' : 'Expand review panel'}
      title={expanded ? 'Restore width' : 'Expand panel'}
      onclick={onToggleExpanded}
      >{#if expanded}<Minimize2 size={14} />{:else}<Maximize2 size={14} />{/if}</button
    >
  </header>

  <div class="action-card">
    <Sparkles size={17} />
    <div>
      <strong>Review changes</strong>
      <p>{availability?.scopeLabel ?? 'Checking the selected comparison…'}</p>
      {#if availability && !availability.available}<small>{availability.reason}</small>{/if}
      <label class="output-language">
        <span>Output</span>
        <SelectMenu
          id="review-output-language-panel"
          label="Review output language"
          value={outputLanguage}
          options={[...reviewOutputLanguageOptions]}
          disabled={loading}
          fluid
          onChange={(language) => onOutputLanguageChange(language as ReviewOutputLanguage)}
        />
      </label>
    </div>
    <button onclick={onReview} disabled={loading || !availability?.available}>
      {#if loading}<LoaderCircle class="spin" size={16} /> Reviewing…{:else}Run review{/if}
    </button>
  </div>

  <div class="results">
    {#if loading && !report}<div class="thinking">
        <LoaderCircle class="spin" size={16} />
        <span>Codex is reviewing this comparison. Large changes may take a while…</span>
      </div>{/if}
    {#if report}
      <article class="report">
        <div class="eyebrow">Review report</div>
        <p class="summary">{report.summary}</p>
        <section>
          <h3>Likely intent</h3>
          <p>{report.inferredIntent}</p>
        </section>
        {#if report.groups.length}<section>
            <h3>Change groups</h3>
            <div class="groups">
              {#each report.groups as group}
                <article class="group">
                  <button onclick={() => toggleGroup(group.id)}>
                    {#if openGroups[group.id] ?? true}<ChevronDown size={14} />{:else}<ChevronRight
                        size={14}
                      />{/if}
                    <strong>{group.title}</strong><span>{group.files.length}</span>
                  </button>
                  {#if openGroups[group.id] ?? true}<div>
                      <p>{group.summary}</p>
                      {#if group.keyPoints.length}<ul>
                          {#each group.keyPoints as point}<li>{point}</li>{/each}
                        </ul>{/if}
                      {#if group.files.length}<small>{group.files.join(' · ')}</small>{/if}
                    </div>{/if}
                </article>
              {/each}
            </div>
          </section>{/if}
        <section>
          <h3>Findings <span>{report.findings.length}</span></h3>
          {#if report.findings.length}<div class="findings">
              {#each report.findings as finding}
                <article class="finding {finding.severity}">
                  <div>
                    {#if finding.severity === 'critical' || finding.severity === 'high'}<ShieldAlert
                        size={13}
                      />{:else}<AlertTriangle size={13} />{/if}
                    <strong>{finding.title}</strong><span>{finding.severity}</span>
                  </div>
                  <p>{finding.body}</p>
                  <code>{finding.path}:L{finding.startLine}–{finding.endLine}</code>
                </article>
              {/each}
            </div>{:else}<p class="muted">
              Codex did not identify a concrete issue in this comparison.
            </p>{/if}
        </section>
        {#if report.caveats.length}<section class="caveats">
            <h3>Caveats</h3>
            <ul>
              {#each report.caveats as caveat}<li>{caveat}</li>{/each}
            </ul>
          </section>{/if}
      </article>
    {/if}
  </div>

  <footer>
    Uses the local Codex CLI in a read-only sandbox. Results are not saved by undiffstand.
  </footer>
</aside>

<style>
  .panel {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    min-width: 0;
    min-height: 0;
    max-height: 100%;
    height: 100%;
    overflow: hidden;
    background: #0e141c;
    border-left: 1px solid var(--border);
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 47px;
    padding: 0 13px;
    border-bottom: 1px solid var(--border);
  }
  .panel-header div {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .panel-header strong {
    font-size: 12px;
    font-weight: 620;
  }
  .panel-header button {
    display: grid;
    place-items: center;
    padding: 5px;
    color: var(--muted);
    background: none;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
  }
  .panel-header button:hover {
    color: var(--text);
    background: var(--hover);
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
    font-size: 12px;
  }
  .action-card p {
    margin: 3px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }
  .action-card small {
    display: block;
    margin-top: 7px;
    color: #c5a864;
    font-size: 12px;
    line-height: 1.45;
  }
  .output-language {
    display: flex;
    align-items: center;
    width: 100%;
    max-width: 245px;
    gap: 7px;
    margin-top: 9px;
    color: var(--muted);
    font-size: 12px;
  }
  .action-card > button {
    grid-column: 2;
    justify-self: start;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding: 6px 9px;
    color: #07130e;
    background: var(--accent-bright);
    border: 0;
    border-radius: 5px;
    font: 600 12px inherit;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .results {
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    border-top: 1px solid var(--border);
  }
  .thinking {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 15px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .report {
    padding: 14px;
    font-size: 12px;
    line-height: 1.6;
  }
  .eyebrow {
    margin-bottom: 8px;
    color: var(--accent-bright);
    font-size: 12px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  p {
    margin: 0;
    color: #bbc5cf;
  }
  .summary {
    color: #d0d7df;
  }
  section {
    margin-top: 17px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 8px;
    color: #e0e6ec;
    font-size: 12px;
  }
  h3 span {
    color: var(--muted);
    font-size: 12px;
  }
  .groups,
  .findings {
    display: grid;
    gap: 7px;
  }
  .group,
  .finding {
    overflow: hidden;
    background: #0b1118;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .group > button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px;
    color: var(--text);
    background: none;
    border: 0;
    cursor: pointer;
    text-align: left;
  }
  .group > button strong {
    flex: 1;
    font-size: 12px;
  }
  .group > button span {
    color: var(--muted);
    font-size: 12px;
  }
  .group > div {
    padding: 0 9px 9px 28px;
  }
  ul {
    display: grid;
    gap: 4px;
    margin: 7px 0 0;
    padding-left: 17px;
    color: #aeb8c2;
  }
  .group small {
    display: block;
    margin-top: 8px;
    color: #65717d;
    font: 12px/1.45 var(--mono);
    overflow-wrap: anywhere;
  }
  .finding {
    padding: 9px;
    border-left: 3px solid #c8a457;
  }
  .finding.critical,
  .finding.high {
    border-left-color: var(--red);
  }
  .finding > div {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .finding strong {
    flex: 1;
    font-size: 12px;
  }
  .finding span {
    color: #c8a457;
    font-size: 12px;
    text-transform: uppercase;
  }
  .finding.critical span,
  .finding.high span {
    color: var(--red);
  }
  .finding p {
    margin-top: 6px;
  }
  code {
    display: block;
    margin-top: 7px;
    color: var(--accent-bright);
    font: 12px var(--mono);
    overflow-wrap: anywhere;
  }
  .muted {
    color: var(--muted);
  }
  .caveats {
    color: #bca976;
  }
  footer {
    padding: 9px 12px;
    color: #59636e;
    border-top: 1px solid var(--border);
    font-size: 12px;
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
