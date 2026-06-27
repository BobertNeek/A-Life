# CAR36 Performance and Parity Review

## Verdict

PASS_WITH_NOTES

No blocker, high, or medium findings were found for the CA32-CA36 tranche. CA37
may proceed only after user/ChatGPT consultation accepts this review gate.

## Scope Reviewed

CAR36 reviewed Phase H plans:

- CA32 - Real-time WGSL telemetry in app.
- CA33 - Batched multi-creature GPU runtime.
- CA34 - Sampled CPU-shadow graduation experiment.
- CA35 - Property-fuzz CPU/GPU parity gating.
- CA36 - Multi-hour soak isolation.

The review focused on hardware evidence, parity, GPU claims, sampled CPU-shadow
wording, artifact hygiene, and global invariants.

## Files Inspected

- `docs/creatures_agi_roadmap_pack/review_gates/CAR36_performance-and-parity-review.md`
- `docs/creatures_agi_roadmap_pack/status/CA32_REALTIME_WGSL_TELEMETRY.md`
- `docs/creatures_agi_roadmap_pack/status/CA33_BATCHED_MULTI_CREATURE_GPU_RUNTIME.md`
- `docs/creatures_agi_roadmap_pack/status/CA34_SAMPLED_CPU_SHADOW_GRADUATION.md`
- `docs/creatures_agi_roadmap_pack/status/CA35_PROPERTY_FUZZ_CPU_GPU_PARITY.md`
- `docs/creatures_agi_roadmap_pack/status/CA36_MULTI_HOUR_SOAK_ISOLATION.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/src/gpu_live_runtime.rs`
- `crates/alife_game_app/src/live_brain_bridge.rs`
- `crates/alife_game_app/src/neural_activity_profiler.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/src/soak_isolation.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_gpu_backend/src/full_runtime.rs`
- `crates/alife_gpu_backend/tests/property_fuzz_parity_gating.rs`
- `docs/creatures_agi_roadmap_pack/GLOBAL_INVARIANTS.md`
- `docs/creatures_agi_roadmap_pack/VALIDATION_PROTOCOL.md`
- `docs/master_spec.md`
- `docs/architecture_decisions.md`

## Commands Run

Focused review commands:

```powershell
cargo test -p alife_game_app --test app_shell ca32_realtime_wgsl_telemetry -- --nocapture
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- realtime-wgsl-telemetry-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo test -p alife_game_app --test app_shell ca33_batched_gpu_runtime -- --nocapture
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- batched-gpu-runtime-smoke crates/alife_world/tests/fixtures/gpu_alpha --creatures 3 --ticks 1 --cpu-shadow-every 1
cargo test -p alife_game_app --test app_shell ca34_sampled_gpu_runtime -- --nocapture
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- sampled-gpu-runtime-smoke crates/alife_world/tests/fixtures/gpu_alpha --creatures 3 --ticks 4 --warmup-ticks 1 --cpu-shadow-every 2
cargo test -p alife_gpu_backend --test property_fuzz_parity_gating -- --nocapture
cargo test -p alife_game_app --test app_shell ca36_soak_isolation -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- multi-hour-soak-isolation-smoke
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Post-CA36 main validation had already passed on stable main before this review
branch:

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

CAR36 report branch validation:

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

## Results

- CA32 realtime WGSL telemetry smoke selected `GpuPlastic` on the local GPU,
  reported fallback `None`, CPU shadow gate `true`, parity `true`, compact
  readback `64B`, no bulk readback, and `full_action_authoritative=false`.
- CA33 batched GPU runtime smoke selected `GpuPlastic` on NVIDIA GeForce RTX
  3050 Vulkan driver `581.80`, ran three creatures, checked CPU shadow every
  creature/tick, reported zero parity failures, applied H_shadow updates, and
  kept `full_action_authoritative_claim=false`.
- CA34 sampled GPU runtime smoke selected `GpuPlastic`, ran the sampled
  CPU-shadow experiment with `cpu_shadow_every=2`, reported zero parity failures,
  and used the claim `SampledCpuShadowGuardedStaticPlusLiveHShadow`, not full
  action-authoritative runtime.
- CA35 property fuzz parity tests passed four deterministic static/routing/Oja
  parity tests.
- CA36 soak isolation smoke produced the protocol summary with three 10k manual
  commands, one optional multi-hour command, two graphical/fallback commands,
  six precision/drift counters, untracked target artifact paths, CPU fallback
  preserved, CPU shadow parity preserved, no active bulk readback, and no release
  tag.
- Graphical smoke selected `GpuPlastic` with fallback `None`, GPU scores enabled,
  CPU shadow parity true, stable IDs true, and no GPU fallback.
- Forced fallback smoke selected `CpuReference` with fallback
  `HardwareUnavailable`, GPU claim `None`, GPU scores false, and CPU fallback
  visible.
- `benchmark_tiers -- --gpu-runtime` and `benchmark_tiers -- --gpu-runtime
  --measure-gpu` completed and wrote untracked reports under `target/artifacts/`.

## Findings by Severity

### BLOCKER

None.

### HIGH

None.

### MEDIUM

None.

### LOW

- CAR36-LOW-001: CA36 intentionally adds a manual multi-hour soak protocol, but
  the multi-hour soak itself was not run during normal validation. This is an
  accepted evidence boundary, not a blocker. Future operator evidence should use
  the documented `target/ca36_soak_isolation/` workflow and keep artifacts
  untracked.
- CAR36-LOW-002: CA34 sampled CPU-shadow wording is acceptable only as
  `SampledCpuShadowGuardedStaticPlusLiveHShadow`. It must not be shortened to
  full action-authoritative GPU runtime in UI, docs, or release language.

## Invariant Status

- `alife_core` dependency tree remains clean; no Bevy, wgpu, renderer,
  model-runtime, or app dependency leaked into core.
- CPU fallback remains available and explicitly visible when forced.
- CPU shadow parity remains the correctness gate for GPU proposal use.
- No full action-authoritative GPU runtime claim is made.
- No active bulk neural readback was added.
- H_shadow/lifetime updates remain bounded and post-seal through the existing
  core-owned contract.
- Stable IDs remain the player/developer-facing identity path.
- No screenshots, logs, target artifacts, model files, `S12`, `G25`, `P37`, or
  release tag were tracked.
- No active Ollama runtime, fake provider, paid API, cloud API, or remote hosted
  inference path was introduced by this tranche.

## User-Facing Status

The GPU alpha graphical path remains GPU-first in presentation and validates
bounded smoke on this machine. The local graphical smoke selected the GPU path;
forced fallback showed the CPU degraded mode. CAR36 does not add new visual
polish and does not change player controls or graphical semantics.

## Evidence Gaps

- Multi-hour soak execution remains manual/operator-run and local-hardware
  specific.
- Local RTX 3050/Vulkan evidence is not cross-machine GPU performance evidence.
- CA34 is a sampled CPU-shadow experiment, not a release-grade authority
  graduation.

## Fix Prompt if Needed

No fix prompt is required. There are no blocker, high, or medium findings.

## Next Plan Recommendation

Stop for user/ChatGPT consultation at CAR36. If this review is accepted, the
next manifest item is CA37 - Terrain, props, and world art style pass.
