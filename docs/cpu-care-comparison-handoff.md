# Eight-case native CPU handoff

This public handoff contains source and benchmark controls only. It contains no
retained model, optimizer, personal state, private evidence or assessment outcomes.
Use private immutable model copies and keep generated requests, lives and results
outside the repository. No accepted brain is changed or selected by these tools.

## Exact source and build

The assessment implementation is the source-only branch
`codex/conservative-consolidation-20261103`, runtime commit
`0ba8b8b1263de079e29766bbe06d8e673cbb5fff`, tree
`77363cfa0c8e95f9a05f47069b056fc1b2309b38`. This tree is identical to
local implementation commit `c3baffc780b59d756d473dc12db9c7b09372eba6`.
The handoff adds public fixtures, instructions, a request preparer and a synthetic
runtime fingerprint test; it changes no runtime or evaluator behavior.
`Cargo.lock` SHA256 is
`2444e04142202ffab7d0ace60ef20db7b56ca496883ae007030573835231546c`.
Record `git rev-parse HEAD`, `git status --short`, `rustc -Vv`, Cargo feature set,
target triple, release/profile flags and executable SHA256 for each assessment.

Reference toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13), LLVM 23.1.1,
x86_64-unknown-linux-gnu. The reference immutable release evaluator is 17152920
bytes, SHA256 `d78bdb8c83c7b0c0eaa319b1e02aab821cdefca7f3c538064e1b12889ad4c2a1`.
A different compiler, target or link environment can produce a different binary.
Do not infer equality of runtimes from a branch name or passing unit tests alone.

From the source root (Cargo cache must already be populated for `--offline`):

```text
cargo build --release --locked --offline -p alife_game_app --features cpu-training --bin evaluate_cpu_founder --bin export_native_training_handoff
cargo test --release --locked --offline -p alife_training --features cpu-backend --test cpu_collector founder_only_inference_matches_full_trainer_actions_private_learning_sleep_and_restore -- --exact
cargo test --release --locked --offline -p alife_training --features cpu-backend --test cpu_collector public_terrain_founder_runtime_fingerprint -- --exact --nocapture
cargo test --release --locked --offline -p alife_game_app --features cpu-training --lib founder_only_world_assessment_matches_full_source_and_rejects_tampering_or_restart
```

Keep the executable used by a receipt at its pinned path. On Windows append `.exe`
to executable paths; the request preparer uses Python 3 and native path handling.
Do not run a Linux shell wrapper through an unavailable WSL installation. The
native Current inspector is device-free and read-only:

```text
target/release/export_native_training_handoff --inspect-current PRIVATE_CURRENT_COPY
```

## Cases and preparation

The machine-readable fixture is
[`fixtures/cpu-care-eight-cases-v1.json`](fixtures/cpu-care-eight-cases-v1.json).
All cases use body construction seed **539363617**, temperature **1.0**, **128
evaluated decisions plus one bootstrap decision**. World/action seeds are:

| Case | Lesson | Seed | Physiology |
| --- | --- | ---: | --- |
| navigation-1 | obstacle_navigation | 202611044001 | newborn_v1 |
| navigation-2 | obstacle_navigation | 202611044002 | newborn_v1 |
| relocated-aged-1 | relocated_food | 202611044003 | ordinary_healthy_aging_v1 |
| relocated-aged-2 | relocated_food | 202611044004 | ordinary_healthy_aging_v1 |
| feeding-1-newborn | feeding | 202611044005 | newborn_v1 |
| feeding-1-aged | feeding | 202611044005 | ordinary_healthy_aging_v1 |
| feeding-2-newborn | feeding | 202611044006 | newborn_v1 |
| feeding-2-aged | feeding | 202611044006 | ordinary_healthy_aging_v1 |

The sampler seed is `(world_seed as u32) XOR ((world_seed >> 32) as u32)`.
The fixture has six distinct world seeds. Paired physiology is not an additional
independent seed. These disclosed cases cannot remain a final unseen holdout after
using them for candidate selection. Preserve unknown historical freshness and union
all known training, reserved, interrupted and assessment exclusions from both
parents. Do not relabel assessments as training consumption or fabricate missing
history. The native history must bind to the actual anchor and retain its evidence.

