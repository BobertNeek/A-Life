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

Not selected in CA26. CA27 owns local SLM prior selection and must use a real
local runtime if it proceeds.
