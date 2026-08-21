# A-Life v1.1 repair ledger

Status: D01-D27 source repair complete. Workspace compile green. Behavioural and performance validation remain frozen pending independent architecture review.

## Authorities

- Controlling architecture: `docs/brain/ALife_Adaptive_Brain_Architecture_Spec_v1.1.md`
- Architecture SHA-256: `60EDD478AE460C56F06F5FFA52373B069F6FC0029F40DF0659D29A99866EF302`
- Repair package SHA-256: `9B3127F6BC2F8259E08D9178FB3480859ABDFB63366F824782479E9426A8FED7`
- Integration base: `19b4c0af272f41cb640e9cfd0ee93582408c6dc6`
- Integration branch: `codex/v11-repair-only-intelligent-animal`
- Integration worktree: `D:\A life\.worktrees\v11-repair-only`
- Protected main-checkout WIP: `D:\A life\AGENTS.md`, preserved and excluded from this branch.

## Execution rules

- Maximum three implementation lanes.
- One owner at a time for shared runtime, GPU ABI, WGSL layout, and persistence files.
- One serialized Cargo and shader queue.
- No `cargo test`, EI1 run, GPU journey, corpus, ablation, benchmark, profile, soak, evolution run, or routine review worker.
- Real production mechanisms only. No stubs, policy bypasses, disconnected state, or diagnostic-receipt campaigns.
- Each bundle crosses the integration boundary after source inspection, a targeted compile or static check, and one supervisor diff check.

## Repair bundles

| Bundle | Deficiencies | Owner | Status | Commit | Main files | Semantic result or blocker |
| --- | --- | --- | --- | --- | --- | --- |
| R0 | execution mode | supervisor | complete | R12 handoff commit | this ledger, branch/worktree state | Repair-only mode active. Expensive validation paused. Ten dirty old worktrees preserved as `rescue/pre-repair-*-20260821`; clean duplicate worktrees removed. |
| R1 | D01, D02, D03, D24, D26 | Lane A | complete with R11 cadence cleanup retained | `0ec6d911` | shared causal step, game runtime, Era1 runner, session/world trial integration | Gameplay and Era1 now use one ordered production transaction and canonical registered biology. Existing gameplay sleep scheduling and batch world-tail remain authoritative cadence seams for R11 rather than alternate cognition. |
| R2 | D04, D07, D24 | Lane A | complete | `863f468c` | world sensing, attention, cognitive context | Stable-ID focal targets now trigger a second grounded world query with richer facts; canonical organism biochemistry supplies interoception and both tiers consume deterministic work budgets. |
| R3 | D05, D06, D08, D27 | Lane B | complete with R11 instruction cleanup retained | `a72014f7` | topology, cognitive context, bounded neural context ABI | Target/action-family concept, causal, contradiction, uncertainty, and gap channels now enter the bounded GPU context path; lifecycle utility is broadened and global-max copying removed. |
| R4 | D09, D10, D11, D12, D13 | Lane B | complete with R5 motor-condition seam retained | `16fbf112`, `514f95c6` | predictor, candidate prediction context, checkpoint ABI | Stable grounded state/outcome schemas, categorical motor embeddings, rank-4 interactions, candidate-specific consequences, uncertainty, and anti-collapse evidence now enter the ordinary predecision GPU context. R5 still owns the final factorized motor-condition replacement. |
| R5 | D11, D14, D15 | Lane C | complete with R6 outcome-credit seam retained | `8dfc94cc` | motor, decoder, eligibility ABI and WGSL | Ordinary production now decodes six parallel versioned channels, including orientation and species-specific v1, into one joint world transaction. Decision, eligibility, checkpoint, and replay identity bind every active channel; R6 owns the one measured joint modulator. |
| R6 | D15, D16, D17, D18 | Lane C | complete | `70954f9f` | experience, learning, shared outcome sealing, plasticity WGSL | Canonical biology before/after the joint transaction now produces measured body/homeostatic deltas, distinct reward/value/RPE and auxiliary signals, one joint modulator for all channel eligibilities, and separate fast plasticity versus slow normalization. |
| R7 | D19, D20 | Lane C | complete | `66bf689c` | structural plasticity, dendrites, GPU structural integration | Replay events now nominate bounded sparse candidates; accepted synapses and target-major branches compete under fixed budgets and are uploaded into the live GPU topology. |
| R8 | D21 | Lane C | complete | `5677fd8e` | sleep scheduler, replay snapshot, CPU/GPU staging and rollback | One immutable identity-bound replay snapshot stages memory, predictor, concepts/gaps, structural and dendritic work with GPU consolidation. Gameplay and Era1 use the same biology-scheduled path; GPU state is restored if later CPU/structural publication fails. |
| R9 | D22, D23 | serialized integration | complete | `6347fc80` | genome/phenotype, runtime/world policy, cognitive work | Bounded architecture policy is heritable and digest-bound; acquired state is excluded. Runtime capacity choices consume phenotype policy, cognitive work is complete and hardware-independent, and metabolic conversion is a configurable versioned world policy. N512 and N2048 are both supported construction choices. |
| R10 | D25 | serialized persistence owner | complete | `e29ad762` | exact checkpoint, save/load migration, ABI/schema manifest | Exact resume now binds and restores acquired v1.1 cognition, sparse structural state, work, sleep, identity, and life statistics. Save schema is v4 and exact/founder cognitive schemas are v2. Older incomplete exact state is rejected atomically; founder projection is explicit and excludes transient/world-local state. |
| R11 | D26, D27 | supervisor plus relevant owner | complete | `49c888cc`, `07e8f8e8` | shared transaction audit, current source instructions, live UI wording | R1 already exposed the smallest named shared transaction boundary used by gameplay and Era1; no second wrapper was added. Current instructions, source comments, and live topology labels now describe topology as active bounded cognition, not diagnostic-only or bias-only authority. |
| R12 | D01-D27 | supervisor | complete | R12 handoff commit | compile-only integration and handoff artifacts | One workspace compile passed. The ABI/persistence manifest, repaired stimulus-to-response trace, unresolved-source list, independent-review handoff, and full project review are current. No behavioral or performance validation started. |

