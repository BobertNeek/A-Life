# Full GPU neural runtime gap report

Status: static GPU action scoring is wired into the product smoke path, but the
complete GPU plastic live runtime remains a bounded gap.

## Completed

- Static forward projection can dispatch on the local GPU.
- Routing/supertile masks are consumed through the existing P27 mask contract.
- Compact active-tick readback is limited to the 64-byte action summary.
- CPU shadow parity gates use of GPU action scores.
- GPU-scored proposals still pass through the normal arbitration and sealed patch path.
- CPU fallback remains available.

## Gap

Live H_shadow/Oja plasticity cannot be applied back into `CreatureMind` without
a new `alife_core` public contract for post-seal lifetime-state updates.
`CreatureMind` owns its neural state internally, and the current public surface
does not expose a safe way for an external GPU backend to write H_shadow after
patch sealing while preserving validation and genotype/lifetime separation.

## Current safe behavior

- GPU plasticity can run as post-seal diagnostic/shadow evidence.
- The diagnostic result verifies H_shadow-only updates.
- `W_genetic_fixed`, lifetime-consolidated weights, and H_operational remain unchanged by the GPU plasticity pass.
- The app report explicitly states `live_core_update_applied=false`.

## Required future fix

Add a narrow core-owned post-seal lifetime update hook only if a future user task
explicitly authorizes core API work. The hook must:

- accept validated H_shadow/lifetime deltas, not raw GPU buffers
- reject NaN/out-of-range values
- run only after sealed `ExperiencePatch`
- preserve `W_genetic_fixed`
- keep CPU oracle validation authoritative
- avoid active bulk neural readback

Until that hook exists, A-Life must not claim full static+routing+plasticity
action-authoritative GPU runtime.

