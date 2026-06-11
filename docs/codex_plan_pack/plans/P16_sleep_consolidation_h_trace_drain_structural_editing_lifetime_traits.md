# P16 - Sleep consolidation, H-trace drain, structural editing, lifetime traits

Group: Group 1 - Core serial

Branch: `codex/P16-sleep-consolidation`

Prerequisites: P15, P06, P12, P13, P14

Concurrency: No. Run after CPU reference loop.

Next plan(s): P17, P18, P33, P28

## Purpose

Implement sleep and consolidation safely while preserving genotype/phenotype separation. This turns lifetime learning into stable habits without corrupting inherited genes.

## Owned scope

- `alife_core` sleep/consolidation modules and tests.

## Required implementation steps

1. Define sleep state machine: awake, entering sleep, consolidating, waking, forced recovery sleep. Trigger on fatigue threshold, recovery protocol, or harness command.
2. Implement CPU reference sleep consolidation: synaptic compression, H_shadow to H_operational staging, H_shadow decay, low-salience pruning markers, structural synaptogenesis candidate generation from topology correlations, episodic memory compression, and concept/simplex consolidation.
3. Implement `W_lifetime_consolidated` update for stable long-term habits while keeping `W_genetic_fixed` immutable.
4. Implement cold consolidated trait detection: stable variance across configurable sleep cycles; alpha reset to zero only for lifetime layer if policy allows; never mutate inherited genetic layer by default.
5. Define structural edit batches that can later compile into double-buffered GPU matrix replacements. Keep active tick allocation-free; compile edits during sleep/offline phase.
6. Add recovery hooks for seizure/hyperactivity and catatonia/energy hypoplasia states.
7. Expose deterministic APIs for harness to force sleep and inspect changes.
8. Update traceability for sleep, consolidation, and genotype/phenotype safety.

## Required tests and validation

- Tests for fatigue-triggered sleep, H_shadow drain, H_operational update timing, H_shadow decay, memory compression, concept consolidation, stable trait promotion to lifetime layer, genetic weights unchanged, structural edit batch validation, and recovery sleep.
- Workspace tests and boundary script.

## Acceptance criteria

- Sleep consolidation exists in CPU reference form.
- Lifetime learning can stabilize without corrupting inherited genome.
- Structural edit batches are ready for GPU recompaction work later.

## Failure handling

- If full structural synaptogenesis is too much, implement candidate generation and no-op application with tests, then leave concrete application to a follow-up note. Do not fake completed mutation of matrices.
- If Lamarckian mode is requested by existing code, gate it clearly and keep tests proving default off.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P16 - Sleep consolidation, H-trace drain, structural editing, lifetime traits
Branch: codex/P16-sleep-consolidation
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P17, P18, P33, P28
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
