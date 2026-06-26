# CA22 - Long-run Ecological Soak and Balancing

Status: complete on `codex/CA22-long-run-ecological-soak-balancing`

CA22 adds a bounded ecological soak evidence layer over the existing G19 balance
signals, CA21 degeneracy detectors, and CA19 graphical ecology fixture. It does
not change action arbitration, GPU authority, save/load contracts, or
`alife_core`.

Evidence added:

- `ecological-soak-smoke` records fast CI-safe headless soak evidence with
  survival, energy stability, food success, hazard avoidance, sleep,
  reproduction, social diversity, sealed patch, population, and resource
  metrics.
- The report points at the manual 10k command and records measured
  `headless_ticks_completed`, `first_failure_tick`, and metric sample counts
  from a deterministic headless tick loop rather than assigning completion from
  configuration alone:
  `cargo test -p alife_game_app --test app_shell ca22_manual_10k_ecological_soak -- --ignored --nocapture`
- Graphical bounded soak evidence remains tied to the Windows-safe launcher:
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded`
- CA21 degeneracy classes are carried forward as explicit remaining issues
  rather than hidden or relabeled as broad emergent ecology.

Boundaries:

- Tuning remains config-first.
- `alife_core` was not changed.
- CPU fallback and CPU shadow parity remain preserved.
- Product GPU claim remains `CpuShadowGuardedStaticPlusLiveHShadow`.
- Full emergent ecology and full action-authoritative GPU runtime are not
  claimed.
- No release tag, S12, G25, or P37 was created.

Next: CAR22 ecosystem review.
