# P06 - Genome, development state, and weight split contracts

Group: Group 1 - Parallel core contracts

Branch: `codex/P06-genome-weight-split`

Prerequisites: P04

Concurrency: Yes. Can run with P05, P07, P08, P09 after P04.

Next plan(s): P10, P14, P16, P33

## Purpose

Make the genome and weight split explicit enough that lifetime learning cannot accidentally overwrite inherited instinct. This plan gives later CPU/GPU/evolution work a safe contract.

## Owned scope

- `alife_core` genome, development, plasticity policy, and synaptic split contract modules.

## Required implementation steps

1. Expand `BrainGenome` from placeholder into a schema-versioned contract. Include genome ID, parent/lineage refs, brain class ID, random seeds, lobe ratio overrides or registry reference, macro-connectome masks, sparse density priors, alpha/plasticity masks, endocrine constants, drive thresholds, sensor layout, motor affordances, mutation rates, and developmental schedule.
2. Define `DevelopmentState` for age/maturation, enabled lobes, active sensor/motor affordances, critical periods, and sleep/consolidation cycle counters.
3. Define the full weight split contract: `WGeneticFixed`, `WLifetimeConsolidated`, `AlphaMask`, `HOperational`, `HShadow`, and `WEffective` calculation semantics. This can be type-level/schema-level first; do not allocate huge matrices here.
4. Add flags for experimental Lamarckian inheritance but keep default false and behind explicit feature/config if implemented.
5. Define mutation and crossover data structures without implementing full evolution yet. Evolution implementation belongs to P33.
6. Implement validation: genome schema version, brain class exists, density bounds, alpha range, mutation ranges, developmental schedule monotonicity, no NaN constants.
7. Add deterministic seed behavior and snapshot serialization if the crate already supports serde.
8. Update traceability for genome, development, and genotype/phenotype separation.

## Required tests and validation

- Unit tests for valid/invalid genome construction, schema version rejection, default non-Lamarckian behavior, deterministic seed reproduction, alpha/plasticity bounds, and development schedule validation.
- Tests proving `W_genetic_fixed` is not mutated by lifetime consolidation APIs.
- Boundary script and workspace tests.

## Acceptance criteria

- Genome contract covers the spec fields needed by later CPU/GPU/evolution work.
- Weight split is explicit and default-safe.
- No huge runtime matrix allocation is introduced in the contract crate.

## Failure handling

- If matrix storage details conflict with GPU plans, define abstract traits/types now and leave concrete buffers to P14/P24.
- If serde makes large arrays awkward, serialize metadata now and leave bulk tensor serialization to save/load P34.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P06 - Genome, development state, and weight split contracts
Branch: codex/P06-genome-weight-split
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P10, P14, P16, P33
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
