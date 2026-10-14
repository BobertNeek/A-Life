# Foundation food cue delivery — 2026-10-09

Source base: `e5ad5d0d6d524e3d94681ec87af2d724346733d4`.

The shared food-cue emitter applied EatHeldFood's possession guard to GrabFood.
An unheld GrabFood target therefore received only the noun `[1]`, despite its
configured `[9, 1]` request. The repair restricts noun-only fallback to the
EatHeldFood setup: unheld Grab hears `[9, 1]`, held Eat hears `[8, 1]`, and released
Eat hears `[1]` through ordinary spatial Teacher speech.

`teacher_cue_frames` retains its existing any-configured-token meaning. Hearing
only `[1]` can increment it; the counter does not prove complete command delivery
or comprehension. CPU hearing fixtures check token order, sequence positions,
and a shared Teacher utterance ID instead of using that counter as phrase proof.

AOA-TEACH-004 and AOA-TEACH-005 remain enforced by the ordinary embodied speech
path, without learner state injection. The cue assertions and explicit counter
limits support AOA-TEACH-007's auditability requirement. No receipt schema changes.

The new unheld Grab regression failed before the guard repair: one Teacher token
was heard where two were required. After the repair, all 13 focused CPU hearing
and physical-outcome tests passed. The warm rebuild took 2m03s and the tests took
14.71s. No GPU execution is used by these fixtures.

Validation commands:

```text
cargo test --offline --locked -p alife_game_app --lib --features foundation-training foundation_training::held_food_lesson_tests -- --test-threads=1
cargo fmt --all -- --check
scripts/check.ps1 --quick
```

This repair supplies exposure evidence only. It does not establish comprehension,
teacher-independent competence, or that this bug caused the observed Grab
regression. No training binary is rebuilt or deployed, no neural training runs,
and policies 1581 and the unpromoted 1582 retain their existing optimizer states.

## Local possession-aware follow-up — 2026-10-10

Source base: PR21 `6dbc5e83cc9012997b2868e62f0f28d53c6976dc`.

The earlier repair still repeated GrabFood's `[9, 1]` request after the subject
acquired its target. The local follow-up emits noun-only `[1]` while that exact
target is held by the subject, restores `[9, 1]` after release, and emits no new
cue after consumption. Another organism holding the target does not satisfy the
subject's acquisition request, so it retains the existing `[9, 1]` cue.
EatHeldFood remains `[8, 1]` while held and `[1]` after release.

The added CPU fixtures inspect actual emitted Teacher utterances and ordered
heard tokens through legal acquire, release, reacquire, and consume transactions.
They also check other-holder behavior, the unchanged 24-tick cadence, ordinary
spatial placement and addressee, and the existing emission-tick-plus-one lifetime.
Already emitted speech is not retracted when possession changes.

This is a lesson-semantic clarification under AOA-TEACH-002, AOA-TEACH-004,
AOA-TEACH-005, and AOA-TEACH-007. Reversible world Grab, semantic priors, action
choices, acquisition credit, physiology, and cue timing remain unchanged. There
is no direct Get-token score boost, and this does not establish the cause of a
failed training run, comprehension, or teacher-independent competence. The
candidate remains local and unpromoted; no CI edit, GPU execution, or training
is part of this follow-up.