## Deficiency register

| ID | Bundle | Status | Commit/files | Repair or explicit unresolved reason |
| --- | --- | --- | --- | --- |
| D01 | R1 | complete | `0ec6d911`; shared causal step and both hosts | One production GPU decision/action/sealing/plasticity/context transaction now serves gameplay and Era1. |
| D02 | R1 | complete | `0ec6d911`; Era1 runner and registered world record | Era1 no longer owns a second mutable homeostasis/development trajectory. |
| D03 | R1 | complete | `0ec6d911`; explicit mechanism mask and shared hooks | Evaluator controls now enter through one mask at the shared boundary instead of a reduced cognition implementation. |
| D04 | R2 | complete | `863f468c`; attention, grounded sensing, shared step | Production now has distinct broad peripheral and phenotype-bounded focal acquisition with stable target identity and explicit work. |
| D05 | R3 | complete | `a72014f7`; topology and cognitive context | Concepts and gaps now bind to tracked targets/action families and influence the live candidate context. |
| D06 | R3 | complete | `a72014f7`; topology lifecycle | Activation, split, merge, decay, and capacity replacement use bounded predictive, affordance, drive, memory, social, language, complexity, and contradiction utility. |
| D07 | R2 | complete | `863f468c`; cognitive context and shared step | Hunger, fatigue, pain/injury, temperature stress, sleep pressure, energy, and brain ATP come from the canonical registered organism each tick. |
| D08 | R3 | complete | `a72014f7`; memory-context ABI and WGSL | Four bounded target-context channels reach the learned GPU family decoder without host desirability or action choice. |
| D09 | R4 | complete | `16fbf112`, `514f95c6`; shared causal step | Pre/post state uses one stable 14-lane grounded schema; outcomes use a separate grounded head. |
| D10 | R4 | complete | `16fbf112`; predictor core | Primitive identity is an opaque categorical lookup with bounded learned embeddings; integer/byte/bit/Hamming features are removed. |
| D11 | R4/R5 | complete | `16fbf112`, `514f95c6`, `8dfc94cc`; predictor and shared causal step | Predictor conditions on categorical joint bundles; ordinary production no longer reconstructs a single whole-action compatibility bundle. |
| D12 | R4 | complete | `16fbf112`, `514f95c6`; predictor metrics | Production evidence now supplies action sensitivity and successor separability without source-target distance substitution. |
| D13 | R4 | complete | `514f95c6`; cognitive context and GPU memory ABI | Per-candidate predicted consequences and uncertainty are prepared before selection and consumed by the learned GPU context path. |
| D14 | R5 | complete | `8dfc94cc`; factorized decode, world motor transaction | Locomotion, orientation, manipulation, vocal, posture, and species-specific v1 compete per channel and execute as one bounded bundle with neural direction/intensity/duration/stand-off parameters. |
| D15 | R5/R6 | complete | `8dfc94cc`, `70954f9f`; eligibility and plasticity ABI | Every active channel receives the same outcome-derived joint modulator; no per-channel labels are fabricated. |
| D16 | R6 | complete | `70954f9f`; measured biology outcome | Sealed patches carry measured energy, health, injury, temperature, drive, hormone, pain, and homeostatic changes from canonical biology before/after. |
| D17 | R6 | complete | `70954f9f`; learning contract and GPU outcome ABI | Raw reward, learned expectation, true RPE, pain, injury, homeostatic improvement, frustration, novelty, sensory residual, and social outcome remain named and bounded. |
| D18 | R6 | complete | `70954f9f`; phenotype receptor plan and plasticity WGSL | Outcome-gated fast plasticity is separate from slower normalization, with bounded phenotype learning/receptor validation. |
| D19 | R7 | complete | `66bf689c`; structural plasticity and GPU runtime | Bounded event-driven reservoirs replaced absent-pair fallback discovery; growth and pruning compete with deterministic work accounting. |
| D20 | R7 | complete | `66bf689c`; dendritic allocation and GPU runtime | Replay evidence now allocates/replaces bounded target-major conjunction branches and uploads them into production state. |
| D21 | R8 | complete | `5677fd8e`; sleep transaction and host drivers | Pending/committed/interrupted state is explicit; CPU/GPU cognition commits together or restores the captured GPU checkpoint. Manual Era1 sleep was removed. |
| D22 | R9 | complete | `6347fc80`; genome, compiler inputs, phenotype record | Focal width, predictor rank, structural capacity, and sleep limits are bounded heritable policy, validated against capacity class, compiled into phenotype identity, and recombined without acquired state. Existing heritable architecture rates remain in the same policy record. |
| D23 | R9 | complete | `6347fc80`; cognitive work, runtime/world accounting, evaluator construction | Hardware-independent work counts attention, prediction, topology, six-channel motor control, structural work, and sleep. A versioned configurable policy may convert it into energy, fatigue, and heat. Production construction is no longer forced to N2048. |
| D24 | R1/R2 | complete | `0ec6d911`, `863f468c` | Baseline production uses grounded object slots, score-free peripheral/focal facts, and canonical biology; privileged sensing remains only in explicit legacy/test fixtures. |
| D25 | R10 | complete | `e29ad762`; world persistence, checkpoint codec, live runtime load/save | Exact resume requires identity-bound acquired cognition and life statistics. New state is versioned and restored; semantically incomplete old state is rejected with `ExactResumeUnavailable` rather than defaulted. Durable founder projection is a separate deliberate transformation. |
| D26 | R1/R11 | complete | `0ec6d911`, `49c888cc`; shared causal step and host call sites | `ProductionCausalStage`, hooks, transaction, and step expose the ordered semantics once. Gameplay and Era1 call the same implementation; their remaining code owns cadence, presentation, persistence, and evaluation configuration rather than cognition. |
| D27 | R3/R11 | complete | `a72014f7`, `49c888cc`, `07e8f8e8`; topology, current instructions, source comments, live UI strings | Active target-specific concept/gap topology enters bounded neural candidate context. Current instructions and player-facing topology labels no longer call it diagnostic-only or bias-only, while retaining the rule that topology supplies context rather than direct policy. |

