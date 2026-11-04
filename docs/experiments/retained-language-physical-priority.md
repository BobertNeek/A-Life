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
