# Disposable predictive-to-care smoke

This fixed exploratory comparison starts from the completed motion-coverage
trial's one-tick full predictive founder and its exact untrained source. It
does not select a source from downstream behavior. Original source files and
experimental states stay read-only; all outputs require a fresh directory.

The ordinary care runner requires terrain-vision sensors. This adapter is a
separate experimental typed-handoff path under the pilot's original object-slot
sensor profile. It leaves that ordinary admission guard intact, retains the
original genome/topology, and reuses the actual care scene preparation, teacher
exposure, world action validation, physiology, food observer and native CPU PPO.
It emits no normal care-lineage receipt and cannot promote an accepted source.

Both arms have independent native actor and critic objects. The pretrained arm
retains all predictive Adam moments/ages; the baseline retains the source's
moments/ages. Only the baseline actor learning rate is explicitly matched to
the pretrained actor's 0.0001 downstream setting; other Adam settings must
already match. Both critics start from independent copies of the same source
critic. Every export/rebind checks native weights, moments, ages, mask and
configuration continuity. Recoverable handoffs contain no auxiliary head.

The prespecified schedule is Feeding, RelocatedFood, Feeding, RelocatedFood at
world seeds 41001..41004, 16 loss decisions plus one real bootstrap observation
each, temperature 1, newborn care condition, one native PPO epoch per lesson.
No semantic prior operates in either arm. Training exposes ordinary teacher
cues; evaluation removes both teacher and prior through the existing care hooks.
Failures remain in native PPO rows. There are no imitation actions, fabricated
rewards or host-selected learner actions.

Before and after all four lessons, each arm evaluates the same four independent
world seeds 51001..51004, alternating Feeding/RelocatedFood, 16 scored decisions
plus bootstrap. These worlds are disjoint from predictive collection and care
training. Repeated before/after evaluation is diagnostic; there is no early stop,
test-selected checkpoint or promotion decision. Each evaluation creates a fresh
private life and verifies unchanged offline actor/critic checkpoints; ordinary
lifetime plasticity and biology continue within that life.

```sh
cargo build --locked -p alife_game_app --features cpu-training --bin run_predictive_care_smoke
target/cloud-cpu/debug/run_predictive_care_smoke \
  --state LOCAL_MOTION_RUN/state-h1-full.json \
  --upstream-result LOCAL_MOTION_RUN/result.json \
  --output NEW_DIRECTORY
```

`target/cloud-cpu` is the validation environment's Cargo target directory; use
your configured target directory elsewhere. The state must match the upstream
result's one-tick full-arm checksum and source identity. The upstream result
must identify the fixed motion-coverage seed-71 run and supply positive wall
time. The downstream report adds that complete upstream capture/conversion/
pretraining/control/evaluation duration to its own load/fine-tuning/evaluation/
export time. Cargo builds are reported separately. This conservative accounting
includes the upstream arms that are not used as the downstream founder.

Each episode retains actual raw native decisions, world outcomes, observed food
events, private recovery and checkpoints locally. Bootstrap behavior is retained
but excluded from scored/training behavior counts. Interrupted prefixes remain
available; an existing output is refused. Completed initial and per-lesson
native experimental handoffs are cold-admitted and checked exactly. These are
disposable recovery files, not accepted care sources or player saves.

This tiny sample can expose a broken handoff or obvious regression. Its meal,
acquisition, consumption, movement, blocked-action and failure counts cannot
establish general behavioral improvement or eventual useful capability per hour.

## Fixed CPU result

The single run used executable source `e69d7004bcd549ef7f268615816fbee4c015f4de`.
Both arms completed all four one-epoch native updates, with changed actor
weights each time. Baseline actor steps were 0 to 4; predictive actor steps
were 2,976 to 2,980. Both independent critics advanced from 0 to 4. All ten
initial/per-lesson handoffs were saved and cold-admitted exactly, without the
training-only predictive head. Retained masks, frozen weights/moments and
per-coordinate optimizer ages were also checked against every native update.

