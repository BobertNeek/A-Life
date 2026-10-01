# alife_game_app Instructions

Root [AGENTS.md](../../AGENTS.md) applies. These local rules are current
implementation guardrails; they do not amend the controlling v2.0 architecture.

This crate owns the playable-sim product app shell and launch policy.

Production art uses the approved skinned Hearthling and Blender landscape props.
Keep exported GLBs and production manifest digests together. Animation and
inherited visual proportions remain read-only projections of organism state.

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
- Explicit Nano512 readout candidates enter through their genome-bound canonical
  asset and archive before GPU insertion. Reconstruct that exact foundation on
  restore; preserve builtin admission and deployed lifetime learning.
- Observe each sealed patch in its organism-owned memory sidecar and then its
  diagnostic topology sidecar even when post-seal GPU learning is rejected;
  neither sidecar may abort or influence candidate arbitration.
- Update one fixed-size passive-statistics record per resident tick and archive
  that typed record before GPU retirement; never scan unbounded life history.
- Use `docs/ROADMAP.md` for product sequencing and exit gates. Do not infer
  integration or release readiness from a historical plan label.
