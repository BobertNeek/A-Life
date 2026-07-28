# Procedural World Streaming Evidence

Plan: CA44A follow-up visual/world triage evidence
Branch: codex/procedural-world-streaming-evidence

## Objective

Address the single-screen concern by proving the GPU alpha world is generated
from a deterministic seed around creature anchors. The player camera may show a
local slice, but the world contract now records creature travel across multiple
procedural chunk windows with terrain and content generated without rendering.

## Implementation

- Added `ProceduralWorldTravelReport` and per-step reports in `alife_world`.
- Added `simulate_procedural_world_travel` for deterministic route evidence.
- Added `procedural-world-travel-smoke <fixture-root>` in `alife_game_app`.
- Added tests for deterministic travel, chunk streaming, no-rendering
  generation, and no action or weight authority.

## Evidence Command

```powershell
cargo run -p alife_game_app --bin alife_game_app -- procedural-world-travel-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Expected evidence:

- same seed produces the same report;
- route materializes more unique chunks than one active chunk window;
- active chunks are creature-anchored;
- content and terrain neighborhoods are generated without rendering;
- no chunks are active when no creature anchor exists;
- procedural terrain/content cannot emit actions or rewrite weights.

Local result:

- seed: `4242`;
- selected stable creature: `1`;
- route steps: `6`;
- unique materialized chunks over route: `138`;
- maximum active chunk window: `25`;
- content candidates observed over route: `3072`;
- generated without rendering: `true`;
- rendering required: `false`;
- action authority: `false`;
- weight authority: `false`.

## Visual Scope

The current Player View still needs stronger production art direction. This
status closes only the "single screen" evidence gap by making the world
streaming contract explicit and testable. It does not claim the art now matches
the target mockup.

## Invariant Checks

- `alife_core` remains engine-independent.
- No Bevy, wgpu, renderer, or model-runtime dependencies were added to
  `alife_core`.
- Procedural world reports are context/sensory evidence only.
- P09 action arbitration remains the only action path.
- CPU fallback and CPU shadow parity are unchanged.
- No full action-authoritative GPU runtime claim was added.
- No S12, G25, or P37 was created.
- No release tag was created.

## Commands Run

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo test -p alife_world --test procedural_chunks procedural_world_travel -- --nocapture`
- `cargo test -p alife_game_app --test app_shell ca44a_procedural_world_travel_smoke_streams_seeded_chunks_without_rendering -- --nocapture`
- `cargo run -p alife_game_app --bin alife_game_app -- procedural-world-travel-smoke crates/alife_world/tests/fixtures/gpu_alpha`
- `cargo check --workspace --all-targets`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1`
- `cargo tree -p alife_core`
- `cargo check --workspace --all-features --all-targets`
- `cargo test --workspace --all-features --all-targets`
- `cargo run -p alife_game_app --features "bevy-app gpu-runtime" --bin alife_game_app -- graphical-playground --scenario gpu-alpha --gpu-mode static-plastic-cpu-shadow-guarded --view-mode player --smoke-seconds 3`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 3 -GpuMode static-plastic-cpu-shadow-guarded`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded`
- `$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded; Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue`

## Validation Results

Focused procedural world tests passed. The new smoke reported `138` unique
materialized chunks over a six-step route, with `25` as the maximum active chunk
window and `3072` generated content candidates observed.

Full validation passed, including all-features validation. The standard
30-second graphical smoke passed after clearing a stale timed-out smoke process;
the direct 3-second app smoke and forced CPU fallback smoke also passed.

## Known Limitations

- The generated terrain/content contract is stronger than the current visual
  presentation. A later art pass should replace the remaining placeholder feel
  with a production asset pipeline and better 2.5D composition.
- Fog-of-war remains presentation-side evidence; this report verifies chunk
  activation, not a final exploration memory system.
- World chunks are materialized around creature anchors, not globally cached
  without presence.

## Main Status

Branch validation passed; pending commit, merge, post-merge validation, and
push.

Next plan remains CA44 unless the user explicitly authorizes a different
follow-up. CA45 was not started.
