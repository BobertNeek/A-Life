# P05 - Brain classes, lobe layouts, routing masks, BrainClass registry

Group: Group 1 - Parallel core contracts

Branch: `codex/P05-brain-lobes-routing`

Prerequisites: P04

Concurrency: Yes. Can run with P06, P07, P08, P09 after P04.

Next plan(s): P10, P14

## Purpose

Make brain topology explicit, scalable, and data-driven. Later CPU and GPU code must read this topology instead of duplicating magic lobe boundaries.

## Owned scope

- `alife_core` brain class, lobe, routing/projection metadata modules and tests.

## Required implementation steps

1. Finalize brain class registry with scalable tiers such as Nano512, Small1024, Standard2048, Large4096, and future/cloud tiers if already scaffolded. Do not hard-code Standard2048 as the only brain size.
2. Define lobe descriptors with stable lobe IDs, inclusive/exclusive index ranges, neuron counts, update frequency, plasticity policy, activation policy, and purpose documentation.
3. Implement Standard2048 lobe topology from the spec: sensory, metabolic drive, lexicon, association, episodic memory, motor arbitration ring, and homeostatic regulation. Represent boundaries as data, not scattered constants.
4. Implement scale rules for smaller/larger classes: either proportional ranges or explicit registries. Validate alignment to 16-wide microtiles and 128-wide supertiles where required.
5. Define routing masks between lobes: source lobe, target lobe, allowed projection type, active tile policy, update frequency, and optional biological priority.
6. Add compute-budget metadata: max active synapses per agent, lobe throttling priority, and essential/non-essential classification.
7. Expose read-only query APIs for brain class lookup, lobe by neuron index, lobe range iteration, routing matrix iteration, and validation.
8. Update traceability rows for lobe topology, routing masks, and compute budget.

## Required tests and validation

- Tests for every brain class validating total neuron counts, no overlapping lobe ranges, complete coverage or deliberate gaps, 16/128 alignment where required, and stable lobe lookup.
- Tests that Standard2048 matches the spec boundaries.
- Tests for routing mask validation and no invalid lobe references.
- Workspace tests and core boundary script.

## Acceptance criteria

- Brain topology is data-driven and scalable.
- Standard2048 lobe segmentation is exactly represented.
- Routing masks can feed CPU and GPU projection schemas later.
- No active-loop resizing assumptions are introduced.

## Failure handling

- If proportional scaling creates invalid small lobes, use explicit class tables instead of clever formulas.
- If existing scaffold has conflicting names, add compatibility aliases only if tests prove they map correctly.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P05 - Brain classes, lobe layouts, routing masks, BrainClass registry
Branch: codex/P05-brain-lobes-routing
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P10, P14
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
