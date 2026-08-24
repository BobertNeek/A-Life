# A-Life Adaptive Brain Architecture Specification

> **SUPERSEDED / HISTORICAL:** A-Life v2.0 replaced this specification as
> normative authority. Consult it only for design lineage. It cannot amend,
> narrow, or override
> `../architecture/ALife_Complete_Organism_and_Intelligence_Architecture_v2.0_CONTROLLING.md`.

## Version 1.1 - Superseded historical architecture

**A-Life Project**  
**Effective date:** 2026-08-12  
**Document ID:** ALIFE-BRAIN-ARCH-001  
**Status:** SUPERSEDED / HISTORICAL

**SMART / COMPUTE-EFFICIENT / EVOLVABLE / BIOLOGY-INSPIRED**

Former normative production architecture for scalable, adaptive creature cognition.

This document was the controlling brain-architecture specification before v2.0 adoption. Its implementation findings and design choices are retained as history only.

---

## Document Control

| Field | Value |
|---|---|
| Document ID | ALIFE-BRAIN-ARCH-001 |
| Version | 1.1 |
| Status | SUPERSEDED / HISTORICAL |
| Effective date | 2026-08-12 |
| Canonical format | Repository Markdown |
| Publication formats | Markdown, DOCX, PDF |
| Recommended repository path | `docs/brain/ALife_Adaptive_Brain_Architecture_Spec_v1.1.md` |
| Implementation baseline | `BobertNeek/A-Life` main snapshot reviewed in this project, commit `32e04d47c22cf8d0b986abfc2804b8678325e110` |
| Supersedes | ALIFE-BRAIN-ARCH-001 v1.0; ALife Brain Runtime Specification v0.1; Flat Sparse Tensor ALife Engine v1.1; and any later implementation note that conflicts with this specification |
| Retained contracts | Engine-independent IDs, three-phase ExperiencePatch, MemoryExpectancy, concept topology, separated inherited/lifetime/fast weights, sparse tiled execution, multirate lobes, bounded logging |
| Change rule | A normative change requires a user-approved Architecture Decision Record and a versioned revision. Reference mechanisms may be replaced through the evidence process in Section 24. |

### Former controlling principle

The goal is not CPU authority, GPU authority, architectural purity, or biological literalism. The goal is the smartest, fastest, most compute-efficient creature brain that remains evolvable, developmentally adaptable, physically grounded, causally correct, bounded, and inspectable.

### v1.1 revision summary

Version 1.1 preserves the v1.0 architecture and closes implementation escape hatches in seven areas:

1. capability per compute is placed ahead of biological resemblance, while evolvability remains a primary optimization objective;
2. synaptogenesis candidate discovery is explicitly sparse and non-$N^2$, without locking reservoir granularity;
3. concept induction, promotion, splitting, merging, decay, and eviction are driven by broad predictive and cognitive compression utility;
4. predictive objectives must preserve information and make constant or action-insensitive representations non-optimal, without mandating one self-supervised learning family;
5. attention becomes a two-tier, learnable compute-allocation architecture;
6. motor output becomes a factorized parallel command bundle while outcomes remain jointly grounded rather than artificially decomposed into labelled per-channel rewards;
7. cognitive work is measured in hardware-independent work units, while conversion into biological energy remains a configurable world/species policy.

Version 1.1 also explicitly separates concept induction from neural structural plasticity. The two processes may exchange bounded evidence, but neither is a direct serialization of the other.

---

## Contents

1. Purpose and status  
2. Controlling design decisions  
3. Goals, non-goals, and success criteria  
4. Whole-system architecture  
5. Brain classes, phenotype compilation, and scalability  
6. Genome, species foundation, and development  
7. Neural substrate and lobe organization  
8. Sparse connectivity and execution representation  
9. Dendritic subunits  
10. Homeostasis, drives, hormones, and neural metabolism  
11. Perception, grounding, and attention  
12. Episodic memory and MemoryExpectancy  
13. Concept-and-causal topology and concept induction  
14. Predictive learning, curiosity, and deferred planning  
15. Action selection and factorized motor control  
16. ExperiencePatch causal transaction  
17. Waking plasticity  
18. Sleep consolidation and structural plasticity  
19. Reproduction, inheritance, and evolution  
20. Runtime CPU/GPU placement and data movement  
21. Performance budgets, cognitive economics, and graceful degradation  
22. Observability, testing, and capability evaluation  
23. Migration from the current implementation  
24. Change control and anti-drift rules  
25. Rejected and superseded alternatives  
26. Risks and mitigations  
27. Implementation sequence and completion gates  
Appendix A. Normative data contracts  
Appendix B. Reference defaults and tunable ranges  
Appendix C. Requirement index  
Appendix D. Source and decision provenance  
Appendix E. Glossary  
Appendix F. v1.0 to v1.1 change record

# 1. Purpose and Status

This document defines the target production architecture for the A-Life creature brain. It is a normative engineering specification, not a brainstorming note and not a description limited to what is currently implemented.

The current codebase is the migration baseline. Where the code conflicts with this specification, the code is incomplete or non-compliant. An implementation limitation must be recorded as `NOT_STARTED`, `PARTIAL`, `BLOCKED`, or `DEFERRED`; it must not be converted into a new architectural rule merely because it is easier to implement.

The architecture combines:

- a sparse multirate recurrent neural substrate;
- fast local synaptic plasticity;
- stable lifetime consolidation;
- bounded episodic memory;
- active concept and causal topology;
- prediction and self-supervised learning;
- contradiction-driven curiosity;
- selective attention as compute allocation;
- factorized motor control;
- body drives, hormones, and neural metabolic regulation;
- sleep replay, structural growth, and pruning;
- development and evolution of architecture and learning rules.

The system is successful only when these mechanisms participate in the live causal loop and produce measurable behaviour at acceptable compute cost.

## 1.1 Normative language

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

Every important statement belongs to one of the following classes:

| Classification | Meaning |
|---|---|
| LOCKED GOAL | Optimization objective that guides trade-offs. |
| LOCKED CAPABILITY | Observable behaviour or adaptive function that must exist. |
| LOCKED INVARIANT | Rule implementations may never violate. |
| LOCKED INTERFACE | Stable semantic boundary between subsystems. Storage and placement may change while semantics remain. |
| REFERENCE MECHANISM | Favoured current implementation approach, replaceable with measured evidence. |
| TUNABLE DEFAULT | Numeric starting point or policy adjustable within bounds. |
| DEFERRED CAPABILITY | Required architectural destination that is not a gate for the first migration milestone. |
| RESEARCH | Optional experiment that cannot become production-critical without an approved revision. |

A coding agent must not interpret a reference formula as an immutable law or a locked capability as optional merely because the reference implementation is difficult.

## 1.2 Optimization priority order

When legitimate objectives conflict, use this order:

1. behavioural intelligence and lifetime learning capability;
2. capability per compute, memory, latency, and energy;
3. evolvability and developmental adaptability;
4. biologically inspired mechanisms that improve capability, efficiency, robustness, or evolvability;
5. inspectability and reproducibility;
6. implementation convenience.

Biology is a high-value design source, not a veto against more effective mechanisms. A non-biological mechanism may be used when it produces materially better capability, efficiency, robustness, or evolvability and preserves the locked invariants.

## 1.3 Hard invariants outside the ranking

The following are not tradeable priorities. They are LOCKED INVARIANTS:

- **Physical grounding:** the simulated world and body determine physical truth and measured outcomes.
- **Causal correctness:** learning uses the exact pre-state, decision, execution, and successor state in the correct order.
- **Boundedness:** active-loop memory, candidate sets, topology, neural structure, transfer volume, and compute have explicit caps.
- **No hidden host policy:** no host heuristic, teacher, semantic model, or diagnostic subsystem may secretly select actions, targets, rewards, or concepts outside declared interfaces.
- **No silent inheritance of lifetime state:** episodic memory, concept records, $W_{lifetime}$, $H_{fast}$, eligibility, and working state are not ordinarily inherited.
- **No dense active-loop fallback:** sparse execution or sparse discovery may not silently degrade into dense $N^2$ work.
- **No active-loop class resizing:** brain buffers remain within their compiled class during ordinary ticks.
- **Versioned semantics:** every behaviour-shaping subsystem boundary is explicit, versioned, bounded, and observable.

## 1.4 Replacement of reference mechanisms

A reference mechanism may be replaced without changing a locked capability when the replacement:

1. preserves all affected locked invariants and interfaces;
2. passes the same behavioural capability tests;
3. passes causal correctness tests;
4. demonstrates equal or better capability per compute or a justified trade-off;
5. records the change and evidence in an implementation ADR or mechanism-substitution record;
6. does not silently reclassify deferred or missing work as complete.

# 2. Historical design decisions

The following decisions define the architecture.

| ID | Classification | Decision |
|---|---|---|
| BRN-CORE-001 | LOCKED GOAL | Cognition is a hybrid biological control system. CPU/GPU placement follows computational shape and measured efficiency, not processor ideology. |
| BRN-CORE-002 | LOCKED CAPABILITY | The recurrent neural substrate remains sparse, regionalized, multirate, plastic, and GPU-accelerated in the production backend. |
| BRN-CORE-003 | LOCKED INVARIANT | Brain size is class-bucketed and bounded. N2048 is the reference class, not a universal fixed size. Active-loop buffer resizing is prohibited. |
| BRN-CORE-004 | LOCKED INTERFACE | Effective synaptic weight separates foundation/genetic prior, lifetime consolidation, and fast plastic state. |
| BRN-CORE-005 | LOCKED INTERFACE | Episodic recall returns expectancy and context, never direct replay of a historical motor command. |
| BRN-CORE-006 | LOCKED CAPABILITY | Concept-and-causal topology actively supplies bounded context to neural cognition. Diagnostic-only topology is superseded. |
| BRN-CORE-007 | LOCKED CAPABILITY | Contradictions and prediction failures create or strengthen UnresolvedGap records that drive targeted attention and information-seeking. |
| BRN-CORE-008 | LOCKED CAPABILITY | Predictive and self-supervised learning occur during ordinary experience, not only after reward or punishment. |
| BRN-CORE-009 | LOCKED INTERFACE | Motor control uses a bounded factorized command bundle containing learned primitives, targets, and parameters. |
| BRN-CORE-010 | LOCKED CAPABILITY | Structural plasticity includes conservative pruning and useful sleep/development-time synaptogenesis under fixed budgets. |
| BRN-CORE-011 | LOCKED CAPABILITY | Selected neural regions support cheap nonlinear within-neuron conjunction computation. Exact dendritic layout is a reference mechanism. |
| BRN-CORE-012 | LOCKED INTERFACE | ExperiencePatch remains the sealed causal transaction joining pre-action state, decision, and measured joint outcome. |
| BRN-CORE-013 | LOCKED GOAL | Species foundations are inherited priors, not evolutionary prisons. Architecture and learning rules remain heritably variable within class bounds. |
| BRN-CORE-014 | LOCKED INVARIANT | An optional SemanticPrior may supply bounded semantic context but may not directly emit commands, author reward, or overwrite grounded memory. |
| BRN-CORE-015 | RESEARCH | Neural Collapse/ETF and diffusion-generated weights remain optional offline research tools, not required runtime cognition. |
| BRN-CORE-016 | LOCKED GOAL | Capability per compute is evaluated before biological resemblance; evolvability remains a primary objective. |
| BRN-STRUCT-001 | LOCKED INVARIANT | Synaptogenesis candidate discovery and storage remain sparse. Enumerating all absent neuron pairs is prohibited. |
| BRN-CONCEPT-001 | LOCKED CAPABILITY | Concept formation, promotion, split, merge, decay, and eviction are driven by grounded predictive and cognitive compression utility. |
| BRN-CONCEPT-002 | LOCKED INVARIANT | Concept topology and neural structural plasticity remain distinct representations. They may exchange bounded evidence but are not direct copies of one another. |
| BRN-PRED-001 | LOCKED INVARIANT | The predictive objective contains an information-preserving constraint or grounded target construction that makes constant and action-insensitive representations non-optimal. |
| BRN-ATTN-001 | LOCKED CAPABILITY | Attention is a learnable two-tier sensory and compute-allocation mechanism with class-, phenotype-, and budget-dependent focal capacity. |
| BRN-MOTOR-001 | LOCKED INTERFACE | Compatible motor channels execute in parallel, with competition primarily within channels and learned coordination across channels. |
| BRN-EXP-001 | LOCKED INTERFACE | ExperiencePatch preserves all selected channel commands and the joint physical outcome. Per-channel credit is optional and only used where causally supportable. |
| BRN-COST-001 | LOCKED INTERFACE | The runtime reports hardware-independent cognitive work units. Conversion into organism energy expenditure is configurable world/species policy. |
| BRN-PLAN-001 | DEFERRED CAPABILITY | Bounded counterfactual rollout over the learned predictor is retained as the path to primitive planning; simulated outcomes remain distinct from physical experience. |
| BRN-GOV-001 | LOCKED INVARIANT | Locked goals/capabilities/invariants/interfaces are distinguished from reference mechanisms and tunable defaults. |

