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

## Result: no food-care progress; stopped

The one fixed CPU run used source
`2f654450d3ca250fc8648b32e3523d22e0497dbf` and completed all **1,560 native
decisions**: 1,536 training/scored decisions and 24 real bootstraps. All worlds
and outcomes were retained. Both actors completed four actual PPO updates with
changed weights: source steps 0 to 4, predictive steps 2,976 to 2,980. Each
independent critic advanced 0 to 4. Ten initial/per-lesson experimental handoffs
were saved and cold-admitted exactly; frozen coordinates and optimizer ages
were checked for every update. Both arms use the same 24,576 trainable
coordinates. Input states and the earlier negative result remain unchanged.

Both arms recorded **zero acquisitions, consumptions and nourished meals** in
all four training lessons and all before/after evaluations. There were zero
world-failure receipts across all 1,560 decisions, including bootstraps. Every
evaluation had zero teacher cue frames and no semantic prior. Food-care failure
must not be confused with action-execution failure: locomotion commands can
succeed while the creature fails to feed itself.

The held-food scene starts with legitimately prepared native ownership; both
arms retained that food through all 64 scored outcomes in all six held-food
episodes. Setup ownership is not counted as learned acquisition. In the final
within-reach assessments, both arms travelled 0.5 units and reached distance
zero from the food, but did not acquire or ingest it. The exact recorded native
candidate/sampling masks allowed the food-target Ingest candidate in all 64
scored decisions of both final held-food and within-reach assessments. This
follow-up therefore removes the earlier lack of eating opportunity without
manufacturing a meal or selecting a successful seed.

Final teacher-free evaluation, with the same food and rounded displacement
totals for both arms:

| Case | Food held initially / at end | Acquisition / consumption / meal | Nonzero displacement outcomes | Path length (world units) |
| --- | --- | --- | ---: | ---: |
| Held food | yes / yes | 0 / 0 / 0 | 64 | 6.332432 |
| Within-reach food | no / no | 0 / 0 / 0 | 6 | 0.500000 |
| Feeding approach | no / no | 0 / 0 / 0 | 64 | 6.353485 |
| Relocated food | no / no | 0 / 0 / 0 | 30 | 2.988648 |

Each case has 64 scored decisions. The native contact helper tags all 64 as
"Moved", even when displacement is zero after arriving at a target. The table
therefore reports measured displacement rather than treating that tag as proof
of travel. Health remained 1.0; energy fell from 0.7925 to approximately
0.78172–0.78193. The pretrained arm selected Inspect more often in one final
within-reach case, but that change produced no observed food-care benefit.

The [complete per-episode table](performance/predictive-care-long-smoke-behaviors.csv)
retains all 24 training/before/after rows, both arms, every seed, all food and
failure counts, primary action counts, measured path length, nonzero
displacement, energy/health and actual held-food ownership. The portable
summarizer checks the retained traces and native optimizer history and also
continues to reproduce the original 16-decision CSV byte-for-byte.

External monotonic executable time for this longer downstream comparison was
**333.748 seconds (5m33.7s)**, exit code 0. Its internal timer reported 333.721
seconds. Cost accounting retains the failed earlier comparison:

| Executable work | Seconds |
| --- | ---: |
| Full upstream capture, prediction and controls | 837.398 |
| Earlier negative 16-decision care comparison | 120.945 |
| This 64-decision care comparison | 333.748 |
| **Cumulative experimental executable cost** | **1,292.091 (21m32.1s)** |

This run's single-pipeline JSON adds upstream plus the current comparison
(external total 1,171.146 seconds); the cumulative table above additionally
counts the earlier negative. Reported binary builds were 11.51 seconds upstream,
73 seconds for the first care binary and 17.49 seconds for this follow-up:
approximately 23m14.1s including those builds. Cargo checks/tests and
implementation effort are separate and are not represented as training work.

No further trial or dose extension followed this result. The native continuation
and physical opportunity checks work, but this pilot has shown no downstream
food-care advantage at either tested budget. It is an exploratory negative,
not evidence of a universal pretraining failure or a promotion decision. No
behavioral speedup, GPU/major-CSR result or useful capability per hour is claimed.

Validation passed strict Clippy for the library/CLI, formatting and
feature-disabled compilation:

```sh
cargo clippy --locked -p alife_game_app --features cpu-training --lib \
  --bin run_predictive_care_smoke -- -D warnings
cargo fmt --all -- --check
cargo check --locked -p alife_game_app --no-default-features
```

The summarizer validated all 24 episode doses/outcomes, independent identical
initial critics, all native update ages and frozen coordinates. A separate
comparison proved each long-run initial full handoff equals its first-run
initial handoff, and both arms have identical downstream Adam settings and
trainable masks. Existing long output was refused without changing its result.
The first run's result hash remained unchanged, and its published behavior CSV
still reproduced byte-for-byte. No further training run was used for validation.
