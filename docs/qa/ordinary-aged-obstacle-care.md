# Ordinary-aged obstacle care, explicit version 1

The native CPU care command accepts the separate opt-in condition
`ordinary_healthy_obstacle_v1` with the existing `obstacle_navigation` lesson.
It remains a newly prepared life; it does not restore a saved individual's life.
The existing default is `newborn_v1`. The feeding/relocated-food condition
`ordinary_healthy_aging_v1` keeps its previous lesson restrictions.

```sh
train_cpu_care --previous IMMUTABLE_NATIVE_SOURCE --output NEW_EPISODE \
  --seed-history EXACT_SOURCE_BOUND_HISTORY --seed EXPLICIT_UNUSED_SEED \
  --lesson obstacle_navigation --decisions 128 --temperature 1 \
  --food-need-condition ordinary_healthy_obstacle_v1 --known-exclusions
```

Omit `--known-exclusions` when using complete-history strict admission. Never
invent freshness or erase consumed/reserved/interrupted/assessment exclusions.
Use the existing native prior configuration and explicit storage/process guards.
The serial scheduler's global condition option does not yet dispatch this separate
condition; the command above is the native single-episode path, not an automatic
curriculum rollout. No existing schedule is changed by adding this condition.

Preparation advances the existing ordinary biology, using the same bounds as
established care preparation: hunger at least 0.12, energy above 0.2 and at most
0.9, health at least 0.95, injury/pain at most 0.01 and awake. It preserves scene
geometry, food, ordinary physics, teacher cue rules, topology and learning rules.
Setup time does not become captured training rows. The native reservation records
the condition; the resulting initialization must match it. A legacy reservation
cannot silently authorize aged preparation.

The gate closes after two elapsed scenario ticks. This preserves the original
newborn timing at world ticks 0/1 open and 2 onward closed. Prepared aged scenes
receive the same two decision opportunities even though biological setup advanced
the world clock. It does not make the gate transparent, restore a hidden food
candidate, supply a detour or establish that historical failures were caused by
gate closure.

Success still requires a real nourishing meal in the captured window, an initial
food candidate, a later absence, movement and no blocked action. The historical
`food_visible` field still means candidate presence. Keep optical visibility,
observed identity, focus, candidate membership, contact and route accessibility
separate when interpreting outcomes.

Qualification includes three-seed world comparisons against ordinary advancement
of the unchanged scene, newborn timing parity against the old absolute-clock
behavior, wrong-lesson and mismatched-reservation rejection, and three matched
native synthetic-founder pairs with 128 decisions plus one bootstrap each. The
pairs use the same seed, actor/critic bytes and geometry, explicit prior/teacher
removal, and zero offline Adam updates. They compare setup semantics and actual
outcomes, not retained skills in an acquired brain. Waking private learning and
real biology continue. No success improvement or historical causality is assumed.

Build in an isolated target directory to preserve accepted production binaries:

```sh
cargo test --release --offline -j1 -p alife_game_app \
  --no-default-features --features cpu-training --lib aged_obstacle -- --nocapture
cargo test --release --offline -j1 -p alife_game_app \
  --no-default-features --features cpu-training --lib foundation_training::food_need::tests
```

Retain the original newborn case as a named control. A later acquired-parent
comparison needs properly admitted assessment seeds, frozen actor/full Adam,
matched bodies and geometry, teacher/prior removal and original success metrics.
Source qualification does not automatically adopt this condition for training.
