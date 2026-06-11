# P33 - Evolution, breeding, mutation, genome lab

Group: Group 4 - Evolution tools

Branch: `codex/P33-evolution-genome-lab`

Prerequisites: P06, P16, P30

Concurrency: Yes. Can run with P31/P32, but coordinate with P32 if weight assets overlap.

Next plan(s): P34, P35

## Purpose

Add offline evolution and breeding over valid genomes. This turns logs and genomes into an experimentation loop while preserving runtime safety.

## Owned scope

- Genome lab tooling, evolution modules, tests; not main runtime loop unless feature-gated.

## Required implementation steps

1. Implement mutation operators for genome fields: lobe ratios within valid class constraints, macro-connectome masks, sparse density priors, alpha masks, endocrine constants, drive thresholds, sensor layout, motor affordances, mutation rates, and developmental schedules.
2. Implement crossover/lineage records with parent genome IDs, generation, random seed, and compatibility checks.
3. Implement fitness summaries from packed logs: survival time, energy stability, food success, pain avoidance, curiosity resolution, social/word task score, and teacher verifier score if available.
4. Implement selection lab for offline/headless experiments. Keep it deterministic and separate from active gameplay unless explicitly configured.
5. Integrate optional generated weight assets from P32 as birth-only initializers.
6. Add safeguards: no mutation may produce invalid lobe layout, invalid alpha bounds, NaN constants, or runtime dynamic allocation requirements.
7. Add docs explaining genotype, phenotype, lifetime consolidation, and optional Lamarckian experiment flag if present.
8. Update traceability for breeding/evolution.

## Required tests and validation

- Tests for mutation validity, crossover compatibility, lineage serialization, deterministic selection, fitness summary from sample logs, rejection of invalid offspring, and genetic weights unaffected by lifetime state unless explicit experimental mode.
- Tool/harness smoke test for a tiny generation.

## Acceptance criteria

- The project can run offline evolutionary experiments over valid genomes.
- Breeding does not break core invariants.
- Generated assets and logs can participate without becoming runtime dependencies.

## Failure handling

- If full evolutionary runs are slow, implement tiny deterministic smoke tests and mark larger runs manual.
- If mutation conflicts with P05 lobe scaling, reject mutation rather than creating invalid topology.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P33 - Evolution, breeding, mutation, genome lab
Branch: codex/P33-evolution-genome-lab
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P34, P35
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