# 3. Goals, Non-Goals, and Success Criteria

## 3.1 Primary goals

The brain MUST support:

- rapid adaptation to novel situations during one lifetime;
- persistent learning without immediate catastrophic forgetting;
- causal learning from action and measured outcome;
- self-supervised learning from observation and change;
- targeted curiosity driven by unresolved prediction failures;
- generalization across related objects, places, agents, and situations;
- remembered expectancy without historical action replay;
- competition and coordination among drives without scripted host utility scoring;
- parallel combinations of motor primitives;
- biological development, sleep, pruning, growth, and neuromodulation;
- heritable variation in architecture and learning rules;
- scalable populations rather than a single laboratory agent;
- measurable emergent behaviour rather than hidden host-authored intelligence;
- useful intelligence that pays for its computational and metabolic expense under evolutionary policy.

## 3.2 Non-goals

The production architecture does not attempt to:

- simulate ion channels, detailed morphology, or molecular neuroscience;
- run a transformer or language model inside every creature;
- use a dense all-to-all neural network;
- reproduce the exact human brain;
- guarantee human-level reasoning;
- require every mechanism to have a literal biological analogue;
- permit an external teacher, SLM, host heuristic, or concept graph to bypass the learned decision path;
- use episodic memory as a macro recorder;
- allow unbounded graph, memory, candidate, or synapse growth;
- scan every absent neural connection during structural learning;
- resize brain-class tensors during ordinary active ticks;
- use offline Python as a gameplay dependency;
- require clean labelled reward attribution for every motor channel;
- equate every concept relation with a neural synapse or vice versa.

## 3.3 Operational success criteria

A feature is not accepted merely because its types, receipts, unit tests, or diagrams exist. It must participate in the live causal loop and improve a defined capability or efficiency objective.

Minimum capability targets include:

- one-shot or few-shot avoidance after a harmful novel interaction;
- reversal learning when a formerly beneficial stimulus becomes harmful;
- recovery from misleading perceptual correlations;
- formation of a useful concept from repeated grounded experience;
- splitting an over-broad concept after systematic contradiction;
- merging redundant concepts without losing consequential distinctions;
- curiosity directed toward a specific unresolved relation rather than random wandering;
- transfer of a relation to a similar but unseen object;
- delayed use of episodic memory after intervening behaviour;
- improvement from passive observation without direct reward;
- non-collapsed action-conditioned future prediction;
- simultaneous compatible action such as retreating while tracking and vocalizing;
- useful experience-dependent neural structural differences under different histories;
- retention of earlier survival skills after later learning;
- selective attention that improves capability or lowers compute relative to uniform high-resolution processing;
- bounded cognitive work that can be exposed to evolutionary cost.

The established reference performance objective remains:

- 500 N2048 agents;
- online plasticity;
- 60 simulation ticks per second;
- no blocking bulk neural readback;
- 2-4 GB active VRAM on a named reference hardware profile.

This target is a benchmark objective, not permission to delete locked capabilities. If it is missed, the system first uses multirate scheduling, class batching, sparse culling, selective attention, bounded retrieval, dormant-agent policies, and population/class limits.

## 3.4 Capability-per-compute reporting

Every material cognitive mechanism SHOULD report:

- capability score on its target benchmark;
- change relative to the relevant ablation;
- neural time, CPU time, memory, transfer volume, and cognitive work units;
- effect on learning speed, robustness, and generalization;
- effect on evolvability where measurable.

A reference efficiency measure is:

$$
\eta_{cap} = \frac{\Delta \text{capability}}{\Delta \text{cognitive work units}}
$$

This scalar is diagnostic, not the sole optimization objective. A mechanism may be retained for robustness, evolvability, or qualitative capability even when a single benchmark understates its value.

# 4. Whole-System Architecture

The creature is not a neural network with status variables attached. It is a coordinated adaptive organism containing neural tissue, body state, endocrine state, attention, episodic memory, conceptual memory, predictive models, motor systems, sleep consolidation, development, and evolution.

![Whole-system adaptive cognition loop](diagrams/whole_system.png)

## 4.1 Normal cognitive loop

1. The world and body produce sensory and interoceptive state.
2. A cheap peripheral perception pass summarizes a bounded broad field.
3. The attention system selects a small number of focal targets according to neural state, drives, memory, concepts, gaps, novelty, and current compute budget.
4. Focal perception extracts higher-resolution target features.
5. Episodic memory retrieves related experience as MemoryExpectancy.
6. Concept topology retrieves relevant concepts, causal relations, uncertainty, and unresolved gaps.
7. The optional SemanticPrior contributes bounded semantic context.
8. These streams form a fixed-size CognitiveContextFrame.
9. The sparse recurrent brain integrates perception, body state, memory, concepts, prediction, attention, and recurrent state.
10. Motor circuitry produces a bounded factorized MotorCommandBundle.
11. The world executes the joint command and determines what physically happens.
12. Pre-state, decision, and joint outcome are sealed into an ExperiencePatch.
13. Waking plasticity, predictive learning, memory, concepts, homeostasis, logging, and sleep queues consume that same causal record.
14. During sleep or development, replay, consolidation, concept revision, synaptogenesis, pruning, and sparse recompaction occur within budgets.
15. Across generations, evolution changes foundations, architecture, learning rules, attention budgets, and cognitive cost-benefit trade-offs.

## 4.2 Hybrid cognition by computational shape

Irregular graph traversal, bounded top-k retrieval, persistent object identity, concept induction, causal-edge maintenance, and structural proposal ranking are usually CPU-friendly. Sparse recurrent projection, dendritic branch evaluation, local synaptic learning, predictor heads, and batched motor decoding are usually GPU-friendly.

Placement may change after profiling. The locked rule is that useful cognition may influence behaviour through explicit bounded interfaces regardless of processor placement.

All behaviour-shaping signals MUST be:

- explicit in a versioned contract;
- bounded in size and range;
- observable in diagnostics;
- causally attributable to perception, memory, concepts, physiology, prediction, learning, or genome;
- incapable of bypassing the creature's declared decision path.

## 4.3 Engine and backend boundaries

Recommended crate boundary:

```text
alife_core
  stable IDs and math types
  body, drives, hormones, attention contracts
  brain phenotype, memory, concepts, prediction
  motor bundle, ExperiencePatch, validation
  cognitive work-unit contracts

alife_world_adapter
  Bevy/Avian entity mapping
  peripheral/focal sensing and grounding
  physical execution and joint outcome measurement

alife_gpu_backend
  sparse recurrent and dendritic passes
  predictor heads and local plasticity
  motor-channel decoders
  sleep replay numerical work

alife_cognitive_services
  episodic retrieval
  concept induction/topology
  structural candidate indexing and ranking

alife_offline_tools
  curriculum training and evolution analysis
  behavioural clustering and benchmark receipts
  optional ETF/D2NWG research
```

A CPU reference implementation MAY exist as a deterministic oracle for tests and small runs. It is not required to be the production path.

# 5. Brain Classes, Phenotype Compilation, and Scalability

## 5.1 Class-bucketed brains

Brains MUST compile into fixed capacity classes. A class determines buffer shapes, maximum neurons, maximum synapse slots, context width, attention limits, and compatible foundation assets.

| Class | Intended use | Reference neurons | Reference recurrent synapses | Hard synapse cap | Active episodes | Concept cells | Focal targets $K_{attn}$ |
|---|---|---:|---:|---:|---:|---:|---:|
| N512 | simple fauna, swarm organisms | 512 | 4,096-8,192 | 12,288 | 64 | 128 | 1 |
| N1024 | ordinary creatures | 1,024 | 10,240-16,384 | 24,576 | 128 | 256 | 2 |
| N2048 | reference intelligent creature | 2,048 | 24,576 | 65,536 | 256 | 512 | 4 |
| N4096 | rare advanced or research creature | 4,096 | 65,536 | 163,840 | 512 | 1,024 | 8 |

These are TUNABLE DEFAULTS. The locked rule is bounded class-bucketed scalability, not any exact count. $K_{attn}$ is further modified by phenotype, current state, and runtime budget within class limits.

N4096 is not required to meet the 500-agent reference benchmark. Population scheduling MAY mix classes.

## 5.2 No active-loop resizing

A creature's brain class is fixed when its phenotype is compiled. Ordinary ticks MUST NOT reallocate the creature into another class.

Development MAY:

- unmask reserved neurons;
- modify thresholds, gains, plasticity, and cadence;
- activate or silence preallocated routes;
- change attention policy and focal allocation within the class cap;
- grow or prune synapses within the class cap;
- allocate or remove dendritic branches within reserved metadata capacity.

Development may not resize active class buffers. A class change requires reproduction, a new phenotype, or a separately staged metamorphosis transaction outside the ordinary tick loop.

## 5.3 Phenotype compiler

The phenotype compiler consumes:

- brain class;
- species foundation identity and version;
- genome;
- developmental stage;
- mutation/crossover results;
- optional founder asset identity;
- species/world cognitive-cost policy identity.

It produces:

- lobe allocation and neuron ranges;
- route matrix and sparse structural masks;
- initial synapse records and weight banks;
- dendritic branch metadata;
- neuron activation, leak, threshold, and homeostatic parameters;
- plasticity masks and learning-rate parameters;
- neuromodulator receptor gains;
- motor channels and primitive support;
- sensory and attention channels;
- memory, concept, and context widths;
- structural candidate-store budgets;
- predictor target construction policy;
- developmental schedules;
- compute-budget metadata and work-unit coefficients.

Compilation MUST be deterministic for a fixed seed, genome, foundation, schema version, and policy identity.

## 5.4 Population heterogeneity

The runtime SHOULD support mixed brain classes and phenotypes in one world. Dispatches SHOULD be grouped by class and compatible layouts.

The scheduler MAY reduce cadence for distant, sleeping, or low-salience organisms, but it MUST preserve body safety, homeostasis, minimal sensorimotor responsiveness, and causal patch correctness.

# 6. Genome, Species Foundation, and Development

## 6.1 Inherited starting brain

A newborn begins from:

$$
W_{genetic\_fixed} = W_{foundation} + W_{genetic\_delta}
$$

where:

- $W_{foundation}$ is a versioned species-level prior produced by evolution, curriculum training, or curated promotion;
- $W_{genetic\_delta}$ is individual heritable variation generated by the genome.

A species may use a trained, evolved, hand-seeded, minimal, or near-blank foundation.

## 6.2 Foundation as prior, not prison

A foundation MAY stabilize basic sensorimotor coordination, language grounding, or survival priors. It MUST NOT freeze architecture against evolution.

Within class bounds, heritable variation may alter:

- lobe size allocation;
- route presence and density;
- initial weight deltas;
- plasticity gates and learning rates;
- dendritic branch number, allocation, and thresholds;
- neuronal leak and activation parameters;
- target firing rates and metabolic gain;
- sensory allocation and peripheral/focal resolution;
- attention capacity, persistence, switching cost, and salience receptor gains;
- motor channel and primitive allocation;
- memory and concept capacity emphasis;
- neuromodulator sensitivity;
- predictor heads and target-construction policy within approved families;
- sleep thresholds and consolidation rates;
- synaptogenesis candidate budgets and pruning thresholds;
- update cadence;
- cognitive work-unit cost-benefit strategy.

A promoted foundation MUST carry a stable content hash, schema version, compatibility class, provenance record, and training/evolution receipt.

## 6.3 Developmental schedule

Development MAY control:

- staged lobe and route activation;
- critical periods for sensory, social, language, and motor learning;
- temporary excess structural capacity followed by pruning;
- changing attention bandwidth;
- changing neuromodulator receptor expression;
- changing structural-growth thresholds;
- changing sleep need and replay mix;
- maturation of motor channels and predictor horizons.

Developmental changes are genome-controlled and bounded by compiled class capacity.

## 6.4 Evolvability protections

A foundation or optimization pipeline MUST NOT be promoted solely because it performs well on one lifetime benchmark. Promotion evidence includes:

- behavioural capability;
- capability per compute;
- generalization;
- robustness to perturbation;
- ability to continue lifetime learning;
- mutation tolerance;
- retention of heritable variation;
- absence of hidden host dependence;
- reproducible provenance.

