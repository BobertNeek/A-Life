# Holding-direction presence correction is infeasible

**Reject the one-scalar correction.** No presence-weight change along the measured
holding-response direction can suppress these witnessed premature declarations
while retaining every captured genuine positive. The contradiction already holds
when only the first word is suppressed. It also holds within the matched positive
and negative pair from the same seed, independently of the earlier seed-confounded
sample. This rejects the proposed direction, not every possible gate readout.

[Numerical receipts](retained-language-holding-margin-results.json) contain the
direction, all 18 decoder-step inputs/constraints, seed-separated intervals and
conflicting inequalities. The calculation uses existing V26–V30 artifacts only:
**zero new neural forwards, world decisions, gradients, optimizer/shadow updates,
model/Jules calls or installed hypothetical weights**. All **463** protected prior
files, including original actors, full Adam histories and V30 evidence, remain
byte-identical. No engine, schema, policy, admission guard or brain is changed.

## Direction and permitted hypothetical change

Let `x10b/x10t` and `x14b/x14t` be the 32 first-step speech-source features from
the original V30 baseline and holding-toggle forwards. Row 10 toggles holding
1→0 and row 14 toggles 0→1. Before consulting factual labels, define:

```
u = ((x10b - x10t) + (x14t - x14b)) / 2
d = u / ||u||2
w_presence(lambda) = w_original_presence + lambda * d
```

No label, fitted intercept, per-seed scaling or selected feature is used to choose
`d`. Both actual signed responses are nearly collinear (cosine
**0.9999999999986**); their norms are 0.430108 and 0.442669. The unit direction has
17 nonzero lanes, including **+0.909110** on speech-source neuron 1858. It is a
direction in mixed neural-feature space, not a clean holding Boolean.

For a captured speech-step input `x`, the hypothetical raw presence is:

```
p(lambda) = captured_native_p + lambda * dot(x, d)
```

Word, act, stop and recurrent-control weights remain fixed, as do the saved private
offsets. Presence output 16 does not feed recurrent control outputs 18..31, so
later speech-step features remain unchanged under this hypothetical gate-only
change. They are read from existing captures/previously validated feature tapes;
where needed, later inputs reuse recorded controls through native `tanhf` scalar
arithmetic. No dense decoder or neural forward is performed and no actor is
constructed.

Supported reports require positive presence at **both actual token steps**.
Premature reports require nonpositive presence at every nonzero-token step through
the unchanged stop gate. Merely silencing their first token could leave the noun
spoken at step 1. Margin checks also use supported presence ≥m and unsupported
presence ≤−m for m=0.01 and 0.1.

## Actual sample and missing evidence

There are **nine labeled native tapes from four seed units**:

- V26: three genuine emitted EatFood/GetFood/GetToy reports from seed 10309219,
  plus witnessed unsupported EatFood/GetFood declarations from seed 16309347.
  EatFood is a real consumption report with **holding=0**, emitted after the
  ingestion sensation has expired.
- V28: two witnessed premature GetFood declarations from seed 19310729, and
  the matched supported/unsupported GetFood pair from seed 19508423. The latter
  shares the heard words and cue/cooldown timing while genuine pickup differs.

Thus four supported and five unsupported reports produce 18 reachable token-step
constraints. Unlabeled V28 rows remain abstentions, not absence examples.

Another **eight genuine positive utterance rows** in the retained V26 Untouched
pre/post-sleep evaluation omit their neural features. Their original physical and
utterance traces remain valid, but this calculation cannot verify their gate
margins. They are listed as unknown, never recaptured or silently treated as
retained. Already the available positive constraints make the correction
impossible; missing positives cannot rescue it. This is not a test of all future
states or independent generalization.

## Conflicting constraints

At zero margin, preserving the actual consumption report at seed 10309219 gives:

```
0.8122272491 + 3.1946816154 * lambda > 0
lambda > -0.2542435669
```

Silencing the **first token alone** of the premature report at seed 19310729 gives:

```
1.0682656765 + 2.2787277525 * lambda <= 0
lambda <= -0.4687991689
```

The intervals do not overlap. At the premature-report boundary, the retained
consumption presence would be **−0.685437**. At the consumption-preservation
boundary, the premature presence remains **+0.488914**. This is not a near-zero
rounding ambiguity.

Requiring full silence, with stop unchanged, gives these strongest bounds:

| Seed | Captured labels | Positive-retention lower bound | Premature-silence upper bound | Compatible? |
|---|---|---:|---:|---|
| 10309219 | 3 supported | λ > −0.254244 | — | Yes alone |
| 16309347 | 2 premature | — | λ ≤ −1.813897 | Yes alone |
| 19310729 | 2 premature | — | λ ≤ −1.666419 | Yes alone |
| 19508423 | 1 supported, 1 premature | λ > −0.419067 | λ ≤ −1.733948 | **No** |

The same-seed pair also conflicts under the weaker **first-token-only** rule:
λ>−0.419067 versus λ≤−0.449149. Therefore neither seed confounding nor the frozen
stop's later noun explains away this narrow infeasibility.

Global full-silence sensitivity:

| Required raw margin | Positive lower bound | Negative upper bound | Feasible? |
|---:|---:|---:|---|
| 0 | >−0.254244 | ≤−1.813897 | No |
| 0.01 | ≥−0.251113 | ≤−1.828164 | No |
| 0.1 | ≥−0.222942 | ≤−1.956573 | No |

