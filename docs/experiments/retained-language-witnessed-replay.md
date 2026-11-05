# Success-witnessed replay of real grounded failures

This experiment preserves the negative V20 result: its extra margin contributed
zero grounded gradient in all 16 epochs. It now admits a corrective label only
when a real native success is witnessed from the same neural/physical context as
a captured failed action. No world transition, reward or speech is fabricated.

Known V19 worlds (4104491, 4209927, 4308851, 4401393, 4507581, 4609807) are explicitly
retired into training. Starting from the unchanged V18 Grounded step432 actor,
normal scripted GetFood/GetToy commands use the ordinary spatial Player input,
matching the recorded failure interface. The tutor is scripted, not a learned
SLM. Shuffled swaps Food/Toy words; no-command emits none. All retain the same
original learned history and detached physical supervision, not language-naive
or unsupervised controls.

For at most four normal source ticks per case/command, an actual failed row is
captured. The existing native margin must be active. Up to32 disposable sampler
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
pairs (two Food, one Toy), all active, native action-gradient norm6.634790.
Shuffled admits10 (5/5), norm1.390901; no-command admits11 (5/6), norm5.627392.
Every pair has input/state-matched native success, with original failed rewards
untouched. The bounded run must verify these nonzero gradients again before any
update. This addresses V20's missing training signal; it is not behavior proof.

Only ActionCandidate decoder coordinates train, through the native recurrent
backward/Adam path. The stage mask changes explicitly on disposable trained
copies; all numerical parameters, moments, integer ages and configuration are
preserved when installing it. Speech and other coordinates then stay frozen,
including their optimizer moments/ages. No factual speech MSE or other existing
objective is continued. The sole loss is the native0.5 physical ranking margin,
normalized equally over the two observed acquisition classes. Eight updates are
fixed before seeing new assessment results; no model state is reset.

Six new world/sampling seeds are fixed before assessment:8104543,8210021,8309181,
8401903,8508131,8610343. Prior recorded language receipts are checked for overlap.
The same six physical geometry/food-size/nutrition tuples now used in training
are reused with new IDs (617+29*case). This tests new seeds/instances, not new
geometry, words or independently trained founders. Evaluation retains normal
sampling, preparation, cooldown, 24 ticks, cues at0/2/4/6, teacher/prior off,
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

The source actor/checkpoints and all previous positives/negatives remain intact.
There is no live/default brain migration, comprehension claim, promotion, real
SLM request or Luna401 retry. Safety/physical results apply to the short fixtures.
