# CA20 - Lifecycle, Reproduction, Death, and Lineage UI

Status: complete on `codex/CA20-lifecycle-reproduction-death-lineage-ui`

CA20 exposes the existing lifecycle/lineage model as a graphical product-facing
status layer. The implementation reuses the G09 lifecycle smoke and save-state
contracts, then converts them into a CA20 graphical summary and Bevy overlay.

Evidence added:

- `graphical-lifecycle-smoke` reports living population, population cap,
  births, deaths, reproduction-blocked count, lineage rows, genetic/lifetime
  separation, and lineage save/load roundtrip.
- The graphical app now has a display-only lifecycle overlay that shows birth
  and death events, lineage rows, population-cap status, and the boundary that
  birth assets initialize only while lifetime state is not inherited.
- Feature-gated overlay tests assert the text stays player-facing, stable-ID
  based, and free of Bevy `Entity` IDs or full action-authoritative claims.

Boundaries:

- `alife_core` was not changed.
- Lifecycle visuals are presentation only and cannot emit actions.
- Genetic fixed weights remain separate from lifetime state; birth assets are
  initializers only.
- CPU fallback and CPU shadow parity remain preserved.
- No release tag, S12, G25, or P37 was created.