| Arm / phase | Scored decisions | Moved | Blocked / failed | Acquisitions / consumptions / meals |
| --- | ---: | ---: | ---: | ---: |
| Untrained / care training | 64 | 64 | 0 / 0 | 0 / 0 / 0 |
| Untrained / evaluation before | 64 | 64 | 0 / 0 | 0 / 0 / 0 |
| Untrained / evaluation after | 64 | 64 | 0 / 0 | 0 / 0 / 0 |
| Pretrained / care training | 64 | 64 | 0 / 0 | 0 / 0 / 0 |
| Pretrained / evaluation before | 64 | 64 | 0 / 0 | 0 / 0 / 0 |
| Pretrained / evaluation after | 64 | 64 | 0 / 0 | 0 / 0 / 0 |

All 408 actual native decisions were retained: 384 scored/training rows and
24 bootstrap observations. There were no downstream world-failure receipts;
the upstream trial's 169 failures remain retained. Every evaluated world had
zero teacher cue frames. The [complete per-episode behavior table](performance/predictive-care-smoke-behaviors.csv)
also records selected actions/lesson-target counts, travelled distance, energy,
health and failures, for every training and evaluation world in both arms.

Every episode travelled approximately 1.5972 world units in 16 scored decisions.
Health stayed at 1.0; initial energy was 0.7925 and final energy about 0.78984.
Before training, each arm selected Move as its primary representative 45 times
and Inspect 19 times over the evaluation dose; locomotion also executed when
Inspect was the primary representative. After training the baseline remained
45/19, while the pretrained arm was 44/20. This small action-count change yielded
no observed food-care benefit. Food starts 2.2–3.6 units away in the ordinary
scene preparation; this short newborn-condition dose offers sparse opportunity
for a complete meal. Neither the prediction gain nor the absence of meals
establishes the eventual value of predictive pretraining.

External monotonic executable times were 837.398 seconds for the entire upstream
motion capture/prediction/control trial and 120.945 seconds for this downstream
comparison: **958.343 seconds total (15m 58.3s)**. Internal timers sum to
958.291 seconds. The preceding incremental upstream build took 11.51 seconds;
the initial downstream binary build took 73 seconds, including newly needed
application dependencies. Those builds are excluded from executable time
(approximately 17m 22.9s including both reported builds). Compilation/checks and
implementation effort are not represented as useful training work.

The practical outcome is a working native predictive-to-care handoff and no
demonstrated downstream improvement at this budget. A more informative future
care assay would need enough time and ordinary biological need for actual food
interaction; this run's schedule was retained rather than extended after seeing
its result. No accepted source, recovered cloud lineage or PC was changed.

The public behavior table can be reproduced from local retained traces:

```sh
python scripts/summarize_predictive_care_smoke.py \
  --input LOCAL_CARE_RUN --output NEW_BEHAVIOR_TABLE.csv
```

The summarizer checks all episode doses/bootstrap flags, teacher-free evaluation,
recorded food/movement/failure counts, native optimizer history, frozen
coordinates, identical initial critics and all 24 episodes. Its output reproduced
the committed CSV byte-for-byte. The CLI also rejected an existing output with
the complete retained inventory unchanged, and rejected a four-tick state against
the preselected one-tick plan before creating output. The original experimental
state and upstream result were verified unchanged after the completed run.

Validation also passed the existing actual-world care assessment regression,
strict Clippy for the library/new CLI, formatting and feature-disabled compilation:

```sh
cargo test --locked -p alife_game_app --features cpu-training --lib \
  assessment_removes_teacher_and_prior_but_preserves_real_private_outcomes_and_full_optimizers -- --nocapture
cargo clippy --locked -p alife_game_app --features cpu-training --lib \
  --bin run_predictive_care_smoke -- -D warnings
cargo fmt --all -- --check
cargo check --locked -p alife_game_app --no-default-features
```