# 7. Neural Substrate and Lobe Organization

## 7.1 Rate-neuron state

A reference neuron contains:

- activation;
- previous activation or recurrent state;
- bias and threshold;
- leak or persistence;
- activation function;
- activity exponential moving average;
- metabolic load;
- optional dendritic branch range;
- lobe/route identity;
- developmental and plasticity flags.

A reference somatic update is:

$$
\tilde{y}_j(t) = \phi_j\left(\sum_i W^{effective}_{ij}x_i(t) + I_j(t) + D_j(t) + b_j - g_m m_j(t)\right)
$$

$$
y_j(t) = \lambda_j y_j(t-1) + (1-\lambda_j)\tilde{y}_j(t)
$$

where $D_j$ is optional dendritic input and $m_j$ is metabolic/homeostatic feedback.

The equations are REFERENCE MECHANISMS. The locked capabilities are sparse recurrent integration, stable bounded dynamics, multirate updates, and support for nonlinear conjunction where enabled.

## 7.2 Required functional roles

Every behaviour-capable phenotype provides these roles, although small classes may combine them:

1. **Sensory Grounding** - peripheral and focal external state.
2. **Attention and Salience** - focal-target selection, persistence, and compute allocation.
3. **Interoception and Metabolic Drive** - needs, pain, fatigue, arousal, and body safety.
4. **Core Association and Prediction** - multimodal integration, successor prediction, and causal context.
5. **Working Memory** - persistent task/context state.
6. **Episodic Context** - neural reception and transformation of MemoryExpectancy.
7. **Concept/Semantic Context** - neural reception and transformation of concepts, gaps, and language signals.
8. **Motor Planning and Coordination** - channel-local selection, parameter generation, and cross-channel coordination.
9. **Homeostatic Regulation** - threshold/gain control and protective regulation.

Optional roles include auditory speech, glyph vision/writing, navigation, social/affective reasoning, planning/dream, self-model/uncertainty, and species-specific modalities.

## 7.3 Reference N2048 allocation ranges

| Role | Suggested range |
|---|---:|
| Sensory grounding | 8-18% |
| Attention and salience | 2-6% |
| Interoception/metabolic | 4-10% |
| Core association and prediction | 22-40% |
| Working memory | 5-12% |
| Episodic context | 5-12% |
| Concept/semantic context | 7-15% |
| Motor planning/coordination | 7-15% |
| Homeostatic regulation | 1-4% |
| Optional modalities | remaining capacity |

These are TUNABLE DEFAULTS. Phenotypes may depart from them while preserving required roles.

## 7.4 Multirate processing

Routes and lobes support different cadences:

- **FAST:** every simulation tick - pain, balance, immediate sensing, orientation, motor, homeostasis;
- **MEDIUM:** every 2 ticks - association, working memory, social context, focal target refinement;
- **SLOW:** every 4-8 ticks - concept integration, long-horizon prediction, language maintenance;
- **SLEEP_ONLY:** deep replay, compaction, structural ranking and edits.

Cadence is phenotype-controlled within safety limits. The scheduler may throttle lower-priority routes under load.

# 8. Sparse Connectivity and Execution Representation

## 8.1 Sparse macro-connectome

Lobes connect through a route matrix. A route defines:

- source and destination lobe;
- structural permission;
- density prior;
- cadence;
- plasticity policy;
- dendritic targeting policy;
- developmental activation window;
- minimum and maximum synapse budget;
- whether structural growth is allowed;
- candidate-discovery policy and budget;
- vital/protected status.

The runtime processes existing neural tissue rather than a dense theoretical matrix.

## 8.2 Hierarchical tiled representation

The production GPU backend SHOULD retain or improve upon:

- 16 x 16 microtiles;
- 8 x 8 microtiles per supertile;
- 128 x 128 macro-regions;
- supertile masks for early culling;
- microtile metadata selecting dense or sparse storage form;
- class-batched buffers with stable offsets.

Equivalent storage may replace this when it demonstrates equal or better capability, memory use, transfer cost, and performance without weakening structural plasticity.

## 8.3 Synapse record

A logical synapse contains:

```text
pre_neuron
post_neuron or dendritic_branch
route_id
W_foundation
W_genetic_delta
W_lifetime_consolidated
alpha
H_fast
eligibility
stability / age trace
structural utility trace
flags
```

Physical storage may use structure-of-arrays, quantized banks, compressed indices, or other layouts.

## 8.4 Effective weight

$$
W_{effective} = W_{foundation} + W_{genetic\_delta} + W_{lifetime} + \alpha H_{fast}
$$

or:

$$
W_{effective} = W_{genetic\_fixed} + W_{lifetime} + \alpha H_{fast}
$$

Eligibility is not part of the forward weight. It records causal participation and gates later learning.

## 8.5 Precision policy

Precision is backend policy. FP32, FP16, BF16, fixed point, mixed precision, or low-bit banks may be used when they satisfy:

- bounded numerical error against a deterministic reference;
- no systematic loss of small plastic updates;
- sufficiently wide accumulation;
- unbiased or validated handling when quantization would erase updates;
- deterministic seeded test mode;
- no undefined overflow or race-order behaviour.

## 8.6 Sparse synaptogenesis candidate discovery

The system MUST NOT enumerate every absent pair $(i,j)$.

Let $C_{growth}$ be the number of retained growth candidates. The locked invariant is:

$$
C_{growth} \ll N^2
$$

Candidate storage and discovery SHOULD scale approximately as $O(NK)$ or better, where $K$ is a small bounded phenotype/class policy. Equivalent tile-, route-, branch-, or sketch-level representations are permitted.

Permitted candidate-discovery mechanisms include:

- per-neuron or per-branch bounded reservoirs;
- route-level or microtile-pair correlation sketches;
- count-min-style or heavy-hitter sketches;
- reservoir sampling;
- locality-sensitive hashing;
- nearest-neighbour search over active latent summaries;
- correlation of prediction residuals;
- shared concept/gap relevance;
- neighboring active microtiles;
- bounded stochastic partner exploration.

No reservoir granularity is locked. The implementation must provide a complexity receipt proving bounded storage and absence of dense pair enumeration.

# 9. Dendritic Subunits

## 9.1 Purpose

Dendritic subunits provide nonlinear conjunction computation within selected neurons. They are not full morphological neuron simulation.

A selected neuron may receive branch outputs:

$$
r_{jk}(t) = \sum_{i \in B_{jk}} W^{effective}_{ijk}a_i(t)
$$

$$
d_{jk}(t) = \rho_{jk}d_{jk}(t-1) + (1-\rho_{jk})\psi_{jk}(r_{jk}(t)-\theta_{jk})
$$

The soma combines branch outputs with ordinary recurrent input. This can represent contextual conjunctions such as hunger + odour + reachable target more efficiently than a single linear sum.

## 9.2 Locked capability and replaceable mechanism

The locked capability is **cheap nonlinear within-neuron conjunction computation in selected regions**.

The following are REFERENCE MECHANISMS or TUNABLE DEFAULTS:

- 0-4 branches per selected neuron;
- two branches in a reference N2048 phenotype;
- 4-32 inputs per branch;
- branch threshold, decay, activation, and output gain;
- whether multiplicative gates, threshold branches, learned branch allocation, or another bounded equivalent is used.

A phenotype may disable dendritic subunits when ablation shows no benefit.

## 9.3 Storage and execution

Branches MUST use flat bounded metadata/state, not heap objects per neuron. Branch evaluation SHOULD be fused with or immediately precede somatic finalization. Only active branch ranges are dispatched.

## 9.4 Evaluation gate

A dendritic implementation is accepted only when it improves at least one contextual conjunction, working-memory gating, concept disambiguation, or motor coordination task enough to justify its measured compute and memory cost.

# 10. Homeostasis, Drives, Hormones, and Neural Metabolism

## 10.1 Body state

The organism maintains grounded state such as:

- energy and hydration;
- health, injury, pain, and temperature stress;
- fatigue and sleep pressure;
- hunger, fear, curiosity, affiliation, reproduction, and other species drives;
- developmental and reproductive state.

The exact drive vocabulary is species policy.

## 10.2 Neuromodulation

Hormones and neuromodulators may change:

- neuron thresholds and gains;
- learning rates and eligibility decay;
- salience and attention weights;
- route cadence;
- memory retrieval emphasis;
- curiosity and risk tolerance;
- sleep onset and consolidation;
- developmental activation;
- structural-growth thresholds.

A single global reward scalar is insufficient. The implementation SHOULD support at least appetitive, aversive, novelty/surprise, stress/arousal, social, and predictive-error channels.

## 10.3 Neuron-level metabolic regulation

Neuron activity may accumulate metabolic load and receive negative feedback. This supports stable firing and exposes an internal cost signal.

The neuron-level mechanism is distinct from the hardware-independent cognitive work accounting in Section 21. Both may influence evolution, but they measure different things:

- neuron metabolic state is part of creature dynamics;
- cognitive work units are a reproducible accounting interface.

## 10.4 Biological energy policy

The world/species MAY convert cognitive work units into energy expenditure. This conversion is configurable and MUST NOT depend directly on wall-clock timing or the user's GPU model.

# 11. Perception, Grounding, and Attention

## 11.1 Sensory frame

The world adapter produces a bounded SensoryFrame containing continuous signals and a bounded set of entity or cluster summaries.

Possible channels include:

- egocentric direction and approximate distance;
- object/category features;
- movement and contact;
- sound and token channels;
- light, temperature, terrain, and hazard;
- self-motion and proprioception;
- social identity and posture cues;
- uncertainty and occlusion;
- peripheral summaries and focal detail.

The world remains responsible for physical truth. The creature may infer or predict incorrectly; the outcome is measured from the simulation.

## 11.2 Affordance hints, not desirability

The world MAY provide bounded affordance hints such as reachable, edible-looking, graspable, traversable, likely harmful contact, or vocal target present.

Affordances are sensory/physical information. They MUST NOT contain host-authored desirability, reward, or action scores.

The world SHOULD provide a bounded target set for efficiency, but it need not enumerate every semantic action.

## 11.3 Two-tier attention architecture

Attention is a limited, learnable computational resource.

![Two-tier attention and compute allocation](diagrams/attention.png)

### Tier 1: peripheral processing

A broad cheap pass covers a larger bounded entity or spatial set and produces coarse features such as:

- direction and approximate distance;
- motion and intensity;
- collision/threat cues;
- coarse identity confidence;
- novelty and prediction-error hints;
- broad drive relevance;
- occlusion and uncertainty.

Peripheral processing MUST have a bounded per-class cost.

### Tier 2: focal processing

A small number $K_{attn}$ of targets receive higher-resolution work:

- richer sensory features;
- detailed episodic retrieval;
- concept and causal retrieval;
- target-specific prediction;
- motor parameter decoding;
- increased learning eligibility or structural evidence where appropriate.

The focal limit is:

$$
K_{attn} = K(\text{brain class}, \text{phenotype}, \text{state}, \text{current compute budget})
$$

The class sets a hard maximum. The phenotype and current state set a desired allocation. The scheduler may temporarily reduce it within protected minimums.

## 11.4 Salience and selection

A REFERENCE salience score is:

$$
S_i = w_nN_i + w_gG_i + w_mM_i + w_dD_i + w_cC_i + w_pP_i + w_sS_i^{social}
$$

where terms may represent novelty, unresolved-gap relevance, memory expectancy, drive relevance, concept relevance, prediction error, and social relevance.

This exact equation is not locked. Salience weights may be learned, neuromodulated, and heritable.

Attention MUST:

- be influenced by creature state rather than an omniscient host rule;
- expose selected targets, confidence, and budget in diagnostics;
- support persistence/hysteresis so focus does not flicker every tick;
- permit a switching cost or refractory policy;
- protect immediate pain/danger and body-safety signals;
- charge focal work to cognitive work units;
- be included in ablation and compute-savings tests.

## 11.5 Optional SemanticPrior

A SemanticPrior MAY provide:

- token or phrase embeddings;
- semantic similarity between grounded concepts;
- low-dimensional completion priors;
- mapping between creature tokens and presentation text;
- curriculum hints during schooling.

It MUST NOT:

- emit a motor command or command bundle;
- choose a target;
- author reward;
- declare an ungrounded concept true;
- overwrite episodic memory;
- bypass neural motor coordination.

The creature remains functional when SemanticPrior is disabled.

# 12. Episodic Memory and MemoryExpectancy

## 12.1 Purpose

Episodic memory stores bounded personal experience. It remains separate because forcing every event into neural weights is inefficient and interference-prone.

## 12.2 Memory record

A memory record SHOULD contain compressed representations of:

