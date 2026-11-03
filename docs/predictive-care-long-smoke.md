# Fixed longer care follow-up

This is a separate, predeclared follow-up to the negative 16-decision
[care smoke](predictive-care-smoke.md). That run and its branch remain intact.
The same original one-tick full predictive state and untrained source start
fresh independent actors and critics; the earlier care-trained actors are not
used. The original object-slot sensor profile, native PPO, biological reward,
temperature 1, one epoch/update per lesson, learning rate 0.0001 and 16-tick
bounded replay windows remain unchanged.

The fixed schedule is now **EatHeldFood, EatReachableFood, Feeding,
RelocatedFood**, with **64 loss decisions plus one actual bootstrap** per life.
The first two native care scenes provide held-food retention and within-reach
safe-food opportunities. The approach cases retain ordinary scene geometry;
64 decisions permit approximately 6.4 units of unobstructed locomotion versus
the initial food distance of 2.2–3.6. This is an opportunity bound, not a promise
that the creature chooses to approach or eat. Native world validation and the
food observer must record real acquisition, consumption and nourishment.

Training seeds remain 41001..41004, assigned in schedule order. Evaluation seeds
remain 51001..51004, using the same schedule before and after all four lessons
for both arms. These are matched diagnostics on reused heldout seeds, not a
claim of newly unseen-world qualification. Each evaluation starts a fresh
private life and removes teacher and semantic prior. Training has ordinary
teacher cues and no semantic prior. Both arms retain their own optimizer history;
both critics start from separate identical source copies. No host chooses the
learner's actions or injects a reward, meal or biological outcome.

The fixed budget is 256 loss decisions per arm, 256 scored decisions per
evaluation phase/arm, and 24 bootstrap decisions: **1,560 actual decisions** if
all lives complete. Every selected world and failure is retained. There is one
run, no test-selected checkpoint and no extension after inspecting results.
If it yields no progress, stop and report that outcome. Local checkpoints and
interrupted prefixes remain recoverable; nothing enters an accepted lineage.

```sh
cargo build --locked -p alife_game_app --features cpu-training --bin run_predictive_care_smoke
target/cloud-cpu/debug/run_predictive_care_smoke --long-horizon \
  --state LOCAL_MOTION_RUN/state-h1-full.json \
  --upstream-result LOCAL_MOTION_RUN/result.json \
  --output NEW_LONG_DIRECTORY
python scripts/summarize_predictive_care_smoke.py \
  --input NEW_LONG_DIRECTORY --output NEW_LONG_BEHAVIORS.csv
```

The CLI records experiment `fixed-disposable-native-care-long-smoke-v1`, a
separate fixed plan, all real decisions/outcomes, native recovery and aggregate
metrics. It never overwrites a previous output. The report must include the
complete upstream prediction run, this downstream run, and the earlier negative
care run when stating cumulative experimental cost. Cargo build/check overhead
is separate. No PC/GPU, accepted or recovered cloud lineage, `replay.rs`, trained
artifact publication or denied-PR retry is part of this work.
