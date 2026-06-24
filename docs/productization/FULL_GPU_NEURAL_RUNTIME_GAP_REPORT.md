# Full GPU neural runtime gap report

Status: static GPU action scoring is wired into the product smoke path,
validated post-seal H_shadow deltas can now be applied to live `CreatureMind`,
and one combined CPU-shadow-guarded mode runs both in the same live path. The
complete action-authoritative static+routing+plastic GPU runtime remains a
bounded gap.

## Completed

- Static forward projection can dispatch on the local GPU.
- Routing/supertile masks are consumed through the existing P27 mask contract.
- Compact active-tick readback is limited to the 64-byte action summary.
- Post-seal H_shadow diagnostic readback is reported separately and is scoped
  after patch sealing, not during active proposal scoring.
- CPU shadow parity gates use of GPU action scores.
- GPU-scored proposals still pass through the normal arbitration and sealed patch path.
- CPU fallback remains available.
- GPU plasticity output can be converted into a core-owned
  `PostSealLifetimeDeltaBatch`.
- `CreatureMind` can apply validated H_shadow-only deltas after a sealed
  `ExperiencePatch`.
- `static-plastic-cpu-shadow-guarded` combines GPU static proposal scoring,
  normal arbitration, patch sealing, post-seal GPU plasticity, and live
  H_shadow application in one CPU-shadow-guarded smoke run.
- `gpu-longrun-soak` manually validates the same combined mode for longer runs;
  the local RTX 3050/Vulkan 5000-tick soak completed with 5000 CPU shadow
  parity checks, zero parity failures, one post-seal H_shadow application, and
  no full action-authoritative claim.
- `gpu-sustained-learning-soak` adds a manual episode-rotated evidence path for
  repeated valid post-seal H_shadow applications. The local RTX 3050/Vulkan
  5000-tick run completed 5000 sealed patches, 5000 CPU shadow parity checks,
  zero parity failures, and 157 successful H_shadow applications.

## Gap

The previous post-seal lifetime-state contract gap is closed for H_shadow-only
delta application. The previous "separate static scoring and plasticity smoke"
gap is also closed by the CPU-shadow-guarded combined mode. The remaining gap is
broader: no mode currently provides a full action-authoritative
static+routing+plastic runtime without CPU shadow gating.

The static action-authoritative smoke uses GPU static scores for compact action
proposals and gates them with CPU shadow parity, but it does not dispatch live
plasticity. The static-plastic shadow smoke dispatches GPU plasticity and
applies H_shadow deltas after sealing, but it does not use GPU output for
action proposals. The combined smoke does both, but remains CPU-shadow guarded.
The long-run and sustained-learning soaks increase stability and repeated
post-seal H_shadow evidence for this combined mode, but they do not change the
remaining gap: CPU shadow parity is still a runtime gate.

## Current safe behavior

- GPU plasticity can run after sealed patches in static-plastic shadow mode.
- GPU static scoring and post-seal H_shadow application can run together in
  `static-plastic-cpu-shadow-guarded` mode.
- The result verifies and applies H_shadow-only updates through an
  `alife_core` contract.
- `W_genetic_fixed`, lifetime-consolidated weights, and H_operational remain unchanged by the GPU plasticity pass.
- The app report explicitly states whether `live_h_shadow_applied` is true.
- If post-seal GPU plasticity diagnostics are unavailable after static scoring
  succeeds, the combined mode degrades to a static-only `CpuShadowGuarded`
  claim and does not apply H_shadow deltas.

## Remaining future fix

Completing full action-authoritative static+routing+plastic runtime still
requires a mode that:

- can run without CPU shadow parity as a runtime gate, or explicitly changes the
  claim to remain CPU-shadow guarded
- uses GPU static/routing output for proposal scoring
- dispatches GPU plasticity after the sealed patch for the same live tick
- applies H_shadow through the post-seal core contract
- preserves normal action arbitration and ExperiencePatch sealing
- avoids active bulk neural readback

Until an action-authoritative version exists and validates, A-Life must not
claim full static+routing+plasticity action-authoritative GPU runtime.

