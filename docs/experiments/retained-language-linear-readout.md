# Disposable linear word-readout experiment

This materially changes representation/routing after the failed objectives in
`retained-language-paired-objective.md`. It is a CPU-only public-founder experiment;
all older actors, source states, negative results, traces and measured binaries
remain intact. Nothing is promoted into an accepted or default brain.

## Native route and identity

The explicit compiler profile `LinearWordReadoutV3` uses the existing two raw
8-bit token vectors and native action/speech decoder coordinates. Word assignment
gain is 4 instead of 0.5, retaining the encoder's existing ±1 clamp and biological
receptor modulation. The 160 ActionPlanning source cells use native Identity
activation, leak 1, zero bias and zero homeostatic suppression, instead of the
old leaky tanh source dynamics. All other neuron dynamics, topology, candidate
features, decoder semantics, body, ATP accounting, world legality, action sampling
and six-step speech decoder remain native. Recurrent inputs still reach those
cells; this is a stronger linear route, not a separate semantic classifier.
No word meanings, desired actions, targets or entity IDs enter the encoder.

The profile has a new compiler identity. A guarded fork accepts only this pilot's
exported public V2 founder with no live-policy provenance. It copies all weights,
both Adam moments, integer ages, global optimizer step and configuration exactly.
Source private snapshots are rejected under V3. Fresh V3 private state is created
in the real host; the source actor remains unchanged. Old profiles and default
serialized identities retain their previous values.

Training restricts the native mask to action and speech decoder coordinates.
Recurrent and other frozen weights, moments and ages must remain bit-identical
at every update. Decoder optimizer history is retained, not reset. Native
one-step replay uses each actual row's neural boundary, encoded inputs, receptor
context, routes, private offsets and sealed world outcome. Since recurrent
parameters are frozen, this detached readout gradient needs no artificial neural
trajectory or text-only substitute. Adam remains the existing native optimizer.

## Protocol

All three arms fork the same retained V8 grounded actor at step 256. There are
64 updates (two rounds of 32), with native collection before each round. Two
training layouts reverse the food/toy positions at ±0.58 bearings, with distances
0.36/0.41 then 0.44/0.49. Each layout forks the same world/private parent for
`eat food`, `get food` and `get toy`, with 24 real decisions per life.

The scripted teacher speaks through the existing ordinary spatial nursery every
two ticks during an eight-tick instruction window. Grounded words match the
requested action; shuffled words use the established cyclic permutations;
no-request receives no teacher utterance but retains natural self-speech and the
same offline intention schedule. Completed requests receive legal Idle targets;
Grab of an already held requested object must not teach a release. Factual speech
labels require a real completed event, at most three eligible quiet rows, and
start after the instruction window in every arm. No event or reward is invented.

The action objective allocates 0.8/0.1/0.1 to instruction/narration/other rows.
Factual payload supervision balances observed activity classes; absent events
receive no fabricated targets. Labels remain offline supervision, never runtime
world knowledge. This is not an isolated text classifier or a forced-action test.

An untreated V3 baseline is evaluated before any new update, so a representation
change alone cannot be mistaken for new learning. Formal assessment uses new
seeds (seed + 290000/290001), bearings ±0.34/∓0.76, distances 0.41/0.44 and
0.73/0.76, visible distractors, ordinary player requests and silence. Contact,
near and induced native RecoveryProtocol sleep contexts each share a private
parent across cues. Sleep is one shared induced ancestor, not an independent or
spontaneous sleep sample. Teachers and semantic priors are absent.

Additional new seeds (seed + 390000/390001) and distances 0.39/0.44 and 0.67/0.72
probe all four verb/noun combinations plus silence for eight native decisions.
`eat toy` is never taught. Its primitive/target attempt is reported separately
from real acquisition, ingestion and nourishment: an attempted Eat can merely
pick up an object, and an inedible object cannot count as successful feeding.

Exact fresh-process recovery repeats the complete assessment trace, private state
and next native Adam update. No evaluation action is forced. The no-request arm
shares previous V8 learning and truthful narration labels, so this is not a
language-naive or shuffled-production-label ablation.

## Reproduce

With this pilot's measured public V8 recovery directory available:

```sh
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --linear-readout /absolute/path/to/V8-run --output NEW_DIRECTORY \
  --seed 847501 --rounds 2 --epochs 32 --decisions 24
```

The baseline, all native rows, optimizer updates, source/binary provenance and
recoveries are retained locally. Source-safe results accompany this note.

## Provider boundary

The teacher is explicitly scripted. The previously observed Luna WebSocket 401
remains a stop: no request, credential modification, endpoint change or bypass
was attempted. No real SLM delivery or model-grounded learning is claimed.

