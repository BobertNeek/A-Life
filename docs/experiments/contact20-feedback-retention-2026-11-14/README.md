# Contact20: one queued feedback pulse and retained fast-weight influence

Native CPU pilot seed **2026111900** (8 later intervals), followed by fresh
replication seed **2026111901** (32). All five arms shared one real pre-feedback
state in each cohort. Praise, Scold and Slap left a measurable fast-weight
contribution to later activity and logits after the queue was empty. Pet had no
loneliness to relieve and produced only very small effects in this condition.
**No ablation changed a sampled action. No useful feedback meaning or A-specific
learning was established.**

In the replication every arm consumed the same food at outcome tick **15**.
Actual representative and factorized motor receipts matched None throughout.
The same-state intervention changed the fixed actual action's log probability,
including at tick 36, but did not change the draw. This is a valid null result
for changed choice and improved physical behavior in this bounded assay.

## Parent and native procedure

Source base: `d6758f848fd1e3563712fc2bba4a0dd9db503484` (current main when run).
The genetic parent is merged Contact20 from PR #346, with the **full exported
count2 genome**, independent Locomotion + Manipulation. No founder weight,
biology, gain, optimizer, neural state or action was edited. Terminal context
#345 stayed **Off**. No PPO or evolution ran.

- Asset SHA256: `d83a995e802e41b8e17799b0871f75d9ede35f95850049777323112e636654d1`.
- Genome file SHA256: `557da394c045d0b4353009c842825677aa7505f636d6252b20f67bfe0c3a5189`.
- Executed harness SHA256: `e2a5a7b3e2cce132eefa3266909cc9c4a5242030c7e623228523215907b2ab64`.
- Executed binary SHA256: `f7b78826cf6e3676f25e4baba7c50fbcab69c8c7a2161e18cf6df4a4ecaf8862`.

The exported genome is expressed and compiled directly; its embedded asset and
every compiled genetic weight are checked. A fresh canonical world has one
creature at the origin and food at `(2.5, 0, ±0.5)`, mirrored by seed. Four ordinary
`host.step` calls precede the genuine A decision at tick **4**. Cloning the entire
actual state after A creates None/Praise/Scold/Pet/Slap experimental controls.
Each non-None arm calls the native feedback queue **once**, at tick **5**, from
the creature's actual current position.

B selects at tick **5**, before delivery. Its sealed physical outcome and biology
are at tick **6**; the queued item is then absent and remains absent. The queue
contains no identity of A. B's learning contains the native decayed prior trace
and B-local eligibility. The published results therefore make no attribution to
the intended A action. The POST neural checkpoint's eligibility is the actual
staged eligibility: native `apply_credit` copies that staged vector unchanged,
while fast weights receive the sealed credit. Full rows and PRE/POST checkpoints
are retained locally.

Actual A-time hunger, loneliness and pain were **0**, energy **0.791806**, health
**1.0**, ATP **0.75**, in both cohorts. These are newborn short-life trials with
unchanged parent biology, not hungry/lonely or mature-life generalization tests.
Pet therefore had no extra scalar reward from relief of loneliness.

B scalar reward, identical across the two seeds:

| None | Praise | Scold | Pet | Slap |
| ---: | ---: | ---: | ---: | ---: |
| −0.00406514 | +0.04458351 | −0.00869834 | −0.00406514 | −0.00839021 |

These are consequences projected by native inherited biology, not injected
teaching labels or raw reward replay.

## Same-state intervention

At each later interval, use the **actual arm's** collected `TrainingSequence`.
The public `evaluate_replay` reproduces its activity exactly and native joint
sampling reproduces the complete recorded action receipt exactly. Then clone
that detached sequence and change only each effective offset by:

```
offset_copy[i] = offset_actual[i]
               - alpha[i] * (fast_arm_PRE[i] - fast_None_PRE_same_tick[i])
```

The actual arm's lifetime weights remain present. Initial activity, metabolism,
EMA, dendrites, encoded sensory input, current receptor gains/thresholds,
projection gates, microsteps, candidate list and its memory/cognitive inputs,
structural edges, founder weights and random counter remain fixed. Eligibility
is neither replaced nor replayed as reward. The diagnostic copy never enters a
host. Every other sequence field is checked equal in the independent Python
checker; the fast-only subtraction is reconstructed from the saved native CPs
in float32 and checked exactly.

The comparison estimates the retained **fast-weight contribution associated
with the feedback history**. Feedback also changes physiology and subsequent
recurrent experience/learning; the fast difference is not a pure direct effect
of one synaptic update. Even where action receipts match, biology and internal
experience differ. Same-state ablation isolates the contribution of those fast
offsets at the selected boundary while holding current physiology fixed. It
does not erase all historical consequences or test a new autonomous trajectory.

## Measured later effects

At the last pilot decision, tick **12**, max absolute logit changes from the
fast-only ablation were Praise **0.0321999**, Scold **0.00457382**, Pet
**0.00000190735**, Slap **0.00290298**. None was exactly zero. Sampled choices did
not change.

At the last replication decision, tick **36** (31 intervals after queueing,
30 after delivery), the matched PRE checkpoints have previous decision tick
**35** in both arm and None:

