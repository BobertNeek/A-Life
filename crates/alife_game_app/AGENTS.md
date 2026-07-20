# alife_game_app Instructions

This crate owns the playable-sim product app shell and launch policy.

Rules:

- Keep the default path headless and CI-safe.
- Keep Bevy integration feature-gated.
- Do not put game app state, Bevy, renderer, windowing, or adapter types into
  `alife_core`.
- Use P34 runtime config and asset manifest validation instead of bypassing
  persistence contracts.
- In the GPU live loop, seal the measured world outcome before applying or
  explicitly discarding the matching pending eligibility transaction.
- Observe each sealed patch in its organism-owned memory sidecar and then its
  diagnostic topology sidecar even when post-seal GPU learning is rejected;
  neither sidecar may abort or influence candidate arbitration.
- Do not implement visible world content, creature rendering, gameplay loops, or
  product release packaging before their assigned G-plans.
