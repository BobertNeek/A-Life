# Explicit awake-rest recovery V1

## Versioned stages

- Exact-recipe predecessor: fully qualified at
  `43426ffaf2ceef137f18912683ff071d5ac74447`, tested code `09f908f42`.
  It rejected altered V1 rows at generic graph compile/load. This prevented
  zero-input firing but could reject ordinary candidate gamete mutations.
- Event-gated successor: production change
  `e3c8fac280b4e7da889431100181e63764a719df`, final source
  `7792e6cf3d770b0520d47dab7b23e3eb1fb371af`. It gates only the appended source's
  release count to zero whenever no positive typed event exists. Otherwise-valid
  mutant rows compile/load/inherit unchanged; positive V1 event admission still
  requires the original exact fatigue circuit. Core mechanism, all 17 world/
  Rest fixtures, ordinary byte compatibility and all 9 canonical-new-game tests
  pass. All 78 existing core regressions, formatting and strict core/world
  all-target Clippy also pass. No ordinary or composite entry is enabled.

The predecessor source, logs and hashes remain preserved. All 79 measured
physiology observations below match the successor exactly, including the 42
matrix intervals and 12 saved/restored continuation executions. Neither stage
changes the ordinary default, a recovery coefficient, or any existing source's
semantics.

## Scope and status

This is an opt-in body/biochemical mechanism candidate on accepted main
`af1f2652ef245ccf168298f3a71e1124aef7e0c7`. It is not a default change, neural
policy, learned behavior, sleep scheduler repair, or evidence of voluntary
self-care. No private lives, checkpoints, acquired weights or assets are inputs.
The candidate separates measured fatigue relief from sleep-pressure recovery
while preserving actual bodily sleep as a separate route. Both stages are
bounded construction/body/chemistry experiments. The successor's seeded genetic
regressions are not a breeding population, mutation study or fitness claim.

