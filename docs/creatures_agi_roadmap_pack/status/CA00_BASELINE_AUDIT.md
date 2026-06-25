# CA00 Baseline Audit

Status: CA00 complete. This is a docs/status wiring audit only; no runtime code
or gameplay behavior changed.

## Repository Baseline

- Branch at start: `main`
- Main status at start: clean and equal to `origin/main`
- Main commit at start: `57f5b08`
- Roadmap pack location: `docs/creatures_agi_roadmap_pack/`
- Manifest first plan: `CA00`
- Next executable plan after CA00: `CA01`

## Pack Verification

The roadmap pack is present with:

- `README.md`
- `plan_manifest.json`
- `GLOBAL_INVARIANTS.md`
- `VALIDATION_PROTOCOL.md`
- `EXECUTION_ORDER.md`
- `workflow/DEEP_PLANNING_PROTOCOL.md`
- `workflow/SPEC_PONY_LOOP_USAGE.md`
- implementation plans `CA00` through `CA59`
- review gates `CAR04` through `CAR59`
- `status/ROADMAP_PROGRESS.md`

The pack starts a new explicit roadmap and does not replace historical P/G/R/S
work. It does not create `S12`, `G25`, `P37`, or a release tag.

## Product Baseline

Current player-facing status is GPU-first alpha:

- Recommended launcher:
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -GpuMode static-plastic-cpu-shadow-guarded`
- Default graphical fixture:
  `crates/alife_world/tests/fixtures/gpu_alpha`
- Visible app title:
  `A-Life GPU Alpha Playground`
- Product claim:
  `CpuShadowGuardedStaticPlusLiveHShadow`

That claim means GPU static scores may feed proposal scoring only after CPU
shadow parity passes, normal action arbitration remains in place, and post-seal
H_shadow learning is applied through the core lifetime-delta contract. It is
not a full action-authoritative GPU runtime claim.

## Current Evidence

Existing productization reports record:

- RTX 3050/Vulkan local GPU runtime evidence.
- 5000-tick sustained-learning soak with zero CPU shadow parity failures.
- Repeated post-seal H_shadow applications through the core contract.
- Persistent graphical alpha window with creature, food, hazard marker,
  overlays, read-only inspector, controls text, and GPU/fallback status.
- CPU fallback and forced fallback remain available and must remain explicit
  degraded/safety modes rather than silent player-facing success.

## Known UX Gaps For CA01+

CA00 does not fix these, but records them as the handoff:

- The graphical alpha still reads as a dense technical dashboard rather than a
  clear creature-simulation game.
- Space/N and visible tick changes need stronger first-user feedback.
- Terminal/invalid state handling and restart flow still need player-facing
  hardening.
- UI panels can still feel crowded at common desktop capture sizes.
- Vulkan loader diagnostics from third-party overlay layers may still confuse
  users when they appear in terminal output; they are not GPU correctness
  evidence or A-Life runtime failures.
- Independent human external alpha evidence remains separate from local Codex
  Computer Use evidence.

## Invariants Confirmed By Audit

- `alife_core` must remain engine/GPU/UI independent.
- CPU shadow parity and CPU fallback remain internal correctness/safety paths.
- GPU is the target player-facing graphical alpha path.
- Bevy visuals mirror model state and are not authoritative.
- Stable IDs remain the portable/player-facing identity surface.
- No screenshots, logs, target artifacts, release tags, or hidden new roadmap
  plans were created by CA00.

## Next

Proceed to `CA01 - GPU-first visible playable loop`.
