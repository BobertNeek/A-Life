# Canonical developmental Rest recovery

## Scope and interpretation

A bounded native Rust body/world fixture on accepted main
`af1f2652ef245ccf168298f3a71e1124aef7e0c7`. Production physiology and neural policy
are unchanged. The fixture is `crates/alife_world/tests/developmental_rest_recovery.rs`.

The experiment starts one fresh canonical phase-3 founder per test with fixed
world seed 961060, population 1, default Nano512 builtin foundation, default
sensor profile and unchanged canonical ecology. It advances authoritative world
biology one tick at a time to ages 0, 59, 60 and 240. This is world-only preparation,
not native neural experience or a trained life. All comparisons clone the same
prepared source state, retaining inherited genome, phenotype, body identity,
embodiment, development cadence, hormones and biochemical recipe.

Each age has paired stationary Rest/Idle and moving Rest/movement-only controls.
The second, explicitly authored arm changes only the fatigue species to 0.65 and
re-derives its homeostatic projection through the canonical graph. This authored
fatigue is not represented as naturally accumulated fatigue. No phenotype,
expression floor, development age, learned weights or body reserve is replaced.
Actions use `apply_registered_motor_bundle`, and the enclosing world closes the
same biology interval once. There are no cognition, CPU replay or GPU calls.

## Measurements

The bounded tests pass. No ordinary newborn unrelievable-fatigue defect was
found. Canonical waking fatigue remains exactly zero through age 59; waking
production and stationary Rest clearance first become active together at the
age-60 development refresh. Full expression is measured at age 240, when
maturation is only 0.131578952.

All values below are measured f32 projections, printed to nine decimal places.
Stationary Rest always has recovery dose 1, zero displacement, `body.sleeping =
true` and sleep-state concentration 1. Idle has dose 0, zero displacement,
`body.sleeping = false` and sleep-state concentration 0.

| Initial chemistry | Age interval | Expression before → after | Fatigue before | After Rest | After Idle |
| --- | --- | --- | ---: | ---: | ---: |
| Canonical | 0 → 1 | 0 → 0 | 0 | 0 | 0 |
| Canonical | 59 → 60 | 0 → 0.299043089 | 0 | 0 | 0.000023924 |
| Canonical | 60 → 61 | 0.299043089 → same | 0.000023924 | 0 | 0.000047848 |
| Canonical | 240 → 241 | 1 → 1 | 0.008661905 | 0 | 0.008741478 |
| Authored fatigue | 0 → 1 | 0 → 0 | 0.649999976 | 0.649967492 | 0.649967492 |
| Authored fatigue | 59 → 60 | 0 → 0.299043089 | 0.649999976 | 0.440640330 | 0.649991393 |
| Authored fatigue | 60 → 61 | 0.299043089 → same | 0.649999976 | 0.440640330 | 0.649991393 |
| Authored fatigue | 240 → 241 | 1 → 1 | 0.649999976 | 0 | 0.650047481 |

The authored newborn change is exactly retention × 0.99995 for Rest and Idle.
At expression 0.299043089, matched Rest-versus-Idle relief is 0.209351063,
consistent within 1e-6 with expression × (0.70 recovery + 0.00007 waking source).
Clamping limits the observable relief at full expression.

Energy and ATP rows are identical between canonical and authored-fatigue arms:

| Age interval | Body energy before | After Rest | After Idle | ATP before | After Rest | After Idle |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 → 1 | 0.792500019 | 0.792498887 | 0.792494297 | 0.750000000 | 0.750000000 | 0.750000000 |
| 59 → 60 | 0.792421758 | 0.792418778 | 0.792414188 | 0.750000000 | 0.748758495 | 0.748758435 |
| 60 → 61 | 0.792418778 | 0.792417705 | 0.792413056 | 0.748758495 | 0.747566640 | 0.747566581 |
| 240 → 241 | 0.792185783 | 0.792184830 | 0.792180300 | 0.659184158 | 0.658660471 | 0.658660352 |

Rest spends reserve rather than supplying food or a direct ATP boost. The
canonical graph contains no SleepRecovery-to-ATP emitter. Idle has event energy
−0.01 versus Rest's 0; this also explains the very small indirect ATP difference.
The legacy advertised Rest drive delta is not the authoritative outcome.

Every moving Rest has recovery dose 0, body sleeping false and sleep-state 0.
Its complete body event and resulting biochemistry match movement-only exactly.
All motion is positive X with zero Y/Z and event energy −0.039999999; movement
cost is paid despite the Rest posture command. Moving fatigue equals the Idle
fatigue above (the canonical fatigue circuit has no EnergyDeficit emitter).

