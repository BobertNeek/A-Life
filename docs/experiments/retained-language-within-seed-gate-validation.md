# Frozen gates on matched within-seed report states

The frozen five-tape gate separator does not transfer as a complete factual-report
correction. On 16 actual native CPU decisions, it preserves **0/1** supported
reports and silences **1/3** unsupported reports in offline captured-state scoring.
The other 12 rows abstain because no actual matching Declare was emitted. These
are four report states across two seeds, including only **one** matched
supported/unsupported pair. No hypothetical gate was installed in a creature.

[Measured receipt](retained-language-within-seed-gate-validation-results.json)
contains every scored row, matching evidence, first setup failure, frozen
coefficients, provenance and hashes. Full native rows, pre/post private checkpoints,
optimizer/source actors and executables remain local. All 360 protected prior
files remain byte-identical. No accepted/default brain, engine guard or gameplay
schema changed.

## Bounded native protocol

Reuse the two actual saved V26 common starts, seeds 19310729 and 19508423, and its
Untouched actor at optimizer step 436. These seeds were previously assessed in
V26 but excluded from the V27 five-tape fit; they are not a newly independent seed
cohort. Each source is cloned into near/far GetFood episodes, four decisions each:
**16 total native decisions, zero preparation decisions**, gradient probes,
native/shadow Adam updates, teacher/prior calls or external model calls.

All initial fields compare exactly except food position and that same food's
derived voxel-resource tile. The same food instance is 0.37 or 1.60 units away,
at relative angle 0.15. Native `can_grab_target` confirms near legal and far illegal;
the native contact radius is 1.25. Other objects, stationary velocities, body,
biology, memories, recurrent/private state, policy binding and RNG are identical
within each initial pair. The standard editor authors stationary initial fixtures;
it creates no acquisition/consumption event, credit or reward.

Player GetFood tokens `[9,1]` arrive at offsets 0 and 2 from the same body-relative
speaker position. Cue-only checkpoint checks hold every non-audio field fixed.
Native selection, cooldown, biology and somatic learning run normally on the
disposable copies. Each step reserves its budget slot before execution and records
the full native row, complete decoder inputs/offsets, all six raw speech-head steps,
action/outcome evidence, actual utterance, ownership, safety and complete pre/post
host state. Every global weight, moment, integer age, mask and config stays exact.
The four final private states restore exactly in-process without another step.

Only actual matching GetFood Declare emissions receive a label. A prior witnessed
GetFood ownership event within three ticks supports a report. An unsupported
observed-prefix report requires no current or prior matching event, food unheld
before/after, and the saved initially unheld source plus its preserved prehistory.
Unlabeled rows do not become negative examples. No target is sent into the actor.

The margin-0.10 presence/stop coefficient vectors from V27 are fixed by SHA256
before collection. Offline readout reuses actual captured neural features and
somatic offsets, with no fitting or new neural forward. All **3072** captured
source raw speech values reproduce bit-for-bit. Counterfactual gate outputs are
single-state arithmetic, not a changed-policy rollout or new actual utterances.

## Actual events and transfer

Both near episodes genuinely acquire food; neither far episode does. No episode
consumes food in these four-decision GetFood windows. All rows survive, health is
1, injury/pain 0, minimum energy 0.79099298. The actor actually emits four GetFood
declarations: one supported and three unsupported under the reporting protocol.

| Seed, offset | Actual event support | Actual phrase | Frozen offline payload | Result |
|---|---|---|---|---|
| 19310729, near 0 | No prior/current event | GetFood | Get only | Unsupported report not silenced |
| 19310729, far 0 | No prior/current event | GetFood | Get only | Unsupported report not silenced |
| 19508423, near 1 | Ownership obtained at offset 0 | GetFood | Food only | Valid report damaged |
| 19508423, far 1 | No ownership/event | GetFood | Silence | Unsupported report silenced |

The 19310729 near episode acquires food at offset 1, after its actual declaration;
that later outcome does not retroactively support the earlier factual report.

The single matched report pair, seed 19508423 offset 1, has exactly equal actual
heard words, native word/confidence lanes, prompted/unprompted cooldown readiness,
world tick, sampling counter and effective speech-weight offsets. Both actually
select Vocalize and emit GetFood. In the near prefix, native Eat (210) first picks
up food without consuming it; this creates a real GetFood ownership event. The
far prefix selects Inspect (4) and gains no ownership. These are real differences
in selected actions and consequences, not evaluator-assigned transitions.

| First-step raw gate | Supported near | Unsupported far |
|---|---:|---:|
| Native presence | 1.515378 | 1.395311 |
| Native stop | -1.059136 | -0.975940 |
| Frozen presence | -0.043621 | -0.097172 |
| Frozen stop | -0.018660 | +0.057531 |

The frozen stop signs distinguish this within-seed pair correctly. Presence
rejects the valid first word too; decoding then emits the noun alone. Thus the
combined correction fails the paired criterion. A first-step positive stop can
also truncate a declaration rather than make it silent, as on seed 19310729.

The paired 32-input vectors have L2 distance 0.586946 and cosine 0.999047. With
identical current speech offsets, the within-seed gate differences come from the
captured decoder inputs. This rules out a purely constant-per-seed response for
the observed stop distinction. It does not establish detection of event validity:
food distance, action identity, ownership and subsequent sensory, physiological,
recurrent and memory histories differ together. The representation has no exact
collision on this pair, but the frozen correction has not demonstrated reliable
validity generalization. No exploratory refit or new training follows this result.

## Preservation, failures and reproduction

A pre-run source check corrected the planned far distance 1.10 to 1.60 before any
native decision. The first instrumented attempt then stopped at the setup-equality
guard with **zero** decision reservations. Its only additional serialized difference
was the food's position-derived voxel tile (`x=4` to `x=3`). The corrected matching
proof excludes precisely that same food position/tile and keeps all unrelated
world/private/RNG fields bound. The failed log, source, executable and initial
preregistration remain intact. Ordinary admission guards were not changed.

The successful ignored test passes in 14.29 seconds (14.30 wall), with exactly
16 decisions. Strict CPU library/test Clippy and formatting pass. Measured source
is `bc4be649147be920d358a0fce1c96b69a2659eda`, executable SHA256
`5d65b465f321e90c9b35f5d8f812bb398989859f10fb6542092131726270021c`.

The opt-in test is
`foundation_training::retained_language::curriculum::factorial::balanced::within_seed_gate_validation::record_matched_food_report_opportunities`.
It requires `ALIFE_WITHIN_SEED_SOURCE` (V24), `ALIFE_WITHIN_SEED_V26` and a new
`ALIFE_WITHIN_SEED_OUT`. Build with CPU training, test debug=0, incremental=0 and
`profile.test.package.alife_training.opt-level=2`. It refuses step 17 and existing
output directories. Do not rerun a partially completed collection: inspect saved
reservations/rows and preserve its consumed budget first.

`scripts/summarize_within_seed_gate.py` scores saved rows with `--run`, `--fit`,
`--actor`, `--preregistration` and `--output-dir`. It verifies the frozen-fit/source
hashes, native f32 arithmetic, original-file preservation and the physical matching
contract. It performs no refit, model call, simulation or optimizer update.

Source-only publication uses `experimental/within-seed-gate-validation-20261105`.
This task prohibits external model calls; repository automation creates a Jules
session for a new PR or new `codex/**` branch. No new PR or such model-triggering
branch publication occurs in this task. No hosted CI or substantive review is
claimed for this source head; the validation above is local.
