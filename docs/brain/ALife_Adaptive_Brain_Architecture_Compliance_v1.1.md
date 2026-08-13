# A-Life Brain Architecture v1.1 Compliance Matrix

**Assessment date:** 2026-08-13
**Controlling source:** `docs/brain/ALife_Adaptive_Brain_Architecture_Spec_v1.1.md`
**Controlling source SHA-256:** `60EDD478AE460C56F06F5FFA52373B069F6FC0029F40DF0659D29A99866EF302`
**Source baseline:** committed production source at `95eff8cfd7ec8277d9e82d4cec290ff3305f867a`; later commits through `db497536` change recovery documentation only
**Excluded evidence:** uncommitted `AGENTS.md` and `crates/alife_game_app/src/production_voxel_renderer.rs` user work

## Status meanings

- `IMPLEMENTED`: the current production architecture supplies the requirement's semantic behavior. This does not imply full quality or scale certification.
- `PARTIAL`: useful implementation exists, but a required semantic route, bound, or production integration is missing.
- `NOT_STARTED`: no real implementation of the named capability or interface participates in production.
- `BLOCKED`: implementation cannot proceed without an external decision or unavailable dependency.
- `DEFERRED`: the controlling specification itself classifies the capability as deferred.

Source presence, serialization, fixtures, diagnostic output, or a passing plumbing test do not by themselves qualify a requirement as implemented.

## Requirement matrix

| Requirement | Classification | Current status | Current evidence and missing work |
|---|---|---|---|
| BRN-CORE-001 Hybrid cognition | LOCKED GOAL | PARTIAL | CPU world authority and GPU neural execution coexist, but attention, prediction, concepts, gaps, body state, and cost do not yet share the v1.1 cognitive context and causal receipt. |
| BRN-CORE-002 Sparse recurrent substrate | LOCKED CAPABILITY | PARTIAL | The production backend is sparse, regionalized, multirate, plastic, and GPU accelerated. The recovered v1.1 context, predictor, branches, growth, and motor bundle are not connected to it. |
| BRN-CORE-003 Scalable class buckets | LOCKED INVARIANT | PARTIAL | N512, N1024, and N2048 classes are declared and active-loop resizing is prohibited. The live foundation compiler rejects N1024 and still carries fixed-class assumptions. |
| BRN-CORE-004 Separated weight banks | LOCKED INTERFACE | IMPLEMENTED | Foundation/genetic, lifetime, and fast state remain distinct in current phenotype, runtime, learning, sleep, and checkpoint paths. Pass 1 must preserve this boundary while extending the architecture. |
| BRN-CORE-005 Expectancy memory | LOCKED INTERFACE | IMPLEMENTED | Episodic memory exposes bounded expectancy/context rather than directly replaying an old motor command. The new attention and context frame still need to consume it causally. |
| BRN-CORE-006 Active topology | LOCKED CAPABILITY | PARTIAL | Concept and causal topology records exist, but the production runtime observes them after decision and explicitly does not upload them to neural input or arbitration. |
| BRN-CORE-007 Functional gaps | LOCKED CAPABILITY | PARTIAL | `UnresolvedGap` and curiosity voltage exist. They do not yet drive pre-decision attention, neural context, prediction, or information-seeking behavior. |
| BRN-CORE-008 Predictive learning | LOCKED CAPABILITY | PARTIAL | Sealed experience retains scalar prediction error and contradiction. No grounded action-conditioned predictor or ordinary self-supervised target/update path exists. |
| BRN-CORE-009 Factorized motor bundle | LOCKED INTERFACE | NOT_STARTED | Production arbitration chooses one semantic candidate and converts it to one world command. There is no bounded multi-channel command bundle. |
| BRN-CORE-010 Structural plasticity | LOCKED CAPABILITY | PARTIAL | Conservative pruning and sparse recompaction machinery exist. Synaptogenesis is explicitly unsupported/deferred and accepted growth cannot change later computation. |
| BRN-CORE-011 Nonlinear conjunction | LOCKED CAPABILITY | NOT_STARTED | No dendritic branch or equivalent within-neuron conjunction affects production neural computation. |
| BRN-CORE-012 Sealed ExperiencePatch | LOCKED INTERFACE | PARTIAL | The three-phase sealed causal transaction and identity checks exist. The decision and outcome still encode one semantic action rather than all motor channels plus one joint result and prediction/work receipts. |
| BRN-CORE-013 Flexible foundation | LOCKED GOAL | PARTIAL | Genome, phenotype, development, separated learning state, and reproduction exist. Ordinary newborn GPU admission can still compile from a global template instead of the child's inherited record, and live class assumptions remain. |
| BRN-CORE-014 SemanticPrior limits | LOCKED INVARIANT | PARTIAL | Optional semantic context infrastructure exists and no approved direct command/reward route was found. The bounded v1.1 cognitive-context interface and explicit baseline-disabled behavior are not installed. |
| BRN-CORE-015 ETF/D2NWG optionality | RESEARCH | IMPLEMENTED | These remain optional offline research paths and are not required by the current production loop. |
| BRN-CORE-016 Capability per compute | LOCKED GOAL | PARTIAL | The project has benchmarks and neural activity cost counters, but lacks complete hardware-independent cross-system CWU beside capability results. |
| BRN-STRUCT-001 Sparse growth discovery | LOCKED INVARIANT | NOT_STARTED | No bounded candidate discovery/store for absent edges participates in production. Pass 1 must use sparse local evidence and prohibit all-pairs enumeration. |
| BRN-CONCEPT-001 Concept induction | LOCKED CAPABILITY | PARTIAL | Bounded concept records and updates exist. Grounded predictive/compression utility does not yet drive the complete form, promote, split, merge, decay, and eviction lifecycle. |
| BRN-CONCEPT-002 Representation separation | LOCKED INVARIANT | IMPLEMENTED | Concept IDs/stores and neural topology are distinct. Pass 1 must preserve that separation while allowing bounded evidence exchange. |
| BRN-PRED-001 Information preservation | LOCKED INVARIANT | NOT_STARTED | No predictive target receipt or objective makes constant and action-insensitive predictions non-optimal. |
| BRN-ATTN-001 Two-tier attention | LOCKED CAPABILITY | NOT_STARTED | Grounded perception and candidate enumeration exist, but no learnable peripheral/focal allocation with hysteresis and bounded class/phenotype budgets controls cognition. |
| BRN-MOTOR-001 Parallel channels | LOCKED INTERFACE | NOT_STARTED | The current decoder/arbitrator selects one candidate. Compatible channel commands cannot execute in parallel or retain cross-channel coordination. |
| BRN-EXP-001 Joint outcome | LOCKED INTERFACE | NOT_STARTED | The current patch cannot retain all selected channel commands because the bundle does not exist. It therefore cannot seal the v1.1 joint outcome contract. |
| BRN-COST-001 CWU accounting | LOCKED INTERFACE | PARTIAL | Deterministic neural activity counters and direct ATP debit exist. Attention, prediction, concepts, gaps, branches, structural work, memory, and sleep are not reported in a versioned hardware-independent receipt. Metabolic conversion is not yet a configurable world/species policy. |
| BRN-PLAN-001 Bounded planning | DEFERRED CAPABILITY | DEFERRED | The specification retains planning as a future destination. Pass 1 does not fabricate it or collapse simulated outcomes into physical experience. |
| BRN-GOV-001 Normative hierarchy | LOCKED INVARIANT | PARTIAL | The controlling v1.1 source and this matrix are now installed. Implementation must still use architecture decisions for substitutions and must not convert migration limits into new rules. |

