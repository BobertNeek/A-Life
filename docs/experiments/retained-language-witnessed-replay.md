# Success-witnessed replay of real grounded failures

This experiment preserves the negative V20 result: its extra margin contributed
zero grounded gradient in all 16 epochs. It now admits a corrective label only
when a real native success is witnessed from the same neural/physical context as
a captured failed action. No world transition, reward or speech is fabricated.

Known V19 worlds (4104491, 4209927, 4308851, 4401393, 4507581, 4609807) are explicitly
retired into training. Starting from the unchanged V18 Grounded step 432 actor,
normal scripted GetFood/GetToy commands use the ordinary spatial Player input,
matching the recorded failure interface. The tutor is scripted, not a learned
SLM. Shuffled swaps Food/Toy words; no-command emits none. All retain the same
original learned history and detached physical supervision, not language-naive
or unsupervised controls.

For at most four normal source ticks per case/command, an actual failed row is
captured. The existing native margin must be active. Up to 32 disposable sampler
forks then use ordinary native sampling, world legality, physics and biology.
Only a real acquisition of the requested object admits the label; its native
selected representative must be the corresponding Grab candidate. The failed
row retains its own action, outcome and biological reward. The successful fork
and its actual outcome are separate evidence for the imitation target.

The explicit CPU `fork_sampling_stream` clones an already-sealed host, retains
its RNG counter, changes only the future sampling seed and reseals its native
authority. Source checkpoint identity, neural initial state, the complete replay
tick (inputs, offsets, structural context and candidates), and cold admission
are checked. The original source remains unchanged. This is training exploration;
scored evaluation does not fork streams or force actions.

A separate preflight-only run completes before training or assessment. It exactly
reproduces the preserved V19 GetFood failure digest. Grounded admits three real
pairs (two Food, one Toy), all active, native action-gradient norm 6.634790.
Shuffled admits 10 (5/5), norm 1.390901; no-command admits 11 (5/6), norm 5.627392.
Every pair has input/state-matched native success, with original failed rewards
untouched. The bounded run must verify these nonzero gradients again before any
update. This addresses V20's missing training signal; it is not behavior proof.

Only ActionCandidate decoder coordinates train, through the native recurrent
backward/Adam path. The stage mask changes explicitly on disposable trained
copies; all numerical parameters, moments, integer ages and configuration are
preserved when installing it. Speech and other coordinates then stay frozen,
including their optimizer moments/ages. No factual speech MSE or other existing
objective is continued. The sole loss is the native 0.5 physical ranking margin,
normalized equally over the two observed acquisition classes. Eight updates are
fixed before seeing new assessment results; no model state is reset. Admission
produces 3/10/11 rows for grounded/shuffled/no-command, respectively: equal update
count and per-noun class weight, with unequal data diversity and exploration cost.

Six new world/sampling seeds are fixed before assessment:8104543,8210021,8309181,
8401903,8508131,8610343. Prior recorded language receipts are checked for overlap.
The same six physical geometry/food-size/nutrition tuples now used in training
are reused with new IDs (617+29*case). This tests new seeds/instances, not new
geometry, words or independently trained founders. Evaluation retains normal
sampling, preparation, cooldown, 24 ticks, cues at 0/2/4/6, teacher/prior off,
GetFood/GetToy/EatFood and silence, authoritative safety and actual utterance
metrics, six paired pre/post induced-sleep states, and full cold/private/next-Adam
checks. Sleep starts from the prepared parent before commands; it is not a
continuous post-command life, independent N12 or an awake-delay causal control.
Native biology/private-state evolution remains enabled.

```sh
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --witnessed-preflight MEASURED_V18 --output PREFLIGHT_DIR \
  --seed 847501 --rounds 1 --epochs 8 --decisions 24
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --witnessed-replay MEASURED_V18 --output NEW_RUN_DIR \
  --seed 847501 --rounds 1 --epochs 8 --decisions 24
python scripts/summarize_balanced_grounding.py NEW_RUN_DIR
```

Training captures at most four source ticks with cues at 0/2; assessment retains
the existing full cue schedule. These durations differ. All 24 admitted replay
rows contain the actual heard Player tokens expected for their arm (grounded
GetFood/GetToy, swapped nouns, or no heard tokens), verified from native sensory
frames rather than inferred from requested tutor messages.

