# Requested physical action before report

This is one bounded, training-only continuation of the disposable V18 Grounded
actor at native optimizer step 432. Untouched, grounded, shuffled and
no-new-teacher clone the same full source actor, moments, integer ages,
configuration and recorded private state. Trained arms receive one native
collection round and 16 updates, reaching step 448. Untouched takes none.

The prior preserved V19 trace distinguishes two problems. In seed 4507581,
GetFood selects Vocalize from cue tick zero, including during cooldown, and
never acquires Food. In seed 4401393, GetToy instead acquires Food first in both
the older and corrected actors. The existing action cross-entropy and factual
payload targets therefore do not reliably separate the requested physical action
from voice or the wrong object under native sampling. This is a testable objective correction,
not proof that one mechanism explains every failure.

The new `--physical-before-speech` stage adds only a native action-logit margin:
`max(0, 0.5 + strongest_legal_competitor - requested_physical_candidate)`. It
applies to the existing Concrete rows while the requested activity is
unfulfilled, within the eight-tick instruction window. Its total mass is 0.2,
normalized equally across the three concrete physical command classes. This is
an objective coefficient, not a probability. It uses the same native replay,
recurrent backward path, readout trainable mask and Adam implementation. A real
row finite-difference test checks the native gradient. A separate disposable
probe on the captured V19 failure row reports native logits and one shadow Adam
step without any world action or retained-learning claim.

Factual speech supervision, the noun margin, fulfilled/other labels and every
previous objective coefficient are unchanged. Reports still require a genuine
prior matching event and existing cooldown eligibility. No event/cooldown or
completion label is added to the creature's inputs. All training transitions,
physiological rewards and speech emissions are native. Evaluation actions and
utterances are sampled normally; no output filter, forced action, cleared
cooldown or runtime vocabulary remap is introduced. Previous modes remain
available, and false/default assessment flags are omitted from old plan JSON.

Before any new outcome, six fresh held-out world/sampling seeds were fixed:
6104531, 6210019, 6309173, 6401879, 6508117 and 6610327. They do not occur in 45
retained prior receipts (30 distinct recorded seeds). The six V19 geometry,
food-size and nutrition tuples are reused, with new object IDs from
379 + 23 * case padding. Physical feasibility controls use scripted native
commands outside assessment; they are not learning or scored behavior.
Training uses the existing four nursery seeds, independently of this fresh set.
The plan is one round x16 updates without tuning after these results.

Teacher/prior-off assessment retains four counterfactual cues including silence,
24 ticks each, normal preparation and 0/2/4/6 player cue timing, six paired seed
units pre/post induced RecoveryProtocol sleep, and full cold recovery. The
post-sleep condition clones the prepared parent and sleeps before commands; it
does not continue a completed pre-sleep command life or double independent N.
It has no spontaneous-sleep or matched awake-delay control. One founder and
familiar word combinations do not establish comprehension or general safety.

Actual timely physical-and-spoken pairs, literal first naming, physical
acquisitions/consumption, premature speech and silence are separate measures.
Read-only authoritative post-native-tick receipts add alive, health, injury,
energy and pain for every scored decision. Safety results apply to these short
food/toy/distractor fixtures. Native within-life private-state evolution and
biological plasticity remain enabled. Cold next-Adam checks use the saved real
gradient on disposable copies, not freshly recomputed gradients or updates to
the source actor.

```sh
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --physical-before-speech /absolute/path/to/measured-V18-run --output NEW_DIRECTORY \
  --seed 847501 --rounds 1 --epochs 16 --decisions 24
python scripts/summarize_balanced_grounding.py NEW_DIRECTORY
```

All earlier results/checkpoints remain intact. No accepted/default brain or live
training is changed. The teacher is scripted ordinary spatial nursery speech;
no real SLM request, Luna 401 retry or candidate promotion is authorized here.

