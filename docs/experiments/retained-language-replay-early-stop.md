# Matched four/eight-update native replay

V21 supplied a real corrective grounded gradient but regressed held-out behavior.
Its grounded margin became zero after four updates; four more native AdamW steps
retained old moments and weight decay. This experiment tests that specific
uncertainty without collecting new labels or changing the native objective.

Five conditions share the original V18 Grounded step432 actor: untouched,
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
receipts. New object IDs use padding853+31*case; the same six known physical
layouts, food properties and familiar words are reused. This tests new seeds
and instances, not new geometry, language or independent trained founders.

Each model uses ordinary native sampling, cooldown and real actions for24 ticks
under EatFood/GetFood/GetToy and silence; commands arrive at0/2/4/6, with teacher
and semantic prior absent. There are no forced evaluation actions, RNG forks,
cooldown resets or output filters. The unchanged native and independent audits
score actual ownership/consumption, nourishment, first spoken phrase pairs,
timely prior-physical declarations, joint physical/spoken pairs, premature
utterances, safety and silence. Six pre/post RecoveryProtocol-sleep seed pairs
remain paired, not N12; sleep begins from the prepared parent before commands,
not a continuous post-command life or spontaneous sleep. No awake-delay control.
Full cold process replay preserves private state and saved-gradient next-Adam
continuity. Older checkpoint schemas/default brain identities stay unchanged.

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
