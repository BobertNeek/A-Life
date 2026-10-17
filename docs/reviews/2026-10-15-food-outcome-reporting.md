# Food assessment reporting, 15 October 2026

Base: `69f00c05ec6774c3655ae87c5d4baa5df3861929`.
This change is a bounded reporting projection over the existing CPU world map
and sealed learner receipts. It does not establish GPU behavioral success.

## Outcome and boundary

- Pilot and lesson JSON now share an optional `food_outcome`: exact learner and
  target IDs plus separate explicit-Grab acquisition, ordinary acquisition,
  consumption, and nourished-meal event counts
- Before/after target ownership copies are attached only to the matching captured
  learner row, ticks, frame digest, and neural dispatch generation. Missing
  object snapshots remain null; present unowned objects retain owner=null
- Ordinary acquisition accepts selected exact-target Eat or Grab plus matching
  Manipulation Touch and an unowned-to-learner transfer. Present empty bundles
  cannot fall back to a representative action
- Consumption uses exact-target selected Eat/Consumed evidence. Nourishment also
  requires prior exact possession, measured body benefit, and no fresh harm,
  using the unchanged held-food predicate. A blocked concurrent Move does not
  erase a successful manipulation outcome
- Explicit Grab still comes solely from the existing strict acquisition list.
  The Grab observer/reward gate, existing food scoring helpers, lesson completion,
  stop conditions, legality, training, and checkpoint schemas are unchanged
- Grab probes still stop at their existing acquisition boundary. Zero meals means
  none observed within that probe, rather than failed meal competence

This preserves read-only, tick-bound telemetry (AOA-OBS-004), distinct physical
and biological evidence (AOA-BIO-024/025), and bounded purpose-driven observation
(AOA-FAIL-005). No controlling architecture was amended.

## CPU verification

The final source passed these bounded CPU-only gates:

- `--lib food_outcome`: 7 passed, including real canonical-world Eat pickup then
  measured meal with explicit Grab=0, real Grab/release, exact ownership/target,
  empty-bundle handling, blocked Move, and row-admission/serialization guards
- `foundation_training::lesson_trace::tests`: 9 passed, retaining the existing
  global-grip-does-not-prove-ownership regression
- `foundation_grab_food::tests`: 4 passed
- `foundation_training::held_food_lesson_tests`: all 16 passed, including the
  existing real-world eating, missing/no-benefit physiology, and blocked-channel
  fixtures
- Historical pilot sampler/receipt round-trip: 1 passed
- `--test foundation_grab_food_world_gate`: 6 passed, including 2 sealed CPU-world
  acquisition/meal tests and the same 4 unchanged Grab unit tests

Production Clippy (`--profile test --lib -- -D warnings`), formatting, and
`scripts/check.sh --quick` also passed with exit 0; quick checks reported core
boundaries passed and docs assertions 77/77. There are 43 passing test executions
across 38 distinct test names (the admission test and four Grab tests recur).

Tests compile the foundation-training feature but execute no GPU/training path.
Builds use one job, no debug symbols or incremental state, package opt-level=0
and codegen-units=256 for core/world/GPU-backend/runtime/app test dependencies,
with assertions retained. The ordinary optimized dependency build was stopped
before tests to keep host resource use bounded.

Initial scaffold-only meal fixtures failed because their legacy sealed outcomes
have no measured physiology. Corrected meal tests reuse existing canonical
GrabFood/registered-Eat fixtures and retain the measured-benefit/harm assertions.
The scaffold now explicitly verifies that physiology is absent. Its legacy and
ObjectSlots sensory profiles omit grip; the unrelated-held-food regression asks
the real GroundedTerrainVision profile and still asserts grip=1 while the exact
lesson target remains unowned. Failed/diagnostic logs are retained separately.
Synthetic selection/channel data is labeled and proves only diagnostic binding
and serialization, never sampled GPU behavior or curriculum admission.

Full workspace/all-target checks, GPU execution, training, checkpoint lifecycle
acceptance, and live probe/JSON generation were not run. Serialization checks
exercise the actual receipt type and serializer helpers with CPU fixtures.
