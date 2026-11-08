# Matched atomic-swap comparison: failed reserved attempt

**Closed without retry. Two attempted lives, zero returned or validated case
summaries, zero complete validated pairs. No behavioral or efficacy result.**

This record preserves the 2026-11-08 attempt to compare the old world with the
atomic-swap world under the same inherited v12 brain asset. It is not a new
experiment, a completed world comparison, or a result for the grip-aware neural
candidate. The eight original public files are copied without changing a byte.
The README and two additional JSON records explain their provenance and limits.

## Question and matched design

The physical obstruction motivating this comparison was specific: a creature
holding harmful food A could select safe food B but fail to acquire B because
its deliberate grip was occupied. Selecting Eat(B) is not the same as obtaining
or consuming B. The earlier observations did not prove that possession caused
every failed attempt.

The world repair validates the new pickup and a supported placement for A before
placing A and acquiring B. Failed pickup or placement preserves A. Atomic
replacement and ordinary-item grounding are **one bundled world factor** here;
the attempt cannot separate their individual effects. The comparison was not
allowed to replace the selected target with whichever food was already held.

The plan was seeds **2026112201** and **2026112202**, each once in old and repaired
worlds: four lives total. The inherited brain asset, scene, initial body,
optimizer, sampling identity and full initial checkpoint were to be matched
within each pair. Each arm had 600 world-only preparation ticks and at most
256 primary ticks, with health, injury, death and repeated-harm safeguards.
Preparation was not neural experience. Ordinary online plasticity and natural
sleep remained enabled; no teacher actions, synthetic memories or forced sleep
were part of this comparison.

Physical swaps, actual meals, failures, reachable opportunities, possession,
injury and episode-memory coverage were separate intended measurements. A swap
alone would not establish feeding or learning. No contact would not prove
avoidance, and unequal safeguard-stopped horizons would require explicit bounds.

## What happened

The single reserved invocation stopped at the terminal gate `native_rejected`.
Both seed-2026112201 arms completed exactly 600 preparation ticks and passed the
full initial-state pairing barrier. Only the old-world arm received a primary
`START`. No summary returned from either arm. The remaining seed was not started.

| Case | Preparation ticks | Primary ticks | Accounting |
| --- | ---: | ---: | --- |
| s1_old_world, seed 2026112201 | exactly 600 | unknown within 0–256 | Attempted, START observed, rejected, no summary |
| s1_new_world, seed 2026112201 | exactly 600 | exactly 0 | Attempted and prepared, no START, waiting peer classified unknown |
| s2_old_world, seed 2026112202 | exactly 0 | exactly 0 | Unstarted |
| s2_new_world, seed 2026112202 | exactly 0 | exactly 0 | Unstarted |

There were **two attempts**, 1,200 exact preparation ticks in total, one rejected
case, one unknown waiting peer and two unstarted cases. The peer's **outcome** is
unknown in the original accounting, but its primary exposure is known to be
zero. A START marker occurs before the first primary advance, so it does not
prove that the old arm completed even one primary tick. Its exact exposure must
remain unknown, not silently converted to zero or 256.

`rows.json` contains no cases. `summary.json` contains no cases or complete
pairs. Zero returned or validated summaries means that no meal, world-effect,
avoidance, learning or brain-outcome comparison is supported. Native processes
were terminated. This allocation is closed; the unstarted cases and prepared
peer are not invitations to resume it.

## Chronology and the limits of prior qualification

1. The earlier eight-case reader study attempted all eight cases and validated
   seven. Its eighth reporting outcome remains unknown. That study is closed
   and must not be rerun. Historical 3-to-1 harmful-meal differences and detached
   memory-lane switches remain local evidence under their original world and
   horizons, not controls for this changed world or evidence of sustained feeding.
