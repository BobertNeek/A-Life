# Reserve threshold composes without restoring removed circuits

**Four structural tests passed, with zero physiology/graph advances.** The
reserve-threshold edit commutes with the existing waking/recovery and play
construction recipes in the qualified symmetric founder cases. It neither
reintroduces removed energy-deficit fatigue coupling nor overwrites intentional
ATP or other emitter gains.

Exact executed addendum source: `ba409c0336c7d20a78757bdaafcf7fdacb21f9ef`.
Prior final source remains `98fe445493192bc80bb9cbf0b2e16fc16c7df2bf`; the prior
physiology execution remains `f3602c25929b099c1a04c10aa2fb7ea4a87eef79`.
Their source, frozen plan, results, qualification and grid bytes were not changed.
The3626-call grid was not repeated. This adds one structural test file and this
addendum only, with no production-code or parameter changes.

## Qualified construction behavior

The source's actual play API is `with_play_stimulation`; there is no
`with_play_drive` method in this checkout. Both homologs receive the same recipes.

| Fixed comparison | Result |
|---|---|
| Reserve→waking versus waking→reserve | Full genome and expressed compiled graph equal. |
| Reserve→play versus play→reserve | Full genome and expressed compiled graph equal. |
| Reserve before/between/after waking→play | All three positions equal, keeping waking→play order fixed. |
| Reserve before/between/after play→waking | All three positions equal, keeping play→waking order fixed. |
| Waking, then explicit gains .11/−.013, then reserve | Gains preserved exactly; threshold fields alone change. |

The tests cover11 explicitly fixed construction paths. They do not reorder
waking and play relative to each other or claim their declaration order is
irrelevant. No organism state is advanced, measured, loaded or persisted.

`with_waking_recovery` intentionally removes the fatigue EnergyDeficit emitter
and the direct SleepRecovery→ATP emitter. It changes the surviving ATP
EnergyDeficit gain from−.12 to−.02, adds slow Awake-driven fatigue/sleep pressure,
and installs its locomotor/sleep-metabolic receptors. After the reserve edit,
all those changes remain intact: **two surviving deficit emitters**, threshold
`1.0_f32 - 0.8_f32`, ATP gain−.02, no restored fatigue or direct sleep→ATP row.

`with_play_stimulation` adds boredom on drive Extension0, the curiosity
novelty/investigation circuit and its explicit inherited value coefficient−.08.
The reserve edit preserves them and every other non-threshold field exactly.
The slow Awake source in the waking recipe is intentionally retained, not added
or removed by the reserve candidate; none of these recipes is an action policy.

## Correct selection and unsupported uses

The sealed `reserve_pressure_setpoint.rs::candidate` helper was specifically
written for the reference founder and requires three EnergyDeficit emitters.
It is **not supported after applying waking/recovery**, which has only two.
It must not be weakened by copying missing rows or legacy gains back into the
new graph. Leaving that original helper unchanged preserves the measured source.

The new structural helper selects existing EnergyDeficit emitters by their
current source/target relationship to Hunger/Fatigue/ATP, verifies analogue and
non-inverted response, then changes only their thresholds with the existing
validated setter. It derives the indices from the current recipe, not stale
reference positions. A comparison copy restoring only those threshold fields
must equal the entire original genome JSON. This catches changes to gains,
declaration order, missing rows, species, receptor mappings, value profile or
other inherited state.

The helper explicitly requires identical homologous emitter arrays before
sharing indices between homologs. Arbitrary asymmetric/nonhomologous genotypes,
custom source/receptor topologies and live graph replacement are **not qualified**
by this addendum. They require allele-specific semantic selection and separate
validation, not inference from these tests. The underlying production setter is
an explicit indexed gene operation, not an automatic recipe migration.

## Physiological interpretation remains bounded

This is structural equivalence, not an execution of the composed chemistry.
The previous reserve-grid endpoints and sleep results apply only to its original
legacy-founder recipe. They must not be transferred numerically to waking/play
combinations. Play intentionally changes an extension drive's meaning; waking
intentionally changes fatigue and ATP dynamics, even though the eleven-lane ABI
is unchanged.

ATP here is an **availability variable with baseline relaxation and regulatory
coupling**, not a literal conserved ATP inventory. The previous low-reserve sleep
caveat concerns a strong inherited availability response, not a demonstrated
creation of physical energy. The body-integrating controls continued spending
organ reserves during recovery. The waking recipe removes that direct sleep→ATP
response by construction, but this addendum does not measure its resulting
trajectory or claim all energy-accounting questions are resolved.

## Checks and artifacts

- Four structural tests passed in one invocation,11 construction paths.
- Zero graph/body advances, zero world intervals, zero neural activity/device.
- Compile-only check, whole-workspace formatting, strict targeted Clippy and
  static diff checks passed.
- Prior archive remains unchanged: SHA-256
  `37ca1a0a722c02e90462b3f6fac2096eeb2c5864c7acaf8f5b9edd55e752fe48`.
- Logs, frozen plan and source hashes are in this addendum directory. Whole core,
  world, GPU, composed physiology and acquired-state suites were not run.

The protected candidate is ready for its parent's publication workflow with
these explicit composition boundaries. No further variant, tuning or physiology
budget was consumed, and the released warm build target is available again.
