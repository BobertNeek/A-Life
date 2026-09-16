# Phase 3.1 BEFORE setup notes

These attempts are rejected setup evidence. They are not performance baselines.

- Canonical player New Game with `--population 1` was rejected before GPU startup. `CanonicalNewGameConfig::phase3` permits 4 through 8 founders.
- The `p34` scenario was rejected before GPU startup because its schema-v1 save has no supported migration to schema v3. It is headless compatibility data and not a player workload.
- The accepted player baseline is `phase31-before-99b4c2bb-population-6.json`.
