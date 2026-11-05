# Genuine care replay during grounded failure correction

The four/eight-update comparison showed substantial food-behavior regression
already by four updates. This bounded experiment adds native successful care
experience to the existing witnessed acquisition-failure objective. It changes
neither the accepted brain nor the underlying optimizer/physics.

All models start from the unchanged public V18 Grounded step 432 actor. The
preserved V21 grounded/no-command failure corpora retain three/eleven actual
failure/success pairs. Care is collected by ordinary native sampling in twelve
retired 6M/8M worlds: up to 24 silent ticks per world, no induced sleep, teacher,
player cue, sampling fork or forced action. Admit at most one row per world only
if the actual selected legal EatFood candidate really consumes food and gains
energy. Original biological rewards, world outcomes, inputs, private recurrent
state and offsets remain unchanged. Natural self-speech is permitted; external
speech in an admitted care row is rejected. These worlds cease being held out.

A separate preflight-only run verifies actual nourishing rows and nonzero native
care/failure/mixed gradients before any update or new assessment. A central
finite-difference check at the strongest trainable care coordinate validates the
native care objective on its real captured states. This numerical probe is not
behavioral learning; disposable perturbations never alter the source actor.

Five conditions: untouched, FailureOnly4, CareOnly4, MixedGrounded4 and
MixedNoCommand4. Every trained model takes four native AdamW updates with the
same configuration, history and ActionCandidate-only mask. FailureOnly4 must
exactly reproduce the preserved four-step actor and update receipts. CareOnly4
uses mean native representative-imitation CE for the actually successful EatFood
choices. Mixed models use a fixed 0.5 failure-margin plus 0.5 care-CE gradient,
without tuning after held-out results. Both mixtures share identical silent care
rows. MixedNoCommand4 also uses no-command failure rows, so no externally spoken
training commands appear in either component; earlier grounded history and
physical imitation labels remain. No new speech objective is introduced. Speech
and all other nonaction weights, moments and integer ages stay frozen, while
native biological/private learning remains enabled.

Fresh assessment seeds are 12104621,12210103,12309269,12402009,12508237,12610457;
object identity padding 1103+37*case. They reuse familiar layouts, food properties
and words, with one retained founder. Each model has six paired pre/post induced
RecoveryProtocol-sleep conditions, 24 decisions under each EatFood/GetFood/GetToy
request and silence, ordinary requests at 0/2/4/6, teacher/prior off, native
sampling/cooldown/biology and actual physical/utterance outcomes. Food consumption
under EatFood is reported separately from meals under other requests; injury,
health, energy and pain are separate from cue-sensitive speech and joint timing.
No forced evaluation actions, hidden physical goals or output filtering.

Fresh cohorts preserve actor/optimizer history rather than the original source
individual private life. Full cold replay verifies each new private life, native
rows and saved-real-gradient next-Adam continuity. Sleep phases share seed parents,
not independent N12, a continuous post-command life, spontaneous sleep or an
awake-delay causal control. Untouched and no-command controls retain past grounded
learning; neither is language-naive. There is no new shuffled-word expansion.

```sh
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --care-retention-preflight MEASURED_V21 --output PREFLIGHT_DIRECTORY \
  --seed 847501 --rounds 1 --epochs 4 --decisions 24
experiment_retained_language --disposable-public-founder --experimental-word-bridge \
  --care-retention MEASURED_V21 --output NEW_DIRECTORY \
  --seed 847501 --rounds 1 --epochs 4 --decisions 24
python scripts/summarize_balanced_grounding.py NEW_DIRECTORY \
  Untreated FailureOnly4 CareOnly4 MixedGrounded4 MixedNoCommand4
```

All earlier negatives and source checkpoints stay intact. This is one practical
retention-objective test, not a comprehension/promotion claim, default/live brain
migration, learned SLM delivery or Luna401 retry.


An initial preflight caught an invalid care target: candidate index 3 was passed
where the imitation API expects the one-bit mask 8. Its numerical derivative
was internally consistent with the wrong objective; gradient existence alone
did not verify the intended label. The actual care events were genuine. That
invalid diagnostic, source and executable are preserved separately; it applied
zero optimizer updates and scored no assessment decisions. The corrected path
uses the recorded native candidate mask and checks each source representative's
CE likelihood against its actual full native joint-action likelihood, before
the finite-difference probe or any training. No invalid-target result counts as
retained learning or as an evaluated treatment.

Measured physical gain and unresolved language result
-----------------------------------------------------

The valid run completed on CPU in 621.273 seconds (621.301 wall), measured source
`547599c5c0cda3e7efbf5c013e7b0f093a81714b`, executable SHA256
`7dabbfbc038ac238ab25e84f879e2c2af53bc6db188bb7ecc13dae08c1b8ae6c`.
[Public receipts](retained-language-care-retention-results.json) contain every
update, the actual care-source/target audit, separate per-seed outcomes,
optimizer continuity, preservation hashes and limitations.