The read-only captured-row probe reproduces the source row digest exactly. Food
has native logit 1.790566 and Vocalize 1.526378: Food already ranks first, but
its 0.264188 gap is small under temperature-one sampling, and the actual choice
is Vocalize. The 0.5 margin is active (loss 0.235812). One disposable native Adam
step reduces this row's margin to zero with gradient norm 12.036429; source actor
and every frozen coordinate remain exact. This is an in-sample path/optimizer
diagnostic, not retained learning or world-behavior evidence.

## Measured result: failed grounded coverage

The bounded run completed at source
`e203b5a4ff070ea48bb8e249caaf83ff3203b738`, binary SHA-256
`504357e45bc4cb037f10fe79093ffd0b16dcdb0f2b3fd305cebb2dcf014e0ee2`.
Native elapsed time was 598.419 seconds (598.453 wall seconds). All four full
1,152-decision assessments and exact fresh-process/private/next-Adam repeats
passed. Untouched remained step 432; each trained arm reached 448. Every frozen
coordinate stayed exact. All 24 original V18/V19 recovery files and 22 prior
public evidence JSONs remain byte-identical. Historical V19 Grounded full
trace/private/next-Adam continuity also reproduces exactly with this binary.

**The added margin contributes zero gradient to Grounded in all 16 updates.**
Its 21 distinct eligible training rows already clear the 0.5 margin at every
epoch. Shuffled has 35 eligible rows and 480 active row/update pairs; no-heard-
teacher has 24 eligible rows and 348 active pairs. The captured V19 failure row
has an active margin, but the actual Grounded nursery collection does not visit
that deficit. This is a curriculum-coverage failure, not a demonstrated repair
or an efficacy test of active Grounded margin training. Geometry, speaker/source
kind and private history differ between the nursery rows and the captured
failure; their individual contributions to this gap were not isolated.

The original action and factual speech losses still train Grounded for 16 steps.
Its observed regression is therefore a continuation of those existing objectives
and optimizer history, without a new-margin contribution. Which unchanged loss
or state-context effect causes the regression is not isolated. No additional
matched continuation-without-margin arm was run, and the fresh set was not used
to tune or rerun this candidate.

Each cell below is **before / after induced sleep**, with six paired seed units
per phase, not independent N=12. Correct first phrase pairs require actual
matching Declares before tick eight. Joint speech/physical success additionally
requires the strict prior-event timing and actual paired physical distinction.

| Arm | Correct first noun phrase pairs / 6 | Correct first verb phrase pairs / 6 | Physical noun pairs / 6 | Physical verb pairs / 6 | Timely requested declarations | Joint timely noun pairs / 6 | Joint timely verb pairs / 6 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Untouched V18 Grounded | 5 / 6 | 2 / 6 | 5 / 4 | 0 / 0 | 2 / 10 | 0 / 2 | 0 / 0 |
| Grounded continuation | 1 / 0 | 2 / 4 | 4 / 3 | 0 / 0 | 2 / 7 | 0 / 0 | 0 / 0 |
| Shuffled continuation | 0 / 0 | 0 / 0 | 4 / 3 | 1 / 1 | 0 / 0 | 0 / 0 | 0 / 0 |
| No new heard teacher | 0 / 0 | 0 / 0 | 5 / 4 | 2 / 2 | 1 / 2 | 0 / 0 | 0 / 0 |

The named noun-pair metric requires GetFood `[9,1]` and GetToy `[9,2]`, including
the correct Get verb in both phrases. Grounded instead produces **EatToy
`[8,2]` under GetToy in four pre-sleep and five post-sleep lives**. The Toy noun
token is correct, but the verb is wrong. Grounded emits GetFood correctly in all
six pre-sleep lives, then five after sleep; the other post-sleep GetFood life
emits EatFood `[8,1]`. This is a speech-verb regression, not evidence that the Toy
noun token alone was forgotten. No EatToy target was taught: Grounded's real
factual labels are seven EatFood, six GetFood and five GetToy rows, all with
matching first two heard teacher words. There are 16 cooldown-legal report labels
and two blocked/Idle labels. Shuffled has eight factual rows (zero aligned heard
pairs); no-heard-teacher has 12, with no heard tokens. All event/legality checks
pass. No absent activity is manufactured to balance these counts.

