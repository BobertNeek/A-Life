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
