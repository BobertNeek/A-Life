# Commit-review validation and bounded cleanup — 2026-09-30

## Source and scope

Baseline: `codex/review-gameplay-fixes-20260916`, commit
`4498bd97add05f23989a87e44d9de387ebfbc575`. This is ahead of `main`; reviewing
`main` would miss both the repairs and the latest concerns.

Read all five repository commit comments and the closed Jules PR report:

- [Sensor-profile build failure](https://github.com/BobertNeek/A-Life/commit/9e5af4cacdd5fd24b78b3a9505d64692d3258655#commitcomment-202529176).
- [Biochemical graph evolution diagnosis](https://github.com/BobertNeek/A-Life/commit/9e5af4cacdd5fd24b78b3a9505d64692d3258655#commitcomment-202532740).
- [Consolidated six-session Jules review](https://github.com/BobertNeek/A-Life/commit/7ca9d45952d545d4a0bae6610e29ef7513892499#commitcomment-202674556).
- [Era 1 lineage-test failure](https://github.com/BobertNeek/A-Life/commit/4498bd97add05f23989a87e44d9de387ebfbc575#commitcomment-202689545).
- [Latest four-pass review](https://github.com/BobertNeek/A-Life/commit/4498bd97add05f23989a87e44d9de387ebfbc575#commitcomment-202692168).
- [Jules report, PR 1](https://github.com/BobertNeek/A-Life/pull/1).

The five commit comments are posted as `BobertNeek`; their bodies attribute the
consolidated findings to Jules and the user's assistant. No separately authored
Muse comment was exposed. Private Jules session pages were not consulted; this
review validates the concrete claims reproduced in the repository comments.

The audit covered all workspace targets, lint diagnostics, dead-code suppressions,
the reported genetics/biochemistry/training paths, and their production callers.
It is not a proof that every line of the 511 Rust/WGSL source and test files is
defect-free. The controlling architecture was not changed.

## Latest review dispositions

Numbers below refer to the latest four-pass review unless marked **earlier**.
Source locations in this table refer to the baseline, before cleanup formatting.

| Claim | Disposition and evidence |
| --- | --- |
| 1–3: hardening, active battery, and Era 1 are inert | **Valid integration blockers.** `evolution.rs:393,583`, `active_battery.rs:363,673,932`, and `era1_trials.rs:209,776` unconditionally reject. Active battery uses locally assembled homeostasis and unregistered challenge agents; hardening fabricates outcomes. The Era 1 gate occurs before neural dispatch, so the claim that every path first completes a GPU tick is overstated. Cleanup adds an explicit constructor error before adapter/session initialization; it does not enable these evaluators. AOA-BIO-001/002 and AOA-LEARN-007 require real organism biology and local receptor evidence. |
| 4: SparseGeneticDelta only changes IDs | **False for builtin foundation compilation.** Reseeding changes `genetic_prior_seed`; `phenotype/construction.rs:283-311,358` adds a seed-dependent inherited weight delta to foundation weights. Added a regression compiling parent and offspring and requiring changed genetic weights with unchanged neuron coordinates. Exact candidate admission intentionally suppresses this overlay. |
| 4: RouteDensity always fails | **Valid, wrong proposed mechanism.** `evolution.rs:181-187` mutates one row. `phenotype/topology_compile.rs:173-218` requires each row to match its own frozen route density, not all rows to equal one another. Mutating every row would still violate the frozen N2048 ABI. The evaluator classifies compile failures as nonviable; this is an unsupported mutation probe, not evidence that all mutations are inert. |
| 4 / earlier 5: BiasLeakResearch is nonviable | **True and explicitly intentional.** `evolution.rs:278-292` states that the frozen genome lacks this heritable lane. Implementing it changes the neural ABI and requires its own architecture/evidence work. |
| 4: mutation robustness is a permanent placeholder | **False as stated.** It starts at zero at `evolution.rs:411`, then finalist descendant evaluations replace it at `:459-491`. No robustness result is currently reachable because of blocker 1. |
| 5,9: PredictionResidual neuroemitters have no production source | **Valid wiring gap.** `gpu_live_runtime.rs:4708-4727` emits only RegionalArousal, MotorCommitment, and ExecutiveSustain; `biochemical_graph.rs:1195-1206` expects PredictionResidual. Current action residual is measured later at `gpu_live_runtime.rs:4807-4819`, after motor/biochemical publication, so a same-tick one-line insertion would violate causal ordering. GPU learning still receives prediction residual through the sealed learning frame. AOA-BIO-008; separate causal integration repair. |
| 6: reactions are order-dependent | **Valid behavior, not nondeterminism.** `BiochemicalGraphState::advance`, `biochemical_graph.rs:1442-1450`, applies reactions to staged state in declaration order. `apply_reaction:1649` reads that state. Cleanup documents the solver and tests producer/consumer ordering, material conservation, and serialization-preserved order. Replacing it with a simultaneous solver would alter dynamics. |
| 7 / earlier graph diagnosis: no graph evolution | **Structural part valid; parameter part stale.** `graph_gamete`, `evolutionary_genetics.rs:2385-2425`, now recombines and mutates bounded homologous graph parameters via `inherited_parameter` and records provenance. There are still no add/remove species, reaction, or edge mutation operators, and unequal topology does not combine homologously (`biochemical_graph.rs:531-560`). AOA-GEN-006/010 remains a capability gap. |
| 8 / earlier 8: N2048 lobe/density loci are ignored | **Valid limitation of the frozen foundation.** `express_brain_genome`, `evolutionary_genetics.rs:1463-1490`, excludes N2048 overrides; `phenotype/inputs.rs:467` records their source identity while the foundation compiler preserves fixed coordinates. This is not a reason to silently alter a trained foundation layout. |
| 10: SocialState has no neural source | **Valid for that class.** It is absent from the production emission frame. Body SocialContact emitters remain live (`biochemical_graph.rs:1165-1170,1214-1218,1806`). Missing neural social-state emission does not mean all social chemistry is inert. |
| 11: social-consequence plasticity lane is always zero | **Valid gap.** `learning.rs:154-164,305-313` constructs the canonical outcome frame with a zero social lane. Plumbing and sleep serialization cannot substitute for a causal producer. AOA-LEARN-007 remains incomplete here. |
| 12: discrete mutation misses the journal cap | **Valid latent contract defect; fixed.** `mutate_continuous_value:1937` and count mutations already stop when the journal is full; `mutate_discrete_allele:2087` did not. Added the same pre-mutation cap and a test proving the last record fits and a subsequent mutation cannot change an allele without provenance. |
| 13: CPU and GPU sleep rules differ | **Valid difference.** `sleep.rs:1229` requires promoted stable traits; `closed_loop_consolidate.wgsl:184-186` stages fast weights unconditionally. The production GPU path is authoritative; this CPU helper cannot certify equivalence for that rule. Separate consolidation-contract repair; no shader semantics changed in cleanup. |
| 14: h_shadow is a dead production upload | **Not established on the closed-loop path.** Its remaining storage/encoding is in `buffers.rs` and recompaction compatibility data. No h_shadow field exists in the current closed-loop slot encoding in `closed_loop_buffers/bucket.rs`. Do not delete persisted/compatibility data based only on no WGSL reader. |
| 15: graph_epoch exists only in biochemistry | **False literally; semantics remain limited.** `closed_loop_exact_capture.rs:49,412,688` exposes a neural `graph_epoch` and binds it to transaction generation. It is not an independently incremented structural epoch. Epoch semantics deserve explicit architecture review; digests alone do not prove AOA-STR-007. |
| Earlier: workspace does not compile | **Stale.** Baseline `cargo check --workspace --all-targets` passes. The unstable match guards and missing enum arms are already repaired. Do not replay those fixes. |
| Earlier: praise clearance is a valuation bug | **Stale / proposed fix incorrect.** `chemistry.rs:72` intentionally prevents an expired praise pulse from punishing the next action. `biochemical_valuation.rs` already tests that exception and passes. |
| Earlier: mutation bound mismatch | **Already fixed.** The allele upper bound now caps mutation range; existing genetics tests cover it. |
| Earlier: distinct genome IDs are required for mating | **Already fixed.** World mating rejects identical organism IDs. Two distinct founders with the same genome may reproduce. |
| Follow-up: duplicate parent genomes need a clonal-mode field | **The failed test is real; the diagnosis conflates genome and organism identity.** Sexual mating between genetically identical organisms is not clonal reproduction. The receipt does not carry parent organism IDs and was never a cryptographic lineage authenticator. Cleanup accepts equal parent genome IDs while rejecting invalid IDs and either self-parent ID; it preserves the world self-mating guard without changing the persisted receipt schema. |
| Earlier: newborn appearance is lost on save | **Overstated.** `gpu_live_runtime.rs:9916-9952` derives appearance from parent save records and stores it in `CreatureSaveState`; `persistence.rs:971,1881` persists and validates it. Losing ancestor availability before that projection and composite-asset provenance are separate lineage questions; no blanket loss claim is justified. |
| Earlier: dopamine/appetitive learning is permanently zero | **Already repaired.** Current graph body emitters produce dopamine; existing graph/valuation tests exercise nutrition, praise, social, and developmental signals. The missing PredictionResidual route remains distinct. |
| Earlier: threshold/ATP genes are wholly unexpressed | **Mixed.** Sleep setpoint is applied in `express_brain_genome`; reproductive threshold is used at `biochemistry.rs:609`; ATP efficiency is used in body metabolism at `biochemistry.rs:206`. Legacy scalar drift fields coexist with graph dynamics and are not interchangeable. Some graph-specific mappings remain incomplete, but the broad dead-gene claim is false. |
| Earlier: material↔regulatory conversion escapes conservation | **False for material stock.** `biochemical_graph.rs:1618-1645` compares material reactant and product totals, so a material-to-regulatory-only reaction fails. Regulatory signals are not conserved material. Existing conservation tests cover mixed reactions and catalyst behavior. |
| Earlier: omitted drives/hormones never affect behavior | **Stale.** Current innate-family masks feed GPU logits; receptor frames feed excitability, attention, plasticity, and sleep. A hormone's absence from the older ChemistryModulation helper does not prove absence from production receptors. |
| Earlier: teachers bypass neural arbitration | **Not supported at the cited sites.** `factorized_selection.rs:100-145` packages already selected commands. `alife_school/src/verifier.rs:207-256` distinguishes heuristic diagnostics from neural evidence and validates the selected candidate, target, duration, and absence of teacher/private trace fields in the neural case. It does not select teacher actions for the production creature. |
| Earlier: brain-class mutation must create derived regions | **Real broader capability gap, proposed change unsafe.** Class promotion is gated to N512/N1024/N2048. Automatically mutating to a larger class is not the same as implementing viable derived-region alleles. AOA-GEN-006 needs a dedicated implementation slice. |
| Earlier: fixed sensory ports ignore every inherited sensor | **Not proven by port-count comparison.** Physical receptor count and encoded neural lane count are different contracts. A causal tracing test is needed before changing the sensory ABI; no silent widening in cleanup. |

## Cleanup delivered

- Corrected the Era 1 stale tamper-test assumption and added rejection coverage
  for invalid and self-parent IDs without changing serialization.
- Bound discrete mutations to the shared mutation-provenance capacity.
- Added named pre-GPU readiness errors to all three blocked legacy evaluators;
  the independent foundation trainer and live production loop retain their gates.
- Removed the redundant app factorized-arbitration re-export module, unused
  `_asset_index`, unused exact-capture consuming helper, and unused arena getters.
- Restricted test-only getters and fake-provider conversion helpers to their
  actual configurations, and Windows archive test helpers to Windows.
- Removed obsolete dead-code suppressions from live archive transaction types
  and the retained staging adapter. Removed the unused fixed-arena ownership
  getter and scoped its slot validation helper to tests; production bucket and
  arena ownership checks remain in use.
- Scoped contact-kind imports directly to two opt-in GPU test modules instead
  of leaving them unused in production builds.
- Kept the arena staging allocation because it is part of admission accounting;
  removing an unused accessor does not authorize changing the memory budget ABI.
- Applied mechanical lint fixes for Copy clones, redundant conversions/iteration,
  manual arithmetic, derivable defaults, and option predicates. Fixed-size chunk
  conversion retains existing remainder validation. No shader, foundation bytes,
  neuron counts, replay/persistence schema, population, or pacing changes.
- Replaced the topology split candidate tuple with named fields while preserving
  its rank and tie-break rules. Kept six explicit multi-field receipt/ABI
  constructors with narrow, reasoned `clippy::too_many_arguments` expectations;
  a large argument count alone is not an architecture defect.

## Validation

Passed:

- `cargo fmt --all -- --check` and `git diff --check`.
- Static core-boundary checks and documentation checks (77/77 assertions).
- `cargo check --workspace --all-targets`.
- App checks with `gpu-runtime,foundation-training,gpu-tests`, all targets.
- Semantic-provider checks with both default and all features, all targets.
- `cargo clippy -p alife_core --lib -- -D warnings`.
- All 16 non-hardware WGSL contract tests, including Naga parsing/validation,
  reflection of the seven heap bindings, and canonical ABI-prefix checks.
- Focused tests: **47 passed, 0 failed**, covering the core library, Era 1
  receipts, biochemical valuation/conservation, hardening genetics, and blocked
  evaluator constructors. All five new regression tests pass.

The full workspace test command completed with **1,442 passed, 225 failed,
15 ignored** across 192 test executables. It is not a green workspace suite.
The original commit independently completed with **1,437 passed, 225 failed,
15 ignored** across 191 executables. The cleanup fixes the original lineage
test failure and adds five passing regressions. The only additional failure
in the broad cleanup run is the atlas test, which encountered zero-byte PNGs;
it passed in the baseline checkout. The optional triangle-iteration cleanup
was reverted; the atlas test then passed both on its own and with the workspace
feature configuration. Those optional changes are excluded from this cleanup.
All other 224 failing test names also fail at the baseline. Two legacy evaluator
tests now report the explicit biology blocker instead of a hardware-adapter
error. The complete suite was not repeated after the revert.

Failures also reveal existing test/implementation debt beyond the review claims:
old genome digests, raw enum rejection boundaries, N2048 capacity expectations,
old GPU receipt sizes and shader-text assertions, unavailable generated/content
assets, and pre-graph save fixtures. Grounded-world hazard, movement/collision,
allocator, and malformed-registry rejection tests also fail; these are potential
behavior or persistence defects, not dismissed as stale fixtures. Updating a
golden value without tracing its contract would conceal an ABI or persistence
change. These require their
own evidence and fixture repair; this cleanup does not rewrite those receipts.

Ordinary workspace Clippy completes, but strict workspace Clippy still fails
on other crates' argument counts and further lint debt. Core library Clippy is
strictly clean; this is not a claim that every workspace lint was removed.

The all-feature graphical app check cannot complete here because the Linux
`libudev-sys` build needs unavailable `pkg-config`/libudev development support.
GPU execution tests reject the available llvmpipe software adapter, as intended.

Local optimized test codegen with Rust 1.98.1 produced empty object/archive
errors; validation uses fresh nonincremental artifacts and unoptimized local
package test overrides for core, world, GPU backend, runtime, and app. All
commands use the locked dependencies. Repository production/test optimization
settings are unchanged.

No RTX 3050 production run or rendered Windows acceptance was available here.
CPU/contract tests and WGSL parsing do not certify production GPU behavior,
performance improvement, or readiness of the blocked evaluators.

## Recommended next implementation slice

Integrate one canonical creature into the Era 1 runner before enabling any of
the three legacy evaluators: register its genome-expressed organism biology in
the grounded world; execute selected actions through that world; seal measured
physiology and receptor evidence; apply or discard the matching pending GPU
eligibility transaction; preserve cleanup/rollback on every error. Keep a
two-tick RTX 3050 causal gate before attempting a whole battery or generation.
Then decide reaction solver semantics and structural graph mutation separately.
