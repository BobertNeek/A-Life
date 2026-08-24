# alife_game_app Instructions

Architecture authority:

- `../../docs/architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`
  is the single normative source.
- This file records current implementation guardrails only. Existing Rust
  structures, processor placement, GPU layouts, constants, brain-size
  assumptions, adapters, tests, and fixtures do not amend v2.0.
- Earlier architecture documents are historical. Report conflicts as
  `AOA-*` gaps and do not start an unrequested repair pass.

This crate owns the playable-sim product app shell and launch policy.

Rules:

- Keep the default path headless and CI-safe.
- Keep Bevy integration feature-gated.
- Do not put game app state, Bevy, renderer, windowing, or adapter types into
  `alife_core`.
- Use P34 runtime config and asset manifest validation instead of bypassing
  persistence contracts.
- Drive the shared `alife_runtime` GPU session, and own explicit policy
  selection, A/B/C/D evidence ingestion, exact gate receipts, and promotion
  derivation; never promote from configuration, an incomplete matrix, or a
  different adapter/tree.
- In the GPU live loop, seal the measured world outcome before applying or
  explicitly discarding the matching pending eligibility transaction.
- Observe each sealed patch in its organism-owned memory sidecar and then its
  diagnostic topology sidecar even when post-seal GPU learning is rejected;
  neither sidecar may abort or influence candidate arbitration.
- Update one fixed-size passive-statistics record per resident tick and archive
  that typed record before GPU retirement; never scan unbounded life history.
- Use `docs/ROADMAP.md` for product sequencing and exit gates. Do not infer
  integration or release readiness from a historical plan label.
