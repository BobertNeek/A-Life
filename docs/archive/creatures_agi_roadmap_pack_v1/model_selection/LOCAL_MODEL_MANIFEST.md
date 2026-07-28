# Local Model Manifest

This file records the active local-only Creatures-to-AGI semantic and SLM model
selection. It is a tracked manifest only. Model weights, llama.cpp binaries,
caches, and logs stay untracked under ignored local directories such as
`models/local/`.

Ollama is superseded for active runtime use. Historical CA26/CA27 review reports
may mention prior Ollama evidence, but current smoke commands and setup use
direct localhost-only llama.cpp / `llama-server`.

Tracked machine-readable manifest:
`examples/model_manifests/local_semantic_models.json`.

## CA26 Semantic Embedding Provider

| Field | Value |
| --- | --- |
| Role | Semantic embedding provider |
| Repository | `Qwen/Qwen3-Embedding-0.6B-GGUF` |
| License | Apache-2.0 |
| Selected file | `Qwen3-Embedding-0.6B-Q8_0.gguf` |
| Runtime backend | `llamacpp-server-gguf` |
| Local path | `models/local/qwen3-embedding-0.6b-gguf/Qwen3-Embedding-0.6B-Q8_0.gguf` |
| llama.cpp alias | `alife-qwen3-embedding-0.6b` |
| Endpoint | `http://127.0.0.1:18082/v1/embeddings` |
| SHA-256 | `06507c7b42688469c4e7298b0a1e16deff06caf291cf0a5b278c308249c3e439` |
| Downloaded locally | yes |
| Inference smoke | passed through direct llama.cpp |
| Projected game-context dims | 32 |

Local setup:

```powershell
hf download Qwen/Qwen3-Embedding-0.6B-GGUF Qwen3-Embedding-0.6B-Q8_0.gguf --local-dir models/local/qwen3-embedding-0.6b-gguf
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/start_llamacpp_embedding_provider.ps1
cargo run -p alife_game_app --bin alife_game_app -- llamacpp-semantic-provider-smoke
```

Equivalent server command:

```powershell
llama-server.exe -m models\local\qwen3-embedding-0.6b-gguf\Qwen3-Embedding-0.6B-Q8_0.gguf --host 127.0.0.1 --port 18082 --alias alife-qwen3-embedding-0.6b --embedding --pooling mean --n-gpu-layers 999
```

## CA27 SLM Prior

| Field | Value |
| --- | --- |
| Role | Internal SLM subconscious prior |
| Target repository | `Qwen/Qwen3-4B-Instruct-2507` |
| Runnable local repository | `Qwen/Qwen3-4B-GGUF` |
| License | Apache-2.0 |
| Selected file | `Qwen3-4B-Q4_K_M.gguf` |
| Runtime backend | `llamacpp-server-gguf` |
| Local path | `models/local/qwen3-4b-gguf/Qwen3-4B-Q4_K_M.gguf` |
| llama.cpp alias | `alife-qwen3-4b-prior` |
| Endpoint | `http://127.0.0.1:18081/v1/chat/completions` |
| SHA-256 | `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5` |
| Downloaded locally | yes |
| Inference smoke | passed through direct llama.cpp |
| Output schema | `alife.ca27.local_slm_prior_output.v1` |
| Context role | private perception/context prior only |

Local setup:

```powershell
hf download Qwen/Qwen3-4B-GGUF Qwen3-4B-Q4_K_M.gguf --local-dir models/local/qwen3-4b-gguf
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/start_llamacpp_slm_prior.ps1
cargo run -p alife_game_app --bin alife_game_app -- llamacpp-slm-prior-smoke
```

Equivalent server command:

```powershell
llama-server.exe -m models\local\qwen3-4b-gguf\Qwen3-4B-Q4_K_M.gguf --host 127.0.0.1 --port 18081 --alias alife-qwen3-4b-prior -c 4096 --reasoning off --reasoning-format none --reasoning-budget 0 --n-gpu-layers 999
```

Boundary status:

- Localhost-only llama.cpp inference; no paid, cloud, remote, or hosted
  inference API.
- Missing model files or stopped servers return `USER_ACTION_REQUIRED` instead
  of synthetic output.
- Embeddings are projected and bounded before entering game context.
- SLM output is parsed as bounded JSON with salience labels, a short context
  summary, lexicon associations, and perception tags only.
- Semantic and SLM outputs are perception/context only.
- Neither provider can issue actions, bypass P09 arbitration, inject hidden
  vectors, or rewrite weights.
