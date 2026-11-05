# Detached holding and candidate-memory intervention

The native holding sensor **causally reaches the speech gates**, but its current
effect does not suppress the unsupported declaration in either of these two
captured contexts. Removing current candidate-memory decoder inputs changes an
action logit and leaves speech exactly unchanged. This is a neural-path diagnostic,
not improved language, new behavior, or evidence that a memory architecture must
change.

[Selected readouts and preservation receipts](retained-language-detached-speech-inputs-results.json)
include all 32 speech-source features, all six raw speech steps, presence/stop
gates, conditional payloads and action logits for every condition. No full actor,
optimizer arrays, canonical checkpoint or detached full neural input is published.

## Source and fixed budget

Measured harness source: `4a2d4d3835e99592524fc07be39b1adede3d0cde`, on
`experimental/within-seed-gate-validation-20261105`.

- Original V26 **Untouched** actor, optimizer step **436**, complete checkpoint
  identity `a64296c15537030f3dd7fbe967472947bd1afb3da42e1a9103bd4217dc9ec58f`.
- Existing V28 native rows **10 and 14**, both seed **19508423**, tick **459**,
  heard tokens `[9,1]` (get food), offset 1, both speech cooldowns ready.
- Exactly **8** full neural forwards: both baselines first, then holding toggle,
  memory zero, and both for each row. **0** new world decisions, gradients,
  native/shadow optimizer updates, fits or external model/Jules calls.
- Test passed in **6.38 s**; diagnostic body, including reconstruction and
  preservation checks, took **4.103 s**. This is not neural-forward-only timing.
- All **430** preregistered prior files remain byte-identical. Full actor weights,
  Adam moments/ages, mask/configuration, private starting state, frames and sealed
  outcome patches are preserved. No policy branch/rebase or admission exception.

Row 10's original native action in the previous tick acquired the food. It is
still owned before and after this captured report; the report has witnessed recent
support. Row 14's original prefix acquired nothing and has no matching event;
its identical declaration is unsupported in that observed prefix. Neither row
consumes food at the report decision. These are preserved recorded physical
outcomes, not counterfactual execution results.

## Exact reconstruction before intervention

The test restores both original hosts through ordinary checkpoint admission and
compares the complete recovered checkpoint identities. It rebuilds receptor
frames/effects from the actual organism biochemistry, restores the original
memory/topology/predictor/hysteresis, runs canonical preparation, and requires
the finalized perception frame to equal the original frame. The recall receipt
must validate against that frame. Native encoder inputs match the captured vector
bit for bit; candidate inputs, initial neural state and effective private weight
offsets also match. Sparse structural context is verified empty in these captures.

Both baseline forwards then reproduce final activations, activity EMA, metabolic
load, six-step raw speech values, payload including confidence, and the sampled
joint action/log probability exactly. The original raw action-logit arrays were
not captured, so those arrays are reconstructed rather than compared with a
previous array. No intervention runs before both baselines pass.

The native encoder default is four microsteps; the captured native ATP/activity
budget selected **three**. The replay retains the actual captured three, routes,
receptor gains and private offsets. No default substitution is used.

Holding override uses a detached copy of the raw sensory flat lane **34**, toggled
between its recorded 0/1 values. The harness reconstructs all encoder assignments,
attention/interoceptive gains, per-assignment clamps, accumulation order, regional
gain and final clamp in native f32 arithmetic. The no-override reconstruction
first matches the native encoder exactly for both rows. Only then is the override
used in a detached replay vector. No changed sensory frame is created/admitted,
and no recall receipt is reused as evidence for a counterfactual world.

| Source row | Raw holding | Encoder targets 68 / 155 / 215, each | Encoder target 1858 |
|---|---:|---:|---:|
| 10, supported | 1 → 0 | +0.677855 → −0.017283 | −0.321407 → −0.712422 |
| 14, unsupported | 0 → 1 | −0.017283 → +0.698156 | −0.732723 → −0.330288 |

These are the only changed encoder targets. All other sensory/word lanes,
homeostasis, initial neural state, effective weights and original outcomes remain
fixed **within each row**. The two original rows have different intervening
history and physiology: regional gains are 0.695139 and 0.715439. They are not
otherwise-identical cross-row states. The intervention may conflict with their
unchanged physical observations; it is an input ablation, not an admitted event.

Memory zero changes only confidence-weighted candidate decoder slots **24..35**
after canonical preparation. Target/family memory is removed for every candidate;
canonical attention, other cognitive projection slots, candidate identities and
innate biases remain fixed. There are 53 nonzero lanes removed in row 10 and 47
in row 14. This does not erase memory from history or remove its indirect effects.

## Gate, payload and action results

First speech-step raw presence and stop thresholds are both zero. Every condition
decodes the same **Declare `[9,1]`**, continuing at step 0 and stopping after step
1. Holding changes confidence; memory alone leaves the complete payload exact.
These payloads are conditional decoder results, not newly emitted utterances.

