# CAR40 Polish and Tutorial Review

Review: CAR40 - Polish and tutorial review

Verdict: PASS_WITH_NOTES

## Scope Reviewed

CAR40 reviewed Phase I polish/tutorial work from CA37 through CA40:

- CA37 - Terrain, props, and world art style pass
- CA38 - Creature animation state machine
- CA39 - Drive-coupled audio and VFX
- CA40 - Onboarding tutorial first session

The review focused on whether a first-time player can understand the current GPU
alpha surface well enough for a 5-10 minute alpha session, without changing the
simulation authority model.

## Files Inspected

- `docs/creatures_agi_roadmap_pack/review_gates/CAR40_polish-and-tutorial-review.md`
- `docs/creatures_agi_roadmap_pack/status/CA37_TERRAIN_PROPS_WORLD_ART_STYLE.md`
- `docs/creatures_agi_roadmap_pack/status/CA38_CREATURE_ANIMATION_STATE_MACHINE.md`
- `docs/creatures_agi_roadmap_pack/status/CA39_DRIVE_COUPLED_AUDIO_VFX.md`
- `docs/creatures_agi_roadmap_pack/status/CA40_ONBOARDING_TUTORIAL_FIRST_SESSION.md`
- `docs/creatures_agi_roadmap_pack/status/ROADMAP_PROGRESS.md`
- `docs/creatures_agi_roadmap_pack/GLOBAL_INVARIANTS.md`
- `docs/creatures_agi_roadmap_pack/VALIDATION_PROTOCOL.md`
- `crates/alife_game_app/src/world_art_style.rs`
- `crates/alife_game_app/src/creature_animation_style.rs`
- `crates/alife_game_app/src/drive_coupled_audio_vfx.rs`
- `crates/alife_game_app/src/onboarding_tutorial.rs`
- `crates/alife_game_app/src/bevy_shell.rs`
- `crates/alife_game_app/src/bin/alife_game_app.rs`
- `crates/alife_game_app/tests/app_shell.rs`

## Commands Run

Focused review evidence:

```powershell
cargo run -p alife_game_app --bin alife_game_app -- world-art-style-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- creature-animation-state-smoke
cargo run -p alife_game_app --bin alife_game_app -- drive-coupled-audio-vfx-smoke crates/alife_world/tests/fixtures/gpu_alpha
cargo run -p alife_game_app --bin alife_game_app -- onboarding-tutorial-smoke crates/alife_world/tests/fixtures/gpu_alpha
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded
$env:ALIFE_GPU_RUNTIME_AVAILABLE="0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 10 -GpuMode static-plastic-cpu-shadow-guarded
Remove-Item Env:\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue
```

Standard validation:

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

- CA37 world-art smoke passed with `large_world_exploration=true`,
  `distributed_objects=true`, `tiles=7081`, and `display_only=true`.
- CA38 creature-animation smoke passed with 8 display-only pose states and no
  action/cognition authority.
- CA39 drive-coupled audio/VFX smoke passed with bounded display-only cues,
  no active bulk readback, no action authority, and no weight authority.
- CA40 onboarding tutorial smoke passed with 5 checklist items, visible
  food/hazard/GPU/fallback guidance, stable IDs, no action authority, and no
  full action-authoritative claim.
- Graphical GPU smoke passed with `gpu_selected=GpuPlastic`, `gpu_scores=true`,
  `cpu_shadow_parity=true`, `fallback=None`, 12 objects, 3 creatures, 3 food,
  and 3 hazards.
- Forced fallback graphical smoke passed with `gpu_selected=CpuReference`,
  `fallback=Some("HardwareUnavailable")`, `gpu_claim=None`, stable IDs, and no
  GPU performance claim.
- Standard validation passed on the merged CA40 main before this review report.
- CAR40 branch validation is required before merge and must pass before the
  report is merged.

## Findings by Severity

### BLOCKER

None.

### HIGH

None.

### MEDIUM

None.

### LOW

- The Phase I experience is now understandable enough for the next packaging
  tranche, but there is still no independent human 5-10 minute first-session
  playtest evidence in this review gate. Existing evidence is deterministic
  smoke/test evidence plus local graphical smoke.
- Some older status documents still contain historical "pending merge" wording
  in their own status sections even though `ROADMAP_PROGRESS.md` and main
  history show the plans are merged. This is not player-facing and does not
  affect the executable roadmap chain.

## Invariant Status

PASS.

- No S12, G25, P37, or hidden continuation plan was created.
- No release tag was created.
- No screenshots, logs, target artifacts, model weights, model caches, or
  generated media are tracked.
- `alife_core` remains engine-independent and dependency-clean.
- Bevy visuals remain presentation-only and are not authoritative.
- Stable IDs remain the player-facing identity surface.
- CPU fallback remains available.
- CPU shadow parity remains the correctness gate.
- No full action-authoritative GPU runtime claim was added.
- No active bulk neural readback was added.
- UI, VFX, tutorial, terrain, animation, teacher, semantic, SLM, GPU, memory,
  and topology surfaces do not emit actions or bypass P09 arbitration.
- W_genetic_fixed, lifetime-consolidated, and H_operational invariants remain
  preserved.

## User-Facing Status

PASS_WITH_NOTES.

The current GPU alpha now has a larger stylized world surface, distributed
stable-ID objects, readable creature pose changes, drive-coupled display/audio
cue descriptors, and a first-session tutorial panel. A first-time tester should
be able to identify the selected creature, understand food/hazard/rock markers,
see GPU/fallback status, and use pause/run/step/follow/reset instructions.

This is still an alpha teaching surface, not a release-ready onboarding flow.
The tutorial is display-only and non-persistent, and "enjoyment" for a 5-10
minute session still needs external human playtest evidence.

## Evidence Gaps

- No independent external human 5-10 minute playtest has been captured after
  CA40.
- No committed screenshot/video evidence is included by design; media remains
  untracked.
- The tutorial does not persist progress and is not a full quest/task system.

## Fix Prompt if Needed

No fix prompt is required before CA41.

If the user wants to close the LOW playtest evidence gap before packaging, use:

```text
Run one external 5-10 minute first-session GPU alpha playtest using the CA40
tutorial panel. Record whether the tester can identify creature/food/hazard,
pause/run/step/follow/reset, read GPU/fallback state, and describe what happened
without reading source docs. Do not implement new features, do not create S12,
G25, or P37, do not tag a release, and do not commit media artifacts.
```

## Next Plan Recommendation

Proceed to CA41 - Windows zip packaging and run script after user/ChatGPT
consultation accepts this CAR40 review.

Do not start CA41 automatically from this review gate.
