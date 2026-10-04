# Physical peer-contact chemistry — 2026-10-04

Cassidy authorized fixing discovered issues with efficient WWCD behavior. Base:
`482948ec116bad505cee0225cad74c1ab110acb7`. This addresses the verified bootstrap
failure: ordinary New Game and genetic newborn peers have static affinity zero,
and movement formerly multiplied their SocialContact source by that label.
A stationary positive-affinity Approach could instead deliver a full source
on every 20 Hz interval. Personal signed recognition remains a separate,
experience-derived mechanism.

## Mechanism and scope

The world now supplies `0.25 / WORLD_TICKS_PER_SECOND` (0.0125) physical peer
exposure at each canonical biology boundary when a living peer is touching at
an interval endpoint. One second of continuous touch supplies 0.25 total source
exposure. This is a tunable physical intensity, not a reward, bond, friendly
label, action preference, or hormone assignment. Actual relief, oxytocin and
other effects still come from each organism's inherited biochemical graph,
including emitter cadence, developmental expression, gains and receptor genes.
No genotype, phenotype, genetic calibration, neural selector, shader or GPU ABI
changes are made.

The real-frame test also found that both grounded profiles left the existing
`nearest_agents` observation port empty. Exact positive peer memory was present,
but the recognition bridge correctly refused to treat an unbound object as a
currently observed creature. World frame assembly now binds peer body identities
from the already extracted object transports, stopping at the existing eight
social slots. Gaze, position and proximity are measured; affinity is always zero.
No hidden nearby peer search or category reward is added. Out-of-frame peers and
dead registered peers are excluded. The personal signed recognition rules and
GPU action selection remain unchanged.

The single motor-bundle transition combines touch with all ordinary measured
body events. Initial/final overlap uses the existing hazard endpoint scan; a
stationary bundle reuses its initial result. Movement reports physical contact
but no longer supplies affinity-weighted chemistry through its channel profile.
Multiple peers and motor channels cannot multiply the source. SocialContact
composition uses maximum interval exposure, including pending player strokes,
so simultaneous reports do not stack. A player stroke retains its existing
0.25 source. Registered legacy/neural commands and passive organisms reach the
same source. Organisms already advanced by an action are skipped at world tick
finalization, as before.

Touch ignores gaze, attention and static affinity; it requires unconsumed peer
geometry within the smaller of the peer radius and canonical touch radius,
solid/terrain reachability, and a living peer when that peer has a registered
organism. Existing unregistered embodied social actors remain physical peers.
Self, dead bodies, consumed bodies and blocked geometry supply no peer dose.
Leaving contact ends exposure on the following interval. Endpoint sampling is a
bounded approximation: contact during part of an interval can receive one small
interval dose; continuous swept-contact duration is not modeled.

Source: [world biology paths](../../crates/alife_world/src/headless.rs),
[inherited contact emitters](../../crates/alife_core/src/biochemical_graph.rs),
[production sealing](../../crates/alife_game_app/src/gpu_live_runtime.rs), and
[personal recognition](2026-10-03-social-recognition.md). Requirements traced:
AOA-BODY-005, AOA-BIO-006, AOA-BIO-015, AOA-BIO-025, AOA-BIO-028,
AOA-MEM-007, AOA-INV-001, AOA-INV-005, AOA-WORLD-001, AOA-PERSIST-004.

## Evidence and limits

Focused [world tests](../../crates/alife_world/src/headless/contact_chemistry_tests.rs)
construct actual canonical zero-affinity founders, let their real newborn
loneliness accumulate for 600 intervals, and compare touching and separated
controls. At this age a small touch produces both measured loneliness relief and
positive inherited biological value. At newborn drive floors, oxytocin can rise
without loneliness relief; this does not manufacture instant affection. The
founder oxytocin value weight is zero. The existing player-praise extension
weight remains genetically configured and unchanged.

