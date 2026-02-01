# Copilot instructions for this repository

## Build / test / lint
This repo is a Rust CLI wrapper (`claudio`) with no build, lint, or automated test suite checked in.

**Quick validation**
- Build the release binary:
  - `cargo build --release`
- Run help/version (requires `claude` on your PATH):
  - `./target/release/claudio --help`
  - `./target/release/claudio --version`

**Manual smoke tests (requires external deps)**
- Start LM Studio’s server (per README):
  - `lms server start --port 1234`
- Interactive model picker (requires `lms`):
  - `./target/release/claudio`
- Non-interactive mode requires an explicit model:
  - `./target/release/claudio -p "Hello" --model openai/gpt-oss-20b`

## High-level architecture
- `src/main.rs` is a thin wrapper around **Claude Code** (`claude`).
- It configures Claude Code to talk to **LM Studio’s Anthropic-compatible endpoint** by exporting:
  - `ANTHROPIC_BASE_URL` (default `http://localhost:1234`)
  - `ANTHROPIC_AUTH_TOKEN` (default `lmstudio`)
- It inspects CLI args to decide whether the invocation *starts a session*:
  - For non-session commands (e.g. `--help`, `--version`, and subcommands like `doctor`/`update`), it should not prompt.
  - For session-starting invocations, if `--model` is not provided and the terminal is interactive, it prompts using an internal picker driven by `lms ls --llm --json`.
- After selection, it executes `claude` with the original args plus `--model <selected>`; otherwise it directly executes `claude "$@"`.

## Key conventions / repo-specific behavior
- Environment passthrough: if `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` are already set, the wrapper preserves them (only sets defaults).
- “Non-interactive” is detected by `-p/--print` or stdin/stdout not being a TTY; in that case the wrapper refuses to prompt and exits with code `2` unless `--model` was provided.
- The wrapper intentionally only prompts when it believes the `claude` invocation would start an interactive session; keep that behavior when adding new argument parsing.
- Dependency `lms` is only required for interactive model selection; avoid introducing hard dependencies for `--help`/`--version`/non-session paths.
