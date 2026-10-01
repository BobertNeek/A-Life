# N2048 vision, maze, and language preflight — 29 September 2026

The next teacher-free overnight campaign was **not launched**. A bounded
eight-epoch warm-up changed the inherited weights and lowered imitation loss,
but the frozen learner did not improve on the held-out stage-0 behavior gate.
The warm-up is a diagnostic checkpoint, not a selected founder or approved
overnight source.

## Repairs

- Speech training now gives grounded positive utterances and meaningful silence
  equal loss budgets within each replay window. The GPU objective and its
  finite-difference diagnostic use the same normalization. No synthetic words
  were added.
- The offline teacher, warm-up, cycle, and frozen evaluator now admit the
  world's existing terrain rule before play. Canonical fixtures use legacy X/Y
  placement; normal terrain admission converts them to horizontal X/Z and
  grounds movement. The old terrain-free setup could pull a creature upward
  while approaching objects. Old replay corpora and warm-ups are unsuitable as
  sources for this campaign.
- Frozen evaluation accepts a second request or silence in an otherwise
  identical scene. It checks the first actual response and records the initial
  world signature. A sleeping or terminal learner seals its preceding waking
  segment instead of making the whole evaluation fail.
- The campaign runner starts its eight-hour wall clock before building, reserves
  45 minutes for final assessment, stops an unfinished cohort at the collection
  deadline, and never uses that partial output as the next source. Its final
  comparison checks navigation, request contrast, literal speaking, and care
  separately. A regression cannot win a tradeoff; ties keep the starting source.
  Selection is recorded, never promoted automatically.

## Focused evidence

- The existing N2048 GPU speech test passed once, including rare-positive and
  silence normalization, finite differences, and export. The runner's focused
  deadline/contrast/selection mock passed. The release trainer and frontend
  built. No broad suite was rerun.
- One grounded feeding teacher lesson ate after 58 decisions; every recorded
  position had Y=0. A sampled grounded maze demonstration reached all 14 route
  nodes, had no blocked action, stayed at Y=0, and ate. Those are **teacher**
  outcomes, not learned competence.
- The fresh grounded corpus at
  `target/founder-training/grounded-corpus-20260929-01/manifest.json` sealed 50
  demonstrations, 4,667 replay records, and 722 speech opportunities. It covers
  18 reception and 18 production words. The Qwen prior delivered 2,079 frame
  hints across the corpus with zero provider failures.
- The diagnostic warm-up at
  `target/founder-training/grounded-preflight-20260929-01/warmup/warmup.json`
  sealed eight epochs, 56 optimizer steps, a changed asset digest, and a verified
  next-cohort optimizer handoff. Mean imitation loss across its seven windows
  fell from 5.94 in epoch 1 to 5.04 in epoch 8. That is optimization evidence,
  not behavior evidence.

The corrected-geometry held-out summaries are
`target/founder-training/grounded-preflight-20260929-01/before/summary.json`
and `target/founder-training/grounded-preflight-20260929-01/after/summary.json`.
Both score **[0, 0, 0, 0]** in the order navigation, request contrast, speaking,
care. Neither checkpoint ate in any of the three held-out mazes; neither said a
correct target utterance or passed the two care cases. The baseline's first
response to “food,” “toy,” and silence was the same inspect-toy action. The
warm-up's first response to all three was the same targetless action. A trained
production case ate twice, but still did not say its target phrase; that does
not clear the language or care gate. The old pre-terrain baseline was excluded
from this comparison.

## Decision

Do not start the next eight-hour teacher-free campaign from this eight-epoch
warm-up. The measured teacher-to-learner gap remains too large, and the final
comparison would retain the starting source. The next bounded step is to
strengthen the **existing** imitation stage or its lesson ordering, then rerun
this same small frozen panel before spending an overnight GPU window. Do not
add a second navigation or reward system. The 10–30 minute food-free survival
gate and a player-visible 1x care/ball/activity-toy playtest remain later
acceptance checks. The visible playtest was not performed while the player was
away; desktop input automation was not reliable enough to substitute for it.
