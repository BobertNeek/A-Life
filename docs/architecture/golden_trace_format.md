# Golden Trace Format

Status: v1 scaffold contract.

Plan: P19 - Golden traces, property/fuzz tests, determinism.

Golden traces freeze deterministic CPU behavior for the P18 headless scenario suite. They are regression fixtures for future GPU, adapter, benchmark, and research work. They are not save files and do not serialize large neural matrices.

## Scope

P19 trace fixtures live under:

```text
crates/alife_world/tests/fixtures/golden_traces/
```

Each file is one JSON trace for one P18 scenario. The current schema is:

```text
schema: "alife.p19.golden_trace.v1"
schema_version: 1
```

The trace records:

- scenario key, human label, seed, and current experience/action/sensory/chemistry schema versions
- sealed `ExperiencePatch` summaries in causal order
- selected action kind/id, target id, status, failure, success/contact, reward and outcome deltas in milliscale integer units
- memory expectancy danger/salience summaries
- bounded world signature and stable memory/topology/sleep counts
- final drive and hormone summaries in milliscale integer units
- compact neural tick counters where stable

The trace intentionally omits:

- dense neural matrices
- full sparse payload pools
- renderer, Bevy, Avian, or GPU resources
- teacher-private or internal SLM state
- exact raw floating point values where milliscale summary values are enough

## Tolerances

`tolerances.milliscale_absolute` is currently `0` because all serialized float-derived values are rounded to deterministic milliscale integers before comparison.

`tolerances.stochastic_fields` currently contains only `scenario.seed`. P19 tests assert that same-seed replay is identical and that changing the seed changes only declared stochastic fields.

If future behavior needs additional tolerated fields, update:

- the JSON fixtures
- `crates/alife_world/tests/golden_traces_determinism.rs`
- this document
- `docs/codex_progress/DECISION_LOG.md` if the tolerance changes public comparison semantics

## Updating Fixtures

Golden updates are opt-in and must be reviewed. Do not edit fixture JSON by hand unless the change is mechanical and obvious.

1. Run the focused scenario tests first:

```bash
cargo test -p alife_world --test golden_traces_determinism
```

2. If the mismatch is intentional, regenerate fixtures:

```bash
ALIFE_UPDATE_GOLDEN_TRACES=1 cargo test -p alife_world --test golden_traces_determinism p18_scenarios_match_versioned_golden_trace_fixtures
```

On Windows PowerShell:

```powershell
$env:ALIFE_UPDATE_GOLDEN_TRACES='1'; cargo test -p alife_world --test golden_traces_determinism p18_scenarios_match_versioned_golden_trace_fixtures; Remove-Item Env:ALIFE_UPDATE_GOLDEN_TRACES
```

3. Review every fixture change:

```bash
git diff -- crates/alife_world/tests/fixtures/golden_traces
```

4. Rerun the normal test with update mode off:

```bash
cargo test -p alife_world --test golden_traces_determinism
```

5. Commit the fixture diff with the code or scenario change that made it necessary.

Mismatch diagnostics name the scenario and first differing patch or neural tick, plus state and world-signature differences when present. This keeps failures small enough to inspect in normal CI logs.
