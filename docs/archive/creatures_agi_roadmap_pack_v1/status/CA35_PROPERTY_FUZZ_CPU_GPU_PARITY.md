# CA35 Property-Fuzz CPU/GPU Parity Gating

## Status

Complete on branch `codex/CA35-property-fuzz-cpu-gpu-parity-gating`.

CA35 adds a CI-safe deterministic property-style fuzz layer around the GPU
backend fixed-point parity contracts. It does not change runtime semantics and
does not graduate any GPU mode to full action-authoritative behavior.

## What Changed

- Added `property_fuzz_parity_gating.rs` under `alife_gpu_backend` tests.
- Added bounded pseudo-random Nano512 fixture generation with fixed seed lists.
- Covered static forward fixed-point GPU oracle against the CPU SpMV/finalize
  reference.
- Covered routing mask derivation and masked static diagnostic counters.
- Covered plasticity/Oja diagnostics, including alpha-zero unchanged slots and
  H_shadow-only parity for alpha-positive slots.
- Added stable repro strings with seed, tile count, dense cadence, and mask mode
  so a failed case can be rerun and minimized manually without committing logs.

## Evidence Boundary

The CA35 fuzz tests are CI-safe CPU-oracle/property checks for GPU packing and
diagnostic parity. Hardware GPU parity and timing remain separate manual
evidence paths:

```powershell
cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture
cargo test -p alife_gpu_backend --features gpu-tests --test plasticity_oja_parity -- --ignored --nocapture
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime
cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime --measure-gpu
```

Passing CA35 does not imply release readiness, GPU performance on other
machines, or full action-authoritative GPU runtime.

## Invariants

- `alife_core` remains unchanged and engine-independent.
- The test uses only supported COO and Dense16x16 tile formats.
- The fuzz cases remain bounded and deterministic for CI.
- No active bulk neural readback is added.
- `W_genetic_fixed`, lifetime-consolidated, and H_operational layers are checked
  to remain unchanged in plasticity diagnostics.
- No screenshots, logs, target artifacts, model files, `S12`, `G25`, `P37`, or
  release tag are created by this plan.

## Focused Evidence

```powershell
cargo test -p alife_gpu_backend --test property_fuzz_parity_gating -- --nocapture
```

The focused test covers:

- `ca35_static_forward_property_fuzz_matches_cpu_reference`
- `ca35_routing_property_fuzz_preserves_masked_static_outputs`
- `ca35_plasticity_property_fuzz_matches_cpu_oja_reference`
- `ca35_fuzz_failure_repro_strings_are_stable_and_shrinkable`

## Known Limitations

- The fuzz corpus is intentionally small and deterministic for normal CI.
- It is property-style coverage without an external shrinking engine; failures
  print stable repro strings for manual minimization.
- Real hardware GPU parity remains covered by ignored/manual GPU tests.
