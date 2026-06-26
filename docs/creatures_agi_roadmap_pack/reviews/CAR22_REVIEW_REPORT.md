# CAR22 - Ecosystem review

Verdict: PASS_WITH_NOTES

## Scope reviewed

CAR22 reviewed the Phase E tranche:

- CA18 - Multi-creature graphical population v1
- CA19 - Resource ecology and terrain zones in graphical play
- CA20 - Lifecycle, reproduction, death, and lineage UI
- CA21 - Behavior tuning metrics loop
- CA22 - Long-run ecological soak and balancing

The review question was whether the current game has a living ecosystem
baseline before school and semantic expansion. The answer is yes for a bounded
alpha ecosystem baseline, with explicit limitations: this is not yet a broad
open-ended emergent ecology claim, and the GPU runtime remains CPU-shadow
guarded.

## Files inspected

- `docs/creatures_agi_roadmap_pack/plan_manifest.json`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`
- `docs/creatures_agi_roadmap_pack/status/CA18_MULTI_CREATURE_GRAPHICAL_POPULATION.md`
- `docs/creatures_agi_roadmap_pack/status/CA19_RESOURCE_ECOLOGY_TERRAIN_ZONES.md`
- `docs/creatures_agi_roadmap_pack/status/CA20_LIFECYCLE_LINEAGE_UI.md`
- `docs/creatures_agi_roadmap_pack/status/CA21_BEHAVIOR_TUNING_METRICS.md`
- `docs/creatures_agi_roadmap_pack/status/CA22_LONG_RUN_ECOLOGICAL_SOAK.md`
- `docs/creatures_agi_roadmap_pack/plans/CA18_multi-creature-graphical-population-v1.md`
- `docs/creatures_agi_roadmap_pack/plans/CA19_resource-ecology-and-terrain-zones-in-graphical-play.md`
- `docs/creatures_agi_roadmap_pack/plans/CA20_lifecycle-reproduction-death-and-lineage-ui.md`
- `docs/creatures_agi_roadmap_pack/plans/CA21_behavior-tuning-metrics-loop.md`
- `docs/creatures_agi_roadmap_pack/plans/CA22_long-run-ecological-soak-and-balancing.md`
- `crates/alife_game_app/src/graphical_population.rs`
- `crates/alife_game_app/src/graphical_ecology.rs`
- `crates/alife_game_app/src/graphical_lifecycle.rs`
- `crates/alife_game_app/src/behavior_tuning.rs`
- `crates/alife_game_app/src/ecological_soak.rs`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_core/Cargo.toml`
- `docs/master_spec.md`
- `docs/architecture_decisions.md`

## Commands run

Focused review commands:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- graphical-population-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- graphical-ecology-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- graphical-lifecycle-smoke
cargo run -p alife_game_app --bin alife_game_app -- behavior-tuning-metrics-smoke
cargo run -p alife_game_app --bin alife_game_app -- ecological-soak-smoke
cargo test -p alife_game_app --test app_shell ca22_manual_10k_ecological_soak -- --ignored --nocapture
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Standard validation commands are recorded by the CA22 branch validation receipt,
the post-merge main validation, and this CAR22 branch validation.

## Findings by severity

BLOCKER: none.

HIGH: none.

MEDIUM: none.

LOW:

- CA21/CA22 keep overfeeding and hazard-suicide as known limitations for the
  current bounded fixture. That is acceptable for this gate because the tranche
  records them honestly and does not convert them into broad ecosystem claims.

INFO:

- CA22 R2 review found one HIGH issue before merge: the manual 10k evidence was
  initially reported from configuration rather than a measured tick loop. That
  was fixed before CA22 merge. The ignored manual command now runs the measured
  headless loop and records completed ticks, first failure tick, and metric
  sample counts.
- The Phase E tranche moves beyond the CAR17 deterministic single-creature
  limitation by showing three visible creatures, resource/terrain zones,
  lifecycle/lineage status, degeneracy metrics, and bounded long-run ecology
  evidence.

## Invariant status

- `alife_core` remains engine-independent.
- Bevy/wgpu/GPU dependencies were not added to `alife_core`.
- CPU fallback remains available.
- CPU shadow parity remains the GPU proposal gate.
- The product runtime claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- Full action-authoritative GPU runtime is not claimed.
- P09 action arbitration remains the action path; UI, semantic, teacher, GPU,
  memory, topology, save/load, and ecology presentation do not emit actions
  directly.
- Stable IDs remain the player-facing and portable boundary; reviewed summaries
  and overlays avoid Bevy Entity IDs.
- No active bulk neural readback was introduced.
- `W_genetic_fixed`, lifetime-consolidated state, and H_operational invariants
  are unchanged by this tranche.
- No S12, G25, P37, release tag, screenshot, log, target artifact, capture, or
  generated media artifact was created for this review.

## User-facing status

The current alpha now has a bounded living-ecosystem baseline:

- CA18 shows three stable-ID creatures, selection cycling, per-creature
  presentation, and social proximity cues.
- CA19 shows stable terrain zones, resource lifecycle counters, regrowth/spawn
  indicators, and hazard-pressure presentation.
- CA20 shows living population, birth/death events, lineage rows, population cap
  status, and genetic/lifetime separation.
- CA21 detects stagnation, catatonia, overfeeding, hazard suicide, and
  population collapse without retuning the fixture to hide weak spots.
- CA22 records fast CI-safe ecological soak evidence and a manual measured 10k
  headless soak command, while keeping full emergent ecology as unclaimed.

This is sufficient to proceed to school/semantic UI planning after consultation,
provided the known ecology limitations remain visible.

## Evidence gaps

- The current ecology remains bounded and deterministic; it is not an
  open-ended ecosystem proof.
- Overfeeding and hazard-suicide remain known limitations in the current
  fixture and should be revisited by later balance/playtest work.
- Graphical smoke validates launch/fallback and overlays, but this report does
  not add new independent human tester evidence.
- GPU remains CPU-shadow guarded and not full action-authoritative.

## Fix prompt if needed

No fix prompt required. No blocker, high, or medium finding remains.

## Next plan recommendation

Proceed to CA23 - Graphical school mode and lesson panel only after this CAR22
hard-stop report is accepted by the user/ChatGPT consultation. Do not start CA23
automatically from this review branch.
