# Matched four/eight-update native replay

V21 supplied a real corrective grounded gradient but regressed held-out behavior.
Its grounded margin became zero after four updates; four more native AdamW steps
retained old moments and weight decay. This experiment tests that specific
uncertainty without collecting new labels or changing the native objective.

Five conditions share the original V18 Grounded step 432 actor: untouched,
Grounded4, Grounded8, NoSpeech4 and NoSpeech8. The exact preserved V21 witness
corpus is hash-bound; actual failed rows, genuine matching native successes,
inputs, recurrent state, candidates, sampling counters, ownership and original
failed rewards are validated again. Within each four/eight comparison all
training rows, optimizer history, action-only mask and class normalization are
identical. Grounded has three rows; no-new-command has eleven. No new tutor,
provider, sampler exploration, fabricated transition or reward is introduced.
Both controls retain earlier grounded learning and physical imitation labels;
no-new-command means no heard commands in those stored rows, not language-naive.
Shuffled results stay preserved but are not expanded in this focused comparison.

The four-step checkpoint is captured before the remaining updates. Reconstructing
eight steps must exactly reproduce V21's actor, optimizer, foundation export,
saved next-gradient and update receipts before any fresh scored assessment.
A separate disposable native optimizer fork from Grounded4 takes four zero-vector
steps and must reproduce Grounded8 exactly. Actual per-step weight movement,
moment decay and integer ages are checked. Momentum and weight-decay arithmetic
components are reported separately. The scored treatment is the complete native
AdamW continuation, not a separate behavioral decay-only ablation. No optimizer
configuration, moment, age or weight reset is used to make the comparison pass.
All nonaction weights/moments/ages remain frozen, while native private biology
and learning remain enabled.

Six new world/sampling seeds are fixed before assessment:10104579,10210057,
10309219,10401957,10508179,10610399. They are absent from 50 prior recorded seed
receipts. New object IDs use padding 853+31*case; the same six known physical
layouts, food properties and familiar words are reused. This tests new seeds
and instances, not new geometry, language or independent trained founders.

Each model uses ordinary native sampling, cooldown and real actions for 24 ticks
under EatFood/GetFood/GetToy and silence; commands arrive at 0/2/4/6, with teacher
and semantic prior absent. There are no forced evaluation actions, RNG forks,
cooldown resets or output filters. The unchanged native and independent audits
score actual ownership/consumption, nourishment, first spoken phrase pairs,
timely prior-physical declarations, joint physical/spoken pairs, premature
utterances, safety and silence. Six pre/post RecoveryProtocol-sleep seed pairs
remain paired, not N12; sleep begins from the prepared parent before commands,
not a continuous post-command life or spontaneous sleep. No awake-delay control.
Fresh assessment cohorts retain the source actor and optimizer history, not
the original source individual private life. Each new private life is replayed
exactly in a fresh process, including saved-gradient next-Adam continuity. Older checkpoint schemas/default brain identities stay unchanged.

```sh
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --witnessed-early-stop MEASURED_V21 --output NEW_DIRECTORY \
  --seed 847501 --rounds 1 --epochs 8 --decisions 24
python scripts/summarize_balanced_grounding.py NEW_DIRECTORY \
  Untreated Grounded4 Grounded8 NoSpeech4 NoSpeech8
```

The earlier negative results and all source checkpoints remain intact. There is
no creature promotion, comprehension claim, accepted/live brain migration,
learned SLM delivery or Luna401 retry.

Measured result
---------------

The native run completed in 596.331 seconds (596.359 wall) at measured source
`781128246e00607f5f91aa248ffc07dcf37f5dd8`, executable SHA256
`e12c75dbcdf4ad13b974d6332477415b11c5652da246f0d9103ccbaf8bebca79`.
[Public receipts](retained-language-replay-early-stop-results.json) include the
exact reconstruction, all updates, momentum/decay accounting, actual per-seed
outcomes, separate sleep phases and complete preservation hashes.

All eight-step reconstructions exactly match the preserved V21 actor, optimizer,
foundation, next-gradient and update receipts. Grounded4 has zero native replay
loss/gradient; four additional zero-gradient native AdamW steps reproduce
Grounded8 exactly. They change 3069 action weights, net L2=0.211256. Each actual
step moves weights by L2=0.044899–0.061295. Its momentum arithmetic component is
L2=0.044899–0.061295, versus decay L2=0.000003609–0.000003615, over 12000 times smaller.
This is real retained optimizer motion, not a fresh teaching signal. No separate
behavioral decay-only actor was scored, so the observed policy effect belongs
to the complete native AdamW continuation.

