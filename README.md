# ReaDiff

Understand code. Review changes.

ReaDiff is a read-only Tauri desktop app for reviewing committed Git changes and asking focused AI questions about local source code.

## Features

- Register and reopen local Git repositories
- Detect a default comparison ref and review `baseRef...HEAD`
- Inspect file statistics and Split/Unified diffs
- Browse Git-tracked files in a read-only CodeMirror viewer
- Ask about selected code or explain a changed file with OpenAI
- Jump from AI references back to source lines
- Persist project metadata with Tauri Store without storing API keys

## Requirements

- Node.js LTS
- pnpm
- Rust stable
- Git available on `PATH`
- Tauri 2 platform prerequisites

## Development

```bash
pnpm install
pnpm tauri dev
```

The frontend can also be opened independently with `pnpm dev`. Outside Tauri it uses deterministic in-memory demo data so Review and Browse flows can be tested without filesystem access or an OpenAI request.

## AI configuration

Set the API key in the environment before launching ReaDiff:

```bash
export OPENAI_API_KEY="your-key"
pnpm tauri dev
```

The default model is `gpt-5.6-terra`. Override it without changing project settings:

```bash
export READIFF_OPENAI_MODEL="gpt-5.6-terra"
```

The API key is read only by the Rust process. It is not exposed to the frontend, logged, or written to `projects.json`. Selected code and surrounding context are sent to the OpenAI Responses API only when an AI action is requested.

## Validation

```bash
pnpm check
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
pnpm build
pnpm tauri build
```

## Architecture

- Svelte 5/SvelteKit owns presentation and transient UI state.
- Typed, purpose-specific Tauri commands form the IPC boundary.
- Rust services handle Git CLI calls, repository-scoped file access, project persistence, and OpenAI requests.
- `@git-diff-view/svelte` and CodeMirror are isolated behind viewer components.
- Git commands run without a shell, and repository paths are canonicalized before files are read.

ReaDiff does not apply patches, modify repository files, or change Git state.
