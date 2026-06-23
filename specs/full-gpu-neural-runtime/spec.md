# Full GPU Neural Runtime Spec

## Mode

- Spec Loop Ponytail mode: 2 - Full Spec Loop.
- Review class target: R1 if a fresh reviewer is available; otherwise R0 same-agent checklist, clearly labeled.
- Loop recipe: bounded implementation/verification loop only.

## Objective

Add an optional product-facing GPU neural runtime smoke path that dispatches real local GPU compute during live A-Life ticks while preserving the CPU oracle, action arbitration, sealed `ExperiencePatch` order, CPU fallback, and the no-bulk-readback active gameplay boundary.

## Acceptance Criteria

- Default/headless app path remains CPU reference.
- GPU runtime must be explicitly requested by command/feature.
- Forced unavailable GPU falls back to CPU and still seals patches.
- Real GPU path selects a local adapter when `alife_game_app` is built with `gpu-runtime`.
- Static forward dispatch runs on GPU and reads back only a compact action summary for active tick use.
- CPU shadow produces the same compact action summary within deterministic tolerance.
- GPU-derived proposals still run through normal CPU action arbitration and world execution.
- Every live tick used by this path seals an `ExperiencePatch` before post-seal learning diagnostics.
- Routing/supertile counters are reported and mask behavior is parity-checked for the fixture.
- GPU plasticity/Oja evidence updates only `H_shadow` in a diagnostic/shadow report and never mutates `W_genetic_fixed`.
- Product report states whether the mode is CPU fallback, shadow-only, CPU-shadow-guarded, or action-authoritative.
- Product report distinguishes product-runtime static scoring from diagnostic plasticity timing.

## Non-Goals

- Do not make GPU mandatory.
- Do not move GPU dependencies into `alife_core`.
- Do not add active gameplay bulk neural, per-synapse, per-lobe, per-weight, or full activation readback.
- Do not bypass P09 action arbitration.
- Do not bypass `ExperiencePatch` sealing.
- Do not mutate `W_genetic_fixed`.
- Do not persist GPU handles or adapter-local identifiers in portable saves.

