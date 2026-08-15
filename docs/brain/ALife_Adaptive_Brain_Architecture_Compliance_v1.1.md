# A-Life Brain Architecture v1.1 Compliance Matrix

**Assessment date:** 2026-08-15
**Controlling source:** `docs/brain/ALife_Adaptive_Brain_Architecture_Spec_v1.1.md`
**Controlling source SHA-256:** `60EDD478AE460C56F06F5FFA52373B069F6FC0029F40DF0659D29A99866EF302`
**Source baseline:** committed production source at `0fcc0183c53fbd912995c8fd78987f07b9777057`
**Excluded evidence:** uncommitted `AGENTS.md` user work; Pass 2 scale, ablation, long-run, EI corpus, and release evidence

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
| BRN-CORE-001 Hybrid cognition | LOCKED GOAL | IMPLEMENTED | Canonical world biology, GPU neural execution, attention, concepts/gaps, prediction, motor bundles, sleep, structural work, and cognitive-work receipts now meet in the production tick and sealed patch. |
| BRN-CORE-002 Sparse recurrent substrate | LOCKED CAPABILITY | IMPLEMENTED | The production sparse multirate GPU substrate consumes cognitive context, runs nonlinear branches, applies bounded structural edits, decodes factorized motor heads, and persists acquired state. |
| BRN-CORE-003 Scalable class buckets | LOCKED INVARIANT | IMPLEMENTED | N512, N1024, and N2048 remain bounded production classes with class-specific budgets. Current compilation uses class capacity rather than making N2048 a universal architectural rule; active-loop resizing remains prohibited. |
| BRN-CORE-004 Separated weight banks | LOCKED INTERFACE | IMPLEMENTED | Foundation/genetic, lifetime, and fast state remain distinct in current phenotype, runtime, learning, sleep, and checkpoint paths. Pass 1 must preserve this boundary while extending the architecture. |
| BRN-CORE-005 Expectancy memory | LOCKED INTERFACE | IMPLEMENTED | Bounded expectancy evidence feeds focal attention and the finalized cognitive context; old motor commands are not replayed as decisions. |
| BRN-CORE-006 Active topology | LOCKED CAPABILITY | IMPLEMENTED | Grounded concept activation changes pre-decision cognitive context, prediction, and attention through the production GPU upload path. |
| BRN-CORE-007 Functional gaps | LOCKED CAPABILITY | IMPLEMENTED | Exact action-conditioned contradictions and prediction mismatches create bounded gaps whose salience changes later attention and cognitive context. |
| BRN-CORE-008 Predictive learning | LOCKED CAPABILITY | IMPLEMENTED | Each sealed successor produces a grounded action-conditioned target; the non-collapsing predictor updates from ordinary experience and sleep. |
| BRN-CORE-009 Factorized motor bundle | LOCKED INTERFACE | IMPLEMENTED | Six fixed GPU readback slots retain independent physical-channel selections, which become one coordinated bounded command bundle. |
| BRN-CORE-010 Structural plasticity | LOCKED CAPABILITY | IMPLEMENTED | Sparse evidence discovers top-K absent-edge candidates without all-pairs enumeration; accepted growth and bounded pruning rebuild affected sparse spans and alter later computation. |
| BRN-CORE-011 Nonlinear conjunction | LOCKED CAPABILITY | IMPLEMENTED | Bounded sparse dendritic branches apply nonlinear conjunction gates before finalized activation and contribute to work accounting. |
| BRN-CORE-012 Sealed ExperiencePatch | LOCKED INTERFACE | IMPLEMENTED | The sealed patch retains the factorized command bundle, one joint physical outcome, grounded prediction target, cognitive work, and existing causal identity checks. |
| BRN-CORE-013 Flexible foundation | LOCKED GOAL | IMPLEMENTED | Birth and resident replacement compile from the child's canonical inherited genome/phenotype; learned attention, topology, predictor, branch, structural, sleep, and work state are not inherited as genes. |
| BRN-CORE-014 SemanticPrior limits | LOCKED INVARIANT | IMPLEMENTED | Bounded semantic context remains optional input evidence. It cannot directly write commands or rewards, and baseline-disabled behavior remains valid. |
| BRN-CORE-015 ETF/D2NWG optionality | RESEARCH | IMPLEMENTED | These remain optional offline research paths and are not required by the current production loop. |
| BRN-CORE-016 Capability per compute | LOCKED GOAL | IMPLEMENTED | Deterministic hardware-independent cognitive work aggregates neural, attention, prediction, topology, memory, branch, structural, and sleep work beside capability receipts. |
| BRN-STRUCT-001 Sparse growth discovery | LOCKED INVARIANT | IMPLEMENTED | Coactivation, eligibility neighborhoods, branch evidence, and bounded semantic evidence produce top-K candidates without enumerating neuron pairs. |
| BRN-CONCEPT-001 Concept induction | LOCKED CAPABILITY | IMPLEMENTED | Grounded evidence drives bounded formation, strengthening, split, merge, decay, resolution, and eviction; concept utility affects later cognition. |
| BRN-CONCEPT-002 Representation separation | LOCKED INVARIANT | IMPLEMENTED | Concept IDs/stores and neural topology are distinct. Pass 1 must preserve that separation while allowing bounded evidence exchange. |
| BRN-PRED-001 Information preservation | LOCKED INVARIANT | IMPLEMENTED | Grounded successor targets retain action and context identity; constant or action-insensitive prediction cannot minimize the focused contract. |
| BRN-ATTN-001 Two-tier attention | LOCKED CAPABILITY | IMPLEMENTED | Grounded peripheral summaries, bounded focal selection, hysteresis, and phenotype budgets determine the context uploaded before GPU dispatch. |
| BRN-MOTOR-001 Parallel channels | LOCKED INTERFACE | IMPLEMENTED | Compatible GPU-selected locomotion, manipulation, vocal, and posture commands survive readback and execute in one coordinated transaction. |
| BRN-EXP-001 Joint outcome | LOCKED INTERFACE | IMPLEMENTED | The experience patch seals all chosen channel commands and the one measured joint world/body result; it does not invent per-channel rewards. |
| BRN-COST-001 CWU accounting | LOCKED INTERFACE | IMPLEMENTED | Versioned hardware-independent work receipts cover every recovered subsystem. World/species policy optionally converts work to canonical metabolic cost and normally enables it for ecological runs. |
| BRN-PLAN-001 Bounded planning | DEFERRED CAPABILITY | DEFERRED | The specification retains planning as a future destination. Pass 1 does not fabricate it or collapse simulated outcomes into physical experience. |
| BRN-GOV-001 Normative hierarchy | LOCKED INVARIANT | IMPLEMENTED | The recovery preserves locked capability semantics while treating current Rust types, layouts, constants, processors, and fixtures as migration mechanisms. |

