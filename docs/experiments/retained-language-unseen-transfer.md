# Frozen language transfer to unseen worlds

This is an evaluation-only follow-up to the bounded V18 report-choice experiment,
not additional training. Each of its four public disposable actors is assessed
without any weight, Adam moment, integer-age or configuration update. Untouched,
grounded, shuffled and no-new-teacher retain their respective measured V18
histories; no-new-teacher is not language-naive.

Before evaluating these actors, six new world/sampling seeds were fixed:
4104491, 4209927, 4308851, 4401393, 4507581 and 4609807. None occurs in the 43
retained prior results/evidence receipts checked (23 distinct recorded seeds).
Their seed list and physical fixture plan were recorded on issue #2 before the
run. This is a new environment set for frozen actors, not six independently
trained founders or a test of new word combinations.

`balanced/unseen_transfer.rs` declares all fixtures: new food/toy identities use
173 + 19 * case identity padding, opposite spawn order to the old matched case,
new angles/distances, food nutrition 0.85–1.15 and radius 0.105–0.135. Object
categories, vocabulary and visible distractors remain familiar. Physics controls
check real visibility, food acquisition, consumption and positive nourishment
on all six fixtures; these scripted controls are outside scored assessment and
are not learning evidence.

The scored protocol is otherwise the existing native V18 assessment: four
counterfactual requests (EatFood, GetFood, GetToy, silence), 24 decisions each,
player cues on ticks 0/2/4/6, no teacher or semantic prior, and no forced actions.
Natural preparation and inherited speech cooldown are preserved. Counterfactual
cue trials clone the same prepared private parent. The post-sleep parent sleeps
before its cue trials; it does not continue a completed pre-sleep command trial.
This measures the retained actor under paired private states, not continuous
transfer of a prior command life. Every seed has
paired pre/post induced RecoveryProtocol sleep, with no spontaneous-sleep or
awake-delay causal control. Pre/post phases do not double independent N.

Actual first spoken Declare phrases, strict post-event timely speech, real
ownership/ingestion/nourishment, paired physical distinctions and silence are
reported separately. Correct naming alone is not comprehension or reliable
physical grounding. The strict timely metric remains a matching actual prior
event at most three ticks old within the eight-tick cue window; concurrent,
premature and late utterances do not satisfy it.

The new assessment flag is confined to the disposable experiment's plan. Its
false/default value is omitted in JSON, so historical plans roundtrip unchanged.
Existing paths retain their seeds, fixtures and protocol. Full assessment is
repeated after fresh-process reload, including private state and the next native
Adam update with the saved real gradient. That diagnostic update is applied to
a disposable copy, not to the frozen assessed actor or its source files.

```sh
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --unseen-grounding /absolute/path/to/measured-V18-run --output NEW_DIRECTORY \
  --seed 847501 --decisions 24
python scripts/summarize_balanced_grounding.py NEW_DIRECTORY
```

No learned SLM delivery, Luna 401 retry, source-state overwrite, default/accepted
brain migration, live/PC training change or promotion is part of this evaluation.

## Measured unseen-world result

The single frozen-actor run completed at measured source
`28a2711d6149ecef3b991b4041a3b3011e681bda`, binary SHA-256
`79354dcd3a1c60c91401e3bfa8f5fe25a987474f49fc4d952ef83f14f8c752a4`.
Native elapsed time was 493.187 seconds (493.214 seconds wall time). Every arm
completed 1,152 scored decisions, then repeated the full native assessment in a
fresh process. Every actor hash, weight, Adam moment, integer age and optimizer
step stayed unchanged: untouched remained step 400 and the other arms step 432.
All fresh-process/private/next-Adam checks passed. The prior V18 Grounded full
trace/private/next-Adam result also reproduces exactly with this binary. All 12
source recovery files and 21 prior public evidence JSONs remain byte-identical.

The cells below show **before / after induced sleep**, each on the same six new
seed units. The two phases are paired and do not constitute independent N=12.
"First phrase" means the actual first emitted Declare matches the requested
words before tick eight, independently of physical success. "Joint" additionally
requires the existing strict timely speech criterion and the real paired
physical distinction. Noun physical success means the first acquired object is
Food under GetFood and Toy under GetToy. Verb physical success means EatFood
consumes food while GetFood acquires food without consumption over all 24 ticks.

