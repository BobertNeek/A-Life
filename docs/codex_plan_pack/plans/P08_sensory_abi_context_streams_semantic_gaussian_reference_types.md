# P08 - Sensory ABI, context streams, semantic/Gaussian reference types

Group: Group 1 - Parallel core contracts

Branch: `codex/P08-sensory-contexts`

Prerequisites: P04

Concurrency: Yes. Can run with P05, P06, P07, P09 after P04.

Next plan(s): P10, P14, P21, P22

## Purpose

Lock down the sensory/context ABI without coupling core to Bevy, Gaussian renderers, or SLMs. This gives the world, school, semantic adapter, and GPU input packing a stable target.

## Owned scope

- `alife_core` sensory/context modules; no Bevy/Gaussian runtime dependencies.

## Required implementation steps

1. Define the sensory ABI with schema version, fixed channel counts, channel semantics, bounds, and extension policy. Include visual affordance channels, auditory/acoustic channels, smell channels, tactile/contact channels, pain/novelty signals, and nearby affordance bitfields.
2. Define `ContextStream` contracts for atmospheric temperature, ambient light, energy intake/blood sugar trend, vocal tokens, social proximity, and any optional environment streams.
3. Define runtime `SensorySnapshot` using stable IDs and core math, not Bevy entities or engine vectors.
4. Define optional semantic/Gaussian reference structs: cluster IDs, salience entries, egocentric bin hash, compressed semantic codes, confidence, and feature flags. Keep them as refs/metadata, not renderer objects.
5. Define social and language context snapshots: nearest agents by stable ID, gaze/orientation vectors, affinity scores, heard tokens, vocalized token, word confidence, and teacher/school perception channel marker if needed.
6. Implement validation for fixed array lengths, bounded channels, finite vectors, ABI version, and optional-context absence.
7. Add conversion traits/interfaces that adapters can implement later. Do not implement Bevy or 3DGS adapters here.
8. Update traceability rows for sensory ABI, context streams, and optional Gaussian/semantic boundary.

## Required tests and validation

- Tests for ABI version compatibility, channel bounds, optional Gaussian context absence, stable ID usage, fixed-size array behavior, and rejection of NaN sensory data.
- Tests that sensory snapshot can be constructed without semantic/Gaussian context.
- Boundary script and workspace tests.

## Acceptance criteria

- Core sensory contract is stable enough for ExperiencePatch, world harness, Bevy adapter, semantic adapter, and GPU input packing.
- Semantic/Gaussian features are optional and cannot couple rendering into core.
- SLM/teacher inputs are represented as perceptual/modulatory context only.

## Failure handling

- If current code uses variable vectors, separate runtime vectors from packed side buffers and fixed ABI arrays.
- If channel counts are uncertain, define constants and document them as v1 defaults with schema versioning.

## Required completion receipt

Codex must end the plan with this exact information:

```text
Completion receipt
Plan: P08 - Sensory ABI, context streams, semantic/Gaussian reference types
Branch: codex/P08-sensory-contexts
Files changed:
Public APIs changed:
Tests added/changed:
Commands run:
Results:
Invariant checks:
Deviations:
Known limitations:
Next plan(s): P10, P14, P21, P22
```

## Do not proceed past this plan until

- The completion receipt is written.
- Validation has run or unavailable commands are honestly recorded.
- `docs/codex_progress/PLAN_PROGRESS.md` and `SPEC_TRACEABILITY.md` are updated.
- Any architecture decision made during this plan is recorded in `DECISION_LOG.md`.
