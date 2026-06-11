# P15 - CPU reference brain tick loop

Group: Group 1 - Core integration serial

Branch: `codex/P15-cpu-reference-brain`

Prerequisites: P10, P12, P13, P14, P09, P07

Concurrency: No. Integrates the core.

Next plan(s): P16, P17, P21, P23, P24

## Purpose

Integrate the core contracts into a deterministic pure Rust creature tick. This is the first point where the scaffold becomes a functioning brain loop.

## Owned scope

- `alife_core` reference brain loop, creature mind state, integration tests.

## Required implementation steps

1. Define `CreatureMind` or equivalent runtime state: creature ID, genome/development refs, body state placeholder, drive state, hormone state, neural state, memory bank, topological map, action state, tick counter, and diagnostics.
2. Define adapter traits for sensory input gathering and action execution without depending on Bevy. The CPU reference loop may use trait objects/generics or explicit input/output structs.
3. Implement the normal tick flow: read body/drives/hormones; gather sensory frame; query memory expectancy; build pre-action snapshot; run neural projection/proposal generation; apply memory/topology/endocrine biases; arbitrate action; build decision snapshot; execute action through adapter/fake executor; measure outcome; build post-action outcome; seal patch; update memory/topology/endocrine/learning trace/log hooks.
4. Implement retry path for missing affordance/action failure: mark failed, add contradiction salience, update unresolved gap, bias away from invalid action in same context.
5. Implement terminal failure handling: invalid ID, non-monotonic tick, NaN state, missing phase, invalid ABI -> reject patch and enter safe idle/halt for that tick without learning.
6. Implement neutral fallback behavior: idle/inspect/rest when no proposal passes threshold.
7. Expose deterministic single-agent stepping API for harness and tests.
8. Update traceability for CPU reference sequence.

## Required tests and validation

- Integration tests for one full normal tick, failed action retry path, terminal invalid-state rejection, memory update after sealed patch, topology update after contradiction, endocrine update after outcome, deterministic same-seed replay, and no learning from partial patch.
- Workspace tests and boundary script.

## Acceptance criteria

- A pure Rust single-creature CPU reference brain can run deterministic ticks.
- It produces sealed ExperiencePatch records and updates memory/topology correctly.
- It does not depend on Bevy/GPU/Avian.

## Failure handling

- If neural proposal generation is not mature, use deterministic simple proposal fixtures but keep the loop structure exact.
- If adapter traits become too broad, split sensory/action/outcome traits rather than importing engine types.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P15 - CPU reference brain tick loop
Branch: codex/P15-cpu-reference-brain
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P16, P17, P21, P23, P24
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
