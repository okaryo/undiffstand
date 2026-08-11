<script lang="ts">
  import { Bot, Settings, X } from "@lucide/svelte";
  import SelectMenu from "$lib/components/common/SelectMenu.svelte";
  import {
    reviewOutputLanguageOptions,
    type ReviewOutputLanguage,
  } from "$lib/domain/preferences";

  let {
    outputLanguage,
    onOutputLanguageChange,
    onClose,
  }: {
    outputLanguage: ReviewOutputLanguage;
    onOutputLanguageChange: (language: ReviewOutputLanguage) => void;
    onClose: () => void;
  } = $props();
</script>

<div
  class="backdrop"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && onClose()}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
  >
    <header>
      <div>
        <Settings size={17} />
        <h2 id="settings-title">AI settings</h2>
      </div>
      <button aria-label="Close AI settings" onclick={onClose}
        ><X size={17} /></button
      >
    </header>
    <div class="content">
      <div class="language-setting">
        <span>Output language</span>
        <SelectMenu
          id="review-output-language-settings"
          label="Review output language"
          value={outputLanguage}
          options={[...reviewOutputLanguageOptions]}
          onChange={(language) =>
            onOutputLanguageChange(language as ReviewOutputLanguage)}
        />
        <p>
          Used for Inline Ask, file change explanations, and Change Review
          reports.
        </p>
      </div>
      <div>
        <span>Runtime</span><strong>Codex CLI</strong>
        <p>
          Inline and file explanations use <code>codex exec</code>. Change
          Review uses Codex's native
          <code>review</code> target when the selected comparison is compatible.
        </p>
      </div>
      <div>
        <span>Authentication</span><strong>codex login</strong>
        <p>
          Saved Codex CLI authentication and your local Codex configuration are
          reused. undiffstand removes API-key environment variables from the
          child process.
        </p>
      </div>
      <div class="privacy">
        <Bot size={15} />
        <p>
          Codex receives the active comparison or selected changed lines
          according to your local configuration. Results are kept only for this
          app session and may be wrong.
        </p>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    z-index: 50;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 7, 11, 0.72);
    backdrop-filter: blur(6px);
  }
  .dialog {
    width: min(490px, 100%);
    background: #111821;
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 15px 17px;
    border-bottom: 1px solid var(--border);
  }
  header div {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
  }
  header button {
    display: grid;
    padding: 3px;
    color: var(--muted);
    background: none;
    border: 0;
    cursor: pointer;
  }
  .content {
    display: grid;
    gap: 15px;
    padding: 18px;
  }
  .content > div:not(.privacy) {
    display: grid;
    grid-template-columns: 100px 1fr;
    gap: 3px 12px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
  }
  .content span {
    grid-row: 1/3;
    color: var(--muted);
    font-size: 12px;
  }
  .content strong {
    font: 12px var(--mono);
  }
  .content p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.55;
  }
  .content code {
    color: var(--accent-bright);
    font-family: var(--mono);
  }
  .privacy {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px;
    color: #c2ad70;
    background: rgba(183, 145, 52, 0.07);
    border: 1px solid rgba(183, 145, 52, 0.14);
    border-radius: 6px;
  }
  .privacy p {
    margin: 0;
    color: #a99361;
  }
</style>
