# Copilot instructions for this repository

## Build / test / lint
This repo is a single Bash wrapper script (`./claudio`) with no build, lint, or automated test suite checked in.

**Quick validation**
- Check script syntax:
  - `bash -n ./claudio`
- Run help/version without requiring LM Studio:
  - `./claudio --help`
  - `./claudio --version`

**Manual smoke tests (requires external deps)**
- Start LM Studio’s server (per README):
  - `lms server start --port 1234`
- Interactive model picker (requires `lms`, `jq`, `gum`):
  - `./claudio`
- Non-interactive mode requires an explicit model:
  - `./claudio -p "Hello" --model openai/gpt-oss-20b`

## High-level architecture
- `./claudio` is a thin wrapper around **Claude Code** (`claude`).
- It configures Claude Code to talk to **LM Studio’s Anthropic-compatible endpoint** by exporting:
  - `ANTHROPIC_BASE_URL` (default `http://localhost:1234`)
  - `ANTHROPIC_AUTH_TOKEN` (default `lmstudio`)
- It inspects CLI args to decide whether the invocation *starts a session*:
  - For non-session commands (e.g. `--help`, `--version`, and subcommands like `doctor`/`update`), it should not prompt.
  - For session-starting invocations, if `--model` is not provided and the terminal is interactive, it prompts via `gum choose` using the model list from `lms ls --llm --json | jq -r '.[].modelKey'`.
- After selection, it `exec`s `claude` with the original args plus `--model <selected>`; otherwise it directly `exec`s `claude "$@"`.

## Key conventions / repo-specific behavior
- Environment passthrough: if `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` are already set, the script preserves them (only sets defaults).
- “Non-interactive” is detected by `-p/--print` or stdin/stdout not being a TTY; in that case the script refuses to prompt and exits with code `2` unless `--model` was provided.
- The wrapper intentionally only prompts when it believes the `claude` invocation would start an interactive session; keep that behavior when adding new argument parsing.
- Dependencies (`lms`, `jq`, `gum`) are only required for interactive model selection; avoid introducing hard dependencies for `--help`/`--version`/non-session paths.