The [app test](../../crates/alife_game_app/src/gpu_live_runtime/contact_chemistry_tests.rs)
passes a validated test selection into the production sealer, then observes its
actual measured physiology in the ordinary personal memory bank. It checks
absent stranger recognition before experience, target-specific positive recall
after beneficial touch, no borrowed stranger bond, exact memory roundtrip, and
recognition while Idle wins at separated sight range. A matched actual hazard
injury with the same peer creates signed negative evidence and suppresses
recognition, including when combined with the small positive experience. No
synthetic remembered valence or drive assignment is used. These are CPU causal
checks of production sealing and recall, not evidence that the GPU selected an
action, learned new weights, or improved relationship quality across a lifetime.

World coverage also checks static affinities -1/0/+1, zero-motion Approach,
observed-only neutral identity binding in both grounded sensor profiles,
duplicate contact-reporting channels, command durations 1/40, passive/legacy
cadence, no double dose at finalization, one-second exposure, solids, dead peers,
separation, inherited genetic newborns, current save/load continuation, and
late transaction failure followed by exact retry. No new persistent field,
contact queue or legacy migration is introduced. Exact acquired GPU checkpoint
continuation and ecological relationship quality remain Unknown.

## Cost and verification

The endpoint helper allocates no contact list or index. Bundle overlap reuses
an existing traversal, with one rather than two endpoint traversals for zero
motion. Legacy registered commands and passive organisms have no existing
endpoint contact receipt, so they perform a bounded object traversal (two for a
moving legacy command). Once a touching peer is found, further peers need no
reachability checks. Obstruction checks reuse the existing geometry routine.
The world object cap bounds this work; no additional all-pairs relationship
pass or cognitive search is introduced. Candidate traversal is linear per
endpoint, plus obstruction checks for close candidates until one is reachable.
Observed social identity assembly adds no allocation and visits only the
existing bounded transports, stopping once eight living peers are bound.
Dense blocked contacts and large passive populations can cost more; no overall
population speedup is claimed. Focused controls verify unchanged biochemical work receipts.

The manual endpoint microbenchmark compares the exact former hazard-only scan
with the new fused helper in the same process: 20,000 samples per configuration,
unregistered separated physical peers, no biology/neural execution, optimized
world test code, Rust 1.98.1, Linux cloud AMD EPYC 9V74. Mean wall nanoseconds
per endpoint:

| Peers | Previous hazard scan | Fused contact scan |
| --- | ---: | ---: |
| 1 | 8 | 13 |
| 10 | 37 | 52 |
| 100 | 317 | 489 |
| 500 | 1,598 | 2,294 |

These isolated sparse-geometry timings were measured while other compiler work
could run. They are not latency guarantees, dense-obstruction evidence, a full
population profile, or a GPU performance result. The stationary endpoint reuse
and allocation-free helper are source evidence; the biochemical work equality
is test evidence. No training, PC work, renderer, or GPU workload is run.

There is no changed-file overlap with GeneForge PR3 commit `2d1849cb7131e251b402ee6bd032beda16afb3f0`.
The shared `headless.rs` interface may need coordination with the separate island
work; no terrain construction, art, renderer, or asset file is edited.

Final CPU validation passed: all 359 world tests, 77 selected core chemistry and
memory tests, and four app sealing/recognition/late-retry tests (440 total).
Three existing/manual benchmarks remain ignored in those normal invocations;
the contact endpoint benchmark was run separately and passed. Both strict
workspace all-target Clippy and `gpu-runtime` app all-target Clippy pass with
warnings denied. Formatting, `git diff --check`, dependency/source core
boundaries and all 77 documentation assertions pass. No training features,
neural execution, graphical workloads or PC work were run.

```sh
cargo test -p alife_world --all-targets
cargo test -p alife_core --test candidate_memory_retrieval --test candidate_memory_queries --test biochemical_graph_v2 --test biochemistry_development --test biochemical_valuation
cargo test -p alife_game_app --features gpu-runtime --lib contact_chemistry
cargo test -p alife_game_app --features gpu-runtime --lib seal_prepared_selection_
cargo test -p alife_game_app --features gpu-runtime --lib late_sealing_failure
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p alife_game_app --features gpu-runtime --all-targets -- -D warnings
bash scripts/check_core_boundaries.sh
bash scripts/check.sh --quick
```
