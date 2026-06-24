# Graphical GPU Playability Spec

Mode: Full Spec Loop
Review class target: R2 preferred, R1/R0 fallback if no separate reviewer is available.

## Contract

Expose the existing `CpuShadowGuardedStaticPlusLiveHShadow` GPU runtime evidence inside the persistent Bevy graphical playground without making GPU mandatory or changing core semantics.

## Acceptance

- `graphical-playground` accepts `--gpu-mode cpu-reference|static-plastic-cpu-shadow-guarded|auto-with-cpu-fallback`.
- The graphical app can request GPU mode while retaining CPU fallback.
- Runtime overlay and inspector show requested mode, selected backend, fallback, CPU shadow parity, compact readback, H_shadow application, and the boundary that this is not full action-authoritative runtime.
- Creature marker visuals react to GPU/fallback/learning telemetry as presentation-only state.
- The default/headless CPU path remains unchanged.
- `alife_core` remains free of Bevy/wgpu/GPU dependencies.

## Non-Goals

- No full action-authoritative GPU claim.
- No release tag.
- No new S12/G25/P37 plan chain.
- No active bulk neural readback.
- No runtime code in `alife_core`.
