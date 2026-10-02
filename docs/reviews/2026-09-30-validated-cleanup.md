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

## Codex integration review on Windows — 2026-09-30

Reviewed the cleanup commit `65cc8e2a8a2a4c5192865d74988821b25af15d82`
independently before integrating the full gameplay branch into `main`.

- Fixed three cleanup regressions exposed by optional feature combinations:
  `production_run_mode.rs` lost six required imports when its wildcard import
  was removed, and the Bevy checkpoint test in `v11_player_loop.rs` still needs
  the durable asset root that cleanup deleted. The GPU-enabled tools lifecycle
  test also lost its world-signature schema-version import. All three now use
  explicit feature guards. Windows all-feature, all-target workspace compilation
  passes with the locked dependencies.
- Deleted `runtime_structurally_includes_the_real_crate_private_unit_test_module`
  and its unused comment-stripping helper. This retired Task 7 check inspected
  other tests' names and source strings; the 14 actual private runtime tests
  remain registered and pass. The four host runtime tests also pass.
- Updated two stale enum-boundary assertions to include the already implemented
  `HeardLanguage`, `SemanticPrior`, and `Look` variants. Unknown values are still
  rejected; the three affected ABI-mapping tests pass.
- Simplified the waking/recovery emitter exclusion predicate without changing
  which emitters it retains. This removes two strict Clippy diagnostics on the
  Windows Rust 1.96 toolchain.
