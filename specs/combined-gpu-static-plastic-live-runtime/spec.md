# Combined GPU Static/Plastic Live Runtime Spec

## Contract

Add one optional product runtime mode that combines two already validated paths
in the same live tick sequence:

- CPU-shadow-guarded GPU static action scoring feeds compact proposal scores
  into normal action arbitration.
- After the resulting `ExperiencePatch` is sealed, GPU plasticity output is
  converted into the core-owned post-seal H_shadow delta batch and applied to
  live `CreatureMind`.

The mode must remain optional, CPU-shadow guarded, and fallback-capable. It must
not claim full action-authoritative static+routing+plastic runtime while CPU
shadow parity is still a runtime gate.

## Acceptance

- New CLI mode: `static-plastic-cpu-shadow-guarded`.
- GPU output is used for proposals only after CPU shadow parity passes.
- Normal action proposal construction, arbitration, execution, and patch sealing
  are preserved.
- H_shadow deltas apply only after the sealed patch through
  `CreatureMind::apply_post_seal_lifetime_deltas`.
- `W_genetic_fixed`, lifetime-consolidated weights, and H_operational remain
  unchanged.
- Forced `ALIFE_GPU_RUNTIME_AVAILABLE=0` falls back to CPU and still seals
  patches.
- Reports label the mode `CpuShadowGuardedStaticPlusLiveHShadow` and keep the
  unsupported full action-authoritative gap explicit.

## Non-Goals

- No mandatory GPU path.
- No `alife_core` dependency changes.
- No active bulk neural readback.
- No release tag or new roadmap plan.