An independent one-variable HiGHS feasibility check agrees at all three margins,
both on all constraints and on the matched seed alone. Positive-retention-only
constraints are feasible as a sanity control. These are offline real-arithmetic
inequalities with native f32 intercepts, not a quantized hypothetical actor or
observed creature behavior.

## One practical next step: change the factual curriculum

The smallest supported improvement is a **single bounded native gate-training
comparison using the expanded, within-seed factual corpus**, retaining all
consumption/ownership positive anchors. Do not add a holding-only rule or new
memory route on this evidence.

The evidence warrants this choice because:

- V30 establishes an active bodily-evidence-to-speech path.
- The previous five-example gate curriculum put every positive on one seed and
  every negative on another. That is a measured coverage/confounding limitation.
- V28 now provides actual supported and unsupported reports under the same seed,
  heard command, decision offset and cooldown. This supplies a factual comparison
  absent from that earlier training corpus.
- Current native absence supervision already trains both **presence and stop**;
  positive supervision retains both token steps. There is no observed source
  defect justifying a decoder/credit fix. The gate-training implementation also
  checked its direct gate derivatives against native BPTT. This calculation adds
  no new gradient measurement.

This is a curriculum proposal, **not an assertion that all 64 existing gate
coordinates can solve the expanded sample**, and not evidence that four updates
will produce useful generalization. No further numerical probe, architecture
migration or training is started automatically.

### Bounded, falsifiable validation plan — not executed

1. Start a disposable copy of original V26 Untouched at step 436, preserving full
   weights, moments, coordinate ages/configuration and private source states.
   Take **exactly four new native Adam updates**, training only the existing
   64 presence/stop coordinates. Use all nine genuine labeled captures, equal
   total positive/absence mass, and the existing positive gate-normalization
   correction. Keep every real positive, especially the no-holding consumption
   example. No invented event/reward, changed action/word/act/control weights,
   optimizer reset, larger learning rate or automatic budget extension.
2. Reuse the **already saved** original Balanced4 and ZeroGradient4 actors as
   same-start/same-update-count controls, plus original Untouched. Do not retrain
   them. Balanced4 isolates the original five-example curriculum; ZeroGradient4
   measures existing Adam momentum/decay. Identify all four actual arms clearly.
   Verify the new arm has real, target-specific gradients/updates, freeze every
   unrelated coordinate including moments/ages, and preserve all originals.
3. Use the existing verified disposable gate-policy branch operation to bind each
   intervention to identical canonical starts. Keep normal admission intact and
   disclose that a new gate policy is an experimental branch, not exact policy
   continuation. Require exact cold recovery including private/RNG/optimizer
   state before scoring. A provenance or recovery failure aborts the run.
4. Evaluate **two fresh, predeclared seeds**, checked against prior seed receipts,
   excluding all four analysis/training seeds. On each, run EatFood, GetFood,
   GetToy and silence for **eight scored decisions**, with identical physical
   fixtures/player-cue timing across arms. No teacher/prior or forced actions.
   Repeat after genuine native RecoveryProtocol sleep/consolidation, keeping
   before/after paired within each seed rather than counting independent seeds.
5. Add the disclosed original positive seed 10309219's three command episodes as
   a **seen retention control**, separate from held-out results. The total cap is
   **704 scored decisions**: 512 held-out and 192 seen-retention, across four arms
   and two sleep phases. Preparation is capped at 600 native world ticks per seed;
   sleep at 600 per seed/arm, with all extra awake decisions explicitly counted.
   Abort a failed preparation or sleep transition rather than extending the run.
   Untouched must reproduce the known genuine meal and report control; a failed
   positive control makes the comparison uninterpretable, not successful.
6. Score actual timely correct declarations, premature/partial declarations,
   total speech and silence separately. Also score actual nourishment, ownership,
   noun/verb physical distinctions, injury/survival and pre/post-sleep differences.
   The correction is rejected if it merely increases silence, loses the real
   consumption report or nourishment, or fails to improve unsupported reporting
   on both held-out seed units against the trained curriculum and momentum
   controls. Publish negative results and stop at the fixed budget.

Only **four new optimizer updates** are proposed; the three saved controls retain
their complete histories. The behavioral budget is explicit and finite. A loss
drop, fitted margin, green source test or effect on the training seed cannot
qualify the candidate for promotion.

## Reproduction and validation

[`check_holding_margin.py`](../../scripts/check_holding_margin.py) reads the
preregistered hashes and captures, constructs the label-independent direction,
and derives scalar inequalities. It never loads a trainer, fits a multi-feature
separator, runs a neural network or writes an actor. The full local evidence is
in `/workspace/language-holding-margin-v31-analysis`: preregistration, original
analysis/error log, corrected/canonical runs, complete conflict list and independent
feasibility receipts. The initial analysis stopped at a JSON presentation-precision
check; restoring the native f32 bits made the exact comparison pass. No source
capture was altered or assertion loosened to approximate equality.

The portable analysis reproduces the public receipt exactly. Python syntax,
independent feasibility/direct-substitution checks and repository diff checks
pass. There is no new Rust change, native test, hosted CI or substantive review.
Source/results use the existing experimental branch without a PR or any route
that starts external models. All earlier negative results remain intact.
