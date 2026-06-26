# CA26 - Real Semantic Provider Adapter v1

Status: complete.

## Scope

CA26 adds an optional real local semantic embedding provider boundary. The
provider uses a local Ollama model imported from the open-weight
`Qwen/Qwen3-Embedding-0.6B-GGUF` artifact and exposes only bounded semantic
context metadata to the app.

## Local Model Evidence

Selected model:

- Repository: `Qwen/Qwen3-Embedding-0.6B-GGUF`
- File: `Qwen3-Embedding-0.6B-Q8_0.gguf`
- Runtime: Ollama on `127.0.0.1:11434`
- Ollama model: `alife-qwen3-embedding-0.6b`
- SHA-256:
  `06507c7b42688469c4e7298b0a1e16deff06caf291cf0a5b278c308249c3e439`
- Local smoke input: `teacher token food berry short lesson context`
- Observed raw embedding dimensions: 1024
- Bounded projected dimensions: 32

Canonical smoke command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- real-semantic-provider-smoke
```

## Boundary Guarantees

- No fake semantic provider output is used for CA26 evidence.
- No paid API, hosted API, or remote inference endpoint is used.
- Missing local Ollama/model state returns `USER_ACTION_REQUIRED`.
- Context vectors are projected and bounded before entering game context.
- Semantic output is perception/context only.
- The semantic provider cannot issue actions.
- The semantic provider cannot rewrite weights.
- Hidden vector injection into creature state remains blocked.
- P09 action arbitration remains the only action path.

## Tracked Artifacts

Tracked:

- `examples/model_manifests/local_semantic_models.json`
- `docs/creatures_agi_roadmap_pack/model_selection/LOCAL_MODEL_MANIFEST.md`

Ignored/untracked:

- `models/local/`
- `.cache/alife_models/`

No model weights or model caches are committed.

## Known Limitations

- This is semantic embedding provider v1, not the CA27 local SLM prior.
- Normal CI does not require Ollama or downloaded model weights.
- The manual real-inference test is ignored by default and must be run on a
  machine with the local model imported into Ollama.
- The semantic provider remains optional and non-authoritative.

## Next

CA27 owns the real local SLM subconscious prior boundary.
