# Inspect existing prior delivery and teacher cue receipts

Run the standard-library, read-only audit against one or more already-created
probe directories or `pilot.json` paths:

```powershell
python scripts/audit_prior_receipts.py PATH_TO_GRAB_PROBE PATH_TO_EAT_PROBE
```

The CLI prints JSON to stdout. It reads `pilot.json` and optional sibling
`semantic-prior-readiness.json` and `lesson-trace.json`. For unsuccessful lessons,
it uses `lesson-diagnostic.json` when `lesson-trace.json` is absent. It never
launches a learner, neural GPU execution, provider, trainer, or replay inspector,
and never modifies artifacts, weights, or checkpoints.

Provider backend, identity, model, and provenance are **declared receipt values**;
the tool does not independently authenticate them. It selects `semantic_prior`
from the pilot first, then readiness, then trace, and names its source. Conflicting
copies produce warnings and are not silently merged. Missing or malformed files
and fields remain `null` (Unknown); measured zero remains zero. Episode `ticks`
never fill absent prior decision totals.
Provider failure, dropout, timeout, and empty-hint counts remain visible even
when the most recent observed decision inputs were delivered.

`decision_inputs` reports observed rows by status. Confirmed delivered/encoded
rows require unique, valid nonzero organism/sequence identities, a valid tick, native
`delivered` status, and nonzero prior and encoded lanes. Other recognized statuses
are reported as missing or failed delivery; malformed or contradictory delivery
rows remain unknown. Coverage is complete only when observed identities, statuses,
and counts agree with all declared decision totals. The native metrics retain at
most 256 recent decision inputs, so a shorter trace cannot establish lifetime
coverage. Prepared deliveries and cache hits are not confirmed decision inputs.

`teacher_cue_frames` measures exposure to **any** configured Teacher token. It
does not prove hearing the whole cue. The audit retains matching raw evidence for
the exact configured tokens, ordered by native `sequence_position` starting at
zero, in one Teacher utterance within one decision frame. Utterance, speaker,
source entity, and addressee must be consistent; duplicates, split utterances,
other speaker kinds, and reordered tokens cannot establish a full cue.

Generic ordered Teacher evidence is reported separately from full heard-cue
evidence. Hearing requires every matched token's `teacher_channel` to be
`Hearing`. Vision and other channels do not count as hearing; missing channels
leave hearing unknown. The expected learner is read from `grab_food_setup.organism`
or `held_food_setup.organism`, with conflicting identities reported as unknown.
Another explicit addressee does not count as the lesson learner's heard cue.
An explicit `null` addressee is a broadcast: hearing exposure is retained and
clearly labeled as not learner-directed. An absent addressee field is unknown.

Native heard arrays have 16 optional slots. The full-frame total remains unknown
when raw fields, learner attribution, or episode coverage are incomplete; observed
matching evidence is still retained. Hearing and prior delivery do not establish
comprehension or that the creature used a hint.

Warnings are diagnostic under `log_and_continue_without_prior`. Exit zero means
the report was produced, including unknown evidence or unavailable providers; it
is not episode admission, behavioral success, learning, or promotion. This tool
automates raw receipt review without changing existing behavioral validators.

Schema references inspected at PR39 `d5dd288cbbcb0dd944e11ec433332e5071b3bb2c`:

- `crates/alife_game_app/src/gpu_live_runtime/semantic_prior.rs` defines metrics,
  confirmed input fields, statuses, and the bounded receipt queue
- `crates/alife_game_app/src/foundation_training.rs` writes pilot/readiness/trace
  receipts and defines the any-token Teacher exposure count
- `crates/alife_core/src/sensory_abi.rs` defines `HeardToken` and its 16-slot array
- `crates/alife_school/src/language_nursery.rs` emits ordinary lesson speech in
  the Hearing channel, directed to the subject named in lesson setup

The prior schema is identical at PR38 `791df127fec5ba1f242e353906a7f5f6ea5bed75`
and PR40 `d1ee947fad980d4adcdfd08cb85b0068a73aef80`.

Run the synthetic receipt tests without creating a learner or provider:

```powershell
python -m unittest discover -s scripts -p 'test_audit_prior_receipts.py' -v
```

These fixtures establish host reporting behavior only. They do not establish
actual policy2502/policy3179 provider delivery or frozen behavioral results.
