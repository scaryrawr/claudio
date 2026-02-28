# claudio

A tiny wrapper that runs **Claude Code** (`claude`) against **LM Studio**’s Anthropic-compatible endpoint.

It’s a play on **Claude + LM Studio = claudio**.

## Quick start

1. Install Claude Code (`claude` on your PATH).
2. Start LM Studio’s server:

```sh
lms server start --port 1234
```

3. Build the Rust binary:

```sh
cargo build --release
```

4. Run:

```sh
./target/release/claudio
```

If Ollama is running locally, interactive model selection will also include Ollama models.

## Usage

Specify a model (skips any prompt):

```sh
claudio --model openai/gpt-oss-20b
```

Non-interactive mode (e.g. `-p/--print`) requires `--model`:

```sh
claudio -p "Hello" --model openai/gpt-oss-20b
```

Ollama manual setup example:

```sh
ANTHROPIC_AUTH_TOKEN=ollama ANTHROPIC_BASE_URL=http://localhost:11434 ANTHROPIC_API_KEY="" claudio --model qwen3-coder
```

## Configuration

By default it sets:

```sh
ANTHROPIC_BASE_URL=http://localhost:1234
ANTHROPIC_AUTH_TOKEN=lmstudio
```

Set either environment variable yourself to override the defaults.

For interactive selection without `--model`, `claudio` probes:
- LM Studio at `http://localhost:1234` (`/api/v0/models`)
- Ollama at `http://localhost:11434` (`/api/tags`)

The selected model determines which provider base URL is used for the launched `claude` process.

## Optional: interactive model picker deps

Interactive model selection is built-in and queries local LM Studio/Ollama HTTP APIs.

Reference: <https://lmstudio.ai/blog/claudecode>
