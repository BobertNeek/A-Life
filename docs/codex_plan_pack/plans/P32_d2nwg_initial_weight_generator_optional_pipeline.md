# P32 - D2NWG/initial weight generator optional pipeline

Group: Group 4 - Research optional parallel

Branch: `codex/P32-d2nwg-weight-generator`

Prerequisites: P30, P06, P14

Concurrency: Yes. Can run with P31/P33 after P30/core.

Next plan(s): P33, P35

## Purpose

Implement the optional initial-weight generation asset pipeline. The project can consume D2NWG-style assets without requiring a live ML stack.

## Owned scope

- Offline research tooling and asset import/export; no runtime dependency.

## Required implementation steps

1. Define the asset contract for generated initial weights: schema version, brain class, lobe layout hash, W_genetic_fixed payload, alpha mask payload, density/mask metadata, provenance, and validation digest.
2. Implement an importer/exporter for generated weight tensors. If no ML model exists, implement a deterministic procedural fallback that conforms to the same asset schema.
3. Define hooks for external D2NWG training/generation scripts without making Python/ML dependencies required for Rust runtime.
4. Add validation that generated weights match brain class size, lobe boundaries, sparse density limits, alpha ranges, and fixed/lifetime split rules.
5. Add optional seed templates: survival baseline, curious explorer, social learner, language-biased lexicon, and neutral control. These can be procedural until real D2NWG exists.
6. Document the difference between generated inherited weights and lifetime learned weights.
7. Add sample tiny asset for tests, not huge binaries.
8. Update traceability for D2NWG optional pipeline.

## Required tests and validation

- Tests for asset schema versioning, size/layout validation, alpha bounds, density bounds, deterministic fallback generation, import rejection for mismatched brain class, and runtime build without D2NWG feature.
- Tool smoke test for generating tiny fixture asset.

## Acceptance criteria

- The project can consume generated initial weights without embedding ML generation in gameplay.
- Procedural fallback keeps development unblocked.
- Weight assets preserve genotype/lifetime separation.

## Failure handling

- If a real D2NWG model is unavailable, do not fabricate one. Build the contract and fallback generator.
- If generated assets are large, keep them out of git and document artifact storage.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P32 - D2NWG/initial weight generator optional pipeline
Branch: codex/P32-d2nwg-weight-generator
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P33, P35
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