2. [World repair PR #302](https://github.com/BobertNeek/A-Life/pull/302) merged as
   `ebc2b0ff8969d87751e62d34cf8d26906dbbda4b`. Its merge tree matched tested tree
   `2e5bfd15103b42e61599b327eead19758372d592`. Seven exact-head workflows passed,
   including 25 new hands fixtures and the workspace checks. Unfinished automated
   reviews were not counted as approval.
3. [Grip candidate PR #301](https://github.com/BobertNeek/A-Life/pull/301), a
   separate draft research candidate, incorporated that repair. Head
   `031de4ebe10209783e17f55a1f385c2960136545` passed 160 fixed functional fixtures
   and 18 strict Clippy targets in local and hosted qualification. These are
   compatibility, physical-action, continuation, detached-gradient, upload and
   shader-validation contracts, not autonomous or GPU-device qualification.
4. The reconstructed old/new control kernels passed 16 actual fixed host steps
   and eight replay evaluations with full ordinary-versus-audited state equality.
   Those fixtures used **zero preparation ticks** and did not cover the prepared
   manipulation condition that this attempt needed. The active binaries also
   passed 14 zero-life checks and eight malformed-argument CLI cases. Those checks
   did not use the reserved study seeds and did not establish autonomous behavior.
5. The reserved invocation produced only the failure accounting above. The
   control package was a new v2 reconstruction, not byte-identical recovery of
   the lost earlier package. No retry was performed.
6. Subsequent source inspection identified the evidence-contract defect below.
   It did not recover the observed event, tick or precise cause of this rejection.
   This preservation change runs no Cargo command, native executable or study.

## Definite source defect, unknown observed failure

The preserved `common-audit-kernel.patch` adds
`examples/support/world_swap_receipt_projection.rs` in `alife_training`.
Its `event` projection requires `c.observation` with `executed == true` whenever
there is any grip transfer. Its meal projection also requires that value.

However, `measured_motor_channel_observation` in
[`alife_world/src/headless.rs` at the published integrated head](https://github.com/BobertNeek/A-Life/blob/031de4ebe10209783e17f55a1f385c2960136545/crates/alife_world/src/headless.rs#L7654-L7670)
returns `None` for every non-Locomotion channel. Manipulation therefore has no
such measured observation even when its actual per-channel execution succeeded.
A valid manipulation transfer or meal cannot satisfy those projector conditions.
This is a deterministic source-level instrumentation defect, not evidence that
the world action itself failed or that a creature refused food.

**The event and tick that caused this particular rejection remain unknown.**
The native entry path converts any failed `run_reviewed` result to the generic
`case rejected; no retry`, discarding its exact cause. No primary-step progress
was persisted. The source defect must not be promoted into a recovered trace or
an assertion that the observed failure was specifically `transfer_execution`.

The next instrumentation revision should expose the existing per-channel
`execution.succeeded` boolean transiently and use it for affirmative execution
evidence. It should retain physical-contact, target, ownership, order, phase and
failure checks. It must not fabricate a locomotion observation, substitute an
aggregate action result, or weaken the checks to make valid-looking output.
Bounded failure-stage diagnostics should retain only explicitly selected scalar
facts. Fresh fixed fixtures should exercise actual successful and failed
manipulation, meals and the prepared-state condition missing from the earlier
qualification. These are next-revision requirements, not changes made here and
not permission to replay these spent cases.

## Source and byte provenance

- Common source base: `20bcb45104e5bec688b01abefa96cdf95bdcee60`.
- World source head: `30efe5ee3bb5a5cdf270288eb603b227d6c5cdfd`.
- Common audit patch SHA-256:
  `6e9e74226bac2309fa876cd327011366d9a6d2e1c865b889a1270835ffe6c257`.
- Bound world patch SHA-256:
  `5cc4ea55ff8505bff852291ad0037bf05b7d1b1010a8db59cd264584332b4b2c`.
- Both arms' original source trees, source-closure hashes, executable hashes and
  common plan hash are preserved in [source-bindings.json](source-bindings.json).
  Hashes identify those artifacts; they do not include the executables or claim
  that another build or run occurred.

The eight originals total **147,302 bytes**:

- `SHA256SUMS`: unchanged original seven-entry payload checksum list.
- `checks.json`: original attempt, exposure and terminal-gate accounting.
- `common-audit-kernel.patch`: original common source patch, including its defect.
- `report.md`: original short report, not rewritten after diagnosis.
- `rows.json`: original empty cases object.
- `source-bindings.json`: original source, patch, plan and executable identities.
- `summary.json`: original empty case and complete-pair collections.
- `world-factor.patch`: original bundled world-factor source patch.

[preservation-manifest.json](preservation-manifest.json) records each original's
size, SHA-256 and Git blob identity, including the checksum file itself.
[as-run-provenance.json](as-run-provenance.json) is an additional public-safe
scalar projection, not an original returned case summary. It makes the exposure
bounds, source identities, closed allocation and unknown failure event explicit.
All seven original payload checksums were independently verified before copying;
all eight copies were also checked byte-for-byte against their originals.

The archive contains source and bounded scalar records only. No learned
checkpoint, raw journal, dynamic memory bank, raw error payload, binary or
credential is included. Existing source, inherited genes, workflows and prior
historical evidence are unchanged by this preservation.