## Cross-system recovery overlay

These production breaks span several requirement rows and must be repaired in Pass 1:

| Recovery boundary | Status | Current break |
|---|---|---|
| Canonical organism state | IMPLEMENTED | The world organism record is the current canonical implementation for genome, phenotype, body, homeostasis, chemistry, age, lifecycle, archive identity, and sleep. GPU and UI state are projections/consumers. |
| Birth and admission | IMPLEMENTED | Ordinary offspring admission and resident replacement compile from the child's canonical inherited record. Newborn visual construction uses the child's own authoritative admission and stable identity. |
| Multirate causal scheduler | IMPLEMENTED | Every due neural, biological, ecological, developmental, lifecycle, sleep, and persistence subsystem advances once on its configured cadence inside the production causal sequence. |
| Presentation lifecycle | IMPLEMENTED | Double-buffered live frames publish stable-ID organism rows; roots spawn, move, bind, select, follow, inspect, and retire from current authority without template cloning. |
| Truthful controls and persistence | IMPLEMENTED | Pause/speed gate authoritative ticks; breeding and teaching dispatch through authority; durable load candidate-builds and replaces world, GPU runtime, schedule, scene, presentation, selection, follow, and controls together. |
| Autonomous ecology and lifecycle | IMPLEMENTED | Metabolism, consumption, growth, ageing, health, death, reproduction, ecology, cognition, body action, sleep, and presentation now share one authoritative game loop. |

## Pass 1 exit rule

Every active locked requirement now has a simple real production mechanism. The batched GPU-feature app compiler gate is green at `0fcc0183`. The final short integrated player-loop gate remains the only Pass 1 proof still being assembled; until it passes, this matrix records architecture recovery rather than alpha or behavioral certification.

`BRN-PLAN-001` remains `DEFERRED` because the controlling specification classifies it that way. Research-only `BRN-CORE-015` remains optional.

Pass 2 supplies broad regression, long-run validation, ablations, profiling, tuning, population scaling, EI ability evidence, and release certification.