| Row | Detached condition | First presence | First stop | Payload confidence |
|---|---|---:|---:|---:|
| 10 | baseline | +1.515378 | −1.059136 | 0.768627 |
| 10 | holding 1 → 0 | +1.454363 | −0.984553 | 0.788235 |
| 10 | memory zero | +1.515378 | −1.059136 | 0.768627 |
| 10 | both | +1.454363 | −0.984553 | 0.788235 |
| 14 | baseline | +1.395311 | −0.975940 | 0.784314 |
| 14 | holding 0 → 1 | +1.458108 | −1.052701 | 0.764706 |
| 14 | memory zero | +1.395311 | −0.975940 | 0.784314 |
| 14 | both | +1.458108 | −1.052701 | 0.764706 |

Holding changes **17/32** speech-source features (L2 **0.430108 / 0.442669**).
Neuron 1858 changes **−0.391016 / +0.402435**. First presence changes
**−0.0610155 / +0.0627973** and first stop **+0.0745835 / −0.0767613**.
Every executed gate remains on its original side of zero. The whole recurrent
holding path is therefore active and measurable, while current categorical
reporting remains insensitive to this intervention. This does not isolate cue
effects or explain why the original gate acquired these weights.

Memory zero changes exactly one candidate logit per row: the Eat candidate for
the food. Its delta is **−0.0422374** in the supported row and **+8.34465e−7** in
the unsupported row. The latter is a very small native f32 readout effect.
All speech-source features, all six speech output vectors, and the complete
conditional speech payload remain **bit-identical** to baseline. With both
interventions, speech is bit-identical to holding alone.

Holding also changes three Eat candidate logits (maximum absolute deltas
**0.0465441 / 0.0478873**). No counterfactual action is sampled/executed and no
new physical outcome, reward, safety, nourishment or sleep-retention score is
invented. Original physical evidence is only a fixed annotation of each replay.

## Finding and remaining uncertainty

This rules out a wholly disconnected holding-to-speech path in these native
states. It also establishes that useful current candidate-memory readout is
present for the food action, while this particular memory-input ablation has no
current speech path. That matches the native implementation: candidate memory
enters the action decoder after recurrent activations are computed; speech reads
those activations directly.

It does **not** establish that speech cannot use recurrent memory, that holding
alone identifies the requested object or a recent event, that an objective defect
caused the premature report, or that new routing is necessary. The same two
states from one seed cannot resolve generalization, verb/noun understanding,
later history, or preservation of the real positive Eat reports. The original
failed offline separator is never applied. All previous negatives remain intact.

The single proposed next action is a **read-only constrained margin feasibility
check** using existing genuine-positive and witnessed-premature tapes. Restrict
the proposed presence-gate change to the causally measured holding-response
direction, keep word/act/stop weights fixed, and check whether any scalar change
suppresses witnessed premature declarations while preserving every genuine
positive report (including consumption reports without current holding). Keep
seed-separated results and expose any conflicting inequalities. This is a small
offline calculation, with zero world decisions, full neural forwards or Adam
updates; no hypothetical weights would be installed. It tests whether a narrow
holding-directed correction is even consistent with the retained positives before
proposing further training. **It has not been started.**

## Reproduction and source validation

The explicit ignored test lives in
[`detached_speech_inputs.rs`](../../crates/alife_game_app/src/foundation_training/retained_language/curriculum/factorial/balanced/detached_speech_inputs.rs).
It reserves every forward before calling the native evaluator and refuses call
9. A new output directory is mandatory; a failed/finished directory cannot be
silently reused to repeat spent forwards. Its local measured executable SHA256 is
`2914ef2715488c6ba64fa67189a7bb23d8124ea97eb28ebefddcdcc57868ac4b`.

Local evidence remains in `language-detached-encoder-v30-measured` and
`language-detached-encoder-v30-run` under `/workspace`: preregistration/protected
hash map, measured source/binary manifest, build/run logs, per-call reservation
and native output files, reconstruction proofs, exact-baseline receipt and
results. The saved actors and complete optimizer histories remain in their
original local directories.

Native test: **1 passed**, 644 other tests filtered out. Strict app CPU Clippy
(`--lib --tests -- -D warnings`), workspace format and diff checks pass. Early
compile-only macro/type errors were corrected before the successful build and
before any neural forward. No broad simulations/tests were run.

The portable
[`summarize_detached_speech_inputs.py`](../../scripts/summarize_detached_speech_inputs.py)
verifies the eight reservations, baseline receipt, binary/preregistration and all
430 protected hashes, then extracts the public readouts using only saved JSON.
It performs no native call, fitting, actor construction or optimization.
The existing experimental branch is used without a PR/model-triggering workflow.
No hosted CI, substantive review, learned capability or promotion is claimed.