Valid care data are two natural silent meals, in retired worlds 6309173 (case 2,
tick 2) and 8104543 (case 0, tick 3), with genuine energy gains 0.12500149/0.11918092.
Each selected native candidate 3 is action 210 (EatFood), target bitmask 8, with
actual food contact Consumed. No external or self-speech tokens were heard.
Those experience bytes are identical across invalid/corrected/main collections;
the invalid mask diagnostic applied zero updates and scored no treatment.
Corrected representative CE losses 1.764679/1.115969 match recorded native joint
negative log probabilities 1.764685/1.115971. Native care gradient L2=34.176097;
strongest coordinate derivative −4.337428 matches finite difference −4.337430,
relative error 4.4e−7. Grounded/no-command mixture norms 17.473266/17.418091 and
failure-care cosines 0.02036/0.03610 were verified before any optimizer update.
Equal loss weights do not imply equal gradient norms; there was no adaptive
balancing or post-assessment tuning.

Counts below are pre/post induced sleep, each with six paired seed units. Do not
add phases as independent samples. First-spoken pairs require the actual first
utterance to be a Declare matching both requests in the 8-tick cue window. Timely
speech requires a genuine prior matching event within 3 ticks. Strict joint verb
pairs additionally require EatFood consumption and no GetFood consumption across
all 24 ticks; the cue-window physical verb metric is reported separately.

| Measure | Untouched | Failure4 | Care4 | Grounded+care4 | No command+care4 |
|---|---:|---:|---:|---:|---:|
| EatFood nourishing trials within 8 ticks /6 |0/0|0/0|4/4|4/4|4/4|
| GetFood meals (24 ticks) /6 |0/0|0/0|4/4|2/2|2/2|
| GetToy meals (24 ticks) /6 |0/0|0/0|1/2|0/0|0/0|
| All nourished meals (24 trials/phase) |0/0|0/0|10/12|7/7|6/6|
| Physical noun pairs in cue window /6 |4/3|3/2|4/3|4/3|4/3|
| Physical verb pairs in cue window /6 |0/0|0/0|2/2|2/2|2/2|
| Strict physical verb pairs, 24 ticks /6 |0/0|0/0|0/0|2/2|2/2|
| First-spoken noun pairs /6 |4/5|3/3|5/5|5/5|5/5|
| First-spoken verb pairs /6 |3/6|1/1|6/6|6/6|6/6|
| Timely requested declarations |3/6|4/6|4/8|4/8|4/8|
| Joint timely noun pairs /6 |0/1|1/1|0/0|0/0|0/0|
| Joint timely strict verb pairs /6 |0/0|0/0|0/0|1/2|1/2|
| Premature requested declarations |7/11|3/4|7/8|7/8|7/8|
| Precompletion Vocalize selections |54/59|18/15|74/88|72/74|70/82|
| Silent-trial utterances (6 trials/phase) |3/5|2/3|3/6|3/6|3/6|
| Silent-trial nourished meals |0/0|0/0|1/2|1/1|0/0|

Care rehearsal yields a real held-out physical gain, retained through exact cold
replay and present in both paired sleep conditions. Untouched and failure-only
produce no food consumption on this new seed set. That floor prevents a direct
successful-food retention comparison here: the result is a capability gain,
not proof of preserving existing Eat successes in those six worlds. Earlier
positive-food and negative continuation receipts remain unchanged.

Both mixtures reduce later GetFood consumption compared with care-only, producing
strict 24-tick verb pairs on seeds 12210103 and 12402009 in both phases. This does not
improve their initial 8-tick verb pair count over care-only. Timely strict joint
verb success occurs on 12402009 before sleep and both those same seeds afterward,
not three independent successes. Grounded and no-new-command mixtures match on
all aggregate primary requested scores and those exact physical/joint verb seed
sets. No incremental benefit from fresh heard grounded commands is established;
all models retain earlier grounded history. These are narrow physical/utterance
outcomes, not demonstrated new word learning or comprehension.

Care-only can eat under GetFood/GetToy as well as EatFood; event timing and
nourishment are separate from request compliance. The mixtures retain baseline
noun acquisition rates but produce zero timely joint noun pairs, and premature
speech remains common. Neither timing nor broad language competence is repaired.
No candidate is promoted.

All 5760 scored native ticks stay alive with health 1, injury 0 and pain 0; minimum
energy 0.787897. These are short nonlethal fixtures. All five cold/native-private/
full-trace/saved-real-gradient next-Adam checks match exactly. Trained actors
advance 432→436; all 3072 action ages advance by 4, while all 29696 nonaction weights,
moments and integer ages stay exact. Untouched is entirely identical. Failure4
exactly reproduces V22 Grounded4 actor and all updates. Historical V22 full
assessment/private/next-Adam replay is exact on a separate disposable copy.

All 74 original recovery/entire V21/V22 run files, 25 prior public evidence files,
54 seed-evidence receipts and 7 invalid-diagnostic artifacts remain byte-identical.
The corrected source passes 25 native language tests (7 explicit diagnostic ignores),
30 old/new physical visibility/edibility/nourishment layouts, strict Clippy,
headless check, formatting and Python validation. #263 exact head
`681261c5ce8f2e27a9b5b9c868a0acc474f1cb33` has three successful hosted workflows;
Jules comment 6646975764 says its actual review did not complete (IN_PROGRESS).
No learned SLM/provider request, Luna401 retry, accepted/live checkpoint mutation,
default brain migration or promotion occurred.
