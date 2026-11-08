# Manipulation receipt repair v3: fixed qualification passed

**Both old and atomic-swap world arms passed the minimal native receipt
qualification. This is a physical-action and measurement result, not a learned
behavior or feeding study.** The prepared-state audit and full finite failure
diagnostics remain pending.

## Defect and repair

`MeasuredChannelObservation` is produced only for Locomotion. The v2 projector
incorrectly required `observation.executed` to establish a successful
Manipulation transfer or meal, even though those receipts have `observation=None`.

V3 copies each channel's actual `result.execution.succeeded` into a transient
receipt field and uses that field for execution evidence. Measured observations
are unchanged. It does not substitute aggregate transaction success or change
the brain, world-action rules, biology, or learning. Contact, phase, selected
and physical target, ownership, transfer order and failure checks are retained;
Manipulation-only guards and consistency checks reject wrong-channel,
failed-action and forged-transfer receipts.

## Completed fixed qualification

Each world passed seven zero-life tests and exactly one ignored receipt test:
`projection::receipt_regression::actual_manipulation_receipts_without_locomotion_observations`.
That is **14 zero-life passes and two physical-fixture passes**, with no retries.

Each receipt test creates one disposable N512 physical record, using seed
`0x57535634`, then issues five literal Manipulation EAT actions targeting
A, B, B, B, B. The asserted event sequences are:

- Old world: acquired → placed-only → acquired → consumed → failed.
- Atomic-swap world: acquired → atomic-swap → consumed → failed → failed.

Successful manipulation still has no locomotion observation. Corrupted copies
of actual receipts are rejected without additional world actions. Across both
arms the budget was **two physical records, ten registered actions and biology
action advances, and eight world-clock advances**. There were zero neural host
steps, replay evaluations or preparation ticks. These were direct registered
world actions, not choices made by a creature brain.

The recorded Rust 1.99.1 build used Cargo's test profile, locked dependencies and
one job. Example tests used optimization level 0; `alife_core`, `alife_world`
and `alife_runtime` used level 2. Debug assertions and overflow checks were on,
and debuginfo was off. Core dumps were disabled, umask was 077, and each test
process had a 65,536-byte output cap. Raw output was not retained in these
results. The study main remained disabled; no valid study CLI or reserved study
invocation ran.

## Interpretation and remaining work

This establishes that the repaired receipt projection accepts the tested real
manipulation paths, including an atomic swap followed by consumption, and
rejects the covered corrupt receipts. It does not establish autonomous feeding,
learned avoidance, GPU performance or full prepared-host projection.

The [earlier failed reserved attempt](../failed-reserved-attempt/README.md)
remains closed without retry. Its exact failed event and tick remain unknown.
Repairing a definite general source defect does not retrospectively identify
that failure. Its original evidence is preserved unchanged.

Before a later behavioral comparison, the prepared-state audit and bounded,
finite failure-stage diagnostics still need separate implementation and
qualification. No result here clears those gates or resumes the spent cases.

## Preserved source and scalar evidence

- [results.json](results.json) is a public-safe scalar projection of the completed
  qualification, including exact counts, event sequences and explicit limits.
- [provenance.json](provenance.json) binds the original result, frozen package and
  source proof hashes to both source trees, tested executable identities,
  profiles, scalar receipt hashes and changed-path source hashes.
- [common-audit-kernel.patch](common-audit-kernel.patch) is the byte-exact frozen
  v3 common source patch. It includes the receipt repair, disabled example and
  fixed qualification source. It is an archive, not applied production code.
- The unchanged [world-factor.patch](../failed-reserved-attempt/world-factor.patch)
  is reused from the earlier archive. Its SHA-256 is
  `5cc4ea55ff8505bff852291ad0037bf05b7d1b1010a8db59cd264584332b4b2c`.
- [SHA256SUMS](SHA256SUMS) covers the four files above in this directory.

Reconstruction is source-only: start from commit
`20bcb45104e5bec688b01abefa96cdf95bdcee60`. Applying the common patch gives old
world tree `49bade13953af11f689db025266f318579c7acb3`; applying the common and
world patches gives new world tree `f705be430d72c24d3f1d2cad20b30eb099126932`.
The two patches commute exactly. The common patch SHA-256 is
`f75f64bb0246c69ac1c51001454dadcfb7ff2eb078cabb546fc1be7c10bd61a6`.
The frozen constructor identity is preserved. These are historical tested
source identities, not the documentation commit's tree.

This archive contains source and selected scalar evidence only. It contains no
executable, trained checkpoint, memory bank, raw journal, error payload or
credential. Source/hash validation of this archive performs no new build,
native test or world execution. Existing runtime, genes, workflows and previous
archive bytes are unchanged.