- pre-action sensory/interoceptive state;
- peripheral and focal attention state;
- active concepts and targets;
- factorized motor bundle;
- measured joint outcome;
- body, drive, and hormone deltas;
- valence, pain, novelty, and prediction error;
- social identity and location context;
- confidence, salience, age, access count, and consolidation state.

## 12.3 Retrieval

Retrieval is bounded top-k similarity search, optionally conditioned on attended target, contemplated channel command, current concept, location, or social identity.

Retrieval returns MemoryExpectancy, not a historical command.

A MemoryExpectancy may include:

```text
expected_valence
predicted_drive_deltas
predicted_hormone_deltas
expected_outcome_features
affordance_likelihood
danger_safety_bias
social_trust_fear_bias
novelty_relative_to_memory
uncertainty
confidence
retrieved_memory_ids_for_diagnostics
```

The recurrent brain decides how to use it.

## 12.4 Memory tiers

The implementation SHOULD provide:

1. a recent episodic ring;
2. consolidated episodic prototypes;
3. links to concepts, places, agents, and unresolved gaps.

Sleep may merge redundant episodes while protecting rare, high-salience, contradictory, or socially important events.

## 12.5 Bounds

Memory capacity is class- and phenotype-bounded. Retrieval cost may not grow without limit.

Permitted strategies include bounded rings, approximate nearest-neighbour indices, locality-sensitive hashing, product quantization, prototype merging, salience eviction, and spatial/social partitions.

# 13. Concept-and-Causal Topology and Concept Induction

## 13.1 Role

The topology is an active cognitive organ. It compresses repeated experience into grounded concepts, relations, causal expectations, affordances, identities, and unresolved contradictions.

It is not a free-form symbolic planner and does not directly issue motor commands.

## 13.2 Core records

A ConceptCell binds a grounded multimodal pattern such as object, token, location, drive, emotion, action, agent, event class, affordance, or social role.

A CognitiveEdge expresses a weighted typed relation such as:

- predicts;
- causes;
- permits;
- prevents;
- satisfies_drive;
- increases_drive;
- affords;
- belongs_to;
- located_near;
- persists_as;
- socially_liked;
- socially_feared;
- similar_to;
- contradicts.

A CognitiveSimplex binds more than two cells into a recurring multi-way episode or schema.

An UnresolvedGap records a missing, contradictory, or low-confidence relation that matters to prediction or action.

## 13.3 Active topology context

Before a decision, topology produces a bounded ConceptContextFrame containing class-scaled top concepts, relations, predicted consequences, unresolved gaps, confidence, novelty, hazard, social estimates, and gap voltage.

The frame is encoded into fixed-width neural channels. It may influence attention, association, working memory, prediction, and motor coordination, but it does not contain a host-authored action score.

## 13.4 Concept candidate formation

A new concept candidate may be proposed when repeated grounded episodes contain a pattern not adequately explained by existing concepts.

Evidence may include:

- latent or perceptual similarity;
- temporal co-occurrence;
- shared predicted consequences;
- shared motor affordances;
- shared drive or hormone effects;
- persistence of identity across time/occlusion;
- repeated social identity or relationship;
- repeated location/context binding;
- episodic retrieval compression;
- language co-grounding;
- systematic prediction residuals or unresolved gaps.

Perceptual clustering alone is insufficient for high-confidence promotion when it does not support useful cognition.

## 13.5 Predictive and cognitive compression utility

Concept promotion is guided by broad predictive/cognitive utility, not only one-step sensory prediction.

A REFERENCE utility is:

$$
U(C) = \Delta L_{prediction}
+ \lambda_a\Delta L_{affordance}
+ \lambda_m\Delta L_{memory}
+ \lambda_s\Delta L_{social}
+ \lambda_d\Delta L_{drive}
+ \lambda_l\Delta L_{language}
- \lambda_c\operatorname{Complexity}(C)
- \lambda_r\operatorname{ResidualContradiction}(C)
$$

The exact equation is not locked. The locked principle is that a concept earns capacity by improving useful prediction, affordance reasoning, memory retrieval, identity, social reasoning, communication, drive-relevant abstraction, or compression while not hiding systematic contradictions.

For example, a creature may retain both a broad concept such as `edible_fruit` and a specific concept such as `apple` when each supports distinct useful predictions.

## 13.6 Promotion, strengthening, split, merge, and decay

A candidate is promoted only after bounded evidence thresholds and grounding checks.

A concept SHOULD be strengthened when it repeatedly improves prediction or retrieval.

A concept SHOULD split when:

- its residual errors consistently separate by context or hidden property;
- a subgroup has different affordances or drive consequences;
- contradiction remains high despite adequate evidence;
- identity persistence requires multiple entities rather than one category.

Concepts SHOULD merge when their predictions, affordances, and relational roles are equivalent enough that separate storage adds little utility.

Low-utility concepts may decay, merge, or be evicted. Rare high-salience contradictions and critical social identities receive protection.

## 13.7 Update from experience

Only sealed ExperiencePatch records update long-lived topology.

Updates may:

- strengthen or weaken edges;
- add evidence to concept candidates;
- create or resolve gaps;
- adjust confidence and uncertainty;
- propose split or merge operations;
- update grounded identity/location bindings.

Fast per-tick observations may accumulate temporary evidence, but persistent topology changes occur through bounded commit logic.

## 13.8 Separation from neural structural plasticity

Concept induction and synaptogenesis are related but distinct.

The implementation MUST NOT use shortcuts where:

- every ConceptCell becomes a neural neuron;
- every concept edge becomes a neural synapse;
- every strong synapse becomes a ConceptCell or cognitive edge;
- deleting a concept automatically deletes neural structure;
- pruning a neural synapse automatically erases conceptual knowledge.

The two systems MAY exchange bounded evidence:

- a concept relation may increase the relevance of a neural growth candidate;
- correlated prediction residuals may support both a concept candidate and a synaptogenesis candidate;
- a neural representation may provide embeddings used by concept induction;
- a concept split may alter attention and therefore future neural learning.

Each system retains its own IDs, budgets, confidence, update rules, persistence, and diagnostics.

## 13.9 Bounds

Topology has per-class caps. Near capacity it merges, decays, compresses, or evicts low-value structures. Retrieval remains bounded top-k.

# 14. Predictive Learning, Curiosity, and Deferred Planning

## 14.1 Predictive targets

At tick $t$, the brain predicts selected components of $t+1$ or a small set of horizons:

- compressed sensory latent;
- attended target persistence or motion;
- contact and collision outcome;
- body, drive, and hormone deltas;
- channel or joint action success probability;
- likely concept activation;
- short-horizon social response;
- uncertainty of its own prediction.

The predictor need not reconstruct every raw sensor value. It predicts behaviourally useful state.

## 14.2 Information-preserving target invariant

The predictive objective MUST contain an information-preserving constraint or grounded target construction that makes a constant, action-insensitive, or identity-destroying representation non-optimal.

![Information-preserving predictive learning](diagrams/prediction.png)

Permitted REFERENCE MECHANISMS include:

- exponential-moving-average target encoder;
- stop-gradient asymmetric encoder;
- fixed or slowly adapted random projections;
- variance/covariance regularization;
- contrastive structure;
- direct prediction of grounded observables;
- multi-horizon or multi-head prediction;
- combinations of the above.

No single self-supervised learning family is mandatory.

## 14.3 Non-collapse gates

A predictive implementation must demonstrate:

- materially different successor states remain separable;
- different actions can predict different successors;
- constant predictions fail the objective or validation suite;
- representation variance remains nondegenerate;
- identity and consequential distinctions are not erased;
- passive observation improves later prediction or behaviour;
- lower loss is not accepted without representational and behavioural evidence.

## 14.4 Prediction error

After the successor state is measured:

$$
\epsilon(t+1) = z^*(t+1) - \hat{z}(t+1)
$$

The system derives:

- signed local errors for predictor learning;
- scalar surprise;
- novelty relative to memory;
- contradiction salience relative to concept confidence;
- optional channel-conditioned residuals when physically meaningful.

Prediction error is included in PostActionOutcome and the sealed patch.

## 14.5 Self-supervised update

Prediction learning is mandatory because most events have no explicit reward.

The production implementation avoids full online backpropagation through long histories. It may use local error pathways, eligibility, Oja-style normalization, target-side stop-gradient, or another validated bounded mechanism.

Offline species training may use BPTT, evolutionary strategies, gradient descent, or other global optimization as long as runtime capability does not depend on offline Python.

## 14.6 Curiosity and gap resolution

Curiosity is not random action noise. It depends on novelty, uncertainty, high prediction error, unresolved causal gaps, expected information gain, safety, fatigue, and energy.

An UnresolvedGap produces bounded voltage that enters attention, concept, association, and working-memory channels. It may bias inspecting, approaching, manipulating, observing, communicating, or testing the relevant stimulus.

A gap is reduced when repeated evidence supports a stable relation or when a concept split explains the contradiction.

The system SHOULD distinguish environmental stochasticity, insufficient context, mistaken identity, unreliable memory, and causal intervention failure.

## 14.7 Deferred bounded planning

Counterfactual planning is a DEFERRED CAPABILITY.

Once the predictor passes non-collapse and grounded prediction gates, a later system may evaluate bounded rollouts:

$$
(s_t,a_1) \rightarrow \hat{s}_1 \rightarrow a_2 \rightarrow \hat{s}_2 \rightarrow \cdots
$$

Reference bounds are depth 2-4 and small branch factor, but exact limits are TUNABLE DEFAULTS.

Simulated outcomes:

- carry explicit simulated identity;
- never become physical episodic ground truth;
- include uncertainty that normally grows with depth;
- are limited by a work-unit budget;
- must improve detour, delayed-choice, or multi-step tasks over reactive controls before promotion.

# 15. Action Selection and Factorized Motor Control

## 15.1 Principle

The world supplies physical state and bounded affordance hints. The creature supplies intent through learned motor channels.

The system MUST NOT depend exclusively on a host-generated list of complete semantic actions such as `EatObject731`.

## 15.2 Factorized parallel command bundle

The production ABI supports a bounded bundle:

$$
A_t = \left(
A_{locomotion},
A_{orientation},
A_{manipulation},
A_{vocal},
A_{posture}
\right)
$$

![Factorized parallel motor control](diagrams/motor.png)

A species may disable channels it lacks and may add versioned species-specific channels within a fixed class-compatible ABI.

Competition occurs primarily within each channel. Learned cross-channel coordination handles compatibility, shared targets, timing, and inhibition.

Examples of compatible joint action include:

- retreat + track predator + defensive posture + alarm vocalization;
- walk + look at food + reach;
- remain still + inspect + vocalize;
- approach + orient + social gesture.

## 15.3 Reference channel primitives

**Locomotion:** idle, locomote, retreat, pursue, climb, swim, fly where supported.  
**Orientation:** orient gaze/head/body, attend, inspect.  
**Manipulation:** reach, contact, grasp, push, pull, ingest, use, attack.  
**Vocal:** vocalize, gesture-token, call, reply.  
**Posture:** defend, groom, mate posture, rest, sleep, display.

This vocabulary is a REFERENCE MECHANISM. Species may change it through versioned phenotype policy.

## 15.4 Targets and parameters

Each non-empty channel command may contain:

- primitive ID;
- optional entity, concept, or location target;
- egocentric direction;
- intensity/vigour;
- duration;
- approach or stand-off distance;
- manipulator/contact subchannel;
- bounded payload such as vocal tokens;
- confidence;
- coordination group or shared-target identity.

The world resolves physical feasibility and actual result.

## 15.5 Joint outcome and credit assignment

Reality often returns an inseparable joint outcome. ExperiencePatch therefore stores:

- every channel command;
- channel execution observations where available;
- the joint physical result;
- joint body and world consequences.

Learning MAY derive channel-local eligibility or credit where causally supportable. It is not required to invent clean labelled rewards for every channel.

A system MUST NOT fabricate per-channel supervision merely to simplify learning.

## 15.6 Affordance-assisted decoding

For efficiency the world may provide up to $K$ salient targets and physical hints. The creature may bind commands to those targets or to directions/locations.

The host MUST NOT attach desirability scores. Neural, memory, concept, drive, and predictive systems determine preference.

## 15.7 Compatibility migration

The current semantic candidate-logit decoder may remain temporarily as a compatibility bridge.

During migration:

- old candidates map into channel/primitive templates;
- new channel decoders run in parallel;
- joint-command and openness tests are added;
- the old exhaustive path is removed only after behavioural parity and novel-combination tests pass.

# 16. ExperiencePatch Causal Transaction

