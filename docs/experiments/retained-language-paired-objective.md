# Paired token effects and command-first objective experiment

This CPU-only public-founder experiment continues the negative factorial study
in `retained-language-factorial-grounding.md`. All older actors, optimizer/private
recoveries, measured executables and negative reports remain intact. It neither
migrates a gameplay brain nor changes default encoder or neural execution.

## Causal evidence before the change

Three ordinary player requests (`eat food`, `get food`, `get toy`) were delivered
to forks of the same real world/private state, seed 947839, distance 0.41. The
native replay probe verifies that only encoded word inputs differ, with identical
candidates, offsets and initial state. The six repeated-context replay ticks are
an intervention diagnostic, **not a biological trajectory or learning score**.
The measured actor remains unchanged. Every native recurrent microstep, action
logit and six-step speech readout is measured.

For the V11 grounded actor, changing eat→get (noun fixed) changes first-tick action
logits by up to 0.292734 and speech logits by 0.053852. Changing food→toy (verb
fixed) changes action logits by 0.240710 and speech logits by 0.066831. Both changes
reach native recurrent and decoder gradients. Structural disconnection no longer
explains this actor's failure.

The relevant margins still favor the same wrong action. Food Grab minus food Eat
is +0.459817 for `eat food` and +0.520002 for `get food`: a verb effect of only
0.060185. Toy Grab minus food Grab is +0.401245 for `get food` and +0.524487 for
`get toy`: a noun effect of 0.123243, insufficient to reverse the preference.
Paired action-gradient cosine is 0.290127 for verbs and 0.576958 for nouns. With
the label fixed, token changes alter gradient L2 by 0.242027 / 0.354183; with input
fixed, label changes alter it by 2.147938 / 1.725317. These are objective/path
measurements, not evidence of comprehension.

V11 training has 62 request rows, 69 narration rows and 157 other rows in its 288
real grounded decisions. Its uniform action-row objective does not balance the
three matched initial command decisions. A positive narration label supervises
native payload bits, declaration, presence and stopping **after a sealed physical
activity**; it does not reward a correct action, prove cue binding or supply an
object's meaning at runtime. Native action imitation independently supervises the
legal candidate/effector choice. The real body's reward and transition stay
unchanged.

## Targeted experiment

`--focused-grounding /absolute/V8-run` selects the new experimental path. All
three arms clone the same measured V8 grounded actor at native Adam step 256,
including weights, moments, integer update ages and frozen coordinates. Each gets
the same 32-update dose and ordinary on-policy physical trajectories used by the
factorial study. The first actual decision in each of six paired training lives
gets an offline native legal intention label in **all** arms, including the
no-request control. All three commands receive equal first-decision mass.

The action objective allocates 0.8 total weight to those first decisions, 0.1 to
actual post-activity narration action rows and 0.1 to remaining action rows.
Classes absent from real experience get no fabricated rows. Existing balanced
positive-only factual payload supervision is retained. Native BPTT, native Adam,
optimizer configuration, sparse topology, hard speech gates and action sampling
are unchanged. Per-class gradient norms and inner products are recorded.

Grounded requests match the intended action. Shuffled requests use the existing
deterministic cyclic permutation; no-request receives no teacher utterance. All
arms retain truthful offline narration labels and prior V8 learning, so neither
control is a language-naive or shuffled-production-label ablation.

Assessment uses new world seeds (seed + 190000/190001), food/toy bearings
±0.34/∓0.76, distances 0.41/0.44 and 0.73/0.76, reversed sides and a visible
distractor. These combinations are absent from training (±0.58 bearings,
0.36/0.41 and 0.44/0.49 distances). Each common private parent is forked for the
three ordinary player commands and silence. No teacher or semantic prior is
present, and evaluation never forces actions. Contact, near and induced native
RecoveryProtocol sleep are assessed. Exact fresh-process restore compares the
complete native trace, private state and next Adam update.