| Frozen source arm | Correct first noun phrase pairs / 6 | Correct first verb phrase pairs / 6 | Timely requested declarations | Joint timely physical/spoken noun pairs / 6 | Joint timely physical/spoken verb pairs / 6 | Physical noun pairs / 6 | Physical verb pairs / 6 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Untouched | 0 / 0 | 4 / 4 | 3 / 4 | 0 / 0 | 0 / 0 | 5 / 5 | 2 / 2 |
| Grounded | 5 / 6 | 4 / 6 | 7 / 9 | 2 / 2 | 1 / 1 | 4 / 4 | 2 / 1 |
| Shuffled | 0 / 0 | 0 / 0 | 3 / 4 | 0 / 0 | 0 / 0 | 5 / 4 | 0 / 0 |
| No new teacher | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 5 / 5 | 2 / 2 |

The first-phrase noun distinction transfers to these unseen physical worlds.
Grounded's pre-sleep miss is seed 4401393, which emits no phrase during any of
its three requested 24-tick trials. The other five seed pairs actually emit
GetFood `[9,1]` and GetToy `[9,2]`; all six do so in the paired post-sleep phase.
All three controls have zero correct noun phrase pairs in both phases.

Two seeds, 4104491 and 4209927, have joint timely noun outcomes in both phases:
real Food/Toy acquisitions at cue ticks zero/one are followed by matching actual
Declares at ticks one/two. In seed 4104491, GetFood subsequently consumes food at
tick five, so this noun success does not satisfy the separate full-window verb
criterion. The one joint verb success is **4209927 before sleep and 4401393 after
sleep**, not the same stable seed success counted twice. Their EatFood lives
really consume and gain energy at tick one, then declare EatFood at ticks two or
three; paired GetFood lives acquire food and do not consume it. No control has a
joint timely noun or verb pair on this new set.

These are limited descriptive transfer results from one trained founder. They
do not establish comprehension, reliable command following or a CFF ChiChi
capability level. Physical noun performance is not stronger than untouched or
no-new-teacher. Grounded has 6/9 premature requested phrases, and only 4/3 total
nourished meals across all cue/silence trials, compared with untouched's 8/8.
The stronger literal naming result must not hide these limitations.

## Silence, scored separately

Each phase contains six silent 24-tick lives. These counts use actual emitted
utterances, actual recent prior events and actual energy gains. They are not
conditional speech-head decodes or forced reports.

| Frozen source arm | Actual silent utterances | Recent factual activity declarations | Nourished meals | Taught-phrase declarations without a recent matching activity |
| --- | --- | --- | --- | --- |
| Untouched | 5 / 6 | 0 / 0 | 0 / 0 | 0 / 0 |
| Grounded | 4 / 5 | 0 / 0 | 0 / 0 | 0 / 0 |
| Shuffled | 4 / 6 | 0 / 0 | 3 / 3 | 0 / 0 |
| No new teacher | 4 / 6 | 1 / 1 | 3 / 3 | 0 / 2 |

Grounded's silent utterances contain other token sequences, with no taught phrase
or recent factual declaration. The no-new-teacher control actually produces one
recent factual declaration per phase; its post-sleep silent lives also produce
two taught phrases without a recent matching activity. Zero meals here is a
24-tick observation, not a general claim that an actor cannot eat. Prompted
naming transfer does not establish autonomous narration or capable self-care.

[Source-safe results](retained-language-unseen-transfer-results.json) include all
per-seed physical and actual utterance receipts, silent word sequences, fixture
parameters, pre-run seed checks, unchanged source hashes and cold continuity.
There was no offline actor update, new objective tuning, provider request or
Luna retry. No actor is promoted.

Validation: 21 native language tests passed (six explicit diagnostics skipped),
including all six real fixture controls and old-plan JSON compatibility. Strict
CPU Clippy including tests, headless compilation, formatting and Python checks
passed. PR #259's exact head
`f5d60c47e848f412c02edfa798236a8f9070e2d3` has three successful hosted workflows.
Its Jules comment reports that the review itself did not complete
(`IN_PROGRESS`); that workflow success is not approval.