The functional motivation is the useful separation of tiredness from sleepiness
in Creatures 3 / Docking Station. See the official [Genetics Kit manual](https://cdn.cloudflare.steamstatic.com/steam/apps/1838430/manuals/Genetics_Kit_Manual.pdf?t=1640163808)
and the primary [chemical catalogue](https://github.com/videogamepreservation/creatures3/blob/321245bed27cbacb52421d95c9ff4f0bf67a49eb/engine/Catalogues/ChemicalNames.catalogue).
We adapt that distinction, not their exact genomes. The [weight-space programming
experiments](https://rikiyat.github.io/blog/weight-space-programming/) are motivation
only; this candidate contains no weight edits or training.

## Mechanism and preservation surface

- `create_canonical_new_game_with_awake_rest_research_v1` constructs fresh Nano512
  founders. Before expression/newborn construction, both inherited biochemical
  homologs receive one new emitter. No existing creature or loaded phenotype is
  rewritten. The ordinary founder constructors remain unchanged.
- The appended `BiochemicalSourceLocus::AwakeRestRecoveryV1` is the final enum
  variant, ordinal 15. All earlier ordinals and graph schema 3 remain unchanged.
  The new source is explicitly versioned, event-backed and released once per
  advance, including catch-up. Old recipes have no such emitter.
- The new emitter targets fatigue only, with gain −0.70, cadence 1, threshold 0,
  analogue response, no inversion and developmental floor 0. This reuses the
  canonical sleep-to-fatigue coefficient as a bounded comparison, not a tuned
  claim about optimal rest timescales. The separate waking source remains active.
  All old sleep-pressure, sleep-state, metabolic, reward and receptor recipes
  remain untouched. Fatigue is still the existing waking-accumulation signal,
  not newly calibrated physical exertion.
- `AwakeRestRequestV1` is a sealed, ephemeral operation outside ordinary neural
  candidate enumeration. It requires the existing neutral NO_POSTURE channel,
  with no target, direction or payload, optionally alongside locomotion. The
  ordinary registered bundle validates the Rest effector and adapts intensity.
  The inherited circuit and memory phase Awake are required. Existing raw IDs,
  including unknown IDs, receive no new interpretation.
- The world computes an integrated `AwakeRestEventV1` dose from adapted posture
  intensity. Realized motion, fresh damage, a failed action or actual sleep dose
  cancels only this new dose. Injury, nutrition and effort are retained. A failed
  transaction rolls back the whole ordinary operation. Combination with the
  separate contact-reflex experiment is not admitted by this entry.
- The separate event enters the same once-only body/graph update, rather than
  changing a drive projection directly. No `BodyEventDelta`, motor receipt,
  world-save or legacy serialized layout gains a field. Ordinary entry points
  supply no candidate event. No reward, food, ATP, energy or memory work is
  fabricated. SleepPressure keeps its existing retention/waking dynamics, and
  body sleeping/SleepState remain false/zero for awake rest. Only actual legacy
  Rest/body sleep supplies the old sleep-recovery route. No sleep phase is reset.
- Candidate awake-rest and legacy Rest have zero body-event energy charge.
  Idle retains its separate −0.01 charge. Normal awake upkeep and graph execution
  still occur. A new-genotype inactive-emitter control is distinct from an old
  ordinary founder: one added emitter evaluation is real extra work. These
  genotypes are not described as compute-matched.
- There is no persistent world switch to drop on save. The inherited recipe is
  already serialized with the genotype/phenotype. A new typed request must be
  supplied for each interval, also after restore. Unsupported circuit variants
  retain their full inherited data on compile/load in the successor, but fail
  positive typed-event admission unless they have the exact target, gain,
  cadence, threshold, analogue/non-inverted response and developmental floor.
  Independently, a missing/zero event forces the new locus's release count to
  zero, even for digital, inverted or high-floor rows. Thus unsupported mutants
  cannot receive the V1 operation or fire spontaneously. Existing generic graph
  validation and every old source's release semantics are unchanged.

## Bounded fixtures and measured outcomes

`crates/alife_world/src/headless/awake_rest_research/tests.rs` uses fixed seed
961060 and one fresh founder, advanced through authoritative world biology to
ages 0, 60 and 240. Age 240 means full biochemical expression, not adult neural
experience. Each age is paired with a separate authored chemistry arm that sets
fatigue and SleepPressure concentrations to 0.65 and re-derives projections.
The authored arm is never represented as naturally accumulated fatigue.

Within the new genotype, 42 paired intervals compare awake rest, neutral
NO_POSTURE, legacy Rest, Idle, moving awake rest, movement plus neutral posture,
and moving legacy Rest. Every awake-rest interval has sleep dose 0, body sleeping
false and SleepState 0. Every stationary legacy Rest has sleep dose 1, body
sleeping true and SleepState 1. All preserve memory phase Awake and zero memory
sleep work. This fixture does not run the neural sleep-admission controller.

The following are measured f32 projections, printed to nine decimal places.
The neutral and Idle controls have identical fatigue and pressure outcomes.

| Initial chemistry | Age | Expression | Fatigue before | Awake-rest fatigue | Legacy-Rest fatigue | Neutral/Idle fatigue | Awake-rest pressure | Legacy-Rest pressure |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Fresh | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Fresh | 60 | 0.299043089 | 0.000023924 | 0 | 0 | 0.000047848 | 0.000047848 | 0 |
| Fresh | 240 | 1 | 0.008661905 | 0 | 0 | 0.008741478 | 0.008741478 | 0 |
| Authored pair 0.65 | 0 | 0 | 0.649999976 | 0.649967492 | 0.649967492 | 0.649967492 | 0.649967492 | 0.649967492 |
| Authored pair 0.65 | 60 | 0.299043089 | 0.649999976 | 0.440661252 | 0.440640330 | 0.649991393 | 0.649991393 | 0.425688148 |
| Authored pair 0.65 | 240 | 1 | 0.649999976 | 0 | 0 | 0.650047481 | 0.650047481 | 0 |

Pressure before equals fatigue before in these matched rows. Awake-rest pressure
is exactly the neutral/Idle pressure, including ordinary retention; it is not
held numerically constant. At age 60 the candidate's fatigue-only relief relative
to neutral is 0.209330141, consistent with 0.70 × expression within 1e-6. The
additional 0.000020922 fatigue reduction under legacy Rest is consistent with
its suppression of the ordinary waking emitter. At age 0, expression is zero,
so even the authored pair receives no active recovery from either route.

### Energy, work and physical cancellation

| Age | Body energy before | After awake rest / legacy Rest / neutral | After Idle | ATP before | After awake rest / legacy Rest / neutral | After Idle |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 0.792500019 | 0.792498887 | 0.792494297 | 0.750000000 | 0.750000000 | 0.750000000 |
| 60 | 0.792418778 | 0.792417705 | 0.792413056 | 0.748758495 | 0.747566640 | 0.747566581 |
| 240 | 0.792185783 | 0.792184830 | 0.792180300 | 0.659184158 | 0.658660471 | 0.658660352 |

These rows hold for both fresh and authored-pair arms. All start awake; equality
with Rest here is not a general assertion that sustained sleep and awake upkeep
are identical. Awake rest adds no nutrition, reward or ATP. The old Rest and
candidate awake-rest event charges are 0, Idle's is −0.01. The expressed Rest
effector gain is 1.034999967 and full requested intensity clamps the integrated
candidate dose to 1. Requests at intensity 0/0.25/1 and duration 1/64 verify
adaptation without multiplying one interval by declared command duration.

The new genotype reports 83 biochemical evaluation units versus 82 for the old
ordinary genotype. Exactly one emitter evaluation is added even at zero dose;
all other reported work fields match. This counter is not separately debited
from ATP by current world biology. The inactive new genotype's energy/ATP and
other physiology match the old genotype exactly. Neither a compute-matched
claim nor an extra ATP charge is inferred from the counter.

All moving candidate intervals have dose 0 and match the same neutral movement
transaction exactly. They retain movement event cost −0.039999999 and match
moving legacy Rest's complete body event and resulting biology. Fresh damaging
contact retains actual damage and reduces health while zeroing candidate relief;
the result equals the neutral damage control. A blocked failed attempt retains
cost −0.029999999 with zero displacement and zero recovery. This candidate is
more conservative than legacy blocked Rest, which is left unchanged.

Each action's result equals one explicit body/graph advance. Closing the enclosing
world tick leaves that result unchanged; repeating the typed request before the
world closes rejects without mutation. Injected post-action failure rolls back
saved world bytes and prior receipts. Wrong recipe/posture/tick and an absent
Rest effector reject without mutation. Genome, phenotype, embodiment, neural
and memory references, cognitive work and sleep seal remain unchanged.

Actual body-sleep events through the world interval API match legacy Rest
biology. A following awake-rest interval resumes awake body/SleepState semantics
without suppressing the normal pressure trajectory. Portable round trips of
fresh and authored-pair sources preserve recipe and exact state; six continuation
pairs (12 action executions) compare complete receipts, records and world
signatures for awake rest, legacy Rest and Idle.

`crates/alife_world/tests/awake_rest_legacy_wire.rs` is intentionally usable
unchanged on accepted main. It emits only byte counts and digest summaries for
18 ordinary founder/control continuations: ages 0/60/240 crossed with direct and
registered Rest, unknown-ID direct and registered Rest, Idle and NO_POSTURE.
Its optional `ALIFE_AWAKE_REST_WIRE_OUTPUT` writes only these fresh disposable
fixtures locally for a direct byte comparison of portable saves and complete
action receipt representations. Those local state files are not deliverables.
Canonical world signatures are also compared. Core tests freeze old enum
ordinals with standard and fixed-width binary encoding, nonzero BodyEventDelta
binary bytes, and one-shot catch-up emission. No private or acquired life is read.

## Qualified predecessor verification and iteration record

Accepted-main control: `af1f2652ef245ccf168298f3a71e1124aef7e0c7`.
First successful mechanism source: `1379e588b` (2 core tests and 7 world tests).
Predecessor qualification source: `09f908f42f73c081d00117c169a3e1b9e5b3c465`, which
adds explicit cadence/threshold rejection negatives without changing production.
Linux x86_64, Rust 1.99.1 (`58cf8f9c35b355199727627d07ae34a4e2846d67`), LLVM
23.1.1, offline Cargo, jobs 1, incremental 0, dev/test debug symbols 0, existing
core/world test optimization level 2. Only the assigned lane-2 target is used.

The first candidate compile at `f694c4edd` failed in the added test because it
called bincode 1's `serialize`; the repository uses bincode 2. No world fixture
ran from that failed source. The corrected test uses the existing bincode 2
serde encoding API. The failed source and compiler log are retained. Source
review separately found the zero-input inversion/digital hazard and tightened
only the new circuit's validation before physiology qualification.

Predecessor checks on that qualification source:

- `cargo test --offline -p alife_core --lib awake_rest_ -- --nocapture --test-threads=1`:
  2 passed, including eight unsupported new-circuit compile/load variants.
- `cargo test --offline -p alife_world --lib rest_ -- --nocapture --test-threads=1`:
  16 passed: seven candidate fixtures and all nine existing Rest regressions.
- `cargo test --offline -p alife_world --test awake_rest_legacy_wire -- --nocapture --test-threads=1`:
  passed on accepted main and candidate. Eighteen portable saves and eighteen
  complete receipt representations compare byte-for-byte (1,409,755 bytes).
  All eighteen digest/signature summary rows also match exactly. The same
  generated ordinary source and command inputs were used in both builds.
- `cargo test --offline -p alife_world --test canonical_new_game -- --test-threads=1`:
  9 passed.
- Existing core regression suites passed: `action_arbitration` (8),
  `biochemical_graph_v2` (11), `biochemical_material_baselines` (4),
  `biochemical_reaction_conservation` (11), `biochemical_valuation` (4),
  `biochemistry_development` (13), `chemistry_homeostasis` (9). Total 60.
- `cargo clippy --offline -p alife_core -p alife_world --all-targets -- -D warnings`:
  passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.

Predecessor SHA-256 identities (preserved, not successor executable hashes):

| Evidence | SHA-256 |
| --- | --- |
| World fixture source (`headless/awake_rest_research/tests.rs`) | `f38dc0908e8ab095506e104daada6ff77fc59dfc7432bd40c402093c56c05fa3` |
| Legacy wire fixture source | `9f713406739d924fd871756da267d12ae21fc5c8112d86488f0fc7b9ead3300a` |
| Final core test executable (`alife_core-2fd07a44616ca3ca`) | `0f8a294f78c03f0e0f918d62aad03f6d3101e3b7d02af3819a24a56a101b57b9` |
| Final world test executable (`alife_world-6b7d17d635ca0301`) | `ecb5ed4838d05f01e236ebfadb4152747615453f223807390c3a7b27573fba8b` |
| Candidate wire test executable (`awake_rest_legacy_wire-12670af10047bf45`) | `073a37d8e22111ce7ef83cf6a9615d663fec172a9ef09970327856ea64c2838e` |
| Final core mechanism log | `aaace017de8e5e330ebd5460b41f925f0c8f5b49b0fb5cdda7a57c678d2fe829` |
| Final world and legacy Rest log | `32f58b150e48e18d5241ce5d428ae916f194c830ed4f8d055efc7d15bfb59e15` |
| Direct byte-comparison summary | `da40648836bc50bec34f53e4d7d9f01eb8074ffd82fa44c9a239602a5e830104` |
| Core regressions log | `da172571bdf11daada09587a0c29756095025de9d5c047cc78cbf76a28604e0f` |
| Strict Clippy log | `6ba809b1638bb92effb461cdd91f30ee485c193a95e4d73c17c8e11817165e15` |
| Failed bincode-API compile log (`f694c4edd`) | `ad19880e7b9975a99182ed5a6f5d35ff1d4ad8cecba41228a3519343286351e8` |

Only source and these scalar/hash findings are deliverables. The disposable
wire captures and native executables remain local and are not publication assets.
No external publication or ordinary default activation occurred.

## Successor inheritance and inactive-source qualification

Source inspection showed that graph expression preserves an intact selected
allele when homologs have different topology
(`BiochemicalPhenotype::map_homologous_parameters`). Mixing an old graph with a
V1 graph therefore does not halve the new emitter's gain. The successor tests
both selections explicitly and preserves exact genotype/phenotype round trips.

Ordinary `graph_gamete` reproduction does mutate emitter gain, threshold and
floor before compiling the offspring. The predecessor's exact generic guard
could reject those offspring. The successor instead makes absence of a positive
typed event authoritative at release-count level, preserving generic inheritance
without loosening positive V1 event admission or redefining its −0.70 coefficient.

Ten otherwise-valid source variants compile and load exactly: altered target,
inversion, digital response, gain, developmental floor, cadence, threshold,
duplicate row, combined digital/inverted/high-floor, and inverted positive-gain
ATP-targeted row. Each is tested at 1/64 elapsed ticks and expression 0/0.5/1.
All 60 no-event graph updates exactly match an otherwise-identical graph with
only those source rows removed. All 60 positive-event attempts reject. Evaluation
work still counts the inherited rows; no clearance or ATP is obtained from them.
The exact recipe's positive event still releases once across catch-up.

Three fixed seeded ordinary `CreatureGenome::reproduce` controls use fresh
canonical candidate parents, with no lifetime or neural training. Each conception
is replayed deterministically; parents remain byte-identical, and child genotype,
expression and round trip remain exact. Every child's actual newborn graph update
matches the same expressed graph with only the new source omitted.

| Conception seed | Parental point-mutation rate | Recorded mutations | V1 threshold | V1 gain | V1 floor | Positive V1 admission |
| ---: | --- | ---: | ---: | ---: | ---: | --- |
| 901 | Ordinary inherited rate, unchanged | 15 | 0 | −0.699999988 | 0 | Accepted |
| 902 | Explicit authored 0 | 0 | 0 | −0.699999988 | 0 | Accepted |
| 903 | Explicit authored 1 | 666 | 0.073153220 | −0.688934445 | 0.058194399 | Rejected |

The authored-rate controls set discrete mutation to zero; they are not described
as natural selection. The 666 count includes all recorded genome mutations, not
only this emitter. The mutated child compiles, expresses, reloads and advances
normally without its unsupported source firing. This closes the demonstrated
construction failure; it does not qualify a reproductive population or make the
mutant circuit eligible for the exact V1 action.

Successor qualification uses the same offline lane, toolchain and test settings
as the predecessor, with final source `7792e6cf3d770b0520d47dab7b23e3eb1fb371af`:

- Core `--lib awake_rest_`: 2 passed, including all 60 inactive-mutant comparisons,
  all 60 positive-event rejections and the old binary-ABI checks.
- World `--lib rest_`: 17 passed, comprising eight candidate fixture groups and
  nine existing Rest regressions. All 79 printed physiology observations match
  the qualified predecessor exactly.
- Ordinary legacy-wire fixture: passed. All 18 portable save files and 18 full
  receipt representations remain byte-identical to the actual accepted-main
  control, with 1,409,755 bytes compared directly.
- `canonical_new_game`: all 9 passed.
- The predecessor's seven existing core regression suites passed again (60
  tests), plus all 18 `evolutionary_genetics` tests. Total 78 core regressions.
- `cargo fmt --all -- --check`, strict core/world all-target Clippy with
  `-D warnings`, and `git diff --check`: passed.

No autonomous study or default promotion occurred. The qualified source tree is
`fcfec3abb159f22a6fb1de0b5c0cc889b41ab87a`. Later documentation-only changes do not
alter the tested source. Native work has finished; no executables or disposable
save captures are included in the source deliverable.

Successor SHA-256 identities:

| Evidence | SHA-256 |
| --- | --- |
| World fixture source | `6c1e0766b28b201406642ee3e0ee1debc093158f57614bf5135afbf8ac410c66` |
| Core graph source | `b1bc429438723b2972e89d7560026432b01de60b0f5a4b175417a44a2924fc72` |
| Legacy wire fixture source | `9f713406739d924fd871756da267d12ae21fc5c8112d86488f0fc7b9ead3300a` |
| Core test executable (`alife_core-2fd07a44616ca3ca`) | `f44252d580173f9018c8d526d7204c53e5aeb2fa4b2e19c32d30494bc876e95b` |
| World test executable (`alife_world-6b7d17d635ca0301`) | `2ee21f822e734db9f9fa734ea3bcee5166f8f4b31e0eb55315fe843749e2181f` |
| Wire test executable (`awake_rest_legacy_wire-12670af10047bf45`) | `d3796cf57f8bc49baa881ddfd815dfe0f047417276f3ec10c137a3d92134ce50` |
| Core mechanism log | `b44a2fcbbcb7bbe5810d036a8fd1a26d19e1c7d62b2ed8d01eb0ebe1593743c4` |
| World and legacy Rest log | `437de55dea990fecd8f0e6b235bd0e874726617009662a575a4e5cbe14d0ff6b` |
| Physiology comparison | `277f0591783cdbd9aede31db6a670b98ff5680d02add8a0ce94b821a182d789f` |
| Direct byte comparison | `70ab6ca9aacfb05c7d658510556b9b299c84f12fbb17673a32bae48461b9645f` |
| Canonical-new-game log | `15c7cff33521b66529613f94b113235e2d5eaee828a87a33e259ea4d8bc480e3` |
| Core regressions log | `5b20ee86d5312392d08aae44bd9836ab531cc54f4b3d5d066b9fe628b1d10a10` |
| Strict Clippy log | `cf8730efdfc2388ef2a5161d411cf7eaf5723a64cf51f9e960f2d8692ba72719` |

## Limits and remaining work

No autonomous study, brain admission, learned valuation update, CPU/GPU policy
selection, full workspace gate or hardware runtime claim is made. Integration
on any accepted-main revision newer than the control commit is not yet tested.
No explicit new Sleep action is added. The fixtures demonstrate the bounded mechanism, not
voluntary selection or practical self-care. Its coefficient and usefulness
require broader measurement before promotion; the high-pressure authored arm
may cross a neural runtime's independent sleep threshold, which is not exercised
here. Old valuation genes and nominal action descriptors are not edited.
Source publication is not part of this task.