The source actor/checkpoints and all previous positives/negatives remain intact.
There is no live/default brain migration, comprehension claim, promotion, real
SLM request or Luna401 retry. Safety/physical results apply to the short fixtures.


Measured negative result
------------------------

The 8-update run completed on CPU in 504.579 seconds (504.607 wall), measured source
`de0d18f38462ded70057d32dc7bba086b128cefa`, executable SHA256
`94e656e0f4b5720dc62cf38fa4c1ae7063d7158f8c9aee0a21c4761c00b17301`.
The [public receipts](retained-language-witnessed-replay-results.json) include
actual witness pairs, gradients, all 8 updates, separate per-seed outcomes,
utterance timing, safety, silence, continuity and preservation hashes.

Counts below are pre/post induced sleep, each with six paired seed units; do
not add the phases as independent samples. Physical noun pairs require correct
first acquisition under both GetFood/GetToy in the 8-tick cue window. First
spoken pairs require the actual first Declare to match both requested phrases
within that window. Joint pairs additionally require genuine matching prior
physical completion and speech no more than 3 ticks later.

| Measure | Untouched | Grounded | Shuffled | No new command |
|---|---:|---:|---:|---:|
| Physical noun pairs /6 |2/2|1/2|3/3|2/2|
| Physical Eat/Get verb pairs /6 (24 ticks) |1/0|0/0|2/2|0/0|
| First spoken noun pairs /6 |3/5|0/2|4/5|0/1|
| First spoken verb pairs /6 |1/3|0/0|2/4|0/0|
| Timely requested physical declarations |2/6|1/3|3/9|0/1|
| Joint timely noun pairs /6 |0/1|0/0|1/2|0/0|
| Joint timely verb pairs /6 |0/0|0/0|0/0|0/0|
| Premature requested declarations |3/7|2/3|2/5|1/2|
| Precompletion Vocalize selections |33/34|6/4|11/13|3/3|
| Actual nourished meals (all 24 trials/phase) |4/3|0/0|5/5|0/0|
| Silent-trial utterances (6 trials/phase) |2/3|0/0|2/4|0/0|

Grounded has real corrective gradient on the exact missed failure, and its
margin falls from 0.371536 to 0 over its three admitted replay rows. Active rows
across the 8 epochs are 3/3/1/1/0/0/0/0. The final four fixed-budget Adam steps keep
retained moments/decay despite zero new gradient; no early-stop or
zero-gradient-only control isolates their contribution. The original curriculum
coverage gap is addressed, but this small continuation fails its behavioral
test: noun grounding weakens, all meals disappear and joint timely distinctions
remain zero. Reduced premature output comes with much lower Vocalize activity
and no joint grounding gain. It is not a demonstrated timing repair. Speech
weights stay frozen, while action selection and subsequent private trajectories
change actual utterances. Shuffled performs better here; unequal admitted-row
diversity limits causal comparisons. Do not promote this actor.

No arm utters an unmatched taught phrase during silence, and no GetToy trial
emits EatToy in this run. These limited negatives do not establish understanding.
All 4608 scored native ticks stay alive with health 1, injury 0, pain 0; minimum
energy 0.787939. All four cold replays, private-state hashes, full native row
traces and saved-gradient next-Adam checks match exactly. Trained copies advance
432→440 with 3072 action coordinates trainable; all 29696 other weights, moments and
integer ages remain exact. Untouched step 432 is fully identical. All 36 source
recovery files and 23 earlier public evidence files remain byte-identical.

Validation: 24 native language tests passed, 7 explicit diagnostics ignored;
18 old/new physical fixture layouts tested visible, edible and nourishing;
sampler-fork/source-seal and frozen speech/optimizer tests passed. Strict Clippy,
headless check, formatting and Python validation pass. Historical V20 full cold
assessment/private/next-Adam replay remains exact. #261's exact head
`b34cdf977ead13275bcb565560d5ca371038b590` has three successful workflows; Jules'
comment 6645751288 says its review did not complete (IN_PROGRESS), so no completed
review is claimed. There was no learned SLM delivery or Luna401 retry.