Results report first-event discrimination, eventual physical completion,
paired verb/noun outcomes, actual world utterances and timely factual descriptions
separately. Loss and utterance count alone establish no competence.

## Provider authorization

No additional Luna request, health probe, credential modification or alternate
endpoint is attempted. The earlier official local health diagnostic observed a
Responses WebSocket HTTP 401 with configured ChatGPT authentication; it did not
establish the cause. Installed `codex login --help` documents the supported login
command and device-auth option. The supported next step is an owner-managed authorization check/reconnect through
that flow; if the WebSocket 401 persists, account/environment access needs owner
or provider attention. The evidence does not establish expired credentials or
guarantee that reconnecting resolves it. This experiment
uses an explicitly scripted spatial teacher and claims no real SLM delivery.

## First-decision weighting result (V12)

Source `8fada8993074a5dc0d27c6db4a13212e6e30e300`, executable SHA-256
`b9af26aa6da95739d1ec2b1fa67ee5de4850eca2f976f8dd2aa80e4a8b230537`.
Elapsed native run: 631.645 s (631.661 s external). Each arm advances 256→288.
There are 2,016 scored decisions plus 864 cold-repeat assessment decisions.

| Arm | First requested outcome / 9 | Eventual requested outcome / 9 | Correct verb pairs / 3 | Correct noun pairs / 3 | Utterances | Timely factual utterances |
|---|---:|---:|---:|---:|---:|---:|
| Common baseline | 5 | 5 | 0 | 0 | 12 | 0 |
| Grounded | 3 | 3 | 0 | 0 | 12 | 0 |
| Shuffled | 3 | 3 | 0 | 0 | 12 | 0 |
| No request | 3 | 3 | 0 | 0 | 8 | 0 |

All final utterances are `get food`, including after toy acquisition. Grounded
and shuffled actors first acquire the toy for every requested and silent trial.
Increasing the first-decision weight did **not** establish retained language.
All three actors pass exact cold native trace/private/next-Adam checks and induced
sleep assessment. The raw report, full corpus and measured binary are preserved
at `/workspace/language-request-focus-run`.

## Paired command margin objective (V13)

This separate experiment retains the V12 budgets and adds a native paired term.
For two requested candidates i/j and actual first-decision logits A/B from the
same real private parent, its margin is:

```
m = (A[i] - A[j]) - (B[i] - B[j])
loss = softplus(1 - m)
```

The three command pairs receive equal mass; the extra objective has coefficient
2. Its analytic logit adjoints enter the existing sparse native recurrent BPTT.
Any cue-independent preference cancels algebraically. No request inputs have
exactly zero extra paired gradient; the ordinary V12 action/payload terms and
retained Adam history still apply. This is offline intention supervision, not
invented physical success or a change to native evaluation sampling. The matched
controls, held-out fixtures, dose, source actor and cold/sleep assessment are the
same as V12.

The cue-off diagnostic replaces later context with an actual silent first frame
from the same parent. After five silent diagnostic ticks, V11 first speech-step
max logit differences shrink from 0.053852→0.001745 for verbs and
0.066831→0.003421 for nouns. This bounds available short-term cue evidence in this
fixture; it does not prove that memory is the only production failure. Real
positive speech labels do reach trainable recurrent weights and all 1,024 speech
decoder coordinates. Swapping a noun label on the same captured positive row
changes the native gradient substantially, so the target machinery can distinguish
nouns even though the measured actor does not.

A narration target describes the **observed completed activity**, not the incoming
request. If every request causes toy acquisition, correct narration should still
say `get toy` in every such life; command-sensitive words alone would be a poor
production metric. The measured constant `get food` is factually wrong after those
toy events. Existing equal activity-class payload weighting also differs from
lexical balancing: when all three activity classes are observed, two of the three
noun targets are `food`. An actor without enough conditional event evidence can
minimize a large part of this objective with a common noun. The diagnostics show
available word paths and distinct target gradients; they do not yet isolate the
remaining production failure among event representation, recurrent persistence
and optimization. No CFF-level language capability is claimed.