ExperiencePatch remains the shared causal record consumed by learning, memory, concepts, homeostasis, sleep, and logs.

## 16.1 Phase 1: PreActionSnapshot

Required fields include:

- creature ID and tick;
- body, drives, and hormones;
- peripheral sensory frame;
- focal attention frame and budget;
- MemoryExpectancy;
- ConceptContextFrame;
- SemanticPrior context if enabled;
- recurrent state digest;
- predicted successor and confidence;
- available motor-channel masks and affordance hints;
- structural, foundation, phenotype, and schema identities;
- cognitive work budget identity.

## 16.2 Phase 2: DecisionSnapshot

Required fields include:

- selected MotorCommandBundle;
- per-channel alternatives or bounded score summary;
- cross-channel coordination summary;
- confidence;
- active drive/source summary;
- eligibility transaction identity;
- phenotype/foundation identity;
- exact input-frame digest.

A natural-language rationale is not required.

## 16.3 Phase 3: PostActionOutcome

Required fields include:

- joint physical execution result;
- optional channel execution observations where physically measurable;
- success, partial success, or failure;
- contact/collision data;
- energy, pain, injury, temperature, and body deltas;
- drive and hormone deltas;
- successor sensory latent and attended-target changes;
- reward/valence;
- novelty and frustration;
- prediction error vector/summary;
- contradiction salience;
- target persistence and identity changes;
- cognitive work used.

## 16.4 Sealing

A patch may be sealed only when:

- identity and tick order are valid;
- pre-state precedes decision;
- measured outcome follows execution;
- decision refers to the exact input-frame digest;
- numeric fields are finite and bounded;
- target IDs are valid or explicitly missing;
- schema and policy versions are compatible;
- joint-command and outcome records are internally consistent.

A partial or invalid patch MUST NOT update lifetime learning, memory, concepts, or structural evidence as if complete.

## 16.5 Consumers

A sealed patch is consumed exactly once by applicable subsystems:

- waking eligibility credit assignment;
- predictive learning;
- episodic memory;
- concept induction/topology;
- endocrine/homeostatic update;
- structural evidence collection;
- cognitive work accounting;
- logging and research instrumentation;
- sleep replay queue.

Runtime structs and packed log structs remain separate. Variable-length data uses bounded side buffers.

## 16.6 Simulated experience separation

Counterfactual rollouts, dreams, and offline imagined outcomes use a separate simulated-experience record or explicit simulation flag. They never masquerade as physically measured ExperiencePatch outcomes.

# 17. Waking Plasticity

## 17.1 Three-factor learning

The core waking rule combines local participation with later measured modulatory outcome.

A REFERENCE eligibility trace is:

$$
e_{ij}(t) = \rho_e e_{ij}(t-1) + y_j(t)\left(x_i(t)-y_j(t)W^{effective}_{ij}(t)\right)
$$

A REFERENCE behavioural modulator is:

$$
M(t)=\operatorname{clip}(k_rRPE+k_h\Delta H-k_pPain-k_fFrustration+k_nNovelty+k_sSocial,-1,1)
$$

A REFERENCE fast update is:

$$
\Delta H_{ij}=\eta_{ij}M(t)e_{ij}(t)-\lambda_HH_{ij}(t)
$$

Exact equations are replaceable. Locked properties are:

- local participation is recorded before the outcome;
- measured outcome gates credit after execution;
- reward is not the only signal;
- predictive error has a distinct learning path;
- updates are bounded and normalized;
- genetic/foundation banks are not overwritten;
- channel-local credit is used only when causally defensible;
- the joint outcome remains available to the complete organism.

## 17.2 Modulatory channels

Synapses or lobes may respond differently to appetitive, aversive, novelty, stress, social, and predictive-error channels. Receptor patterns are genome-controlled.

## 17.3 Fast learning before sleep

$H_{fast}$ affects subsequent awake behaviour. A creature can adapt immediately without waiting for sleep.

## 17.4 Stability

Fast plasticity includes decay, normalization, clipping, competition, or equivalent controls. Stable useful learning may later move into $W_{lifetime}$.

## 17.5 Predictive and behavioural learning coexist

Predictive updates and outcome-gated updates may modify overlapping or separate plastic components. Implementations must report interference and demonstrate that prediction learning does not erase critical survival learning.

# 18. Sleep Consolidation and Structural Plasticity

Sleep is a computational and biological maintenance state, not merely a pause.

## 18.1 Sleep triggers

Sleep may be triggered by fatigue, circadian/species schedule, plasticity load, memory pressure, structural backlog, or safe environmental opportunity.

Sleep deprivation SHOULD have behavioural and learning consequences.

## 18.2 Consolidation phases

A complete bounded cycle SHOULD:

1. select replay episodes;
2. replay neural and predictive context;
3. consolidate stable $H_{fast}$ into $W_{lifetime}$;
4. decay unstable or contradictory traces;
5. compact episodic memory;
6. update concept candidates, concepts, relations, splits, merges, and gaps;
7. update neuron homeostatic targets;
8. rank neural pruning and growth proposals;
9. compile a dormant sparse topology;
10. validate and atomically swap at a safe boundary;
11. emit exactly-once receipts and work-unit accounting.

## 18.3 Lifetime consolidation

Stable learning may transfer into $W_{lifetime}$. It MUST NOT be baked into $W_{foundation}$ or $W_{genetic\_delta}$ during ordinary sleep.

A REFERENCE transfer is:

$$
W_{lifetime} \leftarrow W_{lifetime}+\gamma_cH_{stable}
$$

$$
H_{fast} \leftarrow (1-\gamma_c)H_{fast}
$$

## 18.4 Conservative pruning

A synapse is eligible when it has low magnitude, low eligibility, low structural utility, low causal/predictive contribution, and no vital/protected status.

A connection may not be pruned solely because it was inactive briefly. Hysteresis, age protection, route minimums, and rollback apply.

## 18.5 Sparse structural evidence

Awake processing accumulates compact evidence without editing topology every tick.

![Sparse structural candidate discovery and sleep-time edits](diagrams/structural.png)

Candidate evidence may arise from:

- repeated coactivity;
- prediction-residual correlation;
- unresolved-gap relevance;
- neuromodulated utility;
- repeated cross-lobe temporal relation;
- working-memory conjunction;
- concept relation relevance;
- attention co-selection;
- bounded stochastic exploration.

The evidence store obeys BRN-STRUCT-001. It may be per neuron, branch, route, microtile pair, sketch, or hybrid.

## 18.6 Proposal ranking

A REFERENCE growth score is:

$$
G_{ij}=C_{ij}P_{ij}U_{ij}R_{ij}
-\lambda_{red}Redundancy_{ij}
-\lambda_{cost}Cost_{ij}
$$

where terms may represent coactivity, predictive usefulness, outcome utility, route/gap relevance, redundancy, and cost.

The exact score is replaceable.

During sleep/development:

1. request top proposals within bounded candidate stores;
2. reject proposals outside route/genome masks;
3. reject duplicates and excessive redundancy;
4. require evidence and stability thresholds;
5. create weak lifetime synapses;
6. keep class and route budgets;
7. replace only weaker eligible structure when no free slot exists;
8. stage edits in a dormant buffer;
9. validate connectivity, complexity, and protected paths;
10. atomically swap or roll back.

## 18.7 Budget-neutral structural learning

After steady-state capacity is reached, synaptogenesis normally competes with weak structure. Early development may fill reserved slots before replacement begins.

New lifetime synapses are not inherited by default.

## 18.8 Concept and neural growth separation

Concept induction and neural structural learning may observe the same ExperiencePatch and may exchange bounded hints, but they commit independently.

A consolidation receipt MUST identify concept edits and neural edits separately.

## 18.9 Deferred counterfactual replay

Bounded counterfactual replay is retained under BRN-PLAN-001. It is not a gate for initial structural-plasticity migration.

# 19. Reproduction, Inheritance, and Evolution

## 19.1 Ordinary inheritance

Offspring inherit:

- species foundation identity;
- genome and mutation/crossover results;
- allowed brain-class gene;
- developmental schedule;
- initial genetic deltas;
- plasticity, dendritic, route, attention, endocrine, predictive, and homeostatic parameters;
- structural candidate and pruning policies;
- cognitive work-cost strategy parameters where heritable.

Offspring do not ordinarily inherit:

- episodic memories;
- concept graph records;
- $W_{lifetime}$;
- $H_{fast}$;
- eligibility;
- working memory;
- injuries or age;
- learned dialect/vocabulary except through an approved cultural mechanism.

## 19.2 Evolution of learning and computation

Selection may act on:

- initial behaviour;
- plasticity magnitude/sign;
- neuromodulator receptors;
- critical periods;
- dendritic complexity;
- synaptogenesis candidate budgets;
- structural thresholds;
- memory retrieval and concept induction;
- attention bandwidth and persistence;
- curiosity strength;
- predictor architecture and target policy;
- motor channel allocation;
- sleep strategy;
- lobe allocation and cadence;
- brain-class cost-benefit.

## 19.3 Cognitive economics in evolution

The runtime reports hardware-independent cognitive work units. Ecology/evolution policy MAY convert them into energy expenditure or fitness cost.

This allows niches to favour cheap N512 reflexive creatures or more expensive N2048/N4096 cognition when intelligence pays for itself.

Fitness analysis SHOULD report capability and cost separately even when a world combines them.

## 19.4 Species foundation promotion

Promotion requires schema compatibility, behavioural results, continual-learning tests, capability-per-compute evidence, generalization, mutation/evolvability evidence, provenance, explicit versioning, and rollback assets.

A foundation is never modified in place.

## 19.5 Learned founder cloning

Cloning a learned individual is a separate `DurableLearnedFounder` operation, not ordinary biological inheritance, and is labelled accordingly.

# 20. Runtime CPU/GPU Placement and Data Movement

## 20.1 Placement by computational shape

| Work | Preferred placement | Reason |
|---|---|---|
| Sparse recurrent projection | GPU | regular, batched, parallel |
| Dendritic evaluation | GPU | local, repetitive, vectorizable |
| Neuron finalization/homeostasis | GPU | per-neuron parallel |
| Eligibility and fast-weight update | GPU | per-synapse parallel |
| Predictor inference/update | GPU | vector operations |
| Motor channel decoding | GPU | batched neural output |
| Peripheral sensing | CPU/world or GPU | depends on renderer/spatial representation |
| Focal target selection | CPU/GPU hybrid | top-k irregularity plus neural salience |
| Episodic retrieval | CPU by default | bounded irregular search; GPU index permitted |
| Concept induction/topology | CPU by default | sparse typed graph and identity |
| UnresolvedGap maintenance | CPU | irregular causal bookkeeping |
| World physics and grounding | world/CPU | ECS and physical simulation |
| Structural evidence sketches | GPU/CPU hybrid | local event capture plus bounded indexing |
| Structural proposal ranking | CPU by default | irregular sorting and constraints |
| Sparse recompaction | CPU compile + GPU transfer | infrequent structural edit |
| Sleep replay weight math | GPU preferred | batched numerical replay |
| Offline species optimization | offline Python/GPU | not gameplay dependency |

Placement may change after profiling. Interfaces remain stable.

## 20.2 Per-tick transfer contract

The production loop avoids bulk activation or weight readback.

CPU-to-GPU transfers are bounded to sensory/interoceptive frames, attention/focal context, MemoryExpectancy, ConceptContextFrame, optional SemanticPrior, motor target hints, neuromodulator/outcome summaries, and infrequent structural/sleep staging.

GPU-to-CPU transfers are bounded to selected motor bundle, optional decoder summary, prediction summary, attention salience summary where needed, work-unit counters, health/error flags, and sampled diagnostics.

## 20.3 Reference tick order

1. World measures body and broad sensory state.
2. Peripheral processing produces coarse bounded summaries.
3. Memory/concepts/neural salience participate in focal target selection.
4. Focal features and bounded cognitive context are assembled.
5. GPU runs due recurrent, dendritic, predictive, and motor-channel passes.
6. World executes the joint motor bundle.
7. World measures the successor state and joint outcome.
8. CPU seals ExperiencePatch.
9. GPU applies eligible waking updates.
10. CPU updates memory, concepts, gaps, homeostasis, structural evidence, and cost records.
11. Logs and sleep queues receive the sealed patch.

Pipelining is allowed only when causal identity is preserved.

## 20.4 Sparse recompaction

The backend SHOULD use active buffer A, dormant/scratch buffer B, bounded asynchronous staging, validation, atomic pointer/index swap, and rollback.

# 21. Performance Budgets, Cognitive Economics, and Graceful Degradation

## 21.1 Reference workload

