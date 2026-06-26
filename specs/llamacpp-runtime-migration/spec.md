# llama.cpp Runtime Migration Spec

## Mode

Mode 2 - Full Spec Loop. This is a blocking runtime migration before CA28 and changes active model runtime behavior.

## Problem

CA26 and CA27 active local model runtime paths use Ollama localhost/GGUF. The user decision is to remove Ollama from active runtime code, manifests, smoke commands, and setup docs, and replace it with direct localhost-only llama.cpp / llama-server using the same real GGUF models.

## In Scope

- Replace active CA26 semantic embedding runtime with a llama-server `/v1/embeddings` client.
- Replace active CA27 SLM prior runtime with a llama-server `/v1/chat/completions` client.
- Keep existing bounded projection, parser, queue, no-action, no-weight, no-hidden-vector, and no-arbitration-bypass boundaries.
- Add llama.cpp smoke commands and preserve the existing compatibility smoke command names by routing them to llama.cpp.
- Add Windows PowerShell scripts that print and start the local llama-server commands.
- Update active manifests and current status docs so Ollama is superseded.

## Out of Scope

- CA28 or any roadmap implementation after CAR27.
- Cloud, paid, remote, hosted, OpenAI, Anthropic, Google, Alibaba, or Hugging Face Inference Provider calls.
- Committing model files, llama.cpp binaries, caches, logs, screenshots, target artifacts, or captures.
- Moving model runtime dependencies into `alife_core`.
- Letting semantic or SLM output issue actions, rewrite weights, bypass arbitration, or inject hidden vectors.

## Acceptance Criteria

- Active code paths no longer call Ollama endpoints or require Ollama model names.
- Active manifest entries use `llamacpp-server-gguf`.
- Remote and HTTPS model runtime URLs are rejected; `127.0.0.1` and `localhost` are accepted.
- Missing llama-server/model produces `USER_ACTION_REQUIRED` and no synthetic output.
- `real-semantic-provider-smoke` and `internal-slm-prior-smoke` use llama.cpp.
- `llamacpp-semantic-provider-smoke`, `llamacpp-slm-prior-smoke`, and `llamacpp-local-model-runtime-smoke` exist.
- llama.cpp local smoke commands run against real local GGUF models, or the migration stops with `USER_ACTION_REQUIRED`.
- Full validation passes and `alife_core` remains dependency-clean.

## Decisions

- Use existing standard-library TCP HTTP style instead of adding a new HTTP dependency.
- Use two llama-server ports: `18081` for SLM and `18082` for embeddings.
- Keep the old smoke command names as compatibility aliases, but their active implementation is llama.cpp.
