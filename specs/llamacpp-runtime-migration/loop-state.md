# llama.cpp Runtime Migration Loop State

## Baseline

- Start branch: `main`
- Migration branch: `codex/migrate-local-model-runtime-to-llamacpp`
- Baseline main and origin/main: `5f46471`
- `llama-server.exe` was not on PATH at start.
- Existing GGUF model directory: `models/local/` and ignored by git.

## Current Status

- Implementation and branch validation complete.
- Local llama.cpp server install: winget `ggml.llamacpp`, `llama-server`
  version `9803 (5c7c22c3e)`.
- Real local model smokes passed through `127.0.0.1:18081` and
  `127.0.0.1:18082`.
- Awaiting R2 reviewer pass, commit, merge, main validation, and push.

## Stop Conditions

- Stop before CA28.
- Stop on validation failure that cannot be fixed locally.
- Stop with `USER_ACTION_REQUIRED` if llama.cpp cannot be installed/run or real local model smoke cannot run.
- Stop if a cloud/remote runtime would be needed.