Every assessment reconstructs a fresh canonical n2048 TerrainVision body using
`compile_cpu_founder_game(world_seed, founder_seed, asset)`, the existing ordinary
compiler and shared care scene. The candidate founder asset is immutable. Native
recurrent state, memory, eligibility, waking plasticity, sleep and biological costs
are active in the new private life. This is not a restore of either parent's lived
experience. No actor, critic or Adam is constructed for the candidate; no offline
optimizer update occurs. The same canonical host, decoder, ordinary senses and
world action legality are used as the full native CPU path.

`newborn_v1` has no added healthy-aging stage. `ordinary_healthy_aging_v1` advances
ordinary world ticks, up to 2400, until hunger >=0.12 and energy <=0.9. Admission
requires finite values, awake, health >=0.95, injury/pain <=0.01, energy >0.2 and
<=0.9, hunger >=0.12. It does not inject hunger or change health. The reference
prepared bodies reach this condition at setup tick 121 (newborn setup tick 0).
Use the native preparation and verify the recorded complete physiology; do not
hardcode an age or edit chemistry to obtain a matching result. Setup ticks are
excluded from the 128 decisions. Aging is supported only for Feeding/RelocatedFood.

Feeding/RelocatedFood use the native seeded angle/distance placement, food variety
from `seed/12`, no custom food position. RelocatedFood moves other obstacles,
hazard and second food away by the shared setup, then attempts one sideways food
relocation at `world_tick - setup_tick >= 2` before the next step, only if unheld.
The target is a seeded quarter turn about the learner's initial position; no
continuous tracking or hints. The native row index records when it actually moved.
ObstacleNavigation uses the shared seeded closing-gate scene; do not substitute a
static obstacle or alternate navigation simulator. Scene formulas and gate timing
are defined by `foundation_training.rs` and `foundation_cpu_care.rs` at the runtime
commit above; running those functions is the fixture.

Assessment mode removes the teacher and clears cue tokens, checks absence before
every step and forces the semantic prior to `None` independent of environment.
Set `ALIFE_SLM_PRIOR=off`, `RAYON_NUM_THREADS=2`, `OMP_NUM_THREADS=2` for the reference
process settings. The CPU baseline also set `VK_ICD_FILENAMES=/no-vulkan-icd.json`
to demonstrate no usable GPU ICD; this path is a disabling control, not an asset.
The evaluator thread has a 32 MiB stack. No external teacher or prior service is
needed. No GPU or GPU-to-CPU numerical parity is claimed.

The exact loop captures 129 decisions, bounded by `128*8+4096` host steps. Rows
0..127 are evaluation rows; row128 is bootstrap. Only the first128 count for meals
and completion. Sleep can consume host ticks without producing a decision; do not
force a fixed final world tick. Incomplete/dead/censored runs retain their native
private prefix and fail qualification; retain and report them, never auto-restart.
A meal must be actual nourishment of the tracked food/organism. Navigation also
requires initial food visibility, later hiding, movement and no blocked action.
RelocatedFood requires a counted meal at/after its actual relocation row. Report
per-case meal/completion, acquisition/consumption, first-event decision, native
health/injury/sleep and relevant navigation/relocation fields; absent trace fields
are unknown, not zero. Never replace this with one aggregate skill score.

## Private requests and the present input boundary

The existing evaluator requires an admitted **native CPU directory** as
`seed_anchor`. This authenticates exclusions and fixes the body seed; it does not
transplant that source's critic/Adam or prove the candidate's ancestry. The exact
body seed must match the catalogue. A standalone `.alife-checkpoint` Current/GPU
save is **not accepted as this anchor**, and `--inspect-current` does not convert
it. Do not manufacture CPU receipts, zero Adam or edit `founder_seed_base` to
force admission. A supported read-only Current-to-assessment adaptation and
qualification is still needed if that is the only available PC input. The
candidate canonical founder must also be proven to contain the actual saved
actor weights under the same graph, frozen-coordinate and sensor semantics;
a stale genetic asset is insufficient. Keep original full Adam and private lives
sealed even when deriving a fresh-life inference view.

