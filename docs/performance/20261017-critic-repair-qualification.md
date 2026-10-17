# Preserved critic repair qualification - 2026-10-17

The sealed Grab learner at policy 5124 had an actor Adam age of 12132 and a
critic Adam age of 10248. Its critic produced very large negative advantages
even on an explicitly credited acquisition. This change adds a bounded offline
critic experiment and an explicit checkpoint admission lane; it does not change
ordinary PPO, biological rewards, action selection, or creature lifetime learning.

## Method and preservation

The WGSL experiment uses frozen **actual production GPU feature vectors**, not a
CPU replay of the network. It minimizes the sampled one-step Bellman residual
`e = V(phi) - reward - gamma(actual_dt) * V(phi_next)`, including the gradient
through both value predictions. Genuine terminals use gamma zero; rollout
truncations retain the real next-state bootstrap. The bias gradient is
`e * (1 - gamma)`. Existing critic weights and both Adam moments continue.

Sixteen historical cohorts were split into eight whole-cohort fit and eight
held-out cohorts. These data are historical and off policy. A fixed 5000-step
fit was selected using the fit split before opening the fresh validation seeds;
a shorter 500-step trial was insufficient. The temporary fit learning rate was
0.003. Ordinary PPO settings were retained for the clone pilot and continuation.

Only `value-checkpoint.json` changes, with an added `critic-correction.json`
receipt. Policy, actor weights and full actor Adam state, acquired creature
state, and the original `cycle.json` remain byte-identical. The critic age is
15248 after 5000 separately receipted updates. The original cycle receipt still
records its original age, 10248; it is never rewritten as fake PPO work.
See [package admission](20261017-critic-correction-admission.md).

## Executed qualification

| Check | Observed result |
| --- | --- |
| Historical residual MSE, fit | 24644.23 to 0.00551646 |
| Historical residual MSE, held-out | 26029.80 to 0.07911624 |
| Fresh 128-decision seeds 202610179101 / 202610179102 | MSE 3103.221 / 3103.081 to 0.372153 / 0.068679 |
| Analytic GPU Adam fixture with nonzero existing moments | Maximum absolute error 1.49e-8; cold restore exact |
| Matched original and corrected policy-5124 Eating | Both one meal on decision 8; complete behavior traces identical |
| Ordinary disposable PPO pilot | 16 cycles, 16 trained plus one bootstrap row per cycle, two epochs; actor and critic each advance 32 steps |
| Eating after pilot cycles 1, 4, 16 | One meal on decision 8 in every check |
| Fixed-cache residual MSE after pilot cycle 16 | 0.590241 / 0.292686; predeclared limits 1.488612 / 1.0 |
| Independent saved-scalar GAE arithmetic | Maximum absolute error below 2e-7 |

Eating always requests **16 decisions**, with seed 202610169901, founder
539363617, and temperature 1. Evaluation stops early on actual consumption.
An earlier gate incorrectly treated six executed decisions in a successful old
receipt as the requested budget. Both policy-5124 variants failed that shortened
assay. That gate was superseded by the genuinely matched 16-decision comparison.
The successful policy-5124 traces are byte-identical, SHA-256
`5032658aea562ce85c7e57c3435afbf62dce6b3689355851459f939316cb414d`.

The successful pilot uses isolated seeds 202610179241 through 202610179256 at
collection temperature 32. Its policy 5140 / actor 12164 / critic 15280 endpoint
is disposable qualification output. None of its cycles, updates, or seeds is
credited to production. The continuation instead starts from the preserved
policy-5124 actor and corrected critic, using production seed 202610171059.

## Provenance and bounded claims

The qualified training and inspection release binaries were built from clean
source commit `a19dc8e761a884af7ab62e824e83f1f05bde57b8`, tree
`04d567d1fcbcacfc899790122fa461a87b375828`, with optimization level 3 and
debug assertions disabled. Training binary SHA-256 is
`dc50a5516bd576864a7a4ecdb20656b635ce6a69b13cbe51cf515e791ffa4f5b`;
inspector SHA-256 is
`63d930b56ba114eafe56f090fdd13c82d5588f6f1485643681fe3eaba273e049`.
The earlier residual experiment has its own frozen source snapshot and binary
identities; it must not be described as a build of a later PR documentation head.

[The compact evidence index](20261017-critic-repair-evidence.json) records exact
local receipt digests and source identities. Raw captures, checkpoints, process
receipts, and rollback copies remain in the local experiment archive. The index
does not replace those inputs or claim that CI reproduced the GPU campaign.
The eight native correction-admission fixtures and existing Python night-runner
tests are committed with the implementation; hosted CI runs the admission suite
explicitly. Local qualification is already complete and is not rerun to publish.
The publication follow-up wires `--checkpoint-inspector` through the general
night CLI's planning and first-cycle validation, retaining its hash pin after
each seal. Three CPU fixtures cover this path and preserve the separate critic
offset. The already-qualified dated controller supplies its inspector explicitly.
Stable-Clippy follow-ups use fixed-width array chunks, box the internal cycle
result receipt, and initialize the temporary Adam configuration directly. None
changes the qualified live binary or asserts a new GPU qualification.

The inherited production probe at cumulative cycle 2649 retained Eating (one
meal) and recorded zero Grab acquisitions. This is fixed-seed retention evidence,
not Grab competence or a curriculum promotion. Both original rollback snapshots
remain byte-identical. A dated continuation controller validates receipt-bound
prepared scalar values, rewards, bootstrap identity and finiteness after each
real seal, preserving accepted learning before a hold. It rejects predictions
outside objective support `[-1731.7340972020208, 1731.9840972020208]`. Negative
GAE is allowed; residual MSE is observational rather than an invented stop rule.

Values drifted upward during the bounded pilot and continuation. This is not
proof of absolute continuing-return calibration or indefinite PPO stability.
No terminal Monte Carlo anchor, automatic promotion, CPU neural fallback, or
new reward channel is introduced. The live learner stays pinned to its qualified
binary while PR CI and review run. Collection, native work and hard cleanup
cutoffs remain 14:34, 14:55 and 15:00 UTC respectively on 2026-10-17.
