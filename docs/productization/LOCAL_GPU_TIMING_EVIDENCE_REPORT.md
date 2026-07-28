# Historical Local GPU Timing Evidence

Status: superseded diagnostic receipt. These measurements were captured before
ADR-024 established the GPU-authoritative closed loop. The P26 Oja/H-shadow
executor and its parity gate have been removed from the product.

## Preserved historical measurement

The original run used an NVIDIA GeForce RTX 3050 through Vulkan with driver
581.80. Host-observed submit/poll and readback timing recorded:

| Historical workload | Dimensions | Warmup | Measured | GPU total mean ms |
|---|---|---:|---:|---:|
| P25 static-forward diagnostic | 512 neurons, 2 tiles, 258 synapses | 3 | 10 | 2.4794 |
| Retired P26 Oja diagnostic | 512 neurons, 1 tile, 1 synapse | 3 | 10 | 1.5362 |

These numbers are retained only as provenance. They do not satisfy current
causal, plasticity, sleep, soak, or promotion gates.

## Current evidence path

Production waking learning is validated through a matching sealed outcome and
the seven-binding `closed_loop_plasticity.wgsl` pipeline:

```powershell
$env:CARGO_INCREMENTAL = "0"
$env:CARGO_BUILD_JOBS = "1"
cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -j 1 -- --nocapture
```

Current performance evidence must bind the exact adapter, commit/tree, class,
phenotype, payload, and runtime ABI. Missing evidence remains `Unknown`; it is
never replaced by a historical CPU or parity measurement.