Counts are pre/post induced sleep, each with six paired seed units. Physical
noun pairs require correct first acquisition of both requested objects within
the 8-tick cue window. A first-spoken pair requires each actual first utterance
to be a Declare with the exact requested two tokens in that window. Joint pairs
also require genuine prior matching physical completion and speech within 3 ticks
of it. Conditional payloads or lower losses are not counted as successes.

| Measure | Untouched | Grounded4 | Grounded8 | No command4 | No command8 |
|---|---:|---:|---:|---:|---:|
| Physical noun pairs /6 |4/3|3/2|3/3|3/3|4/2|
| Physical Eat/Get verb pairs /6 (24 ticks) |1/1|0/0|0/0|0/0|0/0|
| First-spoken noun pairs /6 |5/6|3/5|3/4|5/4|3/2|
| First-spoken verb pairs /6 |3/4|0/2|0/1|1/1|0/0|
| Timely requested physical declarations |6/8|5/7|5/5|6/5|4/3|
| Joint timely noun pairs /6 |2/2|1/1|1/1|1/0|2/0|
| Joint timely verb pairs /6 |0/0|0/0|0/0|0/0|0/0|
| Premature requested declarations |3/7|2/5|3/4|3/3|1/2|
| Precompletion Vocalize selections |19/25|8/11|5/6|8/8|2/4|
| All nourished meals (24 trials/phase) |3/3|1/1|1/0|2/2|2/1|
| Meals under EatFood request (6 trials/phase) |1/1|0/0|0/0|0/0|0/0|
| Silent-trial utterances (6 trials/phase) |4/4|4/4|2/2|4/4|2/2|

Extra zero-gradient AdamW steps matter to behavior, with mixed effects. Before
sleep, physical noun totals are unchanged; after sleep they improve by one seed,
while a first-spoken noun pair and the remaining meal are lost. Joint noun totals
stay 1/6 in both phases, but their identity changes: each phase gains one seed and
loses another. A descriptive paired audit added after the grounded result finds
actual physical event sequences differ in 3/24 pre-sleep and 5/24 post-sleep trials;
utterance timing/words differ in 14/24 and 15/24. Those sequence diagnostics do not
replace the preregistered metrics or establish reliability.

Four-step stopping is not a repair: it already loses all EatFood consumption,
underperforms untouched physical noun grounding and yields fewer timely joint
pairs. Its one meal per phase occurs under GetFood, not EatFood. The eight-step
continuation changes the failure pattern but does not explain the substantial
regression already present by step 4. No-command4/8 also lose EatFood behavior and
first-spoken noun performance declines with more updates. That control retains
active new physical gradients after step 4 (unlike grounded), so it is not another
zero-gradient treatment. Neither trained model should be promoted, and no
comprehension or cue-learning gain is established.

All five cold-process native assessments, private-state/row hashes and saved-real-
gradient next-Adam checks match exactly. Untouched remains identical at 432;
four/eight-step actors are 436/440, with their 3072 action ages incremented by 4/8.
All 29696 nonaction weights, moments and integer ages are exact. Historical V21
full assessment/private/next-Adam replay is exact. All 54 original recovery/entire
V21 run files, 24 earlier public evidence files and 50 prior seed-evidence receipts
remain byte-identical, including the V21 negative and executable.

All 5760 scored native ticks stay alive with health 1, injury 0, pain 0; minimum
energy 0.787935. Silent trials emit no unmatched taught phrase and no GetToy trial
emits EatToy. These are short nonlethal fixtures, not general safety or language
competence. Validation: 25 native language tests pass, 7 explicit diagnostics ignored;
24 old/new physical layouts are visible, edible and nourishing. Native zero-
gradient/frozen-history tests, strict Clippy, headless check, formatting and
Python validation pass. #262's exact head
`b4f515098161a1aaf81592e573708d3f7c0c8477` has all three workflows successful;
Jules comment 6646577039 says its actual review did not complete (IN_PROGRESS),
so no completed review is claimed. No learned SLM delivery or Luna401 retry.