## Timing diagnostics, including wording failures

The original premature-*requested-phrase* count is 2/7 for untouched and 1/3 for
Grounded. That narrow count misses the new EatToy errors. After seeing this
regression, the audit added the following **post-hoc diagnostics**, retaining all
original preregistered metrics and applying the same definitions to every arm.
Precompletion means before a prior actual completion of the requested activity;
these counts include blocked voice choices and any emitted Declare wording.
They do not claim every precompletion vocalization is semantically inappropriate.

| Arm | Precompletion Vocalize selections | Precompletion actual Declares, any wording | EatToy Declares under GetToy |
| --- | --- | --- | --- |
| Untouched | 27 / 33 | 2 / 7 | 0 / 0 |
| Grounded | 23 / 22 | 1 / 6 | 4 / 5 |
| Shuffled | 27 / 32 | 3 / 7 | 4 / 4 |
| No new heard teacher | 34 / 38 | 2 / 4 | 0 / 0 |

Grounded selects voice less often before fulfillment, but this does not improve
physical noun performance or joint timely outcomes. Its actual early Declare
count improves by only one per phase while phrasing worsens. A lower requested-
phrase premature count is not sufficient evidence of repaired timing.

## Physical safety and silence

Across all 4,608 scored native decisions (192 short lives, followed by the exact
cold repeat), every recorded organism is alive, minimum health is 1.0, and maximum
injury/pain are zero. The lowest recorded energy is 0.787891, in no-heard-teacher
before sleep. These are measurements in short nonlethal food/toy/distractor
fixtures, not a claim of general safety or autonomous CFF capability.

Each phase includes six silent 24-tick lives:

| Arm | Actual silent utterances | Recent factual declarations | Nourished meals | Taught phrases without recent matching activity |
| --- | --- | --- | --- | --- |
| Untouched | 1 / 5 | 0 / 0 | 1 / 2 | 0 / 0 |
| Grounded | 1 / 4 | 0 / 0 | 1 / 2 | 0 / 0 |
| Shuffled | 0 / 4 | 0 / 0 | 0 / 1 | 0 / 0 |
| No new heard teacher | 1 / 6 | 0 / 0 | 1 / 2 | 1 / 5 |

Grounded does not improve silent factual narration or feeding. Total nourished
meals over all cue/silence trials are 5/8 untouched, 6/8 Grounded, 5/7 shuffled and
10/12 no-heard-teacher. No-heard-teacher omits ordinary nursery speech but retains
the common learned history and the same detached, event-based offline targets;
it is not language-naive or a no-supervision arm.

[The source-safe results](retained-language-physical-priority-results.json)
contain every seed/phase outcome, safety receipt, actual words, post-hoc counts,
training-label checks, per-epoch intervention exposure, original-file hashes and
the separate read-only native probe. The original actor/checkpoints remain the
reference; this continuation is not promoted and does not establish
comprehension or a practical repair of the observed limitation.

Validation passed: 22 native language tests (seven explicit diagnostics skipped),
the separately invoked captured-row native probe, native margin finite
differences, old and fresh physical fixture controls (12 layouts), old-plan JSON
compatibility, strict CPU Clippy/tests, headless compilation, formatting and
Python checks. PR #260's exact head
`46f9d9be4b4d08bb0b5819386ea15b6eb71b07ed` has three successful hosted workflows;
Jules says its review itself did not complete (`IN_PROGRESS`), not approval.
The scripted-teacher-only limitation and Luna WebSocket 401 remain unchanged;
no endpoint, credential, provider or access retry occurred.