With a legitimate compatible CPU anchor and already-admitted private history:

```text
python scripts/prepare_cpu_care_comparison.py --foundation PRIVATE_FOUNDER_ASSET --anchor PRIVATE_NATIVE_CPU_ANCHOR --history PRIVATE_UNION_HISTORY_JSON --output ABSENT_PRIVATE_REQUEST_ROOT
```

This command only prepares the eight private JSON specs; it does not run, reserve
or validate native state. The evaluator itself performs native source/history,
asset digest and protected-output admission before dispatch. Use absolute private
paths. For each of the eight generated specs, run separately, stopping on failure:

```text
target/release/evaluate_cpu_founder --spec ABSENT_PRIVATE_REQUEST_ROOT/navigation-1-spec.json
target/release/evaluate_cpu_founder --verify ABSENT_PRIVATE_REQUEST_ROOT/navigation-1
```

Repeat for the other seven case IDs. Every output directory must be absent before
its sole run. Keep stdout/stderr and monotonic process wall time, exit status and
the exact executable hash. No timeout/restart loop. No sustained training or
adoption is performed by these commands.

## Parity and acceptance gates

1. Admit the exact saved source and actual founder bytes through supported native
   tooling; prove actor/founder/graph/frozen-coordinate compatibility. A checkpoint
   number or same parameter count does not establish identity or common ancestry.
2. Run the public full-trainer/bare-founder parity tests above. They require exact
   serialized actions and whole private state, nonzero eligibility, forced sleep,
   consolidation, waking, cold restore and subsequent steps, with the original
   trainer's full checkpoint unchanged. The additional TerrainVision fixture uses
   only a public procedural genotype (construction seed `0x7A1123`, sampler42,
   temperature1); compare its printed fingerprint with
   `fixtures/cpu-founder-runtime-reference-v1.json`.
   This is a cross-build calibration, not a proof of all learned policy behavior.
3. Also run an **unmodified actual parent** through the full native CPU assessment
   and its bare inference view on the same cases. Require byte-exact care trace
   and complete native private state. Synthetic tests alone do not establish this
   for an imported learned Current checkpoint. A fresh-body view is not same-life
   training continuation. Any Current adaptation must preserve all real moments
   and ages when using a full-state control; no fake optimizer is acceptable.
4. `evaluate_cpu_founder --verify OUTPUT` must validate schema2 seal, all raw-file
   hashes, input pins, exact cold private restore/recapture, no teacher/prior and
   no offline learning outputs. It requires its original private dependencies and
   binary; moving outputs is not an implicitly supported resealing operation.
5. For each parent/candidate paired case use:
   `evaluate_cpu_founder --compare CONTROL_OUTPUT CANDIDATE_OUTPUT`.
   Require JSON `matched:true`; CLI exit0 alone does not imply a match. Equality
   includes the native pre-runtime world projection, complete prepared physiology
   hash, first actual neural-input hash, seeds, dose, lesson, condition, temperature
   **and executable SHA256**. Later private states are outcomes, not match controls.
   Separately read each result and trace for retention/safety.
6. A Windows build and the Linux reference will have different executable hashes;
   the existing strict comparator will reject that pair. Matching synthetic
   fingerprints is helpful evidence, but does not authorize dropping the executable
   field or calling the runs strictly matched. Use one qualified binary/runtime
   for both untouched parents and candidates, or explicitly qualify a cross-build
   comparison protocol with actual-parent controls before causal claims/adoption.

The public case handoff is usable now. Native Current-anchor adaptation, actual
imported-parent parity and cross-build qualification remain independent blockers
when their required inputs/evidence are absent. There is no completed two-parent
comparison in this source artifact. Broader PC skills, more independent seeds,
long-horizon survival and continued-learning retention still require coverage.
