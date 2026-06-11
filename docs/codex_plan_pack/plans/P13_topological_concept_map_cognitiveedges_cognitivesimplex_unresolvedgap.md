# P13 - Topological concept map, CognitiveEdges, CognitiveSimplex, UnresolvedGap

Group: Group 1 - Parallel after P10/P11

Branch: `codex/P13-topological-map`

Prerequisites: P10

Concurrency: Yes. Can run with P12 after P10.

Next plan(s): P15, P16, P18, P23

## Purpose

Build the CPU-side concept-and-experience graph that turns repeated experience and contradictions into concepts, relations, and curiosity targets.

## Owned scope

- `alife_core` topology/concept modules and tests.

## Required implementation steps

1. Define `ConceptCell` with grounded multimodal bindings: objects, words, drives, actions as observed facts, emotions/valence, locations, agents, affordances, and semantic/cluster references by stable ID.
2. Define `CognitiveEdge` with relation kinds: predicts, causes, satisfies_drive, belongs_to, socially_liked, socially_feared, contradicts, co_occurs, enables, blocks, and teacher_labels if needed as perceived evidence.
3. Define `CognitiveSimplex` as a multi-way consolidated episode binding concept IDs and summary statistics.
4. Define `UnresolvedGap` with source concepts, contradiction type, prediction error, curiosity voltage/salience, first/last tick, confidence, and resolution status.
5. Implement `TopologicalMap` with bounded storage, deterministic ID allocation, patch-to-concept update, edge strengthening/decay, contradiction detection, and unresolved gap creation.
6. Keep map CPU-side and engine-independent. Do not implement graph databases or GPU graph structures in core.
7. Add APIs for curiosity bias output to memory/action later.
8. Update traceability for concept map and curiosity gaps.

## Required tests and validation

- Tests for concept creation from patch, repeated patch strengthens concept/edge, contradictory outcome creates unresolved gap, curiosity salience increases on unresolved contradiction, bounded map behavior, deterministic IDs, and no engine type usage.
- Workspace tests and boundary script.

## Acceptance criteria

- Repeated experience can create/update concepts and relationships.
- Prediction errors create unresolved gaps rather than being lost.
- CPU reference brain has a topology API to call after sealing patches.

## Failure handling

- If full multimodal binding is too large, implement minimal concept fields plus extension maps, but tests must cover objects/words/drives/actions/valence.
- If map growth is a problem, add caps and eviction before adding features.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P13 - Topological concept map, CognitiveEdges, CognitiveSimplex, UnresolvedGap
Branch: codex/P13-topological-map
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P15, P16, P18, P23
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
