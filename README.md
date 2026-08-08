# ReaDiff

Understand code. Review changes.

ReaDiff is a read-only Tauri desktop app for reviewing Git changes and asking focused AI questions about local source code.

## Features

- Register and reopen local Git repositories
- Select a local base branch and review from its merge base through the current working tree
- Include committed, staged, unstaged, and untracked changes in one review
- Inspect file statistics and Split/Unified diffs
- Browse tracked and untracked files in a read-only CodeMirror viewer
- Ask about selected code or explain a changed file with the local Codex CLI
- Jump from AI references back to source lines
- Persist project metadata with Tauri Store

## Requirements

- Node.js LTS
- pnpm
- Rust stable
- Git available on `PATH`
- Codex CLI available on `PATH` and authenticated with `codex login` (for AI features)
- Tauri 2 platform prerequisites

## Development

```bash
pnpm install
pnpm tauri dev
```

The frontend can also be opened independently with `pnpm dev`. Outside Tauri it uses deterministic in-memory demo data so Review and Browse flows can be tested without filesystem access or a Codex CLI invocation.

## AI configuration

Install and authenticate Codex CLI before launching ReaDiff:

```bash
codex login
pnpm tauri dev
```

ReaDiff runs `codex exec --ephemeral --sandbox read-only` in an isolated temporary directory and reuses the CLI's saved authentication, model, and local configuration. This avoids loading repository-specific agent instructions into the analysis process. `OPENAI_API_KEY` and `CODEX_API_KEY` are removed from the Codex child process so ReaDiff does not use API-key authentication. Selected code and surrounding context are provided only when an AI action is requested.

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
- Rust services handle Git CLI calls, repository-scoped file access, project persistence, and Codex CLI execution.
- `@git-diff-view/svelte` and CodeMirror are isolated behind viewer components.
- Git commands run without a shell, and repository paths are canonicalized before files are read.

ReaDiff does not apply patches, modify repository files, or change Git state.
