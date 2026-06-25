# CAR17 - Core gameplay review

Verdict: PASS_WITH_NOTES

## Scope reviewed

CAR17 reviewed the Phase D tranche:

- CA13 - Double-buffered graphical/game tick scheduler
- CA14 - Competitive motor ring arbitration presentation/runtime bridge
- CA15 - Allocation-bounded endocrine/homeostasis runtime
- CA16 - Non-scripted movement, approach, eat, and affordance loop
- CA17 - Hazard avoidance, pain, sleep, and failure recovery

The review question was whether the current survival loop is visibly non-scripted
enough for alpha before population expansion. The answer is yes for a
single-creature alpha slice, with limitations noted below.

## Files inspected

- `docs/creatures_agi_roadmap_pack/plan_manifest.json`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`
- `docs/creatures_agi_roadmap_pack/status/CA13_DOUBLE_BUFFERED_SCHEDULER.md`
- `docs/creatures_agi_roadmap_pack/status/CA14_MOTOR_RING_ARBITRATION.md`
- `docs/creatures_agi_roadmap_pack/status/CA15_HOMEOSTASIS_RUNTIME.md`
- `docs/creatures_agi_roadmap_pack/status/CA16_AFFORDANCE_LOOP.md`
- `docs/creatures_agi_roadmap_pack/status/CA17_HAZARD_RECOVERY.md`
- `docs/creatures_agi_roadmap_pack/plans/CA13_double-buffered-graphical-game-tick-scheduler.md`
- `docs/creatures_agi_roadmap_pack/plans/CA14_competitive-motor-ring-arbitration-presentation-runtime-bridge.md`
- `docs/creatures_agi_roadmap_pack/plans/CA15_allocation-bounded-endocrine-homeostasis-runtime.md`
- `docs/creatures_agi_roadmap_pack/plans/CA16_non-scripted-movement-approach-eat-and-affordance-loop.md`
- `docs/creatures_agi_roadmap_pack/plans/CA17_hazard-avoidance-pain-sleep-and-failure-recovery.md`
- `crates/alife_game_app/src/double_buffered_scheduler.rs`
- `crates/alife_game_app/src/motor_ring.rs`
- `crates/alife_game_app/src/homeostasis_runtime.rs`
- `crates/alife_game_app/src/affordance_loop.rs`
- `crates/alife_game_app/src/hazard_recovery_loop.rs`
- `crates/alife_game_app/src/interactive_runtime.rs`
- `crates/alife_game_app/src/schema.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/tests/app_shell.rs`
- `crates/alife_core/Cargo.toml`
- `docs/master_spec.md`
- `docs/architecture_decisions.md`

## Commands run

Focused review commands:

```powershell
cargo test -p alife_game_app --test app_shell ca13 -- --nocapture
cargo test -p alife_game_app --test app_shell ca14 -- --nocapture
cargo test -p alife_game_app --test app_shell ca15 -- --nocapture
cargo test -p alife_game_app --test app_shell ca16 -- --nocapture
cargo test -p alife_game_app --test app_shell ca17 -- --nocapture
cargo run -p alife_game_app --bin alife_game_app -- double-buffered-scheduler-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- motor-ring-arbitration-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- homeostasis-runtime-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- affordance-loop-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- hazard-recovery-smoke crates/alife_world/tests/fixtures/gpu_alpha
```

Standard validation commands are recorded by the CAR17 branch validation receipt
and final main validation after merge.

## Findings by severity

BLOCKER: none.

HIGH: none.

MEDIUM: none.

LOW:

- The current evidence is still a deterministic single-creature alpha slice.
  It proves visible scheduling, action competition, homeostasis, food
  affordance, hazard avoidance, pain, sleep, and recovery, but it does not yet
  prove multi-creature population dynamics or long-horizon emergent behavior.
  That is appropriate to leave for CA18+.

INFO:

- CA17 implementation review previously found one medium issue in the hazard cue
  evidence path. It was fixed before merge by keying the visible hazard cue to
  the `FLEE` action ID instead of a hard-coded stable target ID.
- The product claim remains bounded. The tranche does not claim full
  action-authoritative GPU runtime.

## Invariant status

- `alife_core` remains engine-independent.
- Bevy/wgpu/GPU dependencies were not added to `alife_core`.
- CPU fallback remains available.
- CPU shadow parity remains the GPU proposal gate.
- P09 action arbitration remains the action path; UI, semantic, teacher, GPU,
  memory, and topology systems do not emit actions directly.
- Stable IDs remain the player-facing and portable boundary; no Bevy Entity IDs
  are used in reviewed smoke summaries.
- No active bulk neural readback was introduced.
- `W_genetic_fixed`, lifetime-consolidated state, and H_operational invariants
  are unchanged by this tranche.
- No S12, G25, P37, release tag, screenshot, log, target artifact, capture, or
  generated media artifact was created for this review.

## User-facing status

The current alpha loop is visibly less scripted than the previous debug surface:

- CA13 provides fixed-tick pause/step/run semantics and bounded catch-up.
- CA14 shows action competition through a motor ring while preserving normal
  arbitration.
- CA15 exposes hunger, energy, fatigue, pain, stress, salience, and learning
  modulation as bounded read-only state.
- CA16 demonstrates approach to food, food consumption, hunger reduction, and
  energy increase through sealed world ticks.
- CA17 demonstrates hazard salience, flee selection, increased distance from
  hazard, pain/fear on contact, forced recovery sleep, and recoverable failure
  without terminal stagnation.

This is alpha-credible for a single-creature core gameplay slice. Population
scale, multi-creature social/ecological behavior, and longer emergent play are
not yet proven and should remain future-plan scope.

## Evidence gaps

- No multi-creature population loop is reviewed in this gate.
- No long-horizon ecology or reproduction behavior is reviewed in this gate.
- Graphical playability smoke validates launch/fallback and overlays, but this
  report does not add new human external tester evidence.
- GPU remains CPU-shadow guarded and not full action-authoritative.

## Fix prompt if needed

No fix prompt required. No blocker, high, or medium finding remains.

## Next plan recommendation

Proceed to CA18 - Multi-creature graphical population v1 only after this CAR17
hard-stop report is accepted by the user/ChatGPT consultation. Do not start CA18
automatically from this review branch.