| Arm | Fast offset component L2 | Max activity change | Max logit change | Δ log P(actual joint action) |
| --- | ---: | ---: | ---: | ---: |
| None | 0 | 0 | 0 | 0 |
| Praise | 0.00705015 | 0.0000319183 | 0.0000568931 | −0.00000952527 |
| Scold | 0.00105771 | 0.00000569224 | 0.000362156 | −0.000112002 |
| Pet | 0.00000110176 | 0.0000000894070 | 0.0000000246800 | +0.0000000189350 |
| Slap | 0.000578538 | 0.00000101328 | 0.0000881697 | −0.0000196184 |

Δ log P is actual minus ablated for the **same fixed actual action**, evaluated
in float64 from native float32 logits and the actual representative/motor masks,
including forced-channel arbitration. It is not a new draw's probability. Early
Praise increased the selected action's likelihood slightly; its later sign
reversed. The sign is not evidence of knowing what the human meant. Pet's
activity effects are near float precision and do not establish a useful
no-need learning response.

The pilot had no meal before its tick-13 endpoint. The longer fresh replication
had one meal in all arms, no action-receipt differences from None, no sleep or
consolidation, and **3 microsteps** per decision throughout. Native neural work
was identical across arms: **409,600 synapse ops** per pilot arm and
**1,638,400** per replication arm. Final replication energy was **0.891760** in
None/Praise/Scold/Pet and **0.891369** in Slap. Health remained **1.0** except Slap
(**0.994987**, injury **0.00501295**). These direct physical effects of Slap are
not learned useful behavior.

## Controls and independent verification

For all **200** post-A native decisions:

- An unobserved native twin received the same one queued input and ordinary
  host steps. Full checkpoint bytes matched the observed arm at every boundary.
- Running diagnostics left every byte of the actual completed host unchanged.
- Detached native replay exactly reproduced activations, EMA, metabolic state
  and the joint action receipt; None ablation was exactly zero.
- Full founder/Adam checkpoint bytes were identical at beginning and end.
  Native eligibility, both activation banks, all fast/lifetime coordinates,
  topology, memories and optimizer moments/ages were retained locally.
- All ten arm endpoints and both post-A branch points round-tripped through
  typed native cold restore with identical checkpoint bytes.

Independent Torch **2.15.0+cpu**, one CPU thread, float32, replayed native and
ablated tapes at pilot intervals **5, 7** and replication intervals **1, 7, 15,
31** (30 comparisons, each with native and ablated forward evaluation). Maximum
logit/activity discrepancy from native was **9.53674e−7** in the pilot and
**1.19209e−7** in replication, below the declared **2e−5** tolerance. The checker
uses native canonical edge order per target/family, current gates, live
structural/dendritic evidence, metabolism and decoder semantics; it does not
train. Native f32 comparisons, rather than the tolerance alone, establish the
reported small effects. The results JSON includes exact checkpoint/row hashes
and safe scalar aggregates.

## Reproduction and evidence

The native example and Python checker are opt-in diagnostics. They add no live
runtime behavior. The measured source and executable are archived locally;
publication adds formatting, additional equality/provenance assertions and panic
retention. Source/binary hashes above identify the actual measured version.

```sh
ALIFE_ASSAY_GIT_COMMIT=$(git rev-parse HEAD) cargo build --locked \
  -p alife_training --features cpu-backend --example feedback_retention
asset=docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation
# Choose a new unique ID. Existing evidence directories reject, never overwrite.
target/debug/examples/feedback_retention "$asset" "$asset.genome.json" \
  /workspace/feedback-retention-LOCAL UNIQUE-PILOT 2026111900 8
target/debug/examples/feedback_retention "$asset" "$asset.genome.json" \
  /workspace/feedback-retention-LOCAL UNIQUE-REPLICATION 2026111901 32
python scripts/check_feedback_retention_torch.py \
  /workspace/feedback-retention-LOCAL/UNIQUE-REPLICATION --intervals 1,7,15,31
```

Set `CARGO_TARGET_DIR`-appropriate binary paths when using a custom build target.
The cloud task used the existing `cloud-cpu-validation/env.sh` configuration.
Local raw evidence lives outside the repository at
`/workspace/feedback-retention-LOCAL/{pilot-2026111900,replicate-2026111901}`,
protected by create-only writes and ignore files. It contains acquired CPs,
full replay rows, graph arrays, founder/Adam state, native sensory and eligibility
evidence and the original executed source/binary. **Do not upload those files.**
Only source and this safe aggregate report belong in the source PR. No hosted
Actions pass is claimed; its allocation was already exhausted. No champion
promotion or merge is part of this assay.

The publication harness was also run as an explicitly labelled deterministic
prefix check of seed 2026111900 (2 intervals, **not** fresh replication). All **34**
compared CP, row and founder/Adam files matched the original pilot byte for byte.
Its source SHA256 is
`aa72df0c0f154f5d42c7c1d7887ecc133bb654f213aae9702f27b48805d179ef`;
its binary SHA256 is
`cb404d8fab7327c3be57e8a7305fd18139a635fa4ebf6fd5a6bfe82042219da6`.

Additional local checks passed: the example's four create-only/error-retention
tests, canonical native restore with observed-object identity, zero-microstep
eligibility/both-bank preservation, signed-credit waking normalization, Rust
format checking, Python byte-compilation and `git diff --check`. These local
checks do not stand in for unavailable hosted CI or a GPU test.