The reference objective is 500 N2048 agents, 24,576 reference recurrent synapses each with class headroom, online plasticity, 60 ticks/s, 2-4 GB active VRAM, and no blocking full-brain readback.

Results must name hardware, settings, class mix, active attention counts, and enabled mechanisms.

## 21.2 Frame budget

At 60 Hz total frame time is 16.67 ms. The neural/cognitive subsystem SHOULD target p95 8-12 ms under the reference benchmark, leaving time for world and rendering.

## 21.3 Hardware-independent cognitive work units

The runtime MUST calculate versioned cognitive work units independent of wall-clock time.

A REFERENCE accounting model is:

$$
CWU = c_nN_{updated}+c_sS_{evaluated}+c_dD_{ops}+c_aA_{focal}
+c_mM_{retrieval}+c_cC_{topology}+c_pP_{predict}+c_rR_{replay}+c_gG_{structural}
$$

The coefficients are versioned simulation policy. They do not change because a user installs a faster GPU.

Work-unit records SHOULD distinguish:

- awake neural work;
- focal attention work;
- memory/concept retrieval;
- predictive learning;
- structural evidence;
- sleep replay and recompaction;
- optional semantic work.

## 21.4 Biological conversion policy

World/species configuration MAY convert CWU into organism energy expenditure, fatigue, heat, development cost, or fitness penalty.

Benchmarks may disable biological conversion while still recording CWU. This prevents cognition tests from becoming inseparable from starvation while preserving evolutionary economics.

## 21.5 Attention budget

Every phenotype has:

- peripheral entity/cluster cap;
- focal target maximum;
- protected minimum for safety-critical focus;
- focal feature width;
- retrieval budget per focal target;
- switching/persistence policy;
- per-tick focal work-unit cap.

Attention is therefore both cognitive selection and a concrete throughput mechanism.

## 21.6 Graceful degradation order

When overloaded, degrade in this order:

1. remove or reduce non-salient diagnostics;
2. reduce focal target count toward the phenotype's protected minimum;
3. reduce long-horizon prediction and slow concept cadence;
4. reduce slow association cadence;
5. reduce distant/inactive organism cadence;
6. page dormant memory/topology partitions;
7. reduce optional semantic/language work;
8. cap new structural-evidence collection while preserving existing learning;
9. reduce population or use smaller classes.

Preserve pain/danger response, body regulation, minimal grounding, basic motor execution, causal patch sealing, and safety-critical learning.

## 21.7 Dirty and event-driven work

Permitted techniques include dirty microtile masks, supertile culling, event-driven routes, multirate scheduling, selective attention, dormant-agent paging, validated low-rank components, and class-batched dispatch.

# 22. Observability, Testing, and Capability Evaluation

## 22.1 Observability

Bounded diagnostics include:

- lobe activation summaries;
- drive/hormone and neuron metabolic state;
- peripheral summaries and attended targets;
- memory retrieval IDs/confidence;
- concept candidates, active concepts, gaps, split/merge proposals;
- predictions, target-construction identity, and errors;
- motor channel commands and coordination;
- joint outcome and optional channel observations;
- neuromodulator values;
- fast/lifetime weight statistics;
- structural candidate-store size and complexity receipt;
- accepted growth/pruning edits;
- sleep receipts;
- CPU/GPU transfer volume;
- cognitive work units by subsystem.

Diagnostics are throttleable in production.

## 22.2 Deterministic correctness tests

Required tests include:

- effective-weight decomposition;
- patch phase ordering and exact identity;
- no learning from invalid partial patches;
- no direct action replay field in MemoryExpectancy;
- concept context reaches neural inputs;
- gap voltage changes attention/curiosity channels;
- concept and neural IDs/storage remain separate;
- predictive error uses the correct successor and target-construction identity;
- constant/action-insensitive predictor fails validation;
- materially different successors remain separable;
- eligibility credits only matching transactions;
- joint motor bundle is preserved exactly;
- no requirement for fabricated per-channel rewards;
- lifetime consolidation leaves inherited banks unchanged;
- growth discovery does not enumerate absent $N^2$ pairs;
- candidate-store memory stays within class/profile bound;
- growth obeys route masks and class caps;
- pruning preserves vital routes;
- dormant swap is atomic and rollback-safe;
- class buffers do not resize during active ticks;
- SemanticPrior cannot emit commands;
- CWU is deterministic for a fixed execution trace and policy.

## 22.3 Capability benchmarks

At minimum:

1. **Few-shot harm avoidance** - one or few harmful interactions produce later avoidance.
2. **Reversal learning** - the creature adapts when a formerly good cue becomes harmful.
3. **Misleading-correlation recovery** - concept split or context use repairs a false rule.
4. **Concept induction** - repeated experiences produce a useful grounded abstraction.
5. **Concept split/merge** - systematic residuals split a concept; redundant concepts merge without losing capability.
6. **Targeted curiosity** - contradiction produces investigation focused on the relevant target.
7. **Passive observational learning** - watching transitions improves later prediction or behaviour without direct reward.
8. **Delayed episodic recall** - relevant expectancy is used after intervening activity.
9. **Generalization** - learned relations transfer to related unseen objects.
10. **Factorized motor combination** - compatible commands execute simultaneously and outperform whole-body single-action control.
11. **Attention efficiency** - two-tier attention preserves or improves capability while reducing high-resolution work.
12. **Non-collapsed prediction** - state and action distinctions remain encoded and improve behaviour.
13. **Experience-dependent structure** - different histories produce useful different lifetime connectivity under the same genome.
14. **Continual learning** - later learning does not erase earlier survival competence.
15. **Evolutionary trade-off** - at least one ecological test shows selection responding to capability and cognitive cost.

## 22.4 Required ablations

Claims require relevant controls:

- concept topology on vs off;
- concept induction on vs fixed/manual concepts;
- gaps/curiosity on vs random exploration;
- prediction on vs off;
- information-preserving target mechanism on vs naive predictor;
- attention on vs uniform high-resolution processing;
- factorized channels vs single whole-body action;
- synaptogenesis on vs pruning-only fixed topology;
- dendrites on vs ordinary soma;
- episodic expectancy on vs no episodic memory;
- compute-cost selection on vs no cost where evolutionary claims are made;
- SemanticPrior on vs off;
- later bounded planning on vs reactive predictor-only control.

## 22.5 Performance tests

Performance gates report:

- p50, p95, and worst-frame times;
- VRAM/RAM;
- CPU-GPU transfer volume;
- active micro/supertile counts;
- peripheral and focal attention counts;
- retrieval cost;
- predictor cost;
- candidate-discovery and storage scaling;
- sleep/recompaction duration;
- CWU by subsystem;
- scaling curves for population and class mix.

## 22.6 Evidence standard

A mechanism is complete only when structural, behavioural, causal, performance, failure-handling, and observability evidence all exist.

# 23. Migration from the Current Implementation

## 23.1 Preserve

Preserve and reuse:

- sparse recurrent GPU execution;
- multirate lobe routing;
- neuron activity homeostasis and metabolic load;
- separated inherited/lifetime/fast weights;
- eligibility-gated three-factor learning;
- bounded episodic memory;
- ExperiencePatch sealing;
- concept/topology record types;
- trained/evolved foundation assets;
- class-bucketed support;
- double-buffered structural storage;
- bounded action and speech ABIs;
- deterministic identity and evidence checks.

## 23.2 Change

1. Add two-tier attention and focal compute budgets.
2. Connect bounded concept/gap context to neural inputs.
3. Implement concept candidate formation, promotion, split, merge, and eviction.
4. Route UnresolvedGap voltage into attention, association, and working memory.
5. Add non-collapsing predictive targets, heads, errors, and self-supervised updates.
6. Replace whole-body semantic action dependence with a factorized motor bundle.
7. Preserve joint outcomes and optional causal channel observations in ExperiencePatch.
8. Implement bounded non-$N^2$ structural candidate discovery.
9. Implement useful synaptogenesis and competitive pruning.
10. Add nonlinear within-neuron conjunction support.
11. Integrate memory, concepts, predictor, and structure in bounded sleep.
12. Add deterministic hardware-independent CWU accounting.
13. Expose architecture and cognitive economics to evolution.

## 23.3 Do not regress

Migration MUST NOT:

- reintroduce dense all-to-all matrices;
- permit bulk per-tick neural readback;
- bake lifetime learning into inherited weights;
- make SemanticPrior, topology, attention, or a teacher the hidden policy;
- turn memory into direct action replay;
- let concepts, candidates, memory, or synapses grow without bound;
- scan all absent synapses;
- force concept edges and neural synapses into one representation;
- fabricate labelled per-channel rewards;
- make biological cost depend on real hardware speed;
- change brain class during ordinary ticks;
- remove deterministic transaction identity.

# 24. Change Control and Anti-Drift Rules

## 24.1 Canonical source

Canonical path:

`docs/brain/ALife_Adaptive_Brain_Architecture_Spec_v1.1.md`

PDF and DOCX are generated publications. Markdown controls when they differ.

## 24.2 Normative hierarchy

Agents must identify whether a proposed change affects a locked goal, capability, invariant, interface, reference mechanism, tunable default, deferred capability, or research item.

A missing reference mechanism may be replaced. A missing locked capability may not be deleted by calling the current implementation authoritative.

## 24.3 No silent edits

An implementation agent MUST NOT edit the normative specification during ordinary coding unless the task explicitly requests an architecture revision.

A missing implementation is recorded as `NOT_STARTED`, `PARTIAL`, `BLOCKED`, or `DEFERRED`.

## 24.4 Architecture Decision Record

Changing a locked goal, capability, invariant, or interface requires an ADR containing:

- affected requirement IDs;
- current rule and proposed rule;
- reason;
- capability and performance impact;
- evolvability impact;
- grounding/causality/boundedness review;
- alternatives;
- migration cost;
- tests and evidence;
- explicit user approval.

Approved normative changes increment the document version.

## 24.5 Reference mechanism substitution

Replacing a reference mechanism does not always require a full architecture version bump, but it requires a recorded substitution containing:

- preserved locked capability/interface;
- old and new mechanism;
- benchmark and ablation evidence;
- compute and memory impact;
- compatibility/migration plan;
- rollback plan.

A substitution that changes semantics is an architecture change, not an implementation detail.

## 24.6 Implementation compliance matrix

Every major brain PR or milestone reports:

| Field | Required content |
|---|---|
| Requirement IDs | requirements implemented or touched |
| Normative class | goal, capability, invariant, interface, mechanism, default, deferred, research |
| Status | complete, partial, blocked, deferred, or no change |
| Behavioural evidence | capability benchmark/ablation |
| Performance evidence | timing, memory, transfers, CWU |
| Causal evidence | patch and identity correctness |
| Evolvability evidence | where relevant |
| Deviations | explicit, never hidden |
| Follow-up | exact remaining work |

## 24.7 Definition of done

A subsystem is complete only when:

1. it participates in the live causal loop;
2. its locked capability test passes;
3. its invariants and interfaces pass deterministic tests;
4. its performance and CWU budget passes or is explicitly blocked;
5. failure modes and rollback are handled;
6. observability exists;
7. relevant requirement IDs close with evidence.

Types, serializers, receipts, and unit tests alone are insufficient.

# 25. Rejected and Superseded Alternatives

## 25.1 Superseded rules

The following remain superseded:

- processor-location authority as the primary objective;
- concept topology as diagnostic-only;
- gaps that do not affect live cognition;
- exhaustive semantic legal-action candidates as the only motor interface;
- pruning without synaptogenesis as the final structural design;
- a globally fixed N2048 brain;
- permanently frozen lobe topology around one foundation;
- sleep that writes lifetime learning into inherited genetic banks;
- ETF/Neural Collapse as a required runtime representation;
- monolithic `LOCKED` classification that fails to distinguish capability from mechanism.

## 25.2 Rejected alternatives

**Dense monolithic neural network** - poor population scaling and weak structural evolvability.

**Full transformer per creature** - excessive cost and likely dominance over embodied local learning.

**Host symbolic planner that directly chooses actions** - risks scripted behaviour. Structured cognition may supply bounded context, not bypass the learned organism.

**Pure neural weights for all memory** - excessive capacity and interference.

**Direct replay of historical actions** - brittle macro behaviour.

**Exhaustive absent-synapse scan** - violates structural boundedness and population scale.

**Unbounded structural growth** - destroys predictability and resource competition.

**Every concept edge equals a synapse** - improperly couples two useful representations.

**Every strong synapse becomes a concept** - confuses procedural neural association with grounded explicit abstraction.

**Prediction loss alone proves learning** - permits representational collapse.

**Single mutually exclusive whole-body action** - unnecessarily limits behavioural combinations.

