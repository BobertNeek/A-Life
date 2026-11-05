# Frozen command causality and retention

This experiment uses the unchanged V23 Untreated, CareOnly4, MixedGrounded4 and
MixedNoCommand4 artifacts. No model, provider or tutor is called. Global weights,
optimizer configuration, moments, stage masks and integer ages remain frozen.
The cold-replay path explicitly skips its usual shadow Adam update for this mode;
zero epochs/rounds and exact before/after actor hashes are required. Native private
learning and biology continue normally within each disposable assessment life.

Six predeclared cases are separated into two strata. Known positive controls reuse
V22 Untreated's exact worlds:10210057/fixture1/identity padding884 consumed food
under EatFood at tick2 in both sleep phases;10309219/fixture2/padding915 correctly
distinguished first Food/Toy acquisition in both phases. Selection is disclosed
and they are not held out. Four independent fresh seeds16104687,16309347,16508317,
16610541 use fixtures0/2/4/5 and new paddings1409/1450/1491/1532. All reuse familiar
words and geometry/food properties; one retained founder, not training replicates.

For each case and pre/post-sleep parent, four distinct episodes start from exact
snapshots of the same full physical/private/RNG state: actual delivered EatFood,
GetFood, GetToy, or muted. Ordinary Player speech is emitted at0/2/4/6 for the
first8ticks; each episode lasts24ticks. Teacher and semantic prior remain off,
with native sampling, cooldown, legality, physical consequences and utterances.
Noun swaps compare GetFood↔GetToy; verb swaps compare EatFood↔GetFood. Swapped and
muted contrasts reuse these real delivered-cue trajectories for scoring against
nominal requests, not additional samples or invented transitions. Nominal request
labels are analysis only and never enter the organism through another channel.
Muted has one actual episode per parent, not three independent trials.

Immediately before and after cue emission, full checkpoint values must match
except exactly world.world.audible_utterances and world.world.next_utterance_id.
Positions, objects, ownership, biology, all private neural/memory state, tracked
objects and sampling seed/counter therefore stay unchanged by emission. The first
native row records actual heard Player tokens, whether they occupy the two raw
word-broadcast slots, the native16word-bit/confidence lanes and encoded input
hashes. Pairwise input comparisons identify changed neurons among compiled direct
word targets and record initial state, candidate and offset equality. Routing
failures stay in the report; no speech clearing or input replacement makes a
failed intervention look effective.

Normal/swapped speech shares presence, cadence and location. Muting also changes
the native prompted/spontaneous speech cooldown (8/32), so muted speech-frequency
changes cannot isolate lexical meaning. No cooldown reset or output filtering is
used. Physical behavior, nourishment, injury/survival, first actual declarations,
premature versus factual timely speech, noun/verb pairs and pre/post-sleep effects
are reported separately by known/fresh stratum and model. Both8-tick cue-window
and strict24-tick physical verb criteria remain visible.

Sleep is induced RecoveryProtocol sleep from the prepared parent before cues,
not spontaneous sleep, a continuous post-command life or an awake-delay control.
Phases share seeds and must not double N. Fresh cohorts retain actor/optimizer
history rather than the original source individual's private life; full cold
replay checks every new native trace and private state without any global update.
All source checkpoints and previous successes/failures remain intact.

```sh
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --frozen-commands MEASURED_V23 --output NEW_DIRECTORY \
  --seed 847501 --rounds 0 --epochs 0 --decisions 24
python scripts/summarize_balanced_grounding.py NEW_DIRECTORY \
  Untreated CareOnly4 MixedGrounded4 MixedNoCommand4
```

Current cue-dependent behavior could establish functional use of these familiar
inputs; all source actors already retain grounded history. It cannot establish
when/how meanings were learned, novel language/general comprehension, or a new
SLM's contribution. No optimizer change, source-state overwrite, accepted/default/
live brain migration, promotion, Luna401 retry or denied-upload retry is attempted.

Measured causal and retention result
-------------------------------------

The native run completed in 536.779 seconds (536.806 wall), measured source
`7ef96f09c15c45227a6f628b75b5a19187f7bd58`, executable SHA256
`3ec298c1f14ba970530acb44f5426c9eb599548c609b888f4c826476c595bdc6`.
[Public receipts](retained-language-frozen-causality-results.json) contain separate
strata, every actual episode's physical/speech audit, per-seed nominal cue
contrasts, input/setup proofs, complete frozen-state hashes and limitations.
`scripts/summarize_frozen_language_causality.py` regenerates the full audit from
native traces. Public contrasts refer back to actual episodes instead of copying
their events, and full changed-input lists have counts/hashes plus direct-word
indices; complete inputs/lists remain in the unchanged native traces.

There are 192 unique real episodes (4608 decisions), plus identical cold replay.
Swaps/muting reuse those actual trajectories for scoring against nominal goals;
this does not add samples or independent evidence. All four actors remain at
steps 432/436/436/436 with zero global updates, including zero shadow Adam steps.
Every global weight, moment, age, config, mask and foundation binding stays exact.
Source private recovery is verified; each new assessment private life is cold-
replayed exactly while native private learning/biology remain enabled.