| Age interval | Canonical X displacement | Authored-fatigue X displacement | Body energy after moving | ATP after moving |
| --- | ---: | ---: | ---: | ---: |
| 0 → 1 | 0.099999905 | 0.035000086 | 0.792480648 | 0.750000000 |
| 59 → 60 | 0.099995852 | 0.034998417 | 0.792400420 | 0.748758376 |
| 60 → 61 | 0.099992752 | 0.034998417 | 0.792399406 | 0.747566521 |
| 240 → 241 | 0.099115610 | 0.034993649 | 0.792166471 | 0.658660114 |

Authored fatigue also reduces locomotor displacement through inherited fatigue
receptors. BodySleep is not memory sleep: every interval retains
`SleepPhase::Awake`, unchanged sleep seal and zero sleep work. No consolidation
or neural behavior claim follows from those body flags.

## Persistence and state protection

At age 59, both canonical and authored-fatigue states are serialized through
`PortableSaveFile` in memory and restored. Matched Rest and Idle continuations
across 59 to 60 compare exact action receipts, organism records and canonical
world signatures. No save, life, private acquired state or learned arrays are
written as evidence. The assertions also preserve genome/phenotype, neural and
memory subsystem references, sleep seals and cognitive-work receipts across the
body actions. `body.sleeping` is measured separately from `SleepPhase::Awake`,
sleep-cycle work and memory consolidation.

## Source explanation

- `crates/alife_world/src/new_game.rs`: canonical founder construction and
  inherited waking/recovery calibration, including slow metabolic turnover.
- `crates/alife_core/src/biochemistry.rs`: authoritative age, 60-tick development
  refresh, biochemical expression and one body/chemical update.
- `crates/alife_core/src/biochemical_graph.rs`: expression-scaled emitter release,
  fatigue retention, waking fatigue production and SleepRecovery response.
- `crates/alife_world/src/headless.rs`: registered action transaction, Rest dose,
  and movement cancelling sleep recovery before biology advances.

The original [weight-space programming reference](https://rikiyat.github.io/blog/weight-space-programming/)
is motivation only: experiment 4 concerns targeted repair, experiment 5 a
whole-model edit and experiment 6 iterative feedback. This fixture supplies
physiology evidence before any new weight intervention; it does not reproduce
those experiments or establish that a weight edit is beneficial.

## Verification identity and limitations

Executed 8 November 2026, Linux x86_64, Rust 1.99.1
(`58cf8f9c35b355199727627d07ae34a4e2846d67`), LLVM 23.1.1. Cargo offline;
`CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, dev/test debug symbols 0, repository
test opt-level 2 for alife_core/alife_world. Lane2 target was
`awake-shared-cpu-target-v18`; no cleanup or other lane use occurred.

- Test-source Git blob: `b09c6a5ac0c2683d5d76b65881c44148fd32e7ba`.
- Test-source SHA-256: `e7ea25b756def52d426fb5c281daf806049b426680b96ca1e9a9be29c6a38a7f`.
- Executable: `developmental_rest_recovery-b4997050ed7d959a`.
- Executable SHA-256: `d0a93325e95ba7838372ab370cf4e69ee46e0722333479452fbe4ec8cf8758a3`.
- Scalar test-log SHA-256: `dbe66a7e49b3fa12f4cee92941f58bb7e480a232cc7f7990f81fc8410d8e27a0`.
- `cargo test --offline -p alife_world --test developmental_rest_recovery -- --nocapture --test-threads=1`: 2 passed; 32 matrix intervals plus 4 exact continuation pairs (8 action executions).
- `cargo clippy --offline -p alife_world --test developmental_rest_recovery -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo test --offline -p alife_world --lib rest_ -- --test-threads=1`: 9 passed, 293 filtered out.
- `cargo test --offline -p alife_world --test canonical_new_game -- --test-threads=1`: 9 passed.
- Full workspace and GPU suites were not run for this test-only change.

An initial fixture run failed because it incorrectly treated Idle as zero-cost
and omitted the canonical creature summary required by habitat persistence.
Those test assumptions were corrected; no production repair was needed.

This fixed one-founder fixture does not establish voluntary rest selection,
object-present motor coordination, CPU sleep entry, consolidation, long-horizon
self-care, full adult physiology, GPU behavior or robustness across mutated
biochemical/developmental genes. Age 240 means full biochemical expression in
this founder, not adult neural experience. Only scalar measurements and source
are deliverables. No external publication was performed.
