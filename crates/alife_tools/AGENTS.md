# alife_tools Instructions

This crate controls developer tooling hooks, Graphify helpers, docs validation,
and spec consistency checks.

Rules:

- Tooling must not become a runtime dependency for simulation crates.
- Own GPU-only populated benchmark artifacts and their honest
  Completed/Missed/Unavailable statuses; never substitute host fixtures or
  inferred results for promotion evidence.
- The canonical v1 p95 matrix measures the corrected full causal tick,
  including eligibility capture and post-outcome plasticity. Do not replace it
  with timing from the earlier under-executing diagnostic path.
- Graphify is optional; cargo build/check/test must work without Graphify installed.
- Prefer checks that catch architecture drift: Unity/HLSL, fixed 2048-only assumptions,
  dense neural buffers, and hidden teacher injection.
- Do not put game or neural runtime behavior in this crate.