**Mandatory per-channel labelled rewards** - creates artificial supervision not supplied by reality.

**Actual GPU milliseconds as biological energy** - makes ecology hardware-dependent.

**Full morphological dendrites** - cost is not currently justified; bounded conjunction units are the target capability.

**Per-tick topology recompaction** - excessive fragmentation and synchronization.

# 26. Risks and Mitigations

| Risk | Consequence | Mitigation |
|---|---|---|
| Topology overwhelms neural learning | graph becomes de facto policy | bounded context, no desirability scores, receptor gains, ablations |
| Concept proliferation | RAM/cognitive fragmentation | predictive-cognitive utility, caps, merge/decay/eviction |
| Concepts hide distinctions | systematic wrong abstraction | residual contradiction, split tests, protected identities |
| Concept-neural coupling shortcut | loss of representational diversity | separate IDs, stores, commit receipts, explicit invariant |
| Curiosity loops | fixation on noise | gap decay, stochasticity classification, safety/energy modulation |
| Attention tunnel vision | misses important peripheral events | protected safety channels, stochastic sampling, persistence limits |
| Attention overhead exceeds savings | lower throughput | class caps, cheap peripheral pass, ablation and CWU accounting |
| Predictive collapse | useless constant representation | information-preserving invariant, separability/action tests |
| Predictive learning interferes with survival | forgetting or unstable policy | separate channels/banks, replay, continual-learning tests |
| Multi-channel credit ambiguity | unstable learning | joint outcome retained, eligibility, optional causal local signals, no fake labels |
| Structural candidate false negatives | useful connection never proposed | multiple sparse discovery sources, stochastic exploration, sketch ablation |
| Structural candidate explosion | hidden $N^2$ work | hard store bounds, complexity receipt, class budgets |
| Structural churn | unstable topology and expensive swaps | sleep-only edits, thresholds, hysteresis, age protection, rollback |
| Dendritic overhead | reduced population scale | selective use, tunable layout, fused kernels, capability-per-FLOP gate |
| Foundation lock-in | evolution cannot discover alternatives | genome deltas, route/lobe mutation, multiple foundations, blank controls |
| Cognitive cost policy distorts ecology | arbitrary starvation pressure | report CWU separately, versioned coefficients, configurable conversion |
| Hybrid synchronization | CPU/GPU stalls | fixed-size frames, double buffering, pipelining, no bulk readback |
| Host-authored affordances | scripted intelligence | physical hints only, no desirability, openness tests |
| Reward hacking | narrow exploit | predictive learning, homeostasis, multi-signal evaluation |
| Agent implementation drift | missing mechanisms normalized as design | normative classes, ADRs, compliance matrix, capability-based done |

# 27. Implementation Sequence and Completion Gates

The implementation SHOULD proceed in dependency order.

## Phase 0 - Specification integration and baseline compliance

- Add canonical v1.1 Markdown.
- Mark v1.0 and conflicting documents superseded.
- Add requirement IDs and normative classes to roadmap/status tooling.
- Record current compliance without editing the spec to match code.

**Gate:** v1.1 is versioned, referenced by agent instructions, and has a complete baseline compliance matrix.

## Phase 1 - Causal and accounting contracts

- Extend CognitiveContextFrame, MotorCommandBundle, ExperiencePatch, and receipts.
- Add deterministic CWU accounting.
- Add concept/neural representation separation IDs.
- Add observability for attention, prediction, motor channels, and candidate stores.

**Gate:** contracts serialize, validate, replay deterministically, and do not alter current behaviour through hidden defaults.

## Phase 2 - Two-tier attention

- Implement peripheral summaries.
- Add class/phenotype focal budgets.
- Add learned/neural salience and persistence.
- Charge focal processing to CWU.

**Gate:** attention preserves or improves a grounded task while reducing high-resolution work relative to uniform processing.

## Phase 3 - Active concept context and concept induction

- Connect ConceptContextFrame to neural inputs.
- Add candidate formation, promotion, split, merge, decay, and eviction.
- Implement predictive/cognitive utility and bounded evidence.
- Preserve concept/neural separation.

**Gate:** experience forms a useful concept, systematic contradiction splits it, and concept context measurably improves behaviour.

## Phase 4 - UnresolvedGap curiosity

- Add gap target binding, voltage, decay, resolution, and stochasticity handling.
- Route into attention, association, and working memory.

**Gate:** contradiction produces targeted investigation and resolves when evidence becomes reliable.

## Phase 5 - Non-collapsing predictive learning

- Add predictor heads and target-construction interface.
- Implement at least one validated information-preserving mechanism.
- Seal target identity and successor error.
- Add passive-observation and separability tests.

**Gate:** prediction improves with experience, remains nondegenerate and action-sensitive, and improves behaviour without direct reward.

## Phase 6 - Factorized motor control

- Add fixed-size motor channels and cross-channel coordination.
- Map old candidates into compatibility templates.
- Preserve joint outcomes and optional channel observations.

**Gate:** a creature performs a useful compatible multi-channel behaviour unavailable to the old single-action representation.

## Phase 7 - Sparse structural plasticity

- Implement bounded candidate discovery using one or more permitted granularities.
- Emit complexity receipts proving non-$N^2$ work.
- Rank growth/pruning proposals during sleep/development.
- Add budget-neutral edits and atomic sparse swaps.

**Gate:** experience creates a useful lifetime connection, total candidate and synapse budgets remain bounded, and edits survive reload.

## Phase 8 - Nonlinear dendritic/conjunction capability

- Add bounded flat metadata/state.
- Implement one or more reference conjunction mechanisms.
- Enable selectively and run ablations.

**Gate:** conjunction capability improves a target task enough to justify cost.

## Phase 9 - Integrated sleep

- Coordinate replay, fast-to-lifetime consolidation, memory compaction, concept revision, predictor consolidation, and structural edits.
- Add exactly-once receipts and rollback.

**Gate:** sleep improves retention and adaptation within budget while preserving causal identity and representation separation.

## Phase 10 - Evolution and cognitive economics

- Expose approved attention, concept, prediction, structural, dendritic, motor, and cost parameters to evolution.
- Implement configurable CWU-to-metabolism policy.
- Train/evolve at least one alternative phenotype/foundation.

**Gate:** evolved variants trade cognition against cost without inheriting lifetime state or violating class bounds.

## Phase 11 - Deferred bounded planning

- Implement bounded predictor rollouts after predictor maturity.
- Preserve simulated/physical distinction and uncertainty.

**Gate:** bounded planning improves a detour or multi-step task over reactive controls within its CWU budget.

## Phase 12 - Reference-scale certification

- Run mixed-class scaling.
- Run 500 x N2048 reference benchmark.
- Run long continual-learning ecology.
- Publish capability, ablation, performance, CWU, evolvability, and causal receipts.

**Gate:** requirements meet agreed thresholds or remain explicitly blocked without redefining the architecture.

# Appendix A. Normative Data Contracts

The pseudocode fixes semantic contracts, not physical storage.

## A.1 Weight banks

```rust
pub struct SynapseState {
    pub pre_neuron: NeuronId,
    pub target: SynapseTarget,
    pub route_id: RouteId,
    pub w_foundation: Weight,
    pub w_genetic_delta: Weight,
    pub w_lifetime: Weight,
    pub alpha: PlasticityGain,
    pub h_fast: FastWeight,
    pub eligibility: EligibilityTrace,
    pub stability: f32,
    pub structural_utility: f32,
    pub flags: SynapseFlags,
}
```

## A.2 Attention frame

```rust
pub struct AttentionFrame {
    pub peripheral: BoundedPeripheralEntities,
    pub focal_targets: BoundedFocalTargets,
    pub requested_focal_count: u8,
    pub granted_focal_count: u8,
    pub protected_minimum: u8,
    pub persistence: FixedArray<AttentionPersistence, MAX_FOCAL>,
    pub salience_summary: BoundedSalienceSummary,
    pub budget_identity: AttentionBudgetId,
    pub work_units: CognitiveWorkUnits,
}
```

## A.3 Cognitive context

```rust
pub struct CognitiveContextFrame {
    pub attention: AttentionFrame,
    pub memory: MemoryExpectancy,
    pub concept: ConceptContextFrame,
    pub semantic_prior: Option<SemanticPriorFrame>,
    pub prediction_state: PredictionContext,
    pub body_context: BodyCognitiveContext,
}
```

## A.4 Concept context and induction evidence

```rust
pub struct ConceptContextFrame {
    pub active_concepts: BoundedConceptActivations,
    pub active_relations: BoundedRelationActivations,
    pub unresolved_gaps: BoundedGapActivations,
    pub predicted_drive_delta: DriveVector,
    pub expected_valence: f32,
    pub hazard: f32,
    pub social_trust: f32,
    pub social_fear: f32,
    pub novelty: f32,
    pub uncertainty: f32,
    pub gap_voltage: f32,
}

pub struct ConceptCandidateEvidence {
    pub candidate_id: ConceptCandidateId,
    pub grounded_episode_ids: BoundedEpisodeIds,
    pub latent_similarity: f32,
    pub temporal_cooccurrence: f32,
    pub predictive_gain: f32,
    pub affordance_gain: f32,
    pub memory_gain: f32,
    pub social_gain: f32,
    pub drive_gain: f32,
    pub complexity_cost: f32,
    pub residual_contradiction: f32,
    pub confidence: f32,
}
```

Concept IDs and neural IDs are distinct types.

## A.5 Prediction target contract

```rust
pub enum PredictionTargetFamily {
    EmaTeacher,
    StopGradientAsymmetric,
    FixedProjection,
    VarianceCovarianceConstrained,
    Contrastive,
    GroundedObservables,
    Composite,
}

pub struct PredictionTargetReceipt {
    pub family: PredictionTargetFamily,
    pub policy_version: PolicyVersion,
    pub target_digest: Digest,
    pub representation_variance: f32,
    pub action_sensitivity_score: f32,
    pub successor_separability_score: f32,
}
```

The family is replaceable. Non-collapse properties are not.

## A.6 Factorized motor bundle

```rust
pub enum MotorChannel {
    Locomotion,
    Orientation,
    Manipulation,
    Vocal,
    Posture,
    SpeciesSpecific(u8),
}

pub struct ChannelCommand {
    pub channel: MotorChannel,
    pub primitive: PrimitiveId,
    pub target: Option<TargetBinding>,
    pub direction: EgocentricDirection,
    pub intensity: f32,
    pub duration_ticks: u16,
    pub stand_off_distance: f32,
    pub payload: BoundedMotorPayload,
    pub confidence: f32,
    pub coordination_group: u8,
}

pub struct MotorCommandBundle {
    pub creature_id: CreatureId,
    pub tick: u64,
    pub channels: FixedArray<Option<ChannelCommand>, MAX_MOTOR_CHANNELS>,
    pub coordination: BoundedCoordinationSummary,
}
```

## A.7 Experience phases

```rust
pub struct PreActionSnapshot {
    pub creature_id: CreatureId,
    pub tick: u64,
    pub body: BodySnapshot,
    pub drives: DriveSnapshot,
    pub hormones: HormoneSnapshot,
    pub sensory: SensorySnapshot,
    pub cognitive_context: CognitiveContextFrame,
    pub predicted_successor: PredictedState,
    pub prediction_target: PredictionTargetReceipt,
    pub frame_digest: FrameDigest,
}

pub struct DecisionSnapshot {
    pub selected: MotorCommandBundle,
    pub alternatives: BoundedChannelDecisionSummary,
    pub eligibility_id: EligibilityTransactionId,
    pub phenotype_digest: PhenotypeDigest,
    pub frame_digest: FrameDigest,
}

pub struct JointPhysicalOutcome {
    pub execution: PhysicalResult,
    pub channel_observations: BoundedOptionalChannelObservations,
    pub body_delta: BodyDelta,
    pub world_delta: BoundedWorldDelta,
}

pub struct PostActionOutcome {
    pub joint: JointPhysicalOutcome,
    pub drive_delta: DriveVector,
    pub hormone_delta: HormoneVector,
    pub reward_valence: f32,
    pub pain_delta: f32,
    pub novelty: f32,
    pub frustration: f32,
    pub prediction_error: PredictionError,
    pub successor_digest: FrameDigest,
    pub work_units: CognitiveWorkUnits,
}

pub struct ExperiencePatch {
    pub pre: PreActionSnapshot,
    pub decision: DecisionSnapshot,
    pub outcome: PostActionOutcome,
    pub seal: PatchSeal,
}
```

## A.8 Sparse structural candidate interface