## Measured result: narrow verb learning, full factorial test still negative

Measured source: `7aa59024cc81df401625e4d1c84794b13730288a`.
Measured binary SHA-256:
`1fb02baf3a8ccb75a1017cfeb921024ffae042d12c7be13f0139cdff7a9434d1`.
Native elapsed time is **576.114 s** (576.131 s external). Each arm advances
256→320 with 4,096 native decoder coordinates trainable. There are 2,336 scored
native decisions and 1,104 cold-repeat assessment decisions, excluding preparation,
sleep and private continuation diagnostics.

| Arm | First explicit requested activity / 9 | Eventual requested physical event / 9 | Physical verb pairs / 3 | First physical target noun pairs / 3 | Timely factual utterances |
|---|---:|---:|---:|---:|---:|
| Untouched V3 baseline | 3 | 7 | 0 | 0 | 0 |
| Grounded | 6 | 6 | 2 | 0 | 0 |
| Shuffled | 0 | 3 | 0 | 0 | 0 |
| No teacher request | 4 | 7 | 0 | 2 | 0 |

A physical verb pair requires actual food ingestion for `eat food`, and actual
food acquisition with **no ingestion throughout the same 24-step window** for
`get food`. This counts implicit Eat pickups independently of selected primitive.
Grounded succeeds in the near and post-sleep contexts; those two contexts share
a seed, so this is not two independent seed replications. Neither the untouched
profile baseline nor either matched control succeeds. The first-event classifier
reports three verb pairs, but the contact context still eats under both cues;
that apparent third success is rejected by the ingestion audit. GetFood can also
release and reacquire food later, so continuous carrying is not established.

Grounded first-target noun discrimination remains **0/3**; it chooses food for toy
requests. The no-request control's 2/3 noun pairs outperform grounded, and its
larger eventual-event count illustrates why eventual activity alone does not
establish comprehension. In the extra new-seed fixtures, grounded chooses the
correct first primitive/target for 4/6 seen-command trials, shuffled for 1/6 and
no-request for 2/6. All arms score **0/2** correct primitive/target attempts on the
unseen `eat toy` combination. No novel compositional competence is claimed.

Each trained arm emits 12 declarations. Grounded emits `[9,1]` three times and
`[9,3]` nine times; shuffled always emits `[9,1]`; no-request always emits `[9,2]`.
All have zero timely factual matches. No-request has 12 later matches to the latest
physical activity, without new teacher-request grounding. These are reported as
late descriptions, not timely or cue-sensitive language. Training uses eight ticks
of teaching, while formal player requests stop after the first explicit completed
activity; production timing transfer therefore remains an uncontrolled difference.
Zero timely speech is not proof that the readout cannot learn production under
an aligned timing curriculum.

The controlled diagnostic confirms a stronger conditional route: grounded verb
and noun changes alter first-tick action logits by up to 2.563/2.713 and first-step
speech logits by 0.469/0.657. At one diagnostic fixture, food Grab minus food Eat
changes from -2.400 under `eat food` to +0.125 under `get food`; native sampling at
temperature 1 can still select Eat for the latter. For `get toy`, food Grab still
leads toy Grab by 1.211 despite a large noun effect. Exact native resampling
matches the collected action. These are causal/calibration diagnostics, not extra
behavioral success scores.

The teaching audit finds 14 concrete food-Eat, 18 food-Grab and only eight toy-Grab
labels in grounded instruction windows. Ten GetToy rows have no single concrete
action label, and fourteen teach Idle after fulfillment. This is an observed
curriculum imbalance; its causal contribution to noun failure has not been
isolated. No equal-dose V2-routing arm with this exact curriculum was run, so the
individual effects of routing, decoder-only optimization, schedule and dose are
not separated.

All three actors preserve every frozen coordinate's weight, moments and integer
age at all 64 updates, reject the old private life, and pass full fresh-process
trace/private/next-Adam and induced-sleep checks. Historical V13 recovery is exact
under the new compiler. Fourteen native language tests (including fork, private
rejection and recurrent freezing), 37 core tests, explicit native token/sampling
probes, strict Clippy including tests, formatting, headless and Python audit checks
pass. The default and older experimental phenotype identity checks remain exact.

`retained-language-linear-readout-results.json` includes both strict and independent
physical metrics, per-trial results, actual utterance receipts, source/binary
provenance, causal diagnostics, teaching counts and recovery hashes. Full raw
traces, every optimizer update, the measured executable and exact recoveries remain
at `/workspace/language-linear-readout-run`. This is a retained physical verb
signal in a narrow fixture, alongside negative noun, composition and production
results. No creature is promoted and no CFF-level language capability is claimed.
