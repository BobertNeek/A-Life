# Reader v2: completed detached gain calibration

**No winner selected; no promotion.** The fixed gain-4 reader remains rejected,
but the results narrow the reason: its prior is too large relative to the supplied
fixture task margins. They do not establish that ingestion-to-operation memory
transfer is intrinsically unsuitable or requires a new goal representation.
See the [chronological interpretation update](../DAILY-HANDOFF-REVISION-2.md).

## Design and exact records

Five predeclared per-gene gains {0, 1/16, 1/4, 1, 4} replay the exact 24 original
v1 rows: 12 unique contexts × two original-arm provenances. This gives **120
one-tick CPU replay rows, not 120 independent contexts**. The CSV contains 60
unique gain/context combinations. No new physical dispatch, world preparation,
host step, life exposure or autonomous seed allocation occurred. Newborn
constructor objects reconstruct inherited genes; they do not constitute life
exposure. Inputs, zero starting state, unit task prior and fixture argmax stayed
fixed. These are native replay logits, not autonomous behavior or learning.

Gains 1/16 and 1/4 preserve every baseline fixture choice while changing nonzero
port logits. Gains 1 and 4 reverse four of twelve unique choices: both aversive
pickup labels, useful transport retention and safe-food placement. Three are
explicitly constructed useful-handling losses; aversive-object pickup refusal
alone is not inherently a safety gain. Zero-port controls are unchanged.

At |input53|=.25, absolute operation contributions are 0, .125, .5, 2 and 8.
No row reaches the ±12 clamp. MemoryContext sums input × effective weight;
ActionCandidate additionally uses source-neuron activity, but its inputs are
zero here. The existing target-pain route stays matched in all variants.

## Verified bounds, not a production guarantee

The memory contract validates finite family values in [-1,1] and confidence in
[0,1]; the operation port multiplies the family value by confidence after
validation. Supported recall is required, and aggregation respects these ranges.
A validation-only test checks endpoints and rejects invalid/nonfinite inputs.

For eight inherited reader coefficients with zero learned offsets, the maximum
absolute contribution is 8g. At g=1/16 this is **.5 per candidate**. A common
memory contribution followed by the ±12 clamp cannot amplify that perturbation.
This protects a strict unit argmax margin only for one affected operation against
an unaffected zero-port alternative. Two oppositely affected operations can
shift their pairwise margin by **1**, causing a unit margin to tie; smaller
margins can reverse. Gain 1/4's range-wide single-candidate bound is 2, so its
choice preservation here relies on the particular .25 input magnitude.

Learned effective-weight offsets can invalidate these inherited-only bounds.
Production sampling can change action probabilities without changing argmax;
factorized choices, competing targets and natural recurrent operating points
are not covered by these fixtures. Do not call 1/16 a winner because it preserves
the constructed unit margin.

## Identity and recorded verification

`outcomes.json` retains each gain's asset BLAKE3 identity and source-row provenance.
Gain 0 exactly matches v1 OFF
`0bf3400adc03e001be609717a5a325232bc8f86a734cf60ff7a740cab29de7f1`;
gain 4 matches v1 ON
`6aca3530e6f4bae706beefa4c13fc9dcdbf34876f5a1dd15a229c6f49ed74f6d`.
Every nonzero variant changes only the declared 16 inherited coefficients.
The graph, projections, dynamics and other weight bits are unchanged. New gain
variants keep distinct identities; existing creature/checkpoint identities are
not reinterpreted. V1 evidence is frozen.

Recorded CPU checks: four selected tests passed (gain ladder, memory-contract
bounds, legacy identity/64-coordinate accounting and 16-gene guard). Scoped
non-test example Clippy passed; cfg(test) bodies were compiled/executed by the
tests, not linted by that invocation. Rust 1.99.1, one job, incremental disabled,
debug 0 and 32 MiB Rust stack. This is not a full-workspace/GPU qualification.
V1 physical, ABI and checkpoint guards were not rerun for v2.

An initial constructor used -0 for the zero Other coefficient. Final source
preserves canonical +0, recovering v1 OFF identity without changing scores,
gains, inputs, margins or decision rules. This was an identity correction, not
an autonomous retry or fitted outcome. Raw logs are not distributed here.

The exact `v2-over-v1.patch` applies to the archived v1 source, not directly to
an arbitrary repository head. Archival checks apply it to that source and compare
the resulting files with this archive. `bindings.json` retains the recorded
historical source anchor and patch/executable/context hashes. `PROVENANCE.json`
explains curation; `SHA256SUMS` verifies the retained exact files and this summary.
No native build or replay was run to prepare this archive.

## Next candidate, not an executed study

A prospective passive shadow-scoring probe on actual fresh states is proposed:
compare all gains on identical states without applying shadow commands or
changing baseline sampling, learning, recall, RNG or physiology. First establish
instrumentation equivalence. Examine actual margins, acquired reader offsets,
competing operations/targets, support, heldness/reach and useful physical
consequences, not pickup count alone. No life/seed allocation or execution is
claimed. Shadow scores alone cannot establish improved nutrition, learning,
survival or harmless handling. Any physical comparison is a separate decision.