## Dependency and conflict scan

| Producers and consumers | Shared interface or file | Finding and ruling |
| --- | --- | --- |
| R1 -> R2/R3/R4/R5/R6/R8 | shared causal cognition step | R1 fixes the semantic transaction boundary first. Later bundles extend it; they must not create alternate host loops. |
| R2 -> R3/R4 | `CognitiveContextFrame`, focal target identity | R2 owns peripheral/focal and canonical interoception. R3 and R4 consume stable target identities and add bounded channels. |
| R3 <-> R4 | candidate-specific concept/gap and predicted-consequence context | Use one bounded candidate-context ABI. Concepts provide grounded relevance, predictor provides consequences. Neither provides desirability. |
| R4 -> R5 | categorical joint motor condition | Predictor conditions on the factorized bundle. R5 must not restore integer or bit geometry. |
| R5 -> R6 | multi-channel decision evidence and eligibility | R5 records all active channel contributors. R6 applies one measured joint outcome and does not fabricate channel rewards. |
| R6 -> R8 | sealed outcome and learning transaction | Sleep consumes exact sealed patches. It must not reinterpret reward, pain, or homeostatic delta. |
| R7 <-> R8 | structural evidence and sleep commit | R7 owns sparse evidence/ranking. R8 owns the atomic multi-system commit boundary. |
| R2-R8 -> R9 | phenotype and work policy | R9 moves capacities and rates behind bounded policy after mechanism shapes settle. It must not invent fixed N2048 architecture. |
| R1-R9 -> R10 | new acquired and policy state | R10 is serialized last. It must migrate or reject old state explicitly and never fabricate exact continuity. |
| R1/R11 | `gpu_live_runtime.rs` and Era1 orchestration | Extract only semantic transaction boundaries needed to remove duplication. No cosmetic rewrite. |

Ruling: the uploaded repair policy overrides the generic subagent skill's routine per-task and final reviewer loops. Independent review is postponed until R12, and this run stops before that review. Cost if wrong: a defect may survive to the independent review, but the run avoids the known token-heavy review churn the user explicitly prohibited.