- Checked public commit, PR-review, and issue comments again. The additional
  [CI diagnosis on the cleanup commit](https://github.com/BobertNeek/A-Life/commit/65cc8e2a8a2a4c5192865d74988821b25af15d82#commitcomment-202822663)
  correctly identifies that a failed test step skips subsequent checks. CI now
  runs benchmark, Clippy, boundary, and docs checks after successful compilation
  even if tests fail, while honoring cancellation. The full workspace test
  command and its failure status remain intact. Workflow YAML parses locally;
  the changed scheduling still requires a GitHub Actions run.

Windows verification includes 102 distinct passing focused tests: 58 core
library/genetics/Era 1/chemistry tests, seven training-contract tests, three
enum-mapping tests, 14 private runtime tests, four host runtime tests, and 16
WGSL parsing/reflection/contract tests. Strict core-library Clippy, formatting,
core boundaries, docs checks, and the hearthling asset check also pass.

The broad-suite failure counts above belong to the Work thread's original
Linux runs; they are not a new Windows full-suite result. Existing full-suite
failures and the documented biology, graph-evolution, and neural-signal gaps
remain unresolved. N4096 growth remains research-only and its CI failure is
not grounds to delete its persistent-address migration test. No new rendered
playtest or physical-GPU execution is claimed by this integration review.

## GitHub comment follow-up — 2026-10-02

Source baseline: `main` at
`ace5b9a7662f69f5b0e12b699511140969a95bc4`. Reviewed the
[follow-up CI comment on f8c9b35](https://github.com/BobertNeek/A-Life/commit/f8c9b35382f17c925cc6cab1083d18de6613e6b8#commitcomment-202867442)
against current code and the newer diagnostics in GitHub Actions run
`36931482191`. The comment correctly identifies the N4096 research-growth
failure and strict lint failures. Its opening claim that all the later checks
pass conflicts with its own lint details; it is not evidence of a green suite.

### Confirmed repairs and reviewed fixture debt

- **N4096 growth:** migration retains source sensory ports by logical lobe
  address, but phenotype assembly validated that encoder as if it had been
  freshly compiled from the target genome. Added an explicit internal encoder
  origin and source-bound validation of the remapped assignments, including
  profile, scales, biases, clamps, and canonical digest. Ordinary genome
  compilation retains its original validation path. Migration also discarded
  candidate-family bias and innate drive/cue settings; it now preserves them.
  The existing growth test now checks inherited sensory ports, innate hunger
  response, and mismatched source inputs as well as persistent addresses and
  serialization. N4096 remains research-only and production admission still
  rejects it.
- **Biological records versus external teachers:** the old registry check
  allowed any additional Agent object without biology, because external
  teachers legitimately have no biological record. That also allowed removal
  of a creature's record to turn it into an apparent external actor. Exact
  world saves now carry `external_actor_ids`; the registered and external ID
  sets must be disjoint and cover the Agent objects exactly. Direct portable
  restore also validates creature summaries against biological records before
  constructing the world. Existing corruption tests now exercise direct
  restore, and focused regressions cover teacher round trips, missing biology,
  duplicate/overlapping/unknown external IDs, and absent actor metadata.
  Teachers remain ordinary perception sources and acquire no biological or
  neural authority from this metadata.
- **Save compatibility limit:** old saves with a complete biological registry
  still load without external-actor metadata. Legacy saves with neither field
  retain the existing empty-registry path. An old mixed cohort containing both
  biological creatures and unregistered external actors is ambiguous without
  explicit provenance; it now fails validation and needs an explicit
  migration. This repair cannot safely infer which missing records were
  teachers. No automatic migration for that ambiguous case is claimed.
  Loading an old complete registry preserves its absent optional field and
  exact serialized save anchor. The durable checkpoint comparison accepts the
  equivalent empty external cohort produced by a validated live snapshot;
  other world differences still fail. Existing journal-anchor checks now cover
  this older representation instead of rewriting its persisted bytes.
- **Physics and action fixtures:** traced the reported failures to current
  duration-limited movement, contact radii, and action contracts before changing
  expectations. Walking fixtures now exercise the actual 0.1-unit interval;
  obstacle tests still require a swept collision between clear endpoints, and
  hazard tests distinguish travel from physical contact. Carry/save tests use
  the carrier's measured displacement. Rest does not fabricate an energy
  credit, and negative social affinity does not become positive affiliation.
  Grounded object groups include the existing Play action and still reject
  partial groups. These repairs retain the behavioral checks rather than
  removing them or changing production physics to fit old fixtures.
- **Allocator and lifecycle fixtures:** actor spawning already reserves
  organism IDs before biological registration. Overflow checks now test that
  reservation boundary, and the signature test constructs genuinely different
  future allocator states. The malformed lifecycle fixture now alters the
  actual serialized `Dead.death_tick` field. No allocator or lifecycle
  production behavior was changed for these fixtures.
- **Legacy gaze fixture:** the old-object migration fixture retained current
  gaze fields while deleting its other modern identity fields. It now removes
  `body_yaw`, `head_yaw`, and `optical_opacity` together to represent an actual
  legacy object, checks their migrated defaults, and verifies that current
  objects reject missing gaze fields. Production deserialization guards remain
  intact.
- **Strict CI lints:** five receipt/arena/digest functions retain their explicit
  validated arguments with function-scoped, reasoned
  `clippy::too_many_arguments` expectations. The Era 1 corruption fixture uses
  a named mutation alias, and the measured-physiology fixture rounds its
  widened `u32` maturation duration with the equivalent integer `div_ceil`.
  Two unchanged private test modules moved after
  production items. Five atomic counter calls use the equivalent `try_update`
  name reported by the newer compiler; a compatibility probe confirmed that
  signature on the installed Rust 1.96 toolchain. No toolchain requirement was
  raised and no broad lint suppression was added.
  Further diagnostics exposed equivalent Copy, borrowing, arithmetic, and
  iteration repairs in GPU fixtures and training code; checkpoint, scheduler,
  imitation, and archive rollback signatures retain narrowly reasoned
  expectations. The durable authority enum keeps its infrequent snapshot
  evidence inline. Speech labels include silence as one supervised step, so
  their new `is_empty()` method is always false. New Game imports now match
  the GPU feature that uses them.
- **Experimental founder asset incompatibility:** the meaningful
  `new_game_base_save_matches_every_canonical_founder` test passes its default
  BuiltinNano512 staging and save assertions before failing at the explicit
  `ScaledChoiceNociceptiveV1` selection. A read-only diagnostic linked against
  freshly rebuilt core code confirms that the bundled candidate fails canonical
  decoding before world construction. The survival-urgency change `cbf22ad6`
  changed its action decoder contract: the bundled `action_decoder_digest` is
  `[10692070626024972832, 1033931365391295994, 1631642021198394214,
  2994609197215179484]`, while the current compiler produces
  `[3752274248244230604, 11910143686153284588, 15918497978615614238,
  9890794923746952423]`.
  All 1,799 weight values match bit for bit. The layout, route, plasticity,
  persistent-address, memory-decoder, schema, foundation-version, and
  training-stage fields also match. The changed decoder and source-phenotype
  provenance alter the receipt and asset identity: the retained asset digest is
  `a2cdb86da20f8804c878e85369c299e796b5cac8e6613ba4aef43441a7a6ee1e`,
  while a diagnostic reconstruction with the same weights produces
  `af133b466bc8b7286562ee7f666057e1c675d09eb1c6cda2c8f04cbc84d73cbd`.
  That reconstruction was not saved or admitted. No existing candidate
  migration covers this changed contract, and historical evaluation cannot
  certify the changed decoder. The test and historical asset remain intact;
  rejection preserves the explicit incompatible-asset boundary required by
  AOA-PRIOR-008. A replacement requires explicit identity/provenance and current
  behavioral evaluation, rather than an automatic rebase or inferred evidence.
- **Remaining strict lint debt:** fresh strict workspace Clippy also reports
  existing app dead-code, large-error, and type diagnostics for `gpu-runtime`
  without Bevy. These extend beyond the six reviewed locations. No broad
  suppression or test deletion was applied to hide them.

### Verification for this follow-up

Fresh Windows verification uses Rust 1.96, locked dependencies, the existing
target directory, and `CARGO_INCREMENTAL=0`. The first cached growth result was
misleading; the nonincremental baseline reproduced the migration failure before
the repair, and the repaired test passes.

Focused results: **162 distinct tests passed; one remains failed**:

- Core growth and N2048 foundation contracts: 16 passed.
- World actions, grounded sensors, signatures, organism registry/bindings,
  persistence, save/load, and canonical New Game: 135 passed.
- Runtime checkpoint publication/CAS and journal authority: 10 passed.
- GPU-feature checkpoint world compatibility helper: one passed without a GPU.
- The app founder lifecycle test remains failed only at the incompatible
  opt-in asset described above, after its builtin staging assertions pass.

Bookkeeping runtime/app tests use local unoptimized package overrides for the
GPU backend, runtime, and app. Repository profile settings are unchanged. No
hardware execution or performance result is inferred from these CPU checks.

Strict workspace Clippy remains failed on the app configuration debt described
above; the six original review diagnostics and the additional bounded repairs
are addressed. Passed: strict Clippy for the core, world, and GPU backend
libraries plus the Era 1 receipt test; `cargo check --workspace --all-targets`;
`cargo fmt --all -- --check`; `git diff --check`; static core-boundary checks;
and documentation checks (77/77 assertions). Workspace compilation still emits
the documented app feature warnings. Earlier full-suite counts certify their
historical runs only; this follow-up did not repeat the
full suite or claim Linux equivalence, physical GPU execution, or a rendered
gameplay test. The previously documented training, graph-evolution, and
neural-signal integration gaps remain outside these repairs.