```rust
pub enum CandidateGranularity {
    Neuron,
    DendriticBranch,
    Route,
    MicroTilePair,
    Sketch,
    Hybrid,
}

pub struct StructuralCandidateEvidence {
    pub candidate_id: StructuralCandidateId,
    pub source: StructuralSource,
    pub target: SynapseTarget,
    pub route: RouteId,
    pub coactivity: f32,
    pub residual_correlation: f32,
    pub outcome_utility: f32,
    pub concept_gap_relevance: f32,
    pub redundancy: f32,
    pub estimated_cost: f32,
    pub observations: u16,
    pub last_tick: u64,
}

pub struct StructuralCandidateReceipt {
    pub granularity: CandidateGranularity,
    pub retained_candidates: u32,
    pub hard_candidate_cap: u32,
    pub dense_pairs_examined: u32, // MUST remain zero in production discovery
    pub work_units: CognitiveWorkUnits,
    pub store_digest: Digest,
}
```

## A.9 Structural edit

```rust
pub enum StructuralEdit {
    AddLifetimeSynapse {
        pre: NeuronId,
        target: SynapseTarget,
        route: RouteId,
        initial_weight: f32,
        evidence: StructuralEvidenceDigest,
    },
    RemoveLifetimeSynapse {
        synapse: SynapseId,
        reason: PruneReason,
    },
}
```

## A.10 Cognitive work units

```rust
pub struct CognitiveWorkUnits {
    pub neural_updates: u64,
    pub synapses_evaluated: u64,
    pub dendritic_ops: u64,
    pub focal_target_ops: u64,
    pub memory_ops: u64,
    pub concept_ops: u64,
    pub prediction_ops: u64,
    pub replay_ops: u64,
    pub structural_ops: u64,
    pub weighted_total: u64,
    pub policy_version: PolicyVersion,
}
```

## A.11 Deferred counterfactual receipt

```rust
pub struct CounterfactualRolloutReceipt {
    pub simulated: bool, // MUST be true
    pub root_patch_id: PatchId,
    pub depth: u8,
    pub branches: u8,
    pub predicted_states: BoundedPredictedStates,
    pub uncertainty: BoundedUncertaintyTrace,
    pub work_units: CognitiveWorkUnits,
    pub model_digest: Digest,
}
```

# Appendix B. Reference Defaults and Tunable Ranges

| Parameter | Reference default | Allowed/tunable policy |
|---|---|---|
| Fast route cadence | 60 Hz | 30-120 Hz according to simulation rate |
| Medium route cadence | 30 Hz | 15-60 Hz |
| Slow route cadence | 7.5-15 Hz | 5-30 Hz |
| Dendritic branches per selected neuron | 2 | 0-4 reference range; alternative bounded conjunction mechanism allowed |
| Inputs per branch | 8-16 | 4-32 reference range |
| N2048 recurrent synapses | 24,576 | up to hard cap 65,536 |
| N2048 active episodic records | 256 | class/phenotype bounded |
| N2048 concept cells | 512 | class/phenotype bounded |
| Memory top-k | 8 | 1-16 |
| Active concept top-k | 8 | 1-16 |
| Active gaps | 4 | 1-8 |
| Peripheral entity summaries, N2048 | 32 | class/profile bounded |
| Focal targets, N2048 | 4 | 1-8 within class cap and current budget |
| Attention persistence | 4-16 ticks | phenotype and task dependent |
| Structural candidate budget | equivalent to 8-32 per eligible neuron for N2048 | may be implemented per neuron, branch, route, tile, sketch, or hybrid; $O(NK)$ or better |
| Structural edit cycle | sleep/development boundary | never ordinary topology mutation each tick |
| Structural additions per cycle | low bounded count | profile/class dependent |
| Prediction target family | composite stop-gradient/grounded reference | any family satisfying BRN-PRED-001 and tests |
| Predictor horizons | 1 and short multi-step | class/phenotype bounded |
| Motor channels | five reference channels | species may disable/add versioned bounded channels |
| Counterfactual depth | 2-4, deferred | strict work budget and uncertainty |
| Full neural readback | disabled | sampled debug only |
| Staging transfer chunk | 16 MB reference | backend-tunable, non-blocking |
| Active VRAM target | 2-4 GB | reference workload |
| Neural/cognitive p95 frame budget | 8-12 ms | named hardware profile |
| CWU coefficients | versioned policy | hardware-independent; world/species configurable |
| CWU-to-metabolic conversion | disabled in isolated benchmarks | configurable ecology/species policy |

Numeric defaults may change through measured tuning while locked goals, capabilities, invariants, and interfaces remain.

# Appendix C. Requirement Index

| Requirement | Classification | Verification summary |
|---|---|---|
| BRN-CORE-001 Hybrid cognition | LOCKED GOAL | placement table and bounded interfaces |
| BRN-CORE-002 Sparse recurrent substrate | LOCKED CAPABILITY | sparse dispatch and class benchmark |
| BRN-CORE-003 Scalable class buckets | LOCKED INVARIANT | compile all classes; no active resize |
| BRN-CORE-004 Separated weight banks | LOCKED INTERFACE | lifetime learning leaves inherited banks unchanged |
| BRN-CORE-005 Expectancy memory | LOCKED INTERFACE | no action replay API |
| BRN-CORE-006 Active topology | LOCKED CAPABILITY | concept-context ablation changes grounded behaviour |
| BRN-CORE-007 Functional gaps | LOCKED CAPABILITY | contradiction produces targeted curiosity |
| BRN-CORE-008 Predictive learning | LOCKED CAPABILITY | passive observation improves prediction/behaviour |
| BRN-CORE-009 Factorized motor bundle | LOCKED INTERFACE | channel ABI and multi-action benchmark |
| BRN-CORE-010 Structural plasticity | LOCKED CAPABILITY | useful growth under fixed caps |
| BRN-CORE-011 Nonlinear conjunction | LOCKED CAPABILITY | selective implementation and ablation |
| BRN-CORE-012 Sealed ExperiencePatch | LOCKED INTERFACE | phase, identity, and joint-outcome tests |
| BRN-CORE-013 Flexible foundation | LOCKED GOAL | alternative phenotype/foundation with evolvability evidence |
| BRN-CORE-014 SemanticPrior limits | LOCKED INVARIANT | no command/reward path |
| BRN-CORE-015 ETF/D2NWG optional | RESEARCH | core runtime works without them |
| BRN-CORE-016 Capability per compute | LOCKED GOAL | capability and CWU reported together |
| BRN-STRUCT-001 Sparse growth discovery | LOCKED INVARIANT | zero dense absent-pair enumeration; bounded receipt |
| BRN-CONCEPT-001 Concept induction | LOCKED CAPABILITY | formation, split, merge, bounded utility tests |
| BRN-CONCEPT-002 Representation separation | LOCKED INVARIANT | distinct IDs/stores/receipts; no direct mapping shortcut |
| BRN-PRED-001 Information preservation | LOCKED INVARIANT | noncollapse, separability, action sensitivity |
| BRN-ATTN-001 Two-tier attention | LOCKED CAPABILITY | compute savings and focal-behaviour tests |
| BRN-MOTOR-001 Parallel channels | LOCKED INTERFACE | within-channel competition and cross-channel coordination |
| BRN-EXP-001 Joint outcome | LOCKED INTERFACE | commands + joint result retained; no fabricated reward labels |
| BRN-COST-001 CWU accounting | LOCKED INTERFACE | deterministic hardware-independent accounting |
| BRN-PLAN-001 Bounded planning | DEFERRED CAPABILITY | simulated/physical distinction and detour benchmark |
| BRN-GOV-001 Normative hierarchy | LOCKED INVARIANT | agents report classification and substitution evidence |

# Appendix D. Source and Decision Provenance

| Source | Retained contribution | v1.1 treatment |
|---|---|---|
| ALife Brain Runtime Specification v0.1 | lobes, drives, dendritic segments, memory expectancy, concept topology, gaps, action contract, sleep growth/pruning, sparse GPU design | processor ideology removed; active cognition retained; interfaces modernized |
| ExperiencePatch Contract | engine-independent IDs, phase model, rich runtime vs packed logs, expectancy not replay, lifetime/genetic separation, causal validation | expanded with attention, prediction targets, motor bundle, joint outcome, CWU, structural evidence |
| Flat Sparse Tensor ALife Engine v1.1 | class-batched flat tensors, tiled sparsity, fast execution, bounded compute | fixed global N2048 and mandatory ETF path rejected; sparse execution retained |
| Neural Collapse ETF Meta-Learning Framework | outer-loop optimization, plasticity research, tiled sparsity, low-precision considerations | optional research only |
| Current repository implementation review | recurrent GPU brain, multirate routes, trained foundation, eligibility learning, memory/topology records, classes, receipts | valuable baseline; diagnostic topology, missing growth/dendrites/prediction, and semantic-candidate dependence remain migration gaps |
| ALIFE-BRAIN-ARCH-001 v1.0 | hybrid layered architecture, active topology, prediction, motor primitives, synaptogenesis, dendrites, capability gates, anti-drift | superseded and refined by v1.1 |
| User-approved architecture review, 2026-08-12 | capability-per-compute priority, sparse growth discovery, concept induction, stable prediction, attention, parallel channels, compute economics, representation separation | incorporated as normative v1.1 changes |

Where a mechanism is not directly present in an earlier source, it is a new project design decision recorded by this version.

# Appendix E. Glossary

**AttentionFrame:** bounded record of peripheral summaries, focal targets, salience, persistence, budget, and work.  
**Brain class:** fixed capacity bucket defining neural and cognitive limits.  
**Capability per compute:** cognitive improvement relative to measured resources/CWU, interpreted with robustness and evolvability.  
**CognitiveContextFrame:** bounded context supplied to the neural brain from attention, memory, concepts, prediction, semantic prior, and body state.  
**Cognitive work units (CWU):** hardware-independent accounting of cognitive operations.  
**Concept candidate:** provisional abstraction not yet promoted to a stable ConceptCell.  
**ConceptCell:** grounded multimodal abstraction retained because it provides cognitive utility.  
**Dendritic subunit:** bounded nonlinear within-neuron conjunction mechanism.  
**Eligibility trace:** temporary record of causal synaptic participation.  
**ExperiencePatch:** sealed transaction containing pre-action state, decision, and measured joint outcome.  
**Foundation:** versioned species-level inherited neural prior.  
**Focal attention:** expensive high-resolution processing allocated to a small bounded target set.  
**Growth candidate:** absent neural connection retained in a sparse evidence store for possible later creation.  
**$H_{fast}$:** rapid lifetime plastic component.  
**Joint outcome:** physical consequence of the complete motor command bundle, not necessarily decomposable by channel.  
**MemoryExpectancy:** retrieved expectation/context, never a direct action replay.  
**MotorCommandBundle:** bounded parallel set of channel commands.  
**Peripheral perception:** cheap broad sensory summary preceding focal allocation.  
**Reference mechanism:** preferred current solution replaceable through evidence.  
**Structural plasticity:** experience-dependent creation/removal of lifetime neural connections under fixed budgets.  
**UnresolvedGap:** important contradiction or missing causal relation that generates bounded information-seeking pressure.  
**$W_{lifetime}$:** stable learned synaptic component not ordinarily inherited.

# Appendix F. v1.0 to v1.1 Change Record

| Area | v1.0 | v1.1 |
|---|---|---|
| Normative status | LOCKED/TUNABLE/DEFERRED/RESEARCH | separates goals, capabilities, invariants, interfaces, mechanisms, defaults, deferred, research |
| Priority order | biological inspiration ahead of efficiency/evolvability | capability per compute second; evolvability third; grounding/causality are hard invariants |
| Synaptogenesis discovery | structural evidence sources, no explicit discovery complexity contract | explicit non-$N^2$ invariant; flexible neuron/branch/route/tile/sketch stores |
| Concept induction | representation/update emphasized | formation, promotion, broad utility, split, merge, decay, eviction specified |
| Prediction | prediction mandatory; collapse risk noted | information-preserving target invariant and noncollapse gates specified without locking one SSL family |
| Attention | functionally described | explicit peripheral/focal architecture, class/phenotype/budget-dependent $K_{attn}$, CWU accounting |
| Motor output | parallel channels optional | bounded factorized bundle required; joint outcome retained; local credit optional |
| Compute economics | metabolic cost optional | hardware-independent CWU mandatory; biological conversion configurable |
| Concept/neural relationship | not explicit | distinct representations with bounded evidence exchange and separate commits |
| Planning | deferred counterfactual replay | retains deferred status and adds rollout semantics, uncertainty, and evaluation path |
| Dendrites | exact bounded branch reference | capability locked; exact mechanism more clearly replaceable by ablation evidence |