Input and setup checks pass in all 192 first rows. Every spoken cue (144) is heard
with both tokens in the first two broadcast slots, matching all 16 native signed
word/confidence lanes. All 144 lexical first-row pairs have changed direct word
inputs with identical neural initial state, replay candidates, effective offsets
and sampling counter. The full pre/post-emission checkpoint differs only in the
audible queue and utterance counter; physical/private/RNG setup stays exact.
Muted comparisons preserve initial neural state/offsets/RNG but change native
candidate context as well as input/cooldown; this is disclosed and not treated
as an isolated lexical intervention. There is no failed cue-routing case here.

The two known positive controls reproduce all 384 V22 Untreated native row hashes
exactly, including their food/noun successes and failures. They are selected
positive controls, not new generalization. Counts below are pre/post paired
sleep; known N2 and fresh N4 are distinct, neither phase doubles N.

| Known normal-cue controls (N2) | Untouched | Care-only | Grounded mixture | No-command mixture |
|---|---:|---:|---:|---:|
| EatFood nourishment within 8 ticks |1/1|2/2|2/2|2/2|
| Physical noun pairs |2/1|2/1|2/1|2/1|
| Physical verb pairs, 8 or 24 ticks |1/1|2/2|2/2|2/2|
| First-spoken noun/verb pairs |2/2|2/2|2/2|2/2|
| Timely requested declarations |2/3|3/5|3/5|3/5|
| Premature requested declarations |1/3|0/1|0/1|0/1|
| Joint timely noun pairs |1/1|1/1|1/1|1/1|
| Joint timely strict verb pairs |0/0|1/2|1/2|1/2|

| Fresh normal-cue cases (N4) | Untouched | Care-only | Grounded mixture | No-command mixture |
|---|---:|---:|---:|---:|
| EatFood nourishment within 8 ticks |2/1|3/3|2/2|2/2|
| Muted nourishment within 8 ticks |1/1|1/1|1/1|1/1|
| Physical noun pairs |2/2|2/2|2/2|2/2|
| Physical verb pairs within 8 ticks |0/0|1/1|0/0|0/0|
| Strict physical verb pairs over 24 ticks |0/0|0/0|0/0|0/0|
| First-spoken noun pairs |3/4|3/4|3/3|3/3|
| First-spoken verb pairs |3/3|4/4|3/4|3/4|
| Timely requested declarations |2/4|3/5|2/5|2/5|
| Premature requested declarations |3/5|3/5|3/4|3/4|
| Joint timely noun pairs |0/1|0/1|0/1|0/1|
| Joint timely strict verb pairs |0/0|0/0|0/0|0/0|

All models have zero physical noun pairs scored against original nominal goals
under muted or noun-swapped inputs, in both strata/phases; actual spoken output
also follows the delivered rather than original nominal words. All nominal
verb-swapped/muted physical verb pairs are zero. These contrasts reuse the real
response matrix and are not additional independent successes/failures.

The spoken GetFood/GetToy pair itself is the important causal evidence: from an
identical physical/private/RNG parent, changing familiar lexical tokens yields
correctly different first-object acquisition in 2/4 fresh cases per phase, with
matched nonlanguage candidate context. This rules out a wholly cue-independent
object/action policy on those successful cases and rules out disconnected or
masked first-cue input as their explanation. It establishes current functional
use of these familiar inputs in this native cognition/action pipeline.

The successful fresh noun seeds are 16104687 and 16309347 in every actor and both
phases; these are two seed units, not repeated independent successes. That
finding already exists in the untouched retained actor. Neither mixture
improves its fresh noun count; care-only is at least as strong in fresh food and
short-window verb performance. All source actors have earlier grounded training;
this frozen test does not identify when/how the mapping was acquired, assign it
to recent heard-command training, isolate global weights from private online
adaptation, or demonstrate new word/general comprehension. Fresh strict/timely
joint verb pairs remain zero despite positive known controls and often correct
first utterances. Word matching alone can reflect cue-following/echoing and does
not establish factual timely reporting. The earlier global baseline food floor
is not universal, but known successes must not be counted as fresh generalization.

Sleep-phase differences are visible in the tables; they share seeds and prepared
parents, with native private consolidation/body evolution active. There is no
matched awake-delay control, so phase changes cannot be attributed solely to
sleep consolidation. Silent spontaneous utterances remain (fresh U 2/3, others 2/4),
with no unmatched taught phrase. All 4608 scored ticks stay alive, health 1,
injury 0, pain 0; minimum energy 0.787898. These are short nonlethal fixtures only.

All 95 source recovery/entire earlier run files, 26 prior public evidence files,
56 seed-evidence receipts and 7 invalid-target diagnostic artifacts remain byte-
identical. The frozen actor/cold/private traces match exactly. 26 native language
tests pass (7 explicit diagnostics ignored), including a full nonaudio-proof test
that accepts speech-only emission and rejects a physical object move. Strict
Clippy, headless check, formatting and Python validation pass. Default prior-
auditor output remains exactly unchanged on the preserved V23 source trace.
#264 exact head `4317047324d1aed269297f8cee02f2f9f113dd47` has three terminal
successful workflows; Jules comment 6647539865 says its actual review did not
complete (IN_PROGRESS), which is not formal approval or language evidence.
No accepted/live checkpoint mutation, brain promotion, new provider/model call,
Luna401 retry or denied-upload retry occurred.
