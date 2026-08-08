<script lang="ts">
  import { onMount } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { EditorState, Compartment, type Extension } from '@codemirror/state';
  import { EditorView, type ViewUpdate } from '@codemirror/view';
  import { javascript } from '@codemirror/lang-javascript';
  import { json } from '@codemirror/lang-json';
  import { html } from '@codemirror/lang-html';
  import { css } from '@codemirror/lang-css';
  import { markdown } from '@codemirror/lang-markdown';
  import { python } from '@codemirror/lang-python';
  import { rust } from '@codemirror/lang-rust';
  import { createWorkingTreeSelection, type CodeSelection } from '$lib/domain/code-selection';

  let {
    path,
    content,
    language,
    targetLine = 0,
    onSelection
  }: {
    path: string;
    content: string;
    language: string;
    targetLine?: number;
    onSelection: (selection: CodeSelection | null) => void;
  } = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  const languageSlot = new Compartment();
  let previousContent = '';
  let previousLanguage = '';

  function languageExtension(name: string): Extension {
    switch (name) {
      case 'javascript': return javascript({ jsx: true });
      case 'typescript': return javascript({ jsx: true, typescript: true });
      case 'json': return json();
      case 'html': return html();
      case 'css': return css();
      case 'markdown': return markdown();
      case 'python': return python();
      case 'rust': return rust();
      default: return [];
    }
  }

  onMount(() => {
    previousContent = content;
    previousLanguage = language;
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: content,
        extensions: [
          basicSetup,
          languageSlot.of(languageExtension(language)),
          EditorState.readOnly.of(true),
          EditorView.editable.of(false),
          EditorView.lineWrapping,
          EditorView.theme({
            '&': { height: '100%', backgroundColor: '#0b1016', color: '#cdd6df' },
            '.cm-content': { fontFamily: 'var(--mono)', fontSize: '12px', lineHeight: '1.65', padding: '12px 0 80px' },
            '.cm-gutters': { backgroundColor: '#0b1016', color: '#4e5965', border: 'none' },
            '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'rgba(105, 137, 167, .08)' },
            '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'rgba(87, 184, 142, .23) !important' },
            '&.cm-focused': { outline: 'none' }
          }, { dark: true }),
          EditorView.updateListener.of((update: ViewUpdate) => {
            if (!update.selectionSet) return;
            const range = update.state.selection.main;
            if (range.empty) {
              onSelection(null);
              return;
            }
            const from = Math.min(range.from, range.to);
            const to = Math.max(range.from, range.to);
            onSelection(createWorkingTreeSelection(path, update.state.doc.toString(), from, to));
          })
        ]
      })
    });
    return () => view?.destroy();
  });

  $effect(() => {
    if (!view) return;
    if (content !== previousContent) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: content } });
      previousContent = content;
    }
    if (language !== previousLanguage) {
      view.dispatch({ effects: languageSlot.reconfigure(languageExtension(language)) });
      previousLanguage = language;
    }
  });

  $effect(() => {
    if (!view || targetLine <= 0) return;
    const line = view.state.doc.line(Math.min(targetLine, view.state.doc.lines));
    view.dispatch({
      selection: { anchor: line.from },
      effects: EditorView.scrollIntoView(line.from, { y: 'center' })
    });
  });
</script>

<div class="viewer" bind:this={host}></div>

<style>
  .viewer { height: 100%; min-height: 0; overflow: hidden; }
  .viewer :global(.cm-editor) { height: 100%; }
  .viewer :global(.cm-scroller) { overflow: auto; }
</style>