## Cross-system recovery overlay

These production breaks span several requirement rows and must be repaired in Pass 1:

| Recovery boundary | Status | Current break |
|---|---|---|
| Canonical organism state | PARTIAL | The world organism registry owns genome, phenotype, biology, and lifecycle, while resident cognition can advance a separate mutable homeostasis/development copy. |
| Birth and admission | PARTIAL | World birth creates inherited offspring, but ordinary GPU reconciliation can compile the child's brain from a global template rather than the child's record. |
| Multirate causal scheduler | PARTIAL | Neural and world schedules exist, but the unified v1.1 context and receipts do not yet enforce exactly-once advancement for each due subsystem under its own cadence. |
| Presentation lifecycle | PARTIAL | Stable-ID position projection exists in committed source. Newborn phenotype construction, full binding, retirement, live selection, and body-state animation remain incomplete. |
| Truthful controls and persistence | PARTIAL | Pause/speed scheduling and checkpoint helpers exist. Production load does not yet atomically replace world, brain, scheduler, scene, selection, and controls together. |
| Autonomous ecology and lifecycle | PARTIAL | Metabolism, development, conception, birth, ageing, death, and ecology exist. Duplicate cognitive biology and incomplete brain/body/presentation integration prevent one authoritative game loop. |

## Pass 1 exit rule

Every active locked requirement must reach `IMPLEMENTED` with a simple real mechanism that participates in production. `IMPLEMENTED` in Pass 1 means causally present and covered by its focused gate; it does not claim final quality, population scale, ablation evidence, or optimization.

`BRN-PLAN-001` remains `DEFERRED` because the controlling specification classifies it that way. Research-only `BRN-CORE-015` remains optional.

Pass 2 supplies broad regression, long-run validation, ablations, profiling, tuning, population scaling, EI ability evidence, and release certification.
