# Full GPU neural runtime gap report

Status: static GPU action scoring is wired into the product smoke path, and
validated post-seal H_shadow deltas can now be applied to the live
`CreatureMind` in static-plastic shadow mode. The complete action-authoritative
static+routing+plastic GPU runtime remains a bounded gap.

## Completed

- Static forward projection can dispatch on the local GPU.
- Routing/supertile masks are consumed through the existing P27 mask contract.
- Compact active-tick readback is limited to the 64-byte action summary.
- CPU shadow parity gates use of GPU action scores.
- GPU-scored proposals still pass through the normal arbitration and sealed patch path.
- CPU fallback remains available.
- GPU plasticity output can be converted into a core-owned
  `PostSealLifetimeDeltaBatch`.
- `CreatureMind` can apply validated H_shadow-only deltas after a sealed
  `ExperiencePatch`.

## Gap

The previous post-seal lifetime-state contract gap is closed for H_shadow-only
delta application. The remaining gap is broader: no mode currently combines
GPU static scoring, routing, and live plasticity into a single
action-authoritative static+routing+plastic runtime.

The static action-authoritative smoke uses GPU static scores for compact action
proposals and gates them with CPU shadow parity, but it does not dispatch live
plasticity. The static-plastic shadow smoke dispatches GPU plasticity and
applies H_shadow deltas after sealing, but it does not use GPU output for
action proposals.

## Current safe behavior

- GPU plasticity can run after sealed patches in static-plastic shadow mode.
- The result verifies and applies H_shadow-only updates through an
  `alife_core` contract.
- `W_genetic_fixed`, lifetime-consolidated weights, and H_operational remain unchanged by the GPU plasticity pass.
- The app report explicitly states whether `live_h_shadow_applied` is true.

## Remaining future fix

Completing full action-authoritative static+routing+plastic runtime still
requires a mode that:

- uses GPU static/routing output for proposal scoring
- dispatches GPU plasticity after the sealed patch for the same live tick
- applies H_shadow through the post-seal core contract
- keeps CPU shadow parity authoritative
- preserves normal action arbitration and ExperiencePatch sealing
- avoids active bulk neural readback

Until that combined mode exists and validates, A-Life must not claim full
static+routing+plasticity action-authoritative GPU runtime.