Ruling: the root `AGENTS.md` names `docs/master_spec.md` and `docs/architecture_decisions.md`, but neither file exists at this source. `docs/ARCHITECTURE.md`, `docs/REFERENCE.md`, and the controlling v1.1 spec supply the current authority. Cost if wrong: a deleted historical rule may be missed; the final review handoff will flag the stale root pointers.

## Compile queue

| Time | Command | Result | Scope |
| --- | --- | --- | --- |
| 2026-08-21 | `cargo check -p alife_core -p alife_gpu_backend -p alife_runtime -p alife_training -p alife_game_app --message-format=short` | STOP: R7 const conversion errors | First integration compile; no tests or runtime execution. |
| 2026-08-21 | same targeted check | STOP: missing R7 backend structural-event bridge | Second integration compile. |
| 2026-08-21 | same targeted check | STOP: two R1 type mismatches | Third integration compile. |
| 2026-08-21 | same targeted check | PASS | Five affected crates compile; only pre-existing warnings plus three removable R1 closure-drop warnings. |
| 2026-08-21 | `cargo check -p alife_core -p alife_world -p alife_runtime -p alife_training -p alife_game_app --message-format=short` | PASS | R2 attention/interoception integration; no tests or runtime execution. |
| 2026-08-21 | `cargo check -p alife_core -p alife_gpu_backend -p alife_runtime --message-format=short` | STOP: two R3 compile mappings corrected, then PASS | Target-context ABI and WGSL source integration; no runtime execution. |
| 2026-08-21 | five-crate targeted check | STOP: one R4 enum-reference encoding error corrected, then PASS | Predictor core and downstream API compatibility; no runtime execution. |
| 2026-08-21 | `cargo check -p alife_core -p alife_gpu_backend -p alife_runtime -p alife_training -p alife_game_app --message-format=short` | STOP: iterator result typing and one private-field accessor corrected, then PASS | R4 production predictor integration; no tests, GPU, or behavioral execution. |
| 2026-08-21 | `cargo check -p alife_core -p alife_world -p alife_gpu_backend -p alife_runtime -p alife_training -p alife_game_app --message-format=short` | STOP: six-channel checkpoint identity completed, then PASS | R5 factorized motor and eligibility integration; no tests, GPU, or behavioral execution. |
| 2026-08-21 | same six-crate targeted check | PASS | R6 measured outcome and three-factor learning integration; no tests, GPU, or behavioral execution. |
| 2026-08-21 | same six-crate targeted check | STOP: one missing root export corrected, then PASS; final Era1 atomic correction also PASS | R8 atomic sleep/consolidation integration; no tests, GPU, or behavioral execution. |
| 2026-08-21 | `cargo check -p alife_core -p alife_world -p alife_gpu_backend -p alife_runtime -p alife_training -p alife_game_app -p alife_tools --message-format=short` | STOP: three R9 type mismatches, two latent R8 host mappings, and one stale Task 3 type name corrected; then PASS | R9 evolvable policy and cognitive economics integration; no tests, GPU, or behavioral execution. |
| 2026-08-21 | `cargo check -p alife_world -p alife_runtime -p alife_game_app --message-format=short` | PASS | R10 exact persistence/schema integration; no tests, GPU, or behavioral execution. |
| 2026-08-21 | `cargo check --workspace --message-format=short` | PASS in 2m 47s | Final R12 compile-only integration. Warnings remain recorded; no tests, GPU, EI1, journeys, ablations, benchmarks, profiles, soaks, evolution, or CI ran. |

## Rulings and unresolved source defects

Record only real architectural decisions and concrete blockers. Scientific validation debt belongs in the review handoff, not this table.

- No known D01-D27 source deficiency remains unresolved.
- Ruling: old exact-cognitive schemas are rejected instead of migrated because omitted acquired state cannot be reconstructed exactly. Cost if wrong: old saves require deliberate founder import or an approved lossy migration rather than exact resume.
- Ruling: R1's existing shared transaction is the minimal D26 extraction. A second host wrapper would duplicate rather than clarify authority. Cost if wrong: the independent reviewer may require a smaller module boundary inside the shared step.
- Ruling: the absent `docs/master_spec.md` and `docs/architecture_decisions.md` referenced by root instructions were not recreated from history. The controlling v1.1 spec, `docs/ARCHITECTURE.md`, and `docs/REFERENCE.md` remain the source set. Cost if wrong: a deleted historical requirement may need restoration after review.
- Final compile warnings in older curated-founder/game presentation paths are listed in `ALife_v1.1_Unresolved_Source_Defects.md`; none prevented compilation or was promoted to a behavioral claim.
