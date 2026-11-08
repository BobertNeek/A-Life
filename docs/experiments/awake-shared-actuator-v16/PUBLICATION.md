# Shared-time withdrawal component, V16

This opt-in world component lets a brief protective withdrawal interrupt voluntary
translation without granting a second locomotor interval. It is not activated by
the ordinary CPU/GPU host or the default game. No brain weights, biological
initialization, persisted fields, checkpoint formats or learned-state migration
are changed.

## Why this component exists

Cassidy proposed a direct pain shortcut alongside motor reconciliation, with a
brief reversed-heading fallback when direction is unknown. The controlling
architecture permits simple protective reflexes (AOA-BODY-007) while forbidding
silently solving a strategic task (AOA-BODY-008). This component handles a local
physical contact, not planning or food selection. It uses actual contact geometry
and the existing fallback, movement, collision and capacity rules.

An initial damaging contact owns the translation interval. If voluntary motion
first encounters a hazard during the interval, the lawful prefix executes and
withdrawal receives only the remaining time. This matters for slower commands:
half-speed movement does not leave half the tick available after moving for a
whole tick. Free-space elapsed time uses spatial distance; terrain walking retains
its existing ground-plane metric. Receipts distinguish those metrics from spatial
path length, selected intent, actual voluntary motion and involuntary motion.

## Measured qualification

- Accepted baseline: c775c8c92e8f5e625f8a3e861ef79200199b54df,
  tree da127f8d53fd615f3df18e01a5cbff8c2232738c.
- Behavioral source: 820041ba4985b5fb8d479f7679281667f32a6845,
  tree 0e597d5485faba7413d366bf2521a991921981c0. The complete world suite
  passed 623 tests, with zero failures and three existing manual/soak ignores.
- Final source: 82d8ac4b096f65469a4a80ac578e75285c205942,
  tree 02516c654c4efce2cb69d1d4bbbdd5c51b54754f. Only the included two-line
  mutable-borrow lint repair follows the behavioral run. All 15 focused controls
  were rerun successfully; strict world all-target Clippy and workspace formatting
  passed. Do not call this a second complete 623-test run.

Controls include byte/digest-identical quiet execution, quiet terrain slope,
initial priority, early/mid/late contact, very slow motion, diagonal and vertical
free-space movement, opposed normals, carried ownership, blocked/water movement,
thermal contact during a trajectory, one biology interval, atomic rollback, and
JSON restore without a persistent reflex latch. The package preserves the original
compile, expectation, spatial-metric and lint failures, including their fixes.
Fresh expressed effector/proprioceptive calibration is not acquired learning.

## Credit boundary and next experiment

The public API returns a separate physical interval receipt. It must not be fed
silently into the ordinary on-policy learning path as though all selected actions
executed. CPU intervention integration is a separate candidate: overridden intent
must not receive a fictitious actor update, family-memory event or sleep-replay
example. Dispatch authority, checkpoint continuation and unchanged optimizer/history
need their own native tests. Late internal/action-caused injury is also outside
this external-contact component. No autonomous creature, GPU kernel, device or
learned-policy evaluation is claimed here.

## Astra handoff

Keep the original research reference:
https://rikiyat.github.io/blog/weight-space-programming/
Experiment 4 motivates targeted repair, experiment 5 whole small-network design,
and experiment 6 iterative feedback. Our world/credit repairs supply a more honest
evaluation substrate; these fixture results are not replication of that paper or
evidence of broad biological competence. Preserve initial candidates, failures,
source identities and controls before changing neural weights again.

All original package files and their manifest are preserved byte-for-byte. The
source copies, finite test logs and patches contain no executable, generated save,
checkpoint, memory bank or learned array. This publication adds only this note
and narrowly scoped artifact attributes around the original evidence.
