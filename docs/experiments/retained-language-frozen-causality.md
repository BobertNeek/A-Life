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
