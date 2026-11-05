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
