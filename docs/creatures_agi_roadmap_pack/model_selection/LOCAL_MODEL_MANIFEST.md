# Local Model Manifest

This file records model selections for the local-only Creatures-to-AGI semantic
and SLM plans. It is a tracked manifest only. Model weights and caches stay
under ignored local directories such as `models/local/`.

## CA26 Semantic Embedding Provider

| Field | Value |
| --- | --- |
| Role | Semantic embedding provider |
| Repository | `Qwen/Qwen3-Embedding-0.6B-GGUF` |
| License | Apache-2.0 |
| Selected file | `Qwen3-Embedding-0.6B-Q8_0.gguf` |
| Runtime backend | `ollama-localhost-gguf` |
| Local path | `models/local/qwen3-embedding-0.6b-gguf/Qwen3-Embedding-0.6B-Q8_0.gguf` |
| Ollama model | `alife-qwen3-embedding-0.6b` |
| SHA-256 | `06507c7b42688469c4e7298b0a1e16deff06caf291cf0a5b278c308249c3e439` |
| Downloaded locally | yes |
| Inference smoke | passed locally |
| Raw embedding dims observed | 1024 |
| Projected game-context dims | 32 |

Tracked machine-readable manifest:
`examples/model_manifests/local_semantic_models.json`.

Local setup used for CA26:

```powershell
hf download Qwen/Qwen3-Embedding-0.6B-GGUF Qwen3-Embedding-0.6B-Q8_0.gguf --local-dir models/local/qwen3-embedding-0.6b-gguf
ollama create alife-qwen3-embedding-0.6b -f models/local/qwen3-embedding-0.6b-gguf/Modelfile
cargo run -p alife_game_app --bin alife_game_app -- real-semantic-provider-smoke
```

Boundary status:

- Localhost-only Ollama inference; no paid, cloud, or remote inference API.
- Embeddings are projected and bounded before entering game context.
- Semantic context is perception/context metadata only.
- The provider cannot issue actions, bypass motor arbitration, or rewrite weights.
- If the local model or runtime is unavailable, the feature reports
  `USER_ACTION_REQUIRED` instead of producing fake model output.

## CA27 SLM Prior

| Field | Value |
| --- | --- |
| Role | Internal SLM subconscious prior |
| Target repository | `Qwen/Qwen3-4B-Instruct-2507` |
| Runnable local repository | `Qwen/Qwen3-4B-GGUF` |
| License | Apache-2.0 |
| Selected file | `Qwen3-4B-Q4_K_M.gguf` |
| Runtime backend | `ollama-localhost-gguf` |
| Local path | `models/local/qwen3-4b-gguf/Qwen3-4B-Q4_K_M.gguf` |
| Ollama model | `alife-qwen3-4b-prior` |
| SHA-256 | `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5` |
| Downloaded locally | yes |
| Inference smoke | passed locally |
| Output schema | `alife.ca27.local_slm_prior_output.v1` |
| Context role | private perception/context prior only |

Tracked machine-readable manifest:
`examples/model_manifests/local_semantic_models.json`.

Local setup used for CA27:

```powershell
hf download Qwen/Qwen3-4B-GGUF Qwen3-4B-Q4_K_M.gguf --local-dir models/local/qwen3-4b-gguf
ollama create alife-qwen3-4b-prior -f models/local/qwen3-4b-gguf/Modelfile
cargo run -p alife_game_app --bin alife_game_app -- internal-slm-prior-smoke
```

Boundary status:

- The selected target SLM remains `Qwen/Qwen3-4B-Instruct-2507`, but the
  runnable local GGUF artifact used on this Windows machine is
  `Qwen/Qwen3-4B-GGUF` because the exact 2507 target repository exposes
  safetensors rather than the selected local Ollama/GGUF runtime file.
- Localhost-only Ollama inference; no paid, cloud, remote, or hosted inference
  API.
- Output is parsed as bounded JSON with salience labels, a short context
  summary, lexicon associations, and perception tags only.
- The prior cannot issue actions, bypass motor arbitration, inject hidden
  vectors, or rewrite weights.
- If the local model or runtime is unavailable, the feature reports
  `USER_ACTION_REQUIRED` instead of producing fake model output.