## Paired-margin result (V13)

Source `f8d30124ad782c2eaf08038e5c54f92de7de728b`, executable SHA-256
`9614c574445b4a87164a7b662d8a3f57df9c9adca2c682ace82d4c58c71efc30`.
Elapsed native run: 588.295 s (588.317 s external), same 32-update dose (256→288)
and 2,016 scored decisions plus 864 cold-repeat decisions as V12.

| Arm | First requested outcome / 9 | Eventual requested outcome / 9 | Correct verb pairs / 3 | Correct noun pairs / 3 | Utterances | Timely factual utterances |
|---|---:|---:|---:|---:|---:|---:|
| Grounded | 3 | 3 | 0 | 0 | 11 | 0 |
| Shuffled | 3 | 3 | 0 | 0 | 12 | 0 |
| No request | 3 | 3 | 0 | 0 | 8 | 0 |

The negative physical and narration result persists. Every emitted phrase is
`get food`. All six trained actors across V12/V13 pass exact fresh-process native
trace/private/next-Adam and induced-sleep checks. The common baseline is identical
between runs. The no-request final actor is **bit-identical** between objectives:
`27b6a45034df64e7d8ae4369a34016014f79b0744c2becb70ed6d2acb400649c`.
Every one of its 32 observed added paired-gradient norms is exactly zero.

Grounded final training margins for EatFood/GetFood are 0.312/0.310 and for
GetFood/GetToy 0.326/0.437 in the two training layouts. The added objective acts on
the intended conditional readout; those values are not held-out action success.
Neither optimization attempt establishes retained command comprehension or factual
narration. Remaining causes need further isolation; increasing weighting or
observing nonzero gradients alone is inadequate evidence for a stronger creature.

`retained-language-paired-objective-results.json` contains per-trial physical and
utterance outcomes, both source/binary provenances, actual teaching audits,
first/last update evidence and cold hashes. The diagnostic companion records every
microstep and six-step speech effect for the original retained actors; duplicate
candidate arrays are omitted by explicit reference. Raw complete reports, traces,
all updates, measured binaries and recoveries remain in
`/workspace/language-request-focus-run` and `/workspace/language-paired-margin-run`.

Validation includes 13 native language tests, the paired finite-difference/silent
null test, explicit ignored token diagnostics, strict Clippy including tests,
formatting and headless build. Historical V11 cold trace/private/next-Adam remains
exact. Teacher and semantic prior are absent in every scored assessment row;
no scripted action, fabricated transition or invented biological reward enters
assessment. Missing controls remain: independent founder histories, spontaneous
sleep, language-naive untreated production, and shuffled narration labels.

## Action sampling and automatic pickup audit

The additional one-step diagnostic (seed 947839, distance 0.41) verifies exact
native action resampling from recorded seed/counter/temperature. The V13 grounded
actor selects food Eat for `eat food` and food Grab for `get food`; shuffled and
no-request select food Grab for both. **This is not a completed physical verb
distinction.** The sealed Eat receipt shows automatic food pickup without
consumption. All three requested lives end holding food, with zero ingestion and
identical negative energy change. `get toy` still acquires food. That extra fixture
is exploratory, not an added prespecified success score. It illustrates why a
sampled action ID, like a changed logit, can overstate achieved behavior.

The original event collector counts acquisition labels after explicit native Grab
plus ownership change. The companion
`scripts/audit_retained_language_physical_outcomes.py` separately checks actual
ownership changes after **any** primitive. It retains same-tick events as a set and
does not invent an order. The independent audit leaves all six eventual requested
outcome scores at 3/9. There are three additional implicit food acquisitions in
V12 shuffled lives; none creates an additional correct request trial. V13 has no
implicit acquisition in the prespecified assessment. Every first physical event
there remains toy acquisition; no EatFood request consumes food.

The rebuilt final executable also reproduces the full V13 cold assessment,
private state and next Adam update exactly. The source-safe results include these
physical audits, sealed one-step receipt hashes and native sampling diagnostics.
