# Simulated biology consistency audit — 2026-10-02

## Authorized follow-up: bounded taste repair

Local repair branch: `fix/taste-continuity`, still based on `d2e4a157`. This follow-up supersedes the initial audit's deferral for finding 1 only. Findings 2 and 3 below remain unchanged, with their impact uncertainty and static evidence intact. This local validation snapshot precedes publication. No merge, training, GPU execution or desktop use occurred.

The world now stores one `IngestionObservation` per eater: ingestion tick plus signed physical taste sampled at successful consumption. Object-slot chemistry cannot replace this because consumed objects leave observation; episodic memory and tracked-object history are not transient mouth sensation. The sample survives other creatures' actions, subsequent channels in a motor bundle, and later edits/removal of the food. Existing nutrition, damage, reward, and neural state paths are unchanged.

Lifetime is clock-based: ingestion at tick `t` remains observable through sensory tick `t+1` and is removed on world advancement to `t+2`. Re-reading perception does not consume or refresh it. Failed ingestion does not create or extend a sample. Death, retirement, both supported external-actor removal paths, and exact registry replacement prune the relevant state. The map is bounded to current eaters and at most one sample per creature.

Save state carries the samples and includes nonempty observations in the world signature. Older saves lacking the optional field load with no historical taste sample, matching their previous behavior; empty saves retain their existing serialization/signature form. Present samples are checked for range, timing and living/extant ownership. Restore does not infer taste from mutable food or action summaries.

**Independent timing review (R2):** production captures pre-action perception, executes/seals actions, then advances the world. Therefore the sample reaches the next awake perception and GPU selection, through the existing upload of all sensory channels. It does not retroactively enter the eating action's pre-action eligibility or immediate successor prediction. A sleeping creature or one that misses inference does not retain a physical taste forever. This repair provides sensory continuity; it does not claim measured improvement in learned food preference.

**Red/green evidence:** the three initial assertions were inverted into desired-behavior regressions and run before production edits: all three failed (`0` instead of `0.82417995`). After repair, all three passed with unchanged taste `0.82417995`. Six additional tests pass for clock expiration/failed ingestion, independent positive and negative tastes, sampled-food removal/edit behavior, dead-creature cleanup, both external removal APIs, and legacy/malformed saves. The sensory checks use the indexed preparation path used by production; the two-eater test advances the world before checking each creature's distinct sensory lanes.

**Final follow-up validation:** 163 selected CPU tests passed: all 67 world unit tests, 9 taste regressions, 9 canonical-new-game tests, 15 grounded-object tests, 9 world-signature tests, 24 save/load roundtrips, 16 atomic-action tests, 2 v11 biology tests, and 12 organism-registry persistence tests. This includes existing injected late-failure rollback after successful eating; the new sample is covered by the restored canonical world signature. `scripts/check.sh --quick` passed core boundary checks and all 77 documentation assertions. Workspace formatting (`cargo fmt --all -- --check`) and `cargo clippy -p alife_world --all-targets -- -D warnings` pass. `git diff --check` and reverse patch applicability checks pass.

Command (after sourcing `/workspace/cloud-cpu-validation/env.sh`):

```sh
CARGO_BUILD_JOBS=1 cargo test -p alife_world --lib --test biology_audit_taste --test grounded_object_slots --test save_load_roundtrip --test world_organism_registry_persistence --test headless_world_signature --test canonical_new_game --test task_3_2a_atomic_action --test v11_cognitive_biology
```

The production changes and focused regression tests accompany this report. Independent source review found no blocking issue. Publication is authorized on the separate `fix/taste-continuity` branch; full and fast CI use an identical-SHA `phase3-taste-continuity-ci` branch because the configured push filters exclude `fix/**`. No workflow edits or reviewer requests are needed. Main remains unmerged; the publication receipt supplies the exact commit and CI links. No GPU learning-effectiveness run was performed or claimed. No changes were made to the other two audit findings.

## Initial audit record (before the authorized repair)

Baseline: `d2e4a157ec8b14fc21445d55d560ab36c4551d49`, verified as remote `main` before investigation and again after resuming. Local branch: `audit/biology-d2e4a157`, worktree `/workspace/alife-biology-audit`. The original `/workspace/A-Life` checkout was preserved. No production source, remote branch, PR, training state, or GPU state was changed.

Scope: Creatures-inspired organisms with inherited competence and lifetime learning. This review traces production callers and causal state rather than demanding maximum realism. Root/crate AGENTS, controlling v2.0 sections and requirement registry were consulted. No `.agents/skills` existed in the repository; `/workspace/.agents` was empty. Repository Spec Loop Ponytail review guidance was used in review mode. Independent domain agents reviewed metabolism/health, cognition, and development/reproduction; the primary reviewer integrated persistence and CPU reproductions (R2 domain review, shared exact-baseline worktree).

