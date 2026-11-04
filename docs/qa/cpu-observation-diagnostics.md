# CPU care observation diagnostics

These opt-in developer diagnostics explain ordinary observation admission. They
do not add observations, targets, goals, rewards, teacher cues or route knowledge
to the creature. Keep generated traces and checkpoint inspection output private.

Set `ALIFE_CPU_CARE_OBSERVATION_DIAGNOSTICS=1` when running an otherwise admitted
CPU care command built from this source. The default trace and the existing
`food_visible` and completion fields keep their existing meanings. In particular,
`food_visible` means a food-target candidate exists, not geometric visibility.

The additional `observation_diagnostics` row records:

- The body/head pose, food/gate geometry and native optical admission before the
  host step, after lesson preparation.
- The actual prepared perception slots, primary focus, complete candidates and
  homeostasis used by that decision. A tick mismatch fails the diagnostic run.
- A previously observed food identity only when the actual candidate transport
  identity maps to an actual observed slot. A missing identity remains unknown.
- Separate optical, contact and straight-segment facts. Global terrain route
  reachability remains `not_assessed`; these facts are not a path planner.

Optical reasons distinguish missing/consumed objects, range, gaze, canonical
occlusion, the physical observation budget, calibration and successful sensing.
Reason selection has precedence: for example an out-of-range object may also be
outside the gaze angle. The individual booleans remain available.

Use an independent build destination if an accepted production binary is present:

```sh
export CARGO_TARGET_DIR=/path/to/disposable-build
cargo test --release --offline -j1 -p alife_world --lib observation_diagnostics::tests
cargo check --release --offline -j1 -p alife_game_app --no-default-features --features cpu-training --lib
cargo test --release --offline -j1 -p alife_game_app --no-default-features --features cpu-training --lib foundation_cpu_care::collect_only_tests::observation_diagnostics_preserve_native_fixture_actions_and_private_state -- --exact
cargo build --release --offline -j1 -p alife_training --features cpu-backend --example inspect_cpu_observation_terminal
"$CARGO_TARGET_DIR/release/examples/inspect_cpu_observation_terminal" /path/to/sealed/cpu-episode
```

The inspector admits the saved collection source through native tooling and
restores the collector under that collection policy, rather than the later
updated policy. It verifies an exact full checkpoint round trip, performs no
neural dispatch or optimizer update, and reports only selected terminal geometry,
tracking and hysteresis evidence. Its persisted food identity is matched through
the saved object's provenance. `last_seen_tick` is a raw observation record,
before the candidate budget. It is not the first-seen time, a full observation
history, proof of candidate admission or evidence of route accessibility.

Terminal pose and hysteresis cannot reconstruct missing historical per-decision
pose, focus, candidates or ray-rejection reasons. No historical replay is claimed.
Hash the sealed input tree before and after inspection when qualifying a saved
parent, and preserve its full actor, Adam and private-life bytes.

The world tests distinguish real geometry causes, establish that observation does
not imply contact reachability, and check that queries do not change the world or
tracking. The CPU fixture test compares diagnostics off/on using a synthetic
founder, including identical actions, actor/critic checkpoints and complete host
private state. It does not establish saved-parent inference parity or learning
gain. No training updates are performed by that test.

For future obstacle curriculum work, preserve the original lesson as a versioned
control. Ordinary biological aging requires gate timing relative to the prepared
scenario's start; an absolute world-tick threshold can close a gate before the
first aged decision. Any such lesson change needs separate physiology, timing,
geometry and outcome qualification. These diagnostics do not adopt that change.
