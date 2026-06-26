# CA21 - Behavior Tuning Metrics Loop

Status: complete on `codex/CA21-behavior-tuning-metrics-loop`

CA21 adds a metrics loop over the existing bounded G19 balance smoke. It
detects the required degeneracy classes without changing runtime semantics:
stagnation, catatonia, overfeeding, hazard suicide, and population collapse.

Evidence added:

- `behavior-tuning-metrics-smoke` reports five detector statuses, five bounded
  scenario sweep sources, the source balance signature, and the no-hidden-
  overfitting guardrail.
- The report keeps overfeeding and hazard-suicide as explicit known limitations
  for the current bounded fixture instead of hiding them or converting them into
  pass claims.
- Scenario sweeps reuse deterministic survival, ecology, population, lifecycle,
  and performance/LOD signatures; CA21 does not retune fixture data to pass.

Boundaries:

- `alife_core` was not changed.
- No action arbitration, GPU authority, save/load, or Bevy presentation behavior
  was changed.
- CPU fallback and CPU shadow parity remain preserved and are not converted into
  product GPU claims.
- No release tag, S12, G25, or P37 was created.

Next: CA22 owns broader long-run ecological soak and balancing evidence.
