# CA27 Internal SLM Subconscious Prior

Status: complete pending CAR27 review.

## Scope

CA27 adds a real local internal SLM prior boundary for school/semantic context.
The prior is private and optional. It produces bounded perception/context hints
only and cannot issue actions, bypass P09 arbitration, inject hidden vectors, or
rewrite weights.

## Local Model

| Field | Value |
| --- | --- |
| Target model | `Qwen/Qwen3-4B-Instruct-2507` |
| Runnable local artifact | `Qwen/Qwen3-4B-GGUF` / `Qwen3-4B-Q4_K_M.gguf` |
| Runtime | `ollama-localhost-gguf` |
| Ollama model | `alife-qwen3-4b-prior` |
| Local path | `models/local/qwen3-4b-gguf/Qwen3-4B-Q4_K_M.gguf` |
| SHA-256 | `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5` |
| License | Apache-2.0 |

The exact 2507 target repository was verified as available, but the local
runtime used the Qwen3 4B GGUF artifact because it is the clean local
Ollama/GGUF path on this machine. This is recorded in the tracked manifest and
is not presented as the exact 2507 artifact.

## Runtime Boundary

- Localhost-only Ollama inference.
- No paid, cloud, remote, or hosted inference API.
- Queue depth is bounded and inference runs through a worker-thread async
  request boundary so the app-facing queue does not execute model inference
  inline.
- Prompt length and generation budget are bounded.
- Output is structured JSON with:
  - salience labels,
  - short context summary,
  - lexicon associations,
  - perception tags.
- Malformed or authority-bearing output is rejected.
- Unavailable model/runtime reports `USER_ACTION_REQUIRED`; no fake model
  output is produced.

## Evidence

Focused real inference command:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- internal-slm-prior-smoke
```

Manual ignored smoke:

```powershell
cargo test -p alife_game_app --test app_shell ca27_real_local_slm_prior_smoke -- --ignored --nocapture
```

Expected output boundary:

- `target_repo_id=Qwen/Qwen3-4B-Instruct-2507`
- `repo_id=Qwen/Qwen3-4B-GGUF`
- `runtime=ollama-localhost-gguf`
- `model=alife-qwen3-4b-prior`
- `can_issue_actions=false`
- `can_rewrite_weights=false`
- `can_bypass_arbitration=false`
- `hidden_vector_injection=false`

## Known Limitations

- The SLM prior is not a teacher and does not directly affect motor output.
- The local model must be installed and runnable through Ollama for the real
  inference smoke to pass.
- The selected local runtime uses the Qwen3 4B GGUF artifact rather than an
  exact `Qwen/Qwen3-4B-Instruct-2507` GGUF file.
- This does not change the existing GPU runtime claim and does not create a
  release tag.
