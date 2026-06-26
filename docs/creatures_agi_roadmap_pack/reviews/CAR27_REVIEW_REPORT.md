# CAR27 Review Report - School and Semantic Safety

## Verdict

PASS_WITH_NOTES

No blocker, high, or medium findings remain for the CA23-CA27 tranche. CA28 may proceed only after the required user/ChatGPT consultation because CAR27 is a hard-stop review gate.

## Scope Reviewed

- CA23 graphical school mode and lesson panel.
- CA24 teacher avatar, gestures, speech tokens, and perception-only gating.
- CA25 curriculum authoring and verifier UI.
- CA26 real local semantic embedding provider.
- CA27 real local internal SLM subconscious prior adapter.
- Global constraints for school, teacher, semantic, SLM, action authority, persistence, model artifacts, and `alife_core` boundaries.

## Files Inspected

- `docs/creatures_agi_roadmap_pack/review_gates/CAR27_school-and-semantic-safety-review.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`
- `docs/creatures_agi_roadmap_pack/status/CA23_GRAPHICAL_SCHOOL_MODE.md`
- `docs/creatures_agi_roadmap_pack/status/CA24_TEACHER_WORLD_CUES.md`
- `docs/creatures_agi_roadmap_pack/status/CA25_CURRICULUM_AUTHORING_VERIFIER_UI.md`
- `docs/creatures_agi_roadmap_pack/status/CA26_REAL_SEMANTIC_PROVIDER_ADAPTER.md`
- `docs/creatures_agi_roadmap_pack/status/CA27_INTERNAL_SLM_SUBCONSCIOUS_PRIOR.md`
- `docs/creatures_agi_roadmap_pack/model_selection/LOCAL_MODEL_MANIFEST.md`
- `examples/model_manifests/local_semantic_models.json`
- `crates/alife_game_app/src/graphical_school.rs`
- `crates/alife_game_app/src/teacher_world_cues.rs`
- `crates/alife_game_app/src/curriculum_authoring.rs`
- `crates/alife_game_app/src/real_semantic_provider.rs`
- `crates/alife_game_app/src/internal_slm_prior.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_semantic/src/local_ollama.rs`
- `crates/alife_semantic/src/local_slm_prior.rs`
- `crates/alife_school/src/lib.rs`
- `crates/alife_core/Cargo.toml`

## Commands Run

Focused tranche evidence:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- graphical-school-mode-smoke
cargo run -p alife_game_app --bin alife_game_app -- teacher-world-cues-smoke
cargo run -p alife_game_app --bin alife_game_app -- curriculum-authoring-smoke
cargo run -p alife_game_app --bin alife_game_app -- real-semantic-provider-smoke
cargo run -p alife_game_app --bin alife_game_app -- internal-slm-prior-smoke
cargo test -p alife_game_app --test app_shell ca2 -- --nocapture
```

Boundary and artifact scans:

```powershell
git ls-files target models .cache
git tag --points-at HEAD
rg -n "OpenAI|Anthropic|Alibaba|Hugging Face Inference|api\.openai|api\.anthropic|generativelanguage|remote inference|hosted inference|paid API|cloud API" crates/alife_semantic/src crates/alife_game_app/src docs/creatures_agi_roadmap_pack examples/model_manifests/local_semantic_models.json
rg -n "can_issue_actions: true|can_rewrite_weights: true|can_bypass_arbitration: true|hidden_vector_injection: true|Bevy Entity|Entity\(" crates/alife_semantic/src crates/alife_school crates/alife_game_app/src crates/alife_game_app/tests/app_shell.rs docs/creatures_agi_roadmap_pack/status examples/model_manifests/local_semantic_models.json
rg -n "S12|G25|P37|release tag|tag a release" docs/creatures_agi_roadmap_pack crates/alife_game_app/src crates/alife_semantic/src examples/model_manifests/local_semantic_models.json
cargo tree -p alife_core
```

Standard validation protocol was run for CA27 before merge and rerun for this review branch before merge:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1
cargo tree -p alife_core
cargo check --workspace --all-features --all-targets
cargo test --workspace --all-features --all-targets
```

## Findings by Severity

### Blocker

None.

### High

None.

### Medium

None.

### Low / Notes

- The requested CA27 target model remains documented as `Qwen/Qwen3-4B-Instruct-2507`, while the local runnable smoke used the open-weight GGUF artifact `Qwen/Qwen3-4B-GGUF` through Ollama model `alife-qwen3-4b-prior`. The manifest records this distinction and the limitation is acceptable for local-only smoke evidence.
- CA26 and CA27 real inference evidence depends on local Ollama and local model files. Machines without those models should report `USER_ACTION_REQUIRED` instead of synthetic output.
- A deterministic `LocalSlmPriorQueue` remains for schema/parser queue tests; the product smoke uses the bounded asynchronous `LocalSlmPriorAsyncQueue`.

## Invariant Status

- `alife_core` remains engine-independent. `cargo tree -p alife_core` shows no Bevy, wgpu, renderer, school UI, semantic provider, or game-app dependency leak.
- Teacher, school, semantic provider, and SLM prior remain perception/context-only.
- No direct motor bypass was found in CA23-CA27 focused evidence.
- No semantic or SLM system is allowed to issue actions, rewrite weights, bypass arbitration, or inject hidden vectors into creature state.
- CA26 embeddings are bounded before entering game context.
- CA27 SLM output is parsed into bounded salience labels, context summary, lexicon associations, and perception tags only.
- Malformed SLM output is rejected.
- Timeout/unavailable model behavior is explicit and nonfatal.
- Stable-ID boundary remains in player-facing and portable surfaces; Bevy Entity IDs are rejected by tests and validation guards.
- No model weights, model caches, screenshots, logs, captures, target artifacts, `S12`, `G25`, `P37`, or release tag were created or tracked.

## User-Facing Status

The graphical school and teacher surfaces are visible as optional perception/curriculum context:

- School panel and lesson toggle are visible.
- Teacher cues are represented through visible/audible token objects and gesture markers.
- Curriculum authoring validates lesson manifests and verifier conditions.
- Real local semantic and SLM status is represented as optional local-model evidence, not as cloud inference or gameplay authority.

The tranche does not claim public release readiness or full action-authoritative GPU runtime.

## Evidence Gaps

- Independent human UX evidence for the school/semantic surfaces remains outside this review.
- CA26/CA27 local inference evidence is machine-specific; other machines must install or download the documented local models and use localhost-only runtime.
- The SLM prior is a bounded subconscious/perception prior, not a planner, teacher motor bypass, or action source.

## Fix Prompt if Needed

No fix prompt required. No blocker, high, or medium findings were found.

## Next Plan Recommendation

Stop at CAR27 for user/ChatGPT consultation. If the consultation accepts this `PASS_WITH_NOTES` verdict, the next executable plan is CA28. Do not start CA28 until explicitly approved.
