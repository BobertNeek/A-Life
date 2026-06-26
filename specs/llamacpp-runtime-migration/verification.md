# llama.cpp Runtime Migration Verification

All required commands below passed on the migration branch unless noted.

## Focused

- `llama-server.exe --version`
  - PASS: `version: 9803 (5c7c22c3e) built with Clang 20.1.8 for Windows x86_64`.
- `llama-server.exe --help`
  - PASS: help exposes `--host`, `--port`, `--embedding`, `--pooling`, `--n-gpu-layers`, and reasoning controls used by the scripts.
- `cargo run -p alife_game_app --bin alife_game_app -- llamacpp-local-model-runtime-smoke`
  - PASS: semantic=pass, slm=pass, runtime=`llamacpp-server-gguf`, no_cloud=true, no_actions=true, no_weight_rewrite=true.
- `cargo run -p alife_game_app --bin alife_game_app -- llamacpp-semantic-provider-smoke`
  - PASS: endpoint `127.0.0.1:18082`, alias `alife-qwen3-embedding-0.6b`, raw_dims=1024, projected_dims=32.
- `cargo run -p alife_game_app --bin alife_game_app -- llamacpp-slm-prior-smoke`
  - PASS: endpoint `127.0.0.1:18081`, alias `alife-qwen3-4b-prior`, bounded labels/lexicon/tags, no authority flags.
- `cargo run -p alife_game_app --bin alife_game_app -- real-semantic-provider-smoke`
  - PASS: compatibility command routes to llama.cpp.
- `cargo run -p alife_game_app --bin alife_game_app -- internal-slm-prior-smoke`
  - PASS: compatibility command routes to llama.cpp.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/start_llamacpp_embedding_provider.ps1 -PrintOnly`
  - PASS: prints localhost llama.cpp command and rejects Ollama-bundled binaries.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/start_llamacpp_slm_prior.ps1 -PrintOnly`
  - PASS: prints localhost llama.cpp command with reasoning disabled and rejects Ollama-bundled binaries.
- `cargo test -p alife_semantic --features local-llamacpp`
  - PASS.
- `cargo test -p alife_game_app --test app_shell ca26 -- --nocapture`
  - PASS.
- `cargo test -p alife_game_app --test app_shell ca27 -- --nocapture`
  - PASS.

## Full

- `cargo fmt --all -- --check`
  - PASS.
- `cargo check --workspace --all-targets`
  - PASS.
- `cargo test --workspace --all-targets`
  - PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`
  - PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`
  - PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
  - PASS.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
  - PASS.
- `cargo tree -p alife_core`
  - PASS: no Bevy/wgpu/model-runtime dependency leak.
- `cargo check --workspace --all-features --all-targets`
  - PASS.
- `cargo test --workspace --all-features --all-targets`
  - PASS.

## Boundary Scans

- Active Ollama scan: remaining hits are historical/superseded docs or script
  guardrails rejecting Ollama-bundled `llama-server.exe`; active code/manifests
  do not select Ollama.
- Cloud/API scan: remaining hits are no-cloud guardrails or historical scan text;
  no active cloud, paid, or hosted inference call is present.
- Authority scan: semantic/SLM outputs remain perception/context-only and tests
  assert no action authority, weight rewrite, arbitration bypass, hidden vectors,
  or Bevy Entity ID leakage.
- Artifact scan: `git ls-files models .cache target` and
  `git ls-files target/artifacts graphify-out` returned no tracked artifacts.
