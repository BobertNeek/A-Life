# llama.cpp Runtime Migration Report

Status: validated for the blocking runtime migration before CA28.

## Decision

The active local model runtime is direct localhost-only llama.cpp /
`llama-server`. Ollama is superseded and must not be used by active runtime
code, active model manifests, smoke commands, or setup docs.

Historical CA26/CA27 review reports may mention Ollama as prior evidence. Those
mentions are historical only.

## Local Runtime

Installed llama.cpp server:

```text
C:\Users\PC\AppData\Local\Microsoft\WinGet\Packages\ggml.llamacpp_Microsoft.Winget.Source_8wekyb3d8bbwe\llama-server.exe
```

The Ollama-bundled `llama-server.exe`, if present, is intentionally rejected by
the A-Life launcher scripts.

Local server evidence on this machine reports:

- GPU adapter: NVIDIA GeForce RTX 3050.
- llama.cpp backend/device path: Vulkan.
- SLM port: `127.0.0.1:18081`.
- Embedding port: `127.0.0.1:18082`.

## Models

| Role | Model | File | Endpoint |
| --- | --- | --- | --- |
| Semantic embedding provider | `Qwen/Qwen3-Embedding-0.6B-GGUF` | `models/local/qwen3-embedding-0.6b-gguf/Qwen3-Embedding-0.6B-Q8_0.gguf` | `http://127.0.0.1:18082/v1/embeddings` |
| Internal SLM prior | `Qwen/Qwen3-4B-GGUF` for target `Qwen/Qwen3-4B-Instruct-2507` | `models/local/qwen3-4b-gguf/Qwen3-4B-Q4_K_M.gguf` | `http://127.0.0.1:18081/v1/chat/completions` |

Model weights remain untracked under `models/local/`.

## Startup Commands

Embedding server:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/start_llamacpp_embedding_provider.ps1
```

Equivalent direct command:

```powershell
llama-server.exe -m models\local\qwen3-embedding-0.6b-gguf\Qwen3-Embedding-0.6B-Q8_0.gguf --host 127.0.0.1 --port 18082 --alias alife-qwen3-embedding-0.6b --embedding --pooling mean --n-gpu-layers 999
```

SLM prior server:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/start_llamacpp_slm_prior.ps1
```

Equivalent direct command:

```powershell
llama-server.exe -m models\local\qwen3-4b-gguf\Qwen3-4B-Q4_K_M.gguf --host 127.0.0.1 --port 18081 --alias alife-qwen3-4b-prior -c 4096 --reasoning off --reasoning-format none --reasoning-budget 0 --n-gpu-layers 999
```

## Smoke Commands

```powershell
cargo run -p alife_game_app --bin alife_game_app -- llamacpp-local-model-runtime-smoke
cargo run -p alife_game_app --bin alife_game_app -- llamacpp-semantic-provider-smoke
cargo run -p alife_game_app --bin alife_game_app -- llamacpp-slm-prior-smoke
```

Compatibility commands remain active aliases to the llama.cpp client path:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- real-semantic-provider-smoke
cargo run -p alife_game_app --bin alife_game_app -- internal-slm-prior-smoke
```

## Boundaries

- No fake semantic provider or fake SLM prior output.
- No paid, cloud, remote, or hosted inference API.
- Only `127.0.0.1` and `localhost` are accepted.
- Missing server/model state reports `USER_ACTION_REQUIRED`.
- Semantic embedding vectors are projected to bounded game-context dimensions.
- SLM prior output is parsed as bounded JSON.
- Semantic and SLM outputs are perception/context only.
- Outputs cannot issue actions, rewrite weights, bypass arbitration, or inject
  hidden vectors.
- `alife_core` has no llama.cpp, model-runtime, Bevy, wgpu, or game-app
  dependency.

## CA28 Gate

CA28 may proceed only after the llama.cpp smoke commands pass, or after the user
explicitly accepts a documented blocker. This migration does not create S12,
G25, P37, or a release tag.

Current result: the direct llama.cpp semantic provider, SLM prior, compatibility
commands, and combined local-model runtime smoke all pass on this machine.
