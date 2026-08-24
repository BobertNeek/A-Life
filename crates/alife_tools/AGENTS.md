# alife_tools Instructions

Architecture authority:

- `../../docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative source.
- This file records current implementation guardrails only. Existing Rust
  structures, processor placement, GPU layouts, constants, brain-size
  assumptions, adapters, tests, and fixtures do not amend v2.0.
- Earlier architecture documents are historical. Report conflicts as
  `AOA-*` gaps and do not start an unrequested repair pass.

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
