# claudio

A tiny wrapper that runs **Claude Code** (`claude`) against **LM Studio**’s Anthropic-compatible endpoint.

It’s a play on **Claude + LM Studio = claudio**.

## Quick start

1. Install Claude Code (`claude` on your PATH).
2. Start LM Studio’s server:

```sh
lms server start --port 1234
```

3. Run:

```sh
./claudio
```

## Usage

Specify a model (skips any prompt):

```sh
./claudio --model openai/gpt-oss-20b
```

Non-interactive mode (e.g. `-p/--print`) requires `--model`:

```sh
./claudio -p "Hello" --model openai/gpt-oss-20b
```

## Configuration

By default it sets:

```sh
ANTHROPIC_BASE_URL=http://localhost:1234
ANTHROPIC_AUTH_TOKEN=lmstudio
```

Set either environment variable yourself to override the defaults.

## Optional: interactive model picker deps

Interactive model selection uses `lms`, `jq`, and `gum`.

Reference: <https://lmstudio.ai/blog/claudecode>
