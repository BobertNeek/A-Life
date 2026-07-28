# CA33 Batched Multi-Creature GPU Runtime

## Status

Complete on branch `codex/CA33-batched-multi-creature-gpu-runtime`.

CA33 adds a bounded batched runtime smoke path for the GPU alpha population. It
uses the existing `CpuShadowGuardedStaticPlusLiveHShadow` runtime claim without
graduating action authority.

## What Changed

- Added `alife.ca33.batched_gpu_runtime.v1` as the batch runtime evidence
  schema.
- Added `batched-gpu-runtime-smoke <fixture-root>` with bounded options:
  - `--creatures N`
  - `--ticks N`
  - `--cpu-shadow-every 1`
  - `--json path`
- Added a stable-ID batch target loader from the portable save/world fixture.
- Added a shared GPU-session batch path across multiple saved creatures.
- Added per-creature compact summaries for:
  - stable ID,
  - organism ID,
  - selected backend/fallback,
  - GPU score usage,
  - CPU shadow parity,
  - compact readback bytes,
  - sealed patch and packed-log counts,
  - post-seal H_shadow application,
  - routing counters.
- Added explicit CA34 deferral for sampled CPU shadow parity. CA33 keeps
  every-creature/every-tick CPU shadow checks.

## Evidence Boundary

The batch path is a product/developer smoke command for multi-creature GPU
runtime evidence. It shares one selected GPU session for the bounded batch and
applies post-seal H_shadow deltas only through the existing core-owned
`CreatureMind::apply_post_seal_lifetime_deltas` contract.

CA33 does not claim full action-authoritative GPU runtime. CPU shadow parity
remains the correctness gate, and CPU fallback remains explicit.

## Invariants

- `alife_core` is unchanged and remains engine-independent.
- Stable IDs are used for player/developer-facing creature identity.
- No Bevy `Entity`, wgpu resource, GPU buffer, adapter ID, or raw tensor enters
  core or portable output.
- No active bulk neural readback is added.
- `W_genetic_fixed`, lifetime-consolidated weights, and H_operational remain
  unchanged.
- Sampled CPU shadow parity is not enabled in CA33; that experiment remains
  CA34.
- No screenshots, logs, target artifacts, model files, `S12`, `G25`, `P37`, or
  release tag are created by this plan.

## Focused Evidence

Planned/required CA33 focused commands:

```powershell
cargo test -p alife_game_app --test app_shell ca33_batched_gpu_runtime -- --nocapture
cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- batched-gpu-runtime-smoke crates/alife_world/tests/fixtures/gpu_alpha --creatures 3 --ticks 1 --cpu-shadow-every 1
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

## Known Limitations

- CA33 batches the bounded GPU alpha population. It is not a large-population
  benchmark and does not replace CA36 soak isolation.
- CPU shadow parity is still checked every creature/tick. Sampled parity and
  graduation evidence are deferred to CA34.
- The command is local hardware evidence when a GPU is selected and explicit
  fallback evidence otherwise.