## 1. P2 — Taste is erased by unrelated actions, neutral posture, and save/load

**Verified by three focused CPU reproductions of registered motor actions.** The creature actually eats and receives nutrition, but its next grounded sensory frame loses the food's taste.

The grounded nonvisual sensor obtains taste exclusively from the world's single `last_action_result`, filtered to the observer and an Eat action: [headless.rs:3014–3027](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_world/src/headless.rs#L3014-L3027). A motor transaction stores only its last executed channel at [headless.rs:3663](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_world/src/headless.rs#L3663). Vocal and posture channels execute after manipulation, including the neutral posture channel: [headless.rs:5894–5901](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_world/src/headless.rs#L5894-L5901). Any other organism's transaction also replaces the same field. Restore resets it to `None`: [headless.rs:2656](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_world/src/headless.rs#L2656).

This is reachable in production: the live action path calls the same motor implementation at [gpu_live_runtime.rs:4813–4820](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_game_app/src/gpu_live_runtime.rs#L4813-L4820), and the next preparation reads the grounded frame at [staged_tick.rs:561](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_game_app/src/gpu_live_runtime/staged_tick.rs#L561). The immediate successor prediction state does not rescue taste: [gpu_live_runtime.rs:4468–4512](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_game_app/src/gpu_live_runtime.rs#L4468-L4512) encodes body position, velocity and drives, not tactile taste.

**Creature-visible consequence:** taste availability depends on cohort processing order and unrelated simultaneous actions. A creature cannot reliably receive the taste of food it just ate. Loading changes its immediate sensory input despite retaining the same food and body state. This does not demonstrate loss of all food learning: nutrition and biochemical learning still occur.

**Smallest repair:** retain a bounded, organism-owned, tick-stamped ingestion/taste observation when ingestion succeeds; read it for the intended sensory interval and include it in exact world persistence. A neutral action or another creature's action must not erase it. Define expiration explicitly; do not preserve a complete unbounded action history or move cognition to the CPU.

**Evidence:** `crates/alife_world/tests/biology_audit_taste.rs` uses registered organisms, `apply_registered_motor_bundle`, and the production GroundedTerrainVision sensor. Observed taste:

| Case | Control | Defective result |
| --- | ---: | ---: |
| Eater, then another creature rests | 0.82417995 | 0 |
| Eat alone versus Eat + NO_POSTURE | 0.82417995 | 0 |
| Before versus exact world save/restore | 0.82417995 | 0 |

At the initial audit stage, the local tests intentionally asserted the observed defect; green meant reproduction succeeded. The authorized follow-up above converted them to desired-behavior regression assertions and demonstrated red/green. No GPU action-selection or trained preference claim is made. Architecture references: AOA-SENSE-001, AOA-PERSIST-001, AOA-PERSIST-004.

## 2. P2 — Inherited sensitive learning periods do not reach live neural plasticity

**Verified source-level integration gap; no GPU behavior test.** The genome expresses period timing and a nonzero plasticity bias at [evolutionary_genetics.rs:1337–1350](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_core/src/evolutionary_genetics.rs#L1337-L1350). World age correctly updates resident development at [gpu_live_runtime.rs:572](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_game_app/src/gpu_live_runtime.rs#L572).

However, stable production construction clears open periods at [gpu_live_runtime.rs:10143](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_game_app/src/gpu_live_runtime.rs#L10143). N512 foundation projection likewise compiles a separate mature coordinate state at [n512_founder_projection.rs:388–402](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_core/src/phenotype/n512_founder_projection.rs#L388-L402). The only actual period multiplier is in one-time phenotype compilation: [learning.rs:485](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_core/src/phenotype/learning.rs#L485), [learning.rs:700–710](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_core/src/phenotype/learning.rs#L700-L710). Repository-wide searches found no runtime or GPU consumer of current open periods or `critical_period_plasticity_bias`.

**Creature-visible consequence:** passing through the inherited sensitive period does not supply its intended temporary learning-rate advantage. This is specifically the critical-period modulation, not an assertion that all development or lifetime learning is inert. Puberty and biochemical developmental gates are causal.

**Smallest repair:** apply current authoritative developmental modulation to the existing GPU plasticity path for the intended regions, without recompiling topology or resetting learned weights. If such modulation is intentionally deferred, document these inherited loci as currently nonfunctional. AOA-DEV-006 permits several mechanisms; this finding concerns this implementation's expressed-but-disconnected plasticity mechanism, not a demand for every possible developmental mechanism.

**Evidence gap:** existing `critical_period_raises_plasticity_then_closes` tests biochemical status; compiler tests exercise a manually supplied open period. Neither proves a live resident's learning changes as the period opens/closes. A focused future GPU test must compare learning before/during/after with the same acquired state and inputs; no such run was authorized here.

## 3. P2 integration gap — Prediction and social neuroemitter branches have no production source

**Verified by production-source tracing; behavioral importance remains unmeasured.** The inherited biochemical graph contains PredictionResidual → adrenaline / learning signal and SocialState → oxytocin at [biochemical_graph.rs:1197–1220](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_core/src/biochemical_graph.rs#L1197-L1220). The production frame contains only RegionalArousal, MotorCommitment, and ExecutiveSustain at [gpu_live_runtime.rs:4755–4775](https://github.com/BobertNeek/A-Life/blob/d2e4a157ec8b14fc21445d55d560ab36c4551d49/crates/alife_game_app/src/gpu_live_runtime.rs#L4755-L4775). Searches find PredictionResidual constructions only in core tests, and no production SocialState construction.

**Creature-visible consequence:** those configured cognition-to-endocrine pathways never activate. This does not mean surprise cannot affect other learning mechanisms or that social contact cannot release oxytocin through physical body events. AOA-BIO-023 is partly implemented by the other three classes; this is a narrow missing connection, not wholesale failure of neuroemission.

**Smallest next step:** coordinate intended sources and timing before repair. Carry appropriately sealed GPU-authoritative residual/social evidence through the existing emission bridge if these pathways are intended live. Do not fabricate semantic social scores on the host or use future post-action error as a pre-action signal. Core tests manually supplying these emissions do not verify the bridge.

## Existing fixes and non-findings

- Material chemical reserves retain inherited baselines. Local-organ upkeep and repair debit real reserves; locomotion includes fatigue, local reserve and integrity. Sleep itself supplies no energy.
- A concern that sleep suppresses pain and therefore all repair was withdrawn: canonical founders provide basal OrganRepair; existing v11 tests cover repair with sleep-suppressed pain.
- A concern that hazard body-event replacement drops its energy cost was withdrawn: `with_body_event` preserves energy.
- Biochemical plasticity gating reaches production through `OutcomeCreditPacket.with_biochemical_receptors`; unused similarly named effect fields do not prove absent chemical learning.
- Fresh injury, newborn nociception, health distress, praise clearance, failed-action disappointment, and negative-affinity handling include prior repairs.
- Reproduction tracing confirms fresh readiness, late-world newborn age zero, distinct founders sharing a genome, parental reserve provisioning, and archive-before-GPU newborn admission. Six-founder semantics were preserved.
- Exact biological records and acquired cognitive checkpoint mechanisms are already present; no claim that save/load generally resets creatures. The taste observation is a specific omitted sensory state.
- Proximity-driven mating without an explicit mating action, including idle/sleeping pairs, is an unresolved gameplay choice rather than a verified contradiction.
- Whether individual neural/circulatory organ exhaustion should cause additional failure beyond global viability is likewise an unresolved abstraction choice.

## Validation and limits

CPU-only, `CARGO_BUILD_JOBS=1`, using the verified `/workspace/cloud-cpu-validation/env.sh` with its jobs setting overridden. No `env_graphics.sh` exists in this environment. No desktop, GPU, training, or long cohort run was performed. A prior agent test session became unavailable after the usage interruption; its result was not assumed, and relevant checks were run again explicitly.

Completed existing checks: `v11_cognitive_biology` (2), `world_organism_registry_persistence` (12), `biochemical_graph_v2` (11), `biochemistry_development` (12), `newborn_nociception` (1), and reproductive unit module `headless::task_4_3a2_tests` (8): **46 existing tests passed**. All **3 local taste reproductions passed**, including serialization to JSON, deserialization, and world restore. Thus **49 focused CPU tests passed** overall; the three audit tests assert defective baseline behavior.

Reproduction commands, from the audit worktree after sourcing the verified environment (each with `CARGO_BUILD_JOBS=1`):

```sh
cargo test -p alife_world --test biology_audit_taste -- --nocapture
cargo test -p alife_world --test v11_cognitive_biology --test world_organism_registry_persistence
cargo test -p alife_core --test biochemistry_development --test biochemical_graph_v2 --test newborn_nociception
cargo test -p alife_world --lib task_4_3a2_tests
```

Coverage limits: no GPU validation of acquired behavior or developmental plasticity, no end-to-end durable neural checkpoint run, no statistical lifespan/fertility balance, no fitness/training campaign, and no proof of learned taste avoidance. Source tracing covers the production bridges but does not certify whole-life competence. Findings 2–3 need coordinated design/repair and focused accelerator evidence. All fixes are intentionally deferred to the requested coordination stage.
