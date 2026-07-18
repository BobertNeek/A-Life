# A-Life Master Specification

**Project:** A-Life  
**Stack:** Rust + Bevy + wgpu/WebGPU + WGSL  
**Spec revision:** 0.5 N2048 foundation, grounded language, and evolvable lineage
**Date:** 2026-07-17
**Status:** implementation target; GPU-authoritative trainable creature runtime

This document is the controlling engineering specification for A-Life. It supersedes earlier fixed-2048, HLSL, dense-matrix, single-pass-kernel, scaffold-only, and CPU-shadow neural drafts. It preserves scalable brain classes, class-bucketed sparse storage, Rust/Bevy/wgpu/WebGPU/WGSL, separated neural compute passes, and explicit boundaries between internal subconscious semantic priors and external teacher agents. ADR-024 and ADR-027 through ADR-030, together with the approved GPU closed-loop and N2048 foundation program, control production cognition when older milestone prose conflicts with this specification. ADR-025 and ADR-026 remain reserved for their approved memory/grounding and scaling/promotion checkpoints.

The implementation goal is one causally closed production brain in which current perception, recurrent state, candidate scoring, action selection, measured outcome, waking plasticity, sleep consolidation, and later behavior remain connected. Production neural execution is GPU-authoritative WGSL. Pure CPU neural math is test-only or developer-only and never runs as a live shadow, parity gate, or automatic neural fallback. N2048 is the first fully trained class: curated inherited foundations establish robust sensorimotor and language mechanics, evolution hardens them, and personal semantic and episodic knowledge remains lifetime-learned.

The long-term research direction is a grounded developmental generalist agent: an artificial organism whose world model, instincts, language, social behavior, and reasoning arise from evolved brain topology, Hebbian/Oja plasticity, neurochemistry, embodied experience, schooling, and sleep consolidation. The formal spec avoids product claims about AGI. Speculative frontier-scale ideas are quarantined in `docs/future_research_compatibility.md` and must not become v0/v1 requirements.

## Table of Contents

1. Product Vision and Non-Negotiable Decisions
2. Architecture Status and Superseded Drafts
3. Repository and Tooling Goals
4. Runtime Layer Model
5. Rust Workspace Layout
6. Bevy World Layer
7. Engine-Agnostic Cognitive Core
8. Scalable Brain Classes
9. Lobe Layout Generation
10. Genome and Developmental Encoding
11. Genetics, Evolution, and Population Pressure
12. Neurochemistry and Drive System
13. Internal SLM / Semantic Prior Layer
14. External Teacher LLM and Schooling Boundary
15. ExperiencePatch Contract
16. ActionCommand Contract
17. Sensory ABI
18. Hearing, Speech, and Writing ABI
19. Action ABI and Motor Arbitration
20. Memory Architecture
21. Sparse Tensor Storage Model
22. GPU Memory Profiles and Residency
23. Brain Migration and Ascension Compatibility
24. Learning Rules: Hebbian, Oja, and Modulators
25. Weight Decomposition
26. GPU Compute Pipeline
27. WGSL Authoring Rules
28. Sleep, Replay, and Consolidation
29. Topological Memory and Curiosity
30. Graphify Integration
31. DOX / AGENTS.md Hierarchy
32. Codex Operating Rules
33. Testing and Validation Strategy
34. Determinism and Reproducibility
35. Performance and Profiling Plan
36. Data Persistence, Saves, and Lineage Export
37. Schooling and Curriculum Interfaces
38. Future Compatibility Envelope
39. Non-Goals for v0/v1
40. Implementation Milestones
41. Required Files and Modules
42. Glossary

---

## 1. Product Vision and Non-Negotiable Decisions

A-Life is a developmental artificial-life simulation game and research sandbox. The user-facing fantasy is an evolving ecosystem of creatures that learn, adapt, remember, reproduce, and eventually produce exceptional lineages that can be selected for deeper training. The engineering target is a modular sparse neural runtime that lets a population of organisms run on consumer hardware while preserving a path to larger single-agent and future research modes.

The controlling stack is Rust, Bevy, wgpu/WebGPU, and WGSL. Unity and C# are explicitly out of scope. HLSL may appear as a downstream native backend artifact of GPU drivers, but A-Life source shaders are authored in WGSL. CUDA, Triton, Vulkan-only, DirectX-only, TPU, and cluster runtimes are future research backends behind a compute abstraction. They are not initial implementation targets.

A-Life is not a conventional game AI planner. It is not a behavior tree system with learned parameters sprinkled on top. The organism brain is a sparse, plastic, metabolically constrained, genetically structured substrate. The world gathers current perception, enumerates unscored same-tick candidates, validates the selected structured action, and measures its outcome. GPU recurrent dynamics, candidate-conditioned selection, eligibility-gated three-factor plasticity, automatic GPU sleep consolidation, and evolution over brain topology are first-class mechanics.

Candidates contain only observations and command-transport fields; they never
contain caller-provided utilities or scores. Command-transport fields identify
the action family and tick-local target needed to construct the selected
structured command; they do not supply desirability, danger, reward, learned
value, or any other decision hint.

The core organism model must support scalable brains. N512, N1024, and N2048 are the only promoted production neural capacity classes. The earlier `Standard2048` name remains a reference/legacy adapter, not an invariant, and larger saved tier identifiers remain readable but research-gated until their causal, hardware, save, soak, memory, and performance evidence is accepted. Scalable means class-bucketed and profile-gated, not dynamically resizing arbitrary matrices in the hot loop.

`NeuralClosedLoopGpu` is the normal neural policy. `HeuristicBaseline` remains an explicit, separately labelled comparison policy and is never selected because GPU neural execution failed. Missing required GPU support returns a typed `NeuralBackendUnavailable` result and performs no learned action; no error path silently runs a second brain or claims a neural tick occurred.

The system distinguishes three sources of behavior. First, inherited instincts arise from genome-controlled structure: lobe proportions, macro-connectome masks, alpha plasticity gates, endocrine constants, morphology, sensory layout, and immutable genetic weight priors. Second, lifetime habits arise from Oja/Hebbian plasticity and sleep consolidation. Third, optional semantic priors and teachers supply language/culture scaffolding. These three sources must remain separate in data layout and scheduling so experiments can ablate them independently.

The internal SLM is a private subconscious semantic prior. It is used partly as an engineered substitute for the vast evolved instinct and proto-language priors that real animals inherit after millions of years of evolution. It may bias attention, lexicon/concept activity, memory recall, and weakly modulate plasticity. It may not issue actions, bypass action arbitration, directly rewrite weights, or act as an external teacher.

The external teacher LLM is different. It must teach through ordinary simulated perception: speech, hearing, writing, gesture, demonstration, objects, and in-world feedback. It may privately plan curriculum, grade, and choose next lessons, but instructional content must enter through the creature's normal sensory channels. This preserves the grounding problem instead of secretly solving it with hidden vector injection.


## 2. Architecture Status and Superseded Drafts

Earlier architecture notes are valuable but inconsistent. They establish important concepts: flat buffers, super-tile sparse culling, Oja plasticity, ETF sensory anchors, neurochemical drives, sleep compaction, SLM boundary separation, and motor arbitration. They also contain assumptions this spec rejects.

Superseded assumption one: fixed 2048-neuron brains. The new architecture uses scalable `BrainScaleTier` and `BrainClassSpec`. `Standard2048` remains a reference tier only.

Superseded assumption two: dense population-wide `[M, N, N]` weight buffers. Dense conceptual matrices are allowed in diagrams and tiny CPU reference tests only. Runtime storage is sparse, class-bucketed, and payload-pool based.

Superseded assumption three: HLSL production kernels. The production source language is WGSL through Rust `wgpu`. HLSL snippets from older documents are pseudocode only.

Superseded assumption four: one-pass TileSpMV+Oja fusion. The production pipeline separates accumulator clear, sparse projection, activation finalization, and plasticity update. This avoids read/write hazards and improves debuggability.

Superseded assumption five: one-byte action tokens. Actions are structured envelopes with action ID, target entity, target position, intensity, duration, confidence, and drive-source mask. Compact GPU buffers are acceptable; semantic collapse to a single byte is not.

Superseded assumption six: direct raw CPU pointers into GPU kernels. The runtime uses preallocated GPU storage buffers, staging/upload buffers, bind groups, integer offsets, and packed ABI structs. No shader sees a Rust pointer.

Superseded assumption seven: language vector injection as teaching. The internal SLM can modulate Lexicon/Concept lobes. The external teacher must teach through perception.

Superseded assumption eight: a scaffold-only repository that forbids production neural kernels. ADR-024 authorizes the reviewed GPU-authoritative closed loop and requires real WGSL encoding, recurrent, candidate-decoding, winner-selection, waking-plasticity, and sleep-consolidation pipelines.

Superseded assumption nine: a production CPU neural oracle, live shadow, parity-gated handoff, or automatic neural fallback. CPU neural helpers are limited to tests, developer diagnostics, and offline fixture generation. Production GPU failure returns a typed unavailable result.

The repository must preserve these corrections as explicit architecture decisions in `docs/architecture_decisions.md` so future agent sessions do not regress.


## 3. Repository and Tooling Goals

The repository is an implementation platform for the approved GPU-authoritative closed loop, not a scaffold-only container. Workspaces, versioned contracts, docs, AGENTS.md guidance, Graphify integration, and invariant tests protect the architecture while Slice A through Slice D implement and validate the production WGSL runtime.

The initial workspace should contain these crates:

- `alife_core`: engine-agnostic IDs, capacity/phenotype/perception/candidate contracts, genome structures, ExperiencePatch, ActionCommand, memory profiles, and test/developer-only CPU reference helpers.
- `alife_world`: engine-neutral world concepts: ecology, organisms, resources, drives, lesson-world APIs, candidate enumeration, legality, outcomes, and sensory extraction contracts.
- `alife_bevy_adapter`: Bevy-specific app, plugins, rendering, ECS integration, physics adapters, debug UI, and eventual demo scenes.
- `alife_gpu_backend`: the shared `GpuClosedLoopBackend`, generation-checked brain handles, sparse class-bucketed pools, production WGSL pipelines, dispatch scheduling, bounded readback, waking plasticity, and sleep consolidation.
- `alife_runtime`: the single GPU-authoritative session boundary shared by gameplay, foundation training, evolution, challenges, checkpoint capture, and migration.
- `alife_training`: exact-production-graph WGSL gradient training, curricula, evaluation, evolutionary hardening, and foundation promotion receipts. Training shaders and optimizer state never enter normal game binaries or saves.
- `alife_archive`: profile-local cross-save lineage manifests, content-addressed foundation/genome/checkpoint assets, quotas, indexes, secure bundles, and founder resolution.
- `alife_school`: external teacher LLM roles, lesson API, verifier interfaces, curriculum definitions, and in-world teaching object contracts.
- `alife_semantic`: bounded local-model adapters for separate developmental semantic-prior and speech-translation request schemas, including a no-provider ablation implementation.
- `alife_tools`: developer tooling hooks, graph integration helpers, docs validation, and spec consistency checks.

Both `alife_core` and `alife_world` depend on none of Bevy, wgpu, renderer
types, or OS handles. Bevy ECS ownership belongs only to adapter/app layers.
Those layers translate engine-local entities and components into stable IDs and
versioned core/world contracts.

The repo should include `docs/master_spec.md`, `docs/future_research_compatibility.md`, `docs/schooling_and_teacher_architecture.md`, `docs/architecture_decisions.md`, and `docs/codex_handoff_prompt.md`. These docs are not decorative. They are the source of truth for agent work.

Graphify should be treated as a query-first knowledge graph layer over the repository. DOX should be treated as an AGENTS.md discipline: local instructions near every subsystem, updated when meaningful structure changes.

Codex should be instructed to use `/goal` to hold the current implementation-slice target in the active thread. Because the official Codex CLI docs limit `/goal` objectives to 4,000 characters, the `/goal` should be short and point at the docs rather than include the whole spec.


## 4. Runtime Layer Model

The runtime is divided into stable layers. Layer boundaries are more important than exact module names.

Layer A: Host world and ecology. The engine-neutral world layer owns ecology, reproduction, death, lesson-world concepts, unscored candidate enumeration, action legality, and measured outcomes through stable IDs and versioned contracts. Adapter/app layers exclusively own Bevy ECS entities, physics adapters, rendering, engine-local handles, and player interaction. From one authoritative snapshot the host builds current perception and unscored action candidates, validates the GPU-selected structured command, and measures outcomes. The app also schedules brain residency and chooses which organisms are hot, warm, cold, sleeping, or dormant.

Layer B: Core cognitive contracts. This layer is pure Rust and engine-agnostic. It defines IDs, packed ABI structs, capacity classes, phenotype/perception/candidate contracts, lobe ranges, genome structures, ExperiencePatch, ActionCommand, sensory and action ABI versions, and memory profile manifests. It does not execute a production neural tick or import Bevy/wgpu types. Deterministic CPU neural calculations are test-only or developer-only.

Layer C: GPU neural backend. This layer owns the shared wgpu device/queue, bind groups, sparse class-bucketed structure-of-arrays pools, staging buffers, shader modules, compute pipelines, generation-checked handles, and dispatch batches. It is authoritative for production encoding, recurrent neural math, candidate scoring and winner selection, waking plasticity, and sleep consolidation. It remains replaceable behind a narrow backend boundary without introducing a live CPU neural implementation. The world remains authoritative for legality and outcomes.

Layer D: Semantic prior layer. This is an optional internal private system attached to a creature/species/brain class. It produces bounded `LexiconModulationPacket`s from compressed sensory summaries, drives, and limited ExperiencePatch context. It does not act externally.

Layer E: School/teacher layer. This layer controls teacher avatars, curricula, verifiers, lesson APIs, blackboards, books, speech, gesture, demonstrations, praise, penalties, and assessment. It has private planning/evaluation state, but all instructional content perceived by a creature must pass through normal world channels.

Layer F: Tooling and documentation layer. Graphify maps code and docs into a queryable graph. DOX/AGENTS.md provides local agent instructions. CI verifies that architecture invariants and docs references remain coherent.


## 5. Rust Workspace Layout

The repository uses a Cargo workspace. Implement reviewed contracts and runtime algorithms as focused, strongly named, compilable modules in their owning crates. Feature flags may isolate hardware-dependent paths, but empty modules are not architectural endpoints and must not replace required production behavior.

Recommended root layout:

```text
A-Life/
├── Cargo.toml
├── AGENTS.md
├── README.md
├── docs/
│   ├── master_spec.md
│   ├── future_research_compatibility.md
│   ├── schooling_and_teacher_architecture.md
│   ├── architecture_decisions.md
│   └── codex_handoff_prompt.md
├── crates/
│   ├── alife_core/
│   ├── alife_world/
│   ├── alife_gpu_backend/
│   ├── alife_runtime/
│   ├── alife_training/
│   ├── alife_archive/
│   ├── alife_bevy_adapter/
│   ├── alife_school/
│   ├── alife_semantic/
│   └── alife_tools/
├── scripts/
│   ├── setup.sh
│   ├── build.sh
│   ├── test.sh
│   ├── graphify.sh
│   └── docs_check.sh
└── tests/
```

The workspace should not contain Unity project files, `.csproj` files, Unity packages, HLSL production shaders, or dense matrix demo code except in explicitly named `reference_debug` modules.

Core dependencies should be conservative. `alife_core` should use `serde`, `thiserror`, `bytemuck`, `bitflags`, and perhaps `smallvec` only when bounded. `alife_gpu_backend` may use `wgpu`, `pollster`, and shader-packaging helpers. `alife_bevy_adapter` uses Bevy. `alife_school` and `alife_semantic` can define traits without binding to any LLM vendor.


## 6. Bevy World Layer

`alife_world` is the engine-neutral world and legality layer. It imports none of Bevy, wgpu, renderer types, or OS handles. `alife_bevy_adapter` and app layers exclusively own Bevy ECS state, rendering, app lifecycle, plugin scheduling, input, debug visualization, engine-local handles, and physics integration. They translate engine-local entities into stable `WorldEntityId` wrappers before crossing into world or core contracts; world entities are never stored directly inside neural structs.

The Bevy layer should expose systems for spawning organisms, resources, hazards, lesson objects, teacher avatars, blackboards, signs, books, toys, tools, and environmental state. It should also produce sensory packets. Sensory packets are not raw Bevy components; they are packed views according to a versioned sensory ABI.

The world layer owns action legality. A neural backend can propose `ActionCommand`s, but it cannot teleport, eat non-existent objects, reproduce without conditions, or override physics. If the brain proposes an impossible action, the world returns a failure or frustration outcome through ExperiencePatch.

The Bevy adapter eventually needs debug inspectors for brain class, hot/warm/cold residency, neurochemistry, recent ExperiencePatch entries, action arbitration, teacher lesson state, and Graphify/doc links. UI implementation remains a separate reviewed surface and must preserve the module boundaries here.


## 7. Engine-Agnostic Cognitive Core

`alife_core` is the stable contract heart of the project. It must be usable without Bevy, wgpu, renderer types, OS handles, or any LLM. It defines versioned data contracts but does not execute the production neural tick. Pure CPU neural helpers are compiled only for tests or explicit developer tools. If `alife_core` starts depending on Bevy ECS, wgpu resources, rendering, OS/window handles, or a live CPU neural policy, the architecture is drifting.

Core IDs should be newtype wrappers over integers. Packed vectors and quaternions should be defined in core rather than importing Bevy math types. All structs crossing CPU/GPU boundaries need explicit representation strategy. `repr(C)` can be used where appropriate, but the spec should avoid premature bytemuck claims until fields are verified as plain-old-data.

The cognitive core should expose:

- `BrainClassSpec`
- `BrainScaleTier`
- `BrainCapacityClass`
- `BrainPhenotypeManifest`
- `PhenotypeHash`
- `LobeLayout`
- `BrainGenome`
- `EndocrineProfile`
- `SensorProfile`
- `PerceptionFrame`
- `ActionCandidate`
- `CandidateFeatureVector`
- `NeuralActionSelection`
- `NeuromodulatorSample`
- `SleepState`
- `ExperiencePatchHeader`
- `ExperiencePatchView`
- `ActionCommand`
- `SensoryAbiVersion`
- `ActionAbiVersion`
- `SemanticPriorProvider` trait
- `NeuralComputeBackend` trait
- `LineageExportManifest`

The core should include unit tests that assert lobe alignments, legal neuron counts, motor physical stride, brain class invariants, unscored candidate construction, causal perception digests, and action command packing expectations. These tests validate contracts; they do not act as a production neural oracle.


## 8. Scalable Brain Classes

A-Life does not use a globally fixed neuron count. Production brains are assigned to validated `BrainCapacityClass` records. Discrete classes allow efficient batching, stable shader dispatch dimensions, predictable memory planning, bounded candidates, and meaningful performance profiles. `BrainClassSpec`/`BrainScaleTier` remain legacy-save and reference adapters; capacity is not a cognition claim.

Promoted production classes:

- N512: small organisms and the minimum causal/runtime acceptance class.
- N1024: richer small organisms and the intermediate acceptance class.
- N2048: the largest initially promoted production class.

Existing `Large4096`, `Cognitive32768`, `Student131k`, `Ascended1M`, `Ascended5M`, and `ResearchCustom` identifiers remain readable for save inspection, export, and research compatibility. They cannot enter production neural mode until each class passes the documented causal, hardware, save, soak, memory, and performance gates.

Every production neural capacity request above N2048 returns a typed
`ProductionCapacityGateError` before allocation or dispatch. The error carries
`requested_class: BrainClassId` and
`gate_status: ProductionCapacityGateStatus`; `gate_status` is `Unmet` and
identifies the missing causal, hardware, save, soak, memory, and performance
gates. The runtime does not clamp the request to N2048, change policy, allocate
a partial brain, or dispatch until that exact class is promoted.

All classes obey these invariants:

- neuron count is at least 512.
- near-term GPU classes use counts aligned to 128.
- lobe starts and lengths align to 16.
- microtiles are 16x16.
- supertiles are 128x128.
- active-loop resizing is forbidden.
- dispatch batches and sparse structure-of-arrays pools are grouped by class.
- candidate and object-slot ceilings are class-bounded.
- sparse payloads scale with active synapses, not N².

`Standard2048` remains useful for continuity as a legacy/reference adapter, not because it is privileged. Production acceptance compiles nonempty sparse phenotypes for N512, N1024, and N2048 and proves the architecture is neither fixed at 2048 nor silently promoting larger tiers.

`N2048FoundationLayoutV1` is the first trained foundation. N512 and N1024
remain valid promoted runtime capacities but receive separate foundation assets
later; their weights are never derived by truncating N2048. `N4096Research`
exists only as an unpromoted migration target for function-preserving growth
evidence. Capacity-class support, foundation availability, and production
promotion are separate facts and must be reported separately.


## 9. Lobe Layout Generation

A brain class defines a `LobeLayout`. The layout can be produced by a default generator or overridden by genome/species templates within alignment constraints. Lobe ratios are evolutionary variables, but there must be hard safety ranges so organisms do not evolve impossible brains.

Canonical lobes:

- Sensory grounding lobe.
- Metabolic drive lobe.
- Auditory/speech perception lobe.
- Visual glyph / reading lobe.
- Lexicon and concept lobe.
- Core association lobe.
- Episodic memory lobe.
- Working memory / attention lobe.
- Motor arbitration lobe.
- Homeostatic regulation lobe.
- Optional future expansion lobes: language cortex, math/quantity, narrative/history, social reasoning, self-critic, planning/dream, speech/writing motor.

Small ecosystem creatures may merge several of these into compact regions. Ascended classes may allocate separate lobes. The ABI should allow absent lobes by assigning zero length or a disabled flag, not by deleting enum variants.

The motor lobe must distinguish logical motor nodes from physical stride. Logical motor nodes participate in reciprocal inhibition. Physical stride may be padded to a power of two for GPU alignment. The earlier 224-node motor ring becomes a `Standard2048` logical example, not a universal constant.

Evolution may adjust lobe ratios, but curriculum and advanced schooling should be staged. There is no point trying to teach high-level math to a creature whose brain class lacks enough working memory, lexicon capacity, auditory/glyph capacity, and social motivation.

### N2048FoundationLayoutV1

The first trainable layout is frozen at exactly 2,048 neurons:

| Lobe | Neurons | Persistent ordinal range |
|---|---:|---:|
| Sensory | 256 | 0-255 |
| Metabolic | 128 | 0-127 |
| Auditory/Speech | 128 | 0-127 |
| Glyph | 128 | 0-127 |
| Lexicon | 256 | 0-255 |
| Core Association | 448 | 0-447 |
| Episodic | 256 | 0-255 |
| Working Memory | 128 | 0-127 |
| Motor | 224 | 0-223 |
| Homeostatic | 96 | 0-95 |
| **Total** | **2,048** | |

Persistent ordinals are local to a lobe and do not expose packed runtime
offsets. The recurrent route budget is exactly 24,576 synapses:

| Route | Synapses | Lifetime plasticity |
|---|---:|---|
| Sensory -> Core | 3,584 | Slow |
| Auditory -> Core | 1,536 | Slow |
| Glyph -> Core | 1,536 | Slow |
| Metabolic -> Homeostatic | 1,024 | Slow |
| Homeostatic -> Core | 1,024 | Slow |
| Homeostatic -> Motor | 768 | Slow |
| Core -> Motor | 3,072 | 2,048 Slow / 1,024 Fast |
| Motor -> Motor | 1,536 | Slow |
| Core -> Working | 1,536 | Fast |
| Working -> Core | 1,536 | Fast |
| Core -> Episodic | 1,536 | Fast |
| Episodic -> Core | 1,536 | Fast |
| Core -> Lexicon | 1,536 | Fast |
| Lexicon -> Core | 1,536 | Fast |
| Lexicon -> Working | 768 | Fast |
| Working -> Lexicon | 512 | Fast |
| **Total** | **24,576** | |

Action-decoder and memory-decoder budgets are each 4,096 synapses. The action
decoder reserves a bounded recurrent speech payload head used only after the
world's unscored `Vocalize` candidate wins. The speech head never competes with
world candidates or supplies their logits.


## 10. Genome and Developmental Encoding

The genome is a structural controller. It is not merely a random seed for weights. It controls brain scale, lobe ratios, macro-connectome masks, sparse tile density priors, alpha plasticity masks, endocrine constants, drive thresholds, morphology, sensor layout, motor affordances, mutation rates, and developmental schedules.

A genome may encode an internal SLM/semantic-prior capacity gene. This lets species differ in subconscious semantic scaffolding. It also lets advanced or ascended lineages increase semantic prior bandwidth while simple organisms remain purely sensorimotor.

The genome should also encode developmental growth checkpoints. A creature may hatch as a small class and later migrate to a larger class at juvenile/adolescent/adult stages if its species supports it and the simulation profile has available compute. These migrations occur at safe synchronization points, not during active neural dispatch.

Inheritance should support biological and engineered modes. Pure biological mode inherits only genome plus cultural exposure. Practical A-Life mode may allow limited inherited deja-vu, species culture priors, and experimental Lamarckian carryover. This is intentional: the project is interested in smart creatures more than strict biological fidelity. All such inheritance mechanisms must be flagged and ablatable.

The genome should never directly store every learned synapse as heritable default. It should store compressed predispositions, structural tendencies, seed templates, and maybe distillation summaries. This keeps evolution from becoming a hidden supervised checkpoint copy mechanism.

### Curated foundation and Baldwinian inheritance

A `FoundationManifest` binds `FoundationId`, `FoundationVersion`,
`FoundationCompatibilityFamilyId`, capacity class, lobe layout, sensor/action/
language ABIs, persistent-address map, route and plasticity digests,
curriculum/evaluation versions, immutable payload digest, provenance, and a
`FoundationPromotionReceipt`. A `FoundationWeightAssetRef` resolves the
content-addressed immutable payload. Missing or mismatched topology, address
map, class, plasticity, sensory ABI, action ABI, language ABI, or payload digest
fails before GPU allocation.

Every route or section declares a `FoundationSectionPolicy` and
`LifetimePlasticityBand::{Fixed, Slow, Fast}`. Fixed weights never change during
life. Slow bands permit bounded, low-rate individual adaptation while retaining
curated skills. Fast bands support personal working-memory, episodic,
association, and learned-language development. All production dispatches retain
the same effective-weight rule:

The inherited composition identity is
`W_genetic = foundation + compiled genome deltas`; the equations below define
how that immutable genetic payload combines with lifetime state.

```text
W_effective = W_genetic + W_lifetime + alpha * H_fast
W_genetic = foundation + compiled genome deltas
```

At genetic birth, the selected immutable foundation and deterministic compiled
genome deltas produce `W_genetic`. Genetic birth clears lifetime weights, fast
weights, eligibility, semantic and episodic content, and learned lexicon
bindings. The child inherits the foundation identity, compatible neural
structure, endocrine traits, morphology, and sparse genetic deltas, not either
parent's acquired mind.

Cross-foundation mating is allowed only within a declared compatibility family.
Parent genetic values are resolved by persistent logical address, recombined
deterministically, and re-encoded as deltas against the chosen child foundation.
An unmappable address or incompatible ABI makes the offspring nonviable; it
never triggers positional array crossover.

Foundation creation uses two explicit phases. Offline WGSL truncated
backpropagation trains curated sensorimotor, homeostatic, content-neutral memory,
and language mechanics on the exact sparse production graph. Evolutionary
hardening then evaluates memory-empty newborns in the real production runtime
and mutates only causal genetic fields. Personal semantic facts, episodic
content, names, aliases, dialect, teacher-private state, and world bindings are
excluded from foundation promotion.


## 11. Genetics, Evolution, and Population Pressure

Evolution optimizes organisms under ecological, social, metabolic, and curriculum pressure. Fitness is not raw survival alone. Selection should reward survival, novelty handling, learning speed, social cooperation, tool use, memory, communication, teaching ability, emotional stability, and transfer when SLM/teacher support is reduced.

Brain size must cost something. Larger brains consume more BrainATP, require more sleep, develop more slowly, increase reproductive cost, and slow birth rate/population growth. These costs should scale to computational cost so the world ecology mirrors hardware limits. If larger brains are free, evolution will drift toward maximum size and destroy population diversity.

Population profiles should allow many small creatures and fewer large creatures. A 2 GB profile might support many Nano/Small organisms and only a small number of Standard/Large hot brains. A high-memory profile can increase hot-brain count, active synapse budgets, replay depth, and sleep jobs. Ascended classes are single/few-agent modes, not normal ecosystem population modes.

Evolution of language aptitude should be genetic. Genes can affect auditory/lexicon capacity, social reward sensitivity, curiosity, sleep consolidation, plasticity gates, teaching receptivity, and speech/writing morphology. This allows lineages that learn from teachers and peers better over generations.

Cultural transmission should coexist with genetic predisposition. Parents, teachers, and peers can transmit learned behavior through ordinary channels. Species culture may become a stored external memory or world artifact. Experimental inherited deja-vu can bootstrap long evolutionary history into hundreds of generations, but must remain configurable and measurable.


## 12. Neurochemistry and Drive System

Neurochemistry is not flavor. It is the mechanism that makes behavior weighted, conflicted, and animal-like. A creature must weigh hunger, fear, pain, curiosity, social attachment, fatigue, reproduction, and learned goals against each other. Higher reasoning may override fear or hunger, but only through normal arbitration and drive competition. Feelings do not disappear; they are inputs to decision and plasticity.

Core chemicals and drives include BrainATP, hunger, fatigue, cortisol/stress, pain, oxytocin/social bonding, dopamine/reward prediction, curiosity/novelty, temperature stress, reproductive drive, and developmental hormones. The exact list can grow, but the interface must support bounded vectors with versioning.

Chemistry affects:

- neural thresholds,
- lobe learning rates,
- alpha gate modulation,
- action proposal biases,
- attention and salience,
- sleep pressure,
- memory consolidation strength,
- synaptic wear and autophagy,
- developmental migration readiness.

Teacher feedback enters through social/world channels: praise, tone, visible approval, access, reward objects, correction, demonstration, or task success. Hidden reward/plasticity injection is research-only, must be explicitly logged, and cannot satisfy production foundation, grounding, language, or intelligence gates.


## 13. Internal SLM / Semantic Prior Layer

The internal SLM is an optional private subconscious semantic prior. It is enabled/scaled by genome, species template, brain class, or runtime setting. It is not the same as the teacher LLM. It is closer to inherited instinct, proto-language bias, salience scaffolding, and compressed cultural/evolutionary priors.

The internal SLM receives compressed sensory summaries, drive/endocrine state, and limited recent ExperiencePatch summaries. It should not receive raw full brain state by default. It should not have privileged knowledge of future outcomes or hidden teacher answers.

It may produce:

- attention/salience biases,
- Lexicon/Concept lobe modulation,
- memory recall hints,
- weak plasticity modulation,
- indirect influence on motor proposals through normal pathways.

It may not produce:

- direct actions,
- direct world changes,
- direct weight writes,
- direct reward injection,
- privileged teacher intents.

The interface is named `SemanticPriorProvider`, not `SlmHardcodedSystem`. The
production local-model runtime exposes two disjoint request schemas:
`SemanticPriorRequest` and `SpeechTranslationRequest` are separate request
schemas. `NoSemanticPriorProvider` and no-translation modes remain first-class
ablation paths.

`SemanticPriorRequest` produces a bounded, short-lived developmental packet.
Default maximum juvenile gain is 0.20 and packet lifetime is at most 32 ticks.
Unaided probes occur every 64 relevant exposures, fade begins after 128 unaided
exposures, and gain reaches zero after three consecutive probe windows whose
75% lower-confidence bound passes. Novelty may reactivate gain 0.05 for at most
128 ticks with a 1,024-tick cooldown. The packet cannot contain an action,
reward, entity target, desirability score, direct weight delta, or teacher-private
answer.

`SpeechTranslationRequest` maps player surface text into at most 16 canonical,
learned, or explicitly novel tokens, or renders only the raw tokens selected by
a creature. It carries confidence, model identity, assistance status, and
literal/rendered provenance. Unknown concepts stay novel; low confidence is
shown rather than invented away. The SLM never authors creature thought,
action, reward, target, desirability, or hidden comprehension.

The internal semantic prior substitutes for some instincts animals get from deep evolution. It can bootstrap language/salience in ways that would otherwise require impossible evolutionary time. But its influence must remain bounded and testable. If a creature appears intelligent only when the internal SLM is verbose, the learned brain has not internalized the skill.


## 14. External Teacher LLM and Schooling Boundary

The teacher LLM is an external social actor. It does not plug into the internal SLM hooks. It controls teacher avatars, voices, blackboards, books, lesson objects, demonstrations, curricula, and evaluation. It may privately plan and grade. It may not inject hidden semantic vectors into the creature brain.

Teaching content enters through ordinary perception: hearing/speech, vision, visible text, gesture, pointing, objects, demonstrations, social reward, and world outcomes. Written lessons are visible in-world symbols or a simplified glyph sensor during early development. Speech begins as a clean token/phoneme channel if needed, then progresses toward simulated audio as systems mature.

Teacher roles should be separate:

- Tutor: gives lessons and explanations.
- Examiner: administers tests.
- Critic: evaluates behavior and failures.
- Curriculum planner: chooses next lessons.
- Verifier: uses exact tools for math/logic/science checks.
- Translator: helps render creature signals for human debugging without teaching hidden content.
- Storyteller/historian: teaches narrative and causal history.
- Peer tutor: advanced creatures teaching others through normal channels.

Advanced learning must be staged. The school should not attempt abstract subjects before the creature has sufficient grounding, language, working memory, and motivation. A puppy-stage organism gets object labels, social signals, action commands, and simple cause-effect lessons. Advanced math/history/science comes only after ascended or expanded brains can contextualize it.


## 15. ExperiencePatch Contract

ExperiencePatch is the sealed causal transaction between world and brain. It must be scalable by capacity class and sensory ABI. Earlier fixed arrays like `[f32; 256]` are not acceptable as universal runtime contracts. The decision record binds the same-tick world snapshot, unscored candidates, GPU selection, executed command, measured outcome, and post-outcome credit so no learning update can escape the reviewed causal loop.

Use headers and offset/length references for packed logs. Debug views may use slices. The contract should capture:

- creature ID,
- brain class ID,
- sensory ABI version,
- action ABI version,
- sensor profile,
- tick/time,
- pre-action world state summary and `PerceptionBaseDigest`,
- complete ordered GPU input and `PerceptionFrameDigest`,
- sensory packet offsets,
- drive/endocrine values,
- bounded memory/context references and retrieval-context digest,
- unscored decision candidates from the same world snapshot,
- selected candidate index, logit, confidence, and bounded GPU diagnostics,
- selected ActionCommand,
- outcome deltas,
- reward/frustration/success signals,
- neuromodulator/credit packet identity,
- teacher/school episode references when applicable.

ExperiencePatch supports patch-gated GPU waking plasticity, debugging, sleep replay, curriculum assessment, and lineage export. It must record whether any hidden bootstrapping feedback was used. Clean-learning claims require logs that show what the creature actually perceived. Invalid, duplicate, replayed, non-finite, or mismatched learning batches never commit.

The tri-phase structure remains:

1. Pre-action snapshot.
2. Decision snapshot.
3. Post-action outcome.

This should not be confused with the four-pass GPU neural pipeline.


## 16. ActionCommand Contract

`ActionCommand` is the output of motor arbitration and post-processing. It must be expressive enough for real gameplay and schooling. A one-byte winner token is too weak.

Required fields:

- action ID,
- optional target entity ID,
- target position,
- intensity,
- duration ticks,
- confidence,
- drive-source mask,
- optional teacher/lesson response channel,
- optional speech/writing motor payload reference for advanced creatures.

The CPU world layer validates actions. Impossible actions produce failure outcomes, not silent success. If an action is blocked because of grounding or affordance mismatch, the outcome should carry an interpretable reason so the brain can learn.

Motor arbitration may produce compact GPU buffers internally, but the host-facing contract remains structured. This lets different brain sizes and motor ring layouts route into the same action ABI.


## 17. Sensory ABI

The sensory ABI is the stable contract that lets brains scale and migrate. It defines channel IDs, units, ranges, timing, spatial semantics, and optional channel groups. A creature migrated to a larger brain must still perceive core food, danger, social signals, body state, and teacher speech consistently.

Core sensory groups:

- visual affordance fields,
- spatial/proximity fields,
- tactile/contact fields,
- auditory/speech fields,
- smell/chemistry fields,
- internal drive fields,
- social/emotional signals,
- written/glyph fields,
- lesson/environment markers.

Early implementations can use clean symbolic sensory channels as developmental scaffolding, but the spec must record them as perception channels, not hidden truth injection. Later systems can replace clean channels with more raw sensors while preserving semantic ABI versions.

ETF/codebook anchors are versioned. They are useful for sensory invariance and migration. They are not a guarantee that all internal representations are frozen. The association brain may adapt; the sensory ABI should remain stable enough that adaptation does not scramble the world.


## 18. Hearing, Speech, and Writing ABI

Language must be grounded through perception. Teacher and peer speech uses the same hearing pathway as creature speech. Written language uses visible in-world symbols, simplified glyph sensors, or eventually rendered text. The transition can be developmental: clean token/glyph stream first for tests, then lower-level phoneme/audio/glyph perception later.

### LanguageCodebookV1

`LanguageCodebookV1` has 256 stable logical codes independent of neuron indices.
Token IDs never identify neurons or packed GPU offsets. Code 0 is the
silence/unknown sentinel; 1-128 are canonical compositional vocabulary; 129-192
are learned aliases and creature dialect; 193-224 are names and social
bindings; 225-255 are reserved experimental expansion. The canonical range
allocates 24 verbs/actions, 64 ecological/category nouns, 16 drives/internal
states, 16 modifiers/spatial relations, and 8 grammatical/query/social
operators.

The default bounded grammar is:

```text
[addressee] [subject] [modifier] [verb/state] [modifier] [object/target]
```

Heard utterances contain at most 16 tokens and creature-generated utterances at
most six. Localized surface words do not change token IDs. The codebook defines
pronounceable symbols, sequence roles, and protocol scaffolding; it does not
inherit ecological noun meanings, action meanings, personal names, aliases, or
dialects. Those bindings are learned through visible objects, normal hearing,
demonstrated actions, creature actions, and sealed outcomes. Operators such as
`what`, `why`, `self`, `yes`, and `no` may receive content-neutral inherited
protocol scaffolding.

Versioned contracts include `LanguageCodebookId`, `LanguageTokenId`,
`LanguageTokenClass`, `SpeechActKind`, `UtteranceId`,
`UtteranceSourceKind::{Player, Creature, Teacher}`, `PlayerUtterance`,
`AudibleUtterance`, `SpeechMotorPayload`, `CreatureUtteranceReceipt`,
`SpeechTranslationReceipt`, and `LanguageGroundingLedger`. A `HeardToken`
records utterance identity, sequence position, source kind, optional addressee,
spatial origin, confidence, and perception-only semantics.

### Spatial player speech

Player speech is spatial perception, never a direct command channel. Text is
tokenized locally or through the bounded translation schema, then emitted as an
`AudibleUtterance` at the player Hand. A leading recognized name supplies an
optional addressee. Named messages reach that creature only if it can hear the
source; unnamed messages reach every attentive creature in range. Distance,
noise, hearing morphology, and attention determine confidence. Unknown words
remain novel token records instead of becoming hidden concepts. Correctly
hearing a request neither creates a scored candidate nor forces compliance.

### Authentic neural speech and self-narration

`Vocalize` is an unscored world opportunity whose speech act and token payload
are selected by the GPU brain. The opportunity exists only when cooldown and
energy rules permit. If it wins normal candidate arbitration, the recurrent
speech payload head selects a speech act and up to six tokens. The world then
validates the payload, charges a small metabolic cost, emits the literal raw
utterance, and records social consequences in the sealed outcome.

Nearby creatures hear exactly those raw tokens. The player sees a literal
bubble or an optional uncertain translation. The host never reads internal
neural state and invents a narration sentence. Default spontaneous speech is
driven by meaningful learned goal, action, or dominant-drive transitions,
responses to `what`, `why`, or `express`, and occasional learned social speech.
Spontaneous cooldown is 32 ticks and prompted response minimum interval is 8
ticks.

Creature-to-player translation consumes only the raw selected token sequence
and previously grounded ledger associations. Other creatures always receive
raw tokens, never rendered prose. Developer evidence preserves literal tokens,
translation, confidence, assistance status, source, hearing range, and model
identity.

The speech ABI should support:

- utterance ID, source kind, sequence position, and optional addressee,
- speaker ID,
- direction/source location,
- confidence/noise,
- phoneme or clean token stream depending on developmental mode,
- prosody/emotional tone,
- social context,
- optional translated debug text for humans.

The writing ABI should support:

- text object ID,
- glyph positions,
- reading order,
- visual confidence,
- language/script ID,
- links to world objects when physically labelled.

The teacher may write on boards, label objects, create books, draw maps, or arrange symbols. These are world objects. The creature reads them through perception. Hidden token-to-lexicon injection is forbidden for the teacher. Player teaching, teacher nursery lessons, and peer teaching use the same audible/visible world pathway and sealed outcome ledger.


## 19. Action ABI and Motor Arbitration

Action ABI remains stable across brain classes. Internal motor rings can scale. The world still receives structured actions. Logical motor nodes and physical padded stride are distinct.

For a class with `motor_logical_nodes`, reciprocal inhibition uses modulo wrapping over logical nodes. If the physical buffer stride is a power of two, bitwise masking may be used only for physical addressing, not for logical competition. This prevents the old 224-vs-256 bug.

Advanced/ascended creatures may have speech and writing motor systems. These are not special hidden channels. Speaking emits audible tokens/phonemes into the world. Writing creates visible glyph objects or modifies a writing surface. Teaching another creature uses the same action ABI as being taught.

Reasoning can override instinct only through arbitration. A creature may run toward danger to get food or protect a mate if the combined drives, learned values, and social motivations outweigh fear. Fear remains present. The motor system should represent conflict rather than deleting instinctive drives.


## 20. Memory Architecture

A-Life uses multiple memory forms. Synaptic memory stores habits and associations. Episodic memory stores events. Semantic prior memory supplies internal scaffolding. External world artifacts store culture. School records store curriculum progress. Save files store lineage.

Core memory layers:

- `W_genetic`: inherited immutable priors (`W_genetic_fixed` in legacy records).
- `W_lifetime`: durable lifetime learning promoted during sleep (`W_consolidated_habit` in legacy records).
- `H_fast`: immediately active eligibility-gated waking plasticity.
- optional audit/rollback journals: diagnostics only, never waking authority.
- episodic ledger: ExperiencePatch summaries and salient episodes.
- concept ledger: compressed conceptual/memory nodes.
- external artifacts: books, signs, maps, cultural records.
- school mastery ledger: concept-level test outcomes.

Every production dispatch uses `W_effective = W_genetic + W_lifetime + alpha * H_fast`. Candidate-conditional episodic retrieval is attached to the same-tick perception frame. Automatic GPU sleep consolidation promotes bounded fast content into lifetime weights and may prepare a safe double-buffered structural swap; no CPU H-shadow is the consolidation authority.

Memory must be ablatable. To prove grounded learning, tests should turn off teacher hints, reduce or disable internal SLM, test novel speakers/material, and test delayed recall after sleep.


## 21. Sparse Tensor Storage Model

Runtime storage is sparse and class-bucketed. Dense N² matrices are only conceptual and should not appear in production allocation plans.

For each promoted N512, N1024, or N2048 `BrainCapacityClass` bucket, allocate shared structure-of-arrays pools for:

- activation ping/pong buffers sized O(N * slots),
- accumulator buffers sized O(N * slots),
- unscored same-tick candidate descriptors and candidate feature vectors,
- compact candidate logits, winner output, and diagnostic buffers,
- sparse payload pools for genetic weights,
- sparse payload pools for lifetime weights,
- sparse payload pools for alpha masks,
- sparse payload pools for immediately active fast weights,
- eligibility traces and post-outcome credit packets,
- optional audit/rollback journals,
- sparse payload pools for wear/autophagy counters,
- microtile metadata buffers,
- supertile masks,
- replay/event buffers.

Genetics controls sparse structure. Evolution mutates topology and density. The GPU computes active paths, not dead weight lines. Memory budgets cap active synapses, tiles, candidates, object slots, and in-flight growth/swap storage per capacity class and profile. A lightweight `GpuBrainHandle` references shared backend-owned pools; it never duplicates device, queue, pipelines, or a complete projection schema per creature.

The data model should support future payload compression, quantization, and tile format upgrades without changing high-level contracts.


## 22. GPU Memory Profiles and Residency

The minimum supported hardware class is 2 GB GPU memory, but 2 GB is not the full-fidelity target. The engine scales with available compute.

Memory profiles define:

- total GPU budget,
- renderer reserve,
- neural heap,
- scratch heap,
- replay heap,
- sensory cache,
- semantic cache,
- hot brain slots by class,
- warm slots by class,
- active synapse budgets,
- sleep compaction jobs,
- migration jobs.

Residency states:

- HotGpu60Hz.
- WarmGpuTimeSliced.
- ColdHostCompressed.
- SleepCompactionGpu.
- DormantDiskBacked.

The scheduler chooses residency based on salience, distance, social relevance, player focus, ecological importance, hunger/danger/reproduction urgency, and schooling status. Larger brains slow population growth by consuming more compute and metabolic resources.


## 23. Brain Migration and Ascension Compatibility

Ascension is future-only now but must be architecturally possible. A favorite evolved creature can eventually be exported and migrated into a larger brain class. The migration process preserves identity while adding capacity.

Every compiled phenotype publishes a canonical stable address map containing
`PersistentNeuronAddress { lobe, ordinal }`, `PersistentProjectionAddress`,
`PersistentSynapseAddress`, and persistent decoder addresses. Logical addresses
are independent of sparse-pool ordering, tile packing, and runtime slot.
The serialized map and compatibility metadata use a BLAKE3-256 digest. Packed
GPU offsets remain runtime-local.

Checkpoint intent is explicit:

`GeneticRebuild`, `DurableLearnedFounder`, and `ExactResume` are separate,
non-interchangeable persistence contracts.

- `GeneticRebuild`: foundation identity plus genome and genetic deltas only.
- `DurableLearnedFounder`: consolidated lifetime weights, durable semantic and
  episodic state, learned language/dialect, personality-relevant durable state,
  and provenance.
- `ExactResume`: all mutable GPU state needed for same-save continuation,
  including active buffer selectors, fast weights, eligibility, homeostasis,
  sleep phase/cycle, bounded replay state, and deterministic RNG state.

Founder restore is not exact resume. A learned founder receives a healthy new
body and local identity. It retains consolidated learning and durable language,
but clears activations, eligibility, working memory, current conversations,
current targets, injuries, age, and world-local entity/social bindings.

Migration inputs:

- genome,
- brain class,
- sensory ABI version,
- action ABI version,
- `W_genetic` sparse payloads (or migrated legacy `W_genetic_fixed` assets),
- `W_lifetime` sparse payloads (or migrated legacy consolidated-habit assets),
- bounded `H_fast` checkpoint state according to the save policy,
- endocrine baseline,
- morphology/action map,
- memory/concept ledger,
- lineage metadata.

Function-preserving N2048-to-N4096 research migration doubles each lobe by
appending neurons after its preserved prefix. `N4096Research` has 4,096
neurons, 49,152 recurrent synapses, 8,192 action-decoder synapses, and 8,192
memory-decoder synapses while preserving language token IDs and speech ABI. It
remains research-only.

Migration process:

1. Suspend production dispatch at a sealed boundary.
2. Finish pending consolidation exactly once and persist an immutable rollback checkpoint.
3. Compile the target through the versioned migration compiler without changing the active handle.
4. Map genetic, lifetime, fast, activation, eligibility, memory, language, and RNG state by persistent address.
5. Preserve old synapse accumulation order and all stable sensory/action/language ABI mappings.
6. Initialize appended neurons and bridge routes dormant.
7. Replay source and target on the same adapter without emitting world actions.
8. Require identical selected candidates and maximum logit delta at most `1e-6`.
9. Atomically replace the active handle only after every gate passes, otherwise restore the source.
10. Resume production with one active target brain; retain the source as an offline rollback artifact.

Offline deterministic replay and fixture validation exercise the migrated brain
without producing world actions. The production handoff is atomic, and old and
migrated neural brains never run concurrently in production.

A migrated creature should still feel like the same lineage at first. Expanded cortex can later influence behavior by weighing drives, feelings, memories, and goals through normal arbitration.


## 24. Learning Rules: Eligibility, Three-Factor Credit, and Oja Normalization

Production waking plasticity is a GPU three-factor update gated by sealed `ExperiencePatch` evidence. Recurrent and decoder activity accumulate candidate/action-specific eligibility:

```text
e_ij(t) = lambda_e * e_ij(t-1) + F(pre_i, post_j, selected_candidate)
```

After the world executes the selected structured command and the patch is sealed, a bounded neuromodulator combines reward prediction error, pain, frustration, novelty, homeostatic improvement, and the developmental receptor profile:

```text
delta H_fast = eta * alpha * M(t) * e_ij
             - eta_norm * post_j^2 * W_effective
```

The first term supplies target-specific behavioral credit and the Oja term supplies normalization. Oja alone is not the behavioral credit signal. Anti-Hebbian inhibition, homeostatic scaling, novelty modulation, and other terms must remain explicit and ablatable.

The internal SLM can weakly modulate attention/plasticity, but teacher evaluation cannot directly rewrite weights except under logged bootstrapping experiments. Exact verifiers grade math/logic/science; learning still happens through the creature's own sensory/action/outcome loop.


## 25. Weight Decomposition

The production effective-weight formula is:

```text
W_effective = W_genetic + W_lifetime + alpha * H_fast
```

`W_genetic` is inherited and immutable during an organism's lifetime. `W_lifetime` is durable learned habit. `H_fast` is immediately active, so a sealed outcome can affect the next neural dispatch before sleep. A shadow or higher-precision journal may exist only for audit, rollback, or rounding; it is never the sole waking learning target or consolidation authority.

Automatic GPU sleep consolidation may promote bounded stable fast traces into `W_lifetime`. It must not silently bake lifetime learning into `W_genetic` unless an explicit experimental Lamarckian/species-prior mode is enabled. Offspring may inherit genetic predispositions, species culture, limited deja-vu priors, or experimental distillations depending on settings.

This separation is central to evolution. Genes optimize brain structure and priors; experience optimizes lifetime behavior; school/culture provides curriculum.


## 26. GPU Compute Pipeline

The production neural tick is one GPU-authoritative multi-pass causal loop:

1. Gather and validate current perception and unscored candidates from one world snapshot.
2. Upload bounded sensory, body, homeostatic, episodic, and candidate records.
3. Encode inputs and clamp them into compiled populations.
4. Run two through four deterministic recurrent microsteps over active sparse routes.
5. Encode every candidate and decode a candidate-conditioned neural logit.
6. Apply motor lateral inhibition and deterministic GPU winner selection.
7. Read back only the selected candidate index, logit, confidence, and bounded counters.
8. Let the world validate and execute the structured command, then seal the outcome patch.
9. Upload the compact post-outcome credit packet and update eligibility-gated `H_fast`.
10. During canonical sleep phases, run GPU replay, consolidation, and safe structural swap pipelines.

Passes may later be fused after correctness and profiling, but reviewed boundaries must preserve WebGPU safety and causal auditability. The GPU backend batches by promoted capacity class. A dispatch batch contains class ID, slot/generation, active tile range, perception/candidate offsets, activation buffers, sparse payload references, and bounded output offsets.

The host may read the compact winner record and bounded counters each tick. Active play never synchronously reads bulk activation, eligibility, topology, or weight state. Production does not dispatch a CPU shadow, require CPU parity, or fall back automatically to CPU neural math.


## 27. WGSL Authoring Rules

All production shaders are WGSL. Do not create HLSL source files unless explicitly labelled as non-authoritative pseudocode. WGSL modules should be small and testable.

Production modules include:

- `clear_accumulators.wgsl`
- `sensory_encode.wgsl`
- `spmv_projection.wgsl`
- `activation_finalize.wgsl`
- `candidate_encode.wgsl`
- `candidate_decode.wgsl`
- `winner_select.wgsl`
- `fast_plasticity.wgsl`
- `wear_autophagy.wgsl`
- `sleep_consolidation.wgsl`

WebGPU limitations are design constraints. Floating-point atomics should not be assumed. Use scaled integer accumulators where needed. Subgroup features should have portable fallbacks. Workgroup sizes and buffer layouts must be explicit.

Any future CUDA/Triton/TPU backend must implement the same closed-loop contract and must not change organism semantics. Pure CPU equivalents may exist only under test/developer compilation for fixtures and contract checks, never as production dispatch or fallback.


## 28. Sleep, Replay, and Consolidation

Sleep is not a decorative animation or an external harness operation. The canonical scheduler advances `Awake -> EnteringSleep -> Consolidating -> Waking -> Awake`, with forced recovery joining the same path. Entering sleep emits no action. The transition into `Consolidating` submits exactly one GPU consolidation job for a persisted unique cycle ID, and `Waking` restores bounded homeostatic state before actions resume.

Sleep jobs:

- replay salient ExperiencePatches through bounded replay eligibility,
- promote bounded `H_fast` content into `W_lifetime`,
- preserve `W_genetic` immutability,
- prune fatigued low-salience pathways,
- update concept/episodic ledgers,
- test memory stability,
- prepare a double-buffered structural swap or developmental migration if scheduled.

Replay payloads must measurably affect the staged consolidation result; metadata alone is insufficient. Phase and cycle state are saved so interruption, retry, and load cannot consolidate twice. Tests cover every sleep phase, exactly-once consolidation, automatic wake, no-action phases, interruption, and retained post-wake behavior.

Future dream/planning modes can run internal counterfactual simulations. They are not v0/v1 requirements. If implemented later, dream actions must not mutate world state until committed through normal action pathways.

Schooling uses sleep for consolidation. Lessons should have delayed tests after sleep, not just immediate teacher-on performance.


## 29. Topological Memory and Curiosity

Topological memory is a future-compatible representation layer. Čech/Morse terminology from earlier drafts should be interpreted as a design direction: stable concept neighborhoods, gaps/contradictions, and curiosity pressure. It is not a requirement to implement full computational topology in v0.

Near-term interface:

- concept IDs,
- memory IDs,
- similarity metrics,
- contradiction/gap records,
- curiosity salience,
- episodic clusters,
- consolidation candidates.

Future implementation may use nerve complexes, discrete Morse summaries, graph structures, or learned embeddings. The interface should not require one mathematical implementation now.


## 30. Graphify Integration

Graphify should be integrated as a developer/agent aid. Its purpose is to build a queryable knowledge graph over code, docs, diagrams, and future assets so Codex can query project architecture instead of grepping blindly.

The spec should ask Codex to add `scripts/graphify.sh` and document manual installation. Based on the current Graphify README, the PyPI package is `graphifyy`, while the CLI command remains `graphify`. It supports Codex and can install project-scoped guidance with `graphify install --project --platform codex`. It also notes that Codex uses `$graphify` rather than `/graphify`.

Graphify should not become a runtime game dependency. It is a development dependency only. Generated `graphify-out/` should likely be ignored by default unless specific reports are intentionally committed.

Recommended workflow:

```sh
uv tool install graphifyy
graphify install --project --platform codex
graphify .
```

Then ask Codex to prefer graph queries for architecture questions.


## 31. DOX / AGENTS.md Hierarchy

DOX should be used as an AGENTS.md discipline. It is not a package or runtime dependency. It is a pattern: root AGENTS.md contains project-wide instructions and a top-level index; child AGENTS.md files contain local instructions; agents read the hierarchy before editing; meaningful changes update the relevant instructions.

The root AGENTS.md should state:

- Rust + Bevy + wgpu/WebGPU + WGSL only.
- No Unity.
- No HLSL production source.
- 2048 is only `Standard2048`.
- Use scalable brain classes.
- Production neural execution is GPU-authoritative WGSL; do not add a live CPU shadow, parity gate, or automatic CPU neural fallback.
- Keep pure CPU neural helpers test-only or developer-only.
- World code enumerates unscored candidates and remains authoritative for legality and outcomes.
- Promote only N512, N1024, and N2048 until larger tiers pass the documented causal and performance gates.
- Teacher LLM and internal SLM are separate.
- Use Graphify for architecture queries when installed.
- Keep docs synchronized.

Child AGENTS.md files should exist in `crates/alife_core`, `crates/alife_gpu_backend`, `crates/alife_school`, `crates/alife_semantic`, and `docs`.


## 32. Codex Operating Rules

Use `/goal` to set a persistent objective for the task. Because official Codex docs state `/goal` objectives must be non-empty and no more than 4,000 characters, use a compact goal and point at `docs/codex_handoff_prompt.md` for details.

Codex should inspect existing files, report divergences, and implement the approved integration slice. Production cognition uses reviewed GPU-authoritative WGSL pipelines; do not add a live CPU shadow, parity gate, or automatic CPU neural fallback. Keep pure CPU neural helpers test-only or developer-only, keep world candidate enumeration unscored, and preserve world authority over legality and outcomes.

Codex should make small commits if working in a repo. It should run `cargo fmt`, `cargo check --workspace`, and any docs checks. It should not install random dependencies without explaining why.

Codex should initialize DOX hierarchy and add Graphify instructions. It should not require Graphify to be installed for the project to build.


## 33. Testing and Validation Strategy

Validation combines contract tests with behavioral, hardware, save, soak, and performance evidence. Contract tests should assert:

- brain classes have legal neuron counts,
- lobe ranges are aligned and non-overlapping,
- motor logical nodes and physical stride obey rules,
- `Standard2048` reproduces the old reference layout only as a class,
- action command structs are stable,
- ExperiencePatch headers use offsets rather than fixed sensory arrays,
- genome fields include topology, chemistry, morphology, plasticity, and development,
- `NoSemanticPriorProvider` exists,
- teacher interfaces do not use internal SLM hooks.
- production capacity promotion is limited to N512, N1024, and N2048,
- candidates are unscored and share the perception snapshot,
- the production source and telemetry contain no CPU neural shadow, parity gate, or automatic neural fallback.

Behavioral and causal tests perturb sensory input, lesion or zero weights, ablate neuromodulation, and verify target-conditional learning rather than comparing against a CPU oracle. Real-hardware tests name the Vulkan adapter and prove action selection, waking plasticity, and sleep consolidation dispatch through WGSL. Save tests cover every sleep phase and typed GPU-unavailable state. Bounded 10,000-plus-tick soak tests and populated-phenotype performance reports complete acceptance.


## 34. Determinism and Reproducibility

A-Life needs deterministic seeds for credible evolution and debugging. Organism birth, genome mutation, phenotype compilation, tile initialization, candidate ordering, lesson generation, replay selection, and stochastic rounding should be seeded and reproducible where possible.

GPU replay determinism is defined for the same phenotype hash, ordered inputs, seed, backend version, adapter class, and tolerance contract. CPU reference helpers may generate fixtures or check isolated math in tests/developer tools, but they are not a live parity oracle or production acceptance gate. Cross-vendor bitwise identity is not claimed without evidence.

All experiments involving hidden bootstrapping, teacher feedback, SLM support, or inherited deja-vu must be logged. Reproducing a creature requires genome, seed lineage, brain class, ABI versions, profile settings, and migration history.


## 35. Performance and Profiling Plan

Performance scales through sparsity, residency, promoted capacity-class batching, and profile caps. Reports use populated phenotypes and real production dispatches rather than empty schemas, CPU fallback data, or inferred GPU claims. Required metrics include:

- hot-brain tick throughput,
- SpMV tiles per second,
- active synapses per class,
- memory bandwidth,
- action latency,
- candidate count and winner readback bytes,
- waking-plasticity dispatch time,
- sleep jobs per second,
- replay throughput,
- committed slot bytes, physical bucket bytes, unused capacity, shared backend bytes, and peak growth/swap bytes,
- schooling lessons per simulated hour,
- Graphify/doc agent overhead.

N512, N1024, and N2048 must each meet their documented causal and performance gates before promotion. Larger brains remain research-gated special modes. A single 1M-neuron school candidate is not expected to coexist with hundreds of hot ecosystem brains on low hardware.


## 36. Data Persistence, Saves, and Lineage Export

Saves must be versioned. They should include world state, organisms, genomes, brain class specs, ABI versions, memory snapshots, consolidated habits, episodic summaries, school progress, and lineage metadata.

Lineage export supports ascension. It should store enough to recreate the creature later, migrate it to a larger class, and test whether its identity survived migration.

The profile-local lineage library is durable across saves:

```text
platform-local-data/A-Life/lineage-library/
|-- lineage.db
|-- manifests/
|-- assets/<digest>/
|-- checkpoints/<digest>/
`-- staging/
```

Every creature receives an immutable genetic archive before GPU insertion. The
canonical manifest and all referenced genome/foundation assets are copied into
the profile store before the runtime allocates a neural slot. The bundled SQLite
index is rebuildable from immutable manifests and content-addressed assets;
SQLite is not the sole source of truth.

Learned checkpoints use compressed, content-addressed 64 KiB pages. Genetic
archives are retained indefinitely. The default full-state quota is 4 GiB, with
up to 64 temporary peak candidates and 24 automatic permanent learned
checkpoints per run plus user pins. Pinned checkpoints are never auto-evicted.
Quota pressure visibly downgrades an automatic learned capture to genetic-only;
it never silently discards the genetic archive.

Death ordering is transactional:

```text
DeathDetected
-> Quiescent
-> seal final outcome and statistics
-> commit life archive
-> capture selected learned checkpoint
-> obtain retirement receipt
-> scrub GPU handle
-> despawn
-> Dead
```

A dying creature is archived before its GPU handle is scrubbed or its world
entity despawns. Crash recovery can complete or roll back staged content without
publishing a partial manifest.

Founder modes are `GeneticFounder`, `MindStateClone`, and `GeneticOffspring`.
Genetic founder is the default and restores only the inherited foundation/
genome boundary. A chosen checkpoint-capable elite may become a mind-state
clone with a healthy new body, remapped local IDs, and cleared transient or
world-local state. Genetic offspring applies a deterministic mutation seed.
New-save creation records all source runs, archives, checkpoint IDs, remaps, and
founder provenance before validating the complete save.

Portable `.alife-creature` and `.alife-cohort` bundles reject traversal,
oversized entries, duplicate paths, missing assets, digest mismatch,
unsupported class/ABI/codebook/foundation identities, and partial extraction.
Genetic founders lose acquired vocabulary and meanings. Mind-state clones retain
durable learned language and dialect but clear current conversations and working
memory.

A `LineageExportManifest` should include:

- schema version,
- creature/species IDs,
- genome hash,
- brain class ID,
- sensory ABI version,
- action ABI version,
- chemistry profile version,
- memory snapshot references,
- export tick,
- parent lineage references,
- flags for inherited deja-vu or Lamarckian modes.
- foundation ID/version/compatibility family and payload digest,
- persistent-address-map and language-codebook digests,
- checkpoint mode and content-addressed page references when present,
- source run, life statistics, ranking/evaluation versions, and founder provenance.

Passive statistics update in O(1) during ordinary simulation and use `Unknown`
when the creature had no relevant exposure. They cover survival regimes, food,
poison, hazards, energy, movement, reproduction, sleep retention, learning
slope, reversal recovery, vocabulary grounding, unaided versus SLM-assisted
comprehension, narration fidelity, peer communication, dialect divergence, and
GPU dispatch/throttle counts. A bounded active battery grades navigation,
detours, dangerous-short versus safe-long routes, reversals, delayed choice,
unfamiliar edibility, sleep retention, generalization, recovery, named
instruction, word grounding, narration, peer aliases, and SLM-disabled dialect
transfer. Selection is Pareto/category based; no default kinship penalty is
applied, while ancestry and genome distance remain visible.


## 37. Schooling and Curriculum Interfaces

The master spec defines boundaries; detailed schooling lives in `docs/schooling_and_teacher_architecture.md`. Schooling is staged by utility and reasoning capacity.

The perception-only language nursery is the canonical route for vocabulary
grounding. It presents visible objects, demonstrates actions, speaks through
normal spatial hearing, and supplies visible/social/world feedback whose
consequences are sealed in ordinary experience patches. It has no lexicon-vector
write, token-meaning injection, scored candidate, direct reward, or teacher
action bypass. Player teaching and peer teaching use this same path.

Foundation speech training randomizes surface forms and token assignments so it
learns discrimination, name attention, turn taking, grammatical roles, speech
sequencing, vocal mechanics, and content-neutral state-to-role binding rather
than memorizing English meanings. Live vocabulary grounding begins with empty
personal bindings. Language curriculum and evaluation include SLM-disabled
tests, novel surface words, token permutations, novel speakers, delayed recall,
and peer transfer; assisted and unaided results are never merged.

Stages:

1. Preschool grounding: objects, colors, food, danger, simple commands.
2. Language bootstrapping: roles, negation, sequence, requests, social phrases.
3. Writing: reports, descriptions, explanations.
4. Math: manipulable quantities first, symbols later, exact verifiers always.
5. History: simulated world history first, real human history later.
6. Science: experiments, measurement, prediction, repeatability.
7. Social reasoning: promises, obligations, trust, teaching others.
8. Independent exams: teacher off, internal SLM reduced/off, novel environment, delayed recall.

Teacher LLM roles use world-authorized lesson APIs to spawn/arrange objects. The private evaluator can grade and select lessons. Direct hidden reward/plasticity injection is research-only and is never accepted as clean curriculum or promotion evidence.


## 38. Future Compatibility Envelope

The architecture must remain compatible with broad future ideas:

- ascended companion mode,
- larger single-agent brains,
- founder cohorts,
- school worlds,
- peer teaching,
- lineage export/import,
- distributed backend abstraction,
- richer semantic priors,
- dream/planning phases,
- topological memory,
- curriculum acceleration,
- exact verifiers,
- future research-scale experiments.

Compatibility does not imply production promotion. Initial production stays focused on the reviewed N512/N1024/N2048 GPU-authoritative closed loop, core contracts, genetics/chemistry/evolution data models, and clean module boundaries. Larger tiers and alternate compute backends remain research-gated.

Speculative AGI language belongs only in non-requirements appendix text. Formal documents should use “grounded developmental generalist agent research direction.”


## 39. Non-Goals for v0/v1

Do not build:

- Unity integration.
- HLSL production kernels.
- CUDA/Triton/TPU backends.
- 1M-neuron runtime.
- D2NWG training.
- full topological Morse implementation.
- AGI claims.
- arbitrary dynamic brain resizing.
- fully open-ended natural language in N2048.
- production promotion of `N4096Research`.
- live CPU neural shadow execution.
- CPU-parity-gated production handoff.
- automatic CPU neural fallback.
- promotion of capacity classes larger than N2048 without documented gates.

Do build:

- production GPU encoding, recurrent, candidate-decoding, winner-selection, waking-plasticity, and sleep-consolidation pipelines,
- N512/N1024/N2048 sparse class-bucketed pools and populated phenotypes,
- same-tick perception with unscored candidates,
- sealed patch-gated learning and automatic sleep,
- contract, behavioral, hardware, save, soak, and performance tests,
- synchronized docs and AGENTS.md/DOX guidance.


## 40. Implementation Milestones

Historical scaffold Milestones 0 through 6 are complete provenance only. Their scaffold-only, CPU-reference-authority, and placeholder-backend restrictions are superseded by ADR-024 and do not control current production work.

Slice A: GPU causal core. Add capacity/perception/candidate/phenotype contracts, deterministic phenotype compilation, class-bucketed sparse pools, sensory encoding, recurrent microsteps, candidate-conditioned decoding, GPU winner selection, explicit policy selection, and live-bridge cutover. Prove N512 and N1024 before N2048.

Slice B: GPU causal learning and sleep. Add eligibility buffers, three-factor fast plasticity, immediate behavioral effect, canonical automatic sleep, GPU consolidation, save/load phase state, and safe structural swaps.

Slice C: memory, topology, and grounding. Add candidate-conditional memory, nonfatal topology, explicit privileged and grounded sensor profiles, tracked-object bindings, and a 10,000-plus-tick bounded cognition soak.

Slice D: scaling and cleanup. Enforce global/per-route budgets, activity-dependent BrainATP, memory ceilings, tier promotion gates, legacy save migration, documentation cleanup, and removal of superseded backend code and claims.

N2048 Foundation and Lineage Program: after Slice A evidence and the remaining
B/C/D runtime gates, freeze `N2048FoundationLayoutV1`; load immutable foundation
assets at birth; share one checkpointable GPU runtime across game and labs; add
spatial hearing and GPU neural speech; train staged survival/language mechanics
with offline WGSL gradients; harden them through production evolution; add
fading SLM priors and bounded translation; archive every lineage; rank passive
and active performance across runs; support secure founder bundles and new-save
cohorts; add player conversation and lineage UI; and prove unpromoted,
function-preserving N2048-to-N4096 research growth.

Each slice must pass its behavioral and architectural gates before integration. Completion requires the final requirement-by-requirement audit across all four slices.


## 41. Required Files and Modules

Required docs:

- `docs/master_spec.md`
- `docs/future_research_compatibility.md`
- `docs/schooling_and_teacher_architecture.md`
- `docs/architecture_decisions.md`
- `docs/codex_handoff_prompt.md`

Required root files:

- `README.md`
- `AGENTS.md`
- `Cargo.toml`
- `.gitignore`
- `Makefile`

Required scripts:

- `scripts/setup.sh`
- `scripts/build.sh`
- `scripts/test.sh`
- `scripts/graphify.sh`
- `scripts/docs_check.sh`

Required crates:

- `crates/alife_core`
- `crates/alife_world`
- `crates/alife_gpu_backend`
- `crates/alife_runtime` (created by the shared-runtime checkpoint)
- `crates/alife_training` (created by the foundation-trainer checkpoint)
- `crates/alife_archive` (created by the lineage-library checkpoint)
- `crates/alife_bevy_adapter`
- `crates/alife_school`
- `crates/alife_semantic`
- `crates/alife_tools`

Each crate should have a local AGENTS.md with local rules.


## 42. Glossary

`BrainScaleTier`: discrete brain size class.

`BrainClassSpec`: full definition of neuron count, lobe layout, tile grid, motor stride, and budgets for a brain class.

`Internal SLM`: private subconscious semantic prior, not a teacher.

`Teacher LLM`: external social/curriculum actor that teaches through ordinary perception.

`ExperiencePatch`: tri-phase causal transaction between world and brain.

`W_genetic`: inherited immutable weight prior; `W_genetic_fixed` is the legacy save name.

`W_lifetime`: lifetime learning promoted during GPU sleep consolidation; `W_consolidated_habit` is the legacy save name.

`H_fast`: immediately active eligibility-gated waking plastic state.

`FoundationManifest`: immutable identity and compatibility contract for a
curated inherited neural payload plus its training/promotion provenance.

`LanguageCodebookV1`: 256-code compositional language ABI whose logical token
IDs are stable across saves and independent of neuron or GPU addresses.

`PersistentNeuronAddress`: stable lobe-and-ordinal identity used for
checkpointing, recombination, and function-preserving growth.

`Lineage Library`: profile-local immutable genetic archive plus quota-bound
learned checkpoints, ranking records, and portable founder bundles.

`NeuralClosedLoopGpu`: normal GPU-authoritative neural policy.

`HeuristicBaseline`: explicit separately labelled non-neural comparison policy.

`Graphify`: developer tool that maps project files into a queryable knowledge graph.

`DOX`: AGENTS.md hierarchy discipline for keeping agent instructions local and current.


---

# Appendix A: Rust Type Skeletons

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CreatureId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeciesId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BrainClassId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainScaleTier {
    Nano512,
    Small1024,
    Standard2048,
    Large4096,
    Cognitive32768,
    Student131k,
    Ascended1M,
    Ascended5M,
    ResearchCustom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobeKind {
    SensoryGrounding,
    MetabolicDrives,
    AuditorySpeech,
    VisualGlyphReading,
    LexiconConcept,
    Association,
    EpisodicMemory,
    WorkingMemoryAttention,
    MotorArbitration,
    HomeostaticRegulation,
    LanguageExpansion,
    MathQuantity,
    NarrativeHistory,
    SocialReasoning,
    SelfCriticUncertainty,
    PlanningDream,
    SpeechWritingMotor,
}

#[derive(Debug, Clone, Copy)]
pub struct LobeRange {
    pub kind: LobeKind,
    pub start: u32,
    pub len: u32,
    pub alpha_min_q8: u8,
    pub alpha_max_q8: u8,
}

#[derive(Debug, Clone)]
pub struct LobeLayout {
    pub ranges: Vec<LobeRange>,
    pub total_neurons: u32,
}

#[derive(Debug, Clone)]
pub struct BrainClassSpec {
    pub id: BrainClassId,
    pub tier: BrainScaleTier,
    pub neuron_count: u32,
    pub microtile_size: u32,
    pub supertile_size: u32,
    pub lobe_layout: LobeLayout,
    pub motor_logical_nodes: u32,
    pub motor_physical_stride: u32,
    pub max_active_microtiles: u32,
    pub max_active_synapses: u64,
    pub max_replay_events: u32,
}
```

# Appendix B: Validation Matrix

The following checks are required acceptance evidence:

1. No source file references Unity as implementation target.
2. No production shader files use `.hlsl`.
3. `Standard2048` appears only as a reference brain class.
4. Brain classes other than 2048 compile in invariant tests.
5. ExperiencePatch does not hardcode `[f32; 256]` as universal sensory input.
6. ActionCommand is structured and not a single byte.
7. Teacher interfaces do not call `SemanticPriorProvider` directly.
8. `NoSemanticPriorProvider` exists.
9. Graphify instructions mention Codex uses `$graphify`.
10. AGENTS.md says Rust + Bevy + wgpu/WebGPU + WGSL.
11. Production capacity promotion is limited to N512, N1024, and N2048.
12. Each promoted class compiles a nonempty sparse phenotype within class and route budgets.
13. Perception and unscored candidates share one authoritative world snapshot and audited digest chain.
14. Sensory perturbation, weight lesion/zeroing, and neuromodulator ablation change GPU neural behavior as expected.
15. Painful and rewarding outcomes change the matching candidate behavior before sleep without leaking the same bias to unrelated candidates.
16. Automatic sleep enters, consolidates exactly once on the GPU, saves/loads in every phase, wakes, and retains learned behavior.
17. Real-hardware receipts name the Vulkan adapter and show WGSL action selection, waking plasticity, and sleep consolidation without neural fallback.
18. GPU-unavailable paths return the typed unavailable result and do not run or claim a neural tick.
19. Production source and telemetry contain no live CPU neural shadow, parity gate, or automatic neural fallback.
20. A 10,000-plus-tick soak keeps memory, topology, candidates, and GPU buffers bounded without terminal capacity errors.
21. Populated-phenotype performance reports record hardware, class, memory accounting, readback, and missed/unknown targets honestly.
22. N2048 genetic birth loads the exact foundation plus causal genome deltas and clears every lifetime, episodic, semantic, language, eligibility, and transient field.
23. Limited language token IDs remain stable across saves/exports and never equal neuron or packed GPU addresses.
24. Named and broadcast player speech obey spatial hearing, and removing auditory routes causally removes comprehension.
25. `Vocalize` wins normal GPU arbitration and emits only GPU-selected raw tokens; removing speech routes removes meaningful narration.
26. Language grounding and narration gates pass with SLM translation disabled, while assisted evidence remains separately labelled.
27. Dead creatures commit genetic/life archives before retirement; selected learned checkpoints restore durable minds without stale body/world state.
28. Cross-run ranking treats unexposed metrics as `Unknown` and resolves secure genetic-founder or explicit mind-clone cohorts with full provenance.
29. N2048-to-N4096 research migration maps by persistent address and proves same-adapter selection identity plus logit delta at most `1e-6` before atomic handoff.

# Appendix C: Print/Page Estimate

This master specification is intentionally long. When rendered as a conventional technical document with code blocks and headings, the combined docs in this pack should exceed the previous 50-page architecture draft. The master spec should be treated as the controlling implementation envelope; future compatibility and schooling docs are required companions, not optional essays.


---

# Appendix D: Subsystem Deep Dives for Codex

This appendix gives implementation sessions focused subsystem context without weakening ADR-024. These are production contract and ownership notes: neural behavior executes through reviewed GPU-authoritative WGSL pipelines, CPU neural helpers remain test/developer-only, the world supplies unscored same-tick candidates and owns outcomes, and only N512/N1024/N2048 are initially promoted.

### BrainScaleClass Registry

**Purpose.** The registry is the authoritative table of supported brain classes, including consumer ecosystem classes and future ascension classes. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** BrainScaleClass Registry must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** BrainScaleClass Registry must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Lobe Ratio Genome

**Purpose.** Lobe ratios let evolution allocate neural budget among perception, drives, memory, language, motor control, and future reasoning systems. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Lobe Ratio Genome must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Lobe Ratio Genome must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Macro-Connectome Genome

**Purpose.** Macro-connectome genes control which lobe-to-lobe pathways exist, which supertiles are enabled, and how sparse routing capacity is distributed. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Macro-Connectome Genome must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Macro-Connectome Genome must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Sparse Payload Pools

**Purpose.** Sparse payload pools store active synapse data without dense N squared allocation and allow each brain class to have its own budgets. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Sparse Payload Pools must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Sparse Payload Pools must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### GPU Profile Planner

**Purpose.** The profile planner maps available memory and compute into hot, warm, cold, sleep, and dormant brain residency budgets. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** GPU Profile Planner must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** GPU Profile Planner must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Brain Residency Scheduler

**Purpose.** The scheduler decides which organisms receive full neural ticks and which are time-sliced, compressed, sleeping, or dormant. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Brain Residency Scheduler must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Brain Residency Scheduler must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### ExperiencePatch Ledger

**Purpose.** The ledger records causal pre-action, decision, and outcome phases for learning, replay, debug, and school assessment. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** ExperiencePatch Ledger must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** ExperiencePatch Ledger must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### ActionCommand Router

**Purpose.** The action router transforms structured neural proposals into host-authoritative world actions with legality checks and failure feedback. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** ActionCommand Router must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** ActionCommand Router must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Internal Semantic Prior Provider

**Purpose.** The internal semantic prior substitutes for some deep evolved instinct and language bias while remaining private, bounded, and ablatable. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Internal Semantic Prior Provider must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Internal Semantic Prior Provider must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### External Teacher Controller

**Purpose.** The external teacher controls avatars and lesson objects while all instructional content enters through hearing, vision, writing, gesture, or demonstration. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** External Teacher Controller must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** External Teacher Controller must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Lesson World API

**Purpose.** The lesson API lets a curriculum planner arrange objects, boards, maps, tools, rewards, and exams without bypassing creature perception. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Lesson World API must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Lesson World API must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Exact Verifier Interface

**Purpose.** The verifier grades math, logic, and science tasks deterministically so LLM explanations are not confused with truth. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Exact Verifier Interface must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Exact Verifier Interface must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Speech and Hearing ABI

**Purpose.** The speech ABI provides developmental progression from clean token/phoneme streams to more realistic acoustic perception. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Speech and Hearing ABI must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Speech and Hearing ABI must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Glyph and Writing ABI

**Purpose.** The writing ABI treats text as visible world symbols or simplified glyph sensors rather than hidden token injection. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Glyph and Writing ABI must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Glyph and Writing ABI must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Neurochemistry Vector

**Purpose.** The neurochemistry vector mediates fear, hunger, curiosity, social attachment, pain, fatigue, reward, and sleep pressure. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Neurochemistry Vector must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Neurochemistry Vector must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Drive Arbitration Model

**Purpose.** Drive arbitration forces advanced reasoning to weigh motivations against instincts and feelings rather than deleting those signals. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Drive Arbitration Model must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Drive Arbitration Model must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Sleep Replay Engine

**Purpose.** Sleep replay consolidates habits, prunes wear, restores energy, tests memory stability, and can prepare safe brain migration. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Sleep Replay Engine must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Sleep Replay Engine must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Brain Migration Protocol

**Purpose.** Migration preserves an evolved core while adding larger cortex capacity through an offline source checkpoint, deterministic replay/fixture validation, and atomic production handoff. Old and migrated neural brains never run concurrently in production. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Brain Migration Protocol must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Brain Migration Protocol must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Ascended School Candidate

**Purpose.** The first serious school candidate is a future one-million-neuron class, while 131k is the first serious prototype. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Ascended School Candidate must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Ascended School Candidate must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Lineage Export Manifest

**Purpose.** Lineage export packages genome, memories, ABI versions, class ID, chemistry, and migration history for future ascension or analysis. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Lineage Export Manifest must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Lineage Export Manifest must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Graphify Workflow

**Purpose.** Graphify creates a queryable knowledge graph so agents can understand the repository without reading every file blindly. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Graphify Workflow must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Graphify Workflow must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### DOX Hierarchy

**Purpose.** DOX-style AGENTS.md files give agents local instructions and require docs updates after meaningful changes. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** DOX Hierarchy must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** DOX Hierarchy must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Codex Goal Workflow

**Purpose.** The /goal command maintains a persistent task objective while details live in files that avoid the 4000 character goal limit. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Codex Goal Workflow must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Codex Goal Workflow must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Future Backend Interface

**Purpose.** The backend trait keeps future multi-GPU, cluster, or research hardware possible without changing organism semantics. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Future Backend Interface must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Future Backend Interface must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Ablation Framework

**Purpose.** Ablations prove whether a behavior is learned internally or only appears when the teacher or internal semantic prior is active. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Ablation Framework must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Ablation Framework must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Curriculum Progression

**Purpose.** Curriculum moves from grounded objects and actions to language, writing, math, history, science, tool use, and teaching others. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Curriculum Progression must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Curriculum Progression must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Peer Teaching

**Purpose.** Advanced creatures should eventually teach less advanced creatures through the same speech, writing, gesture, and demonstration channels. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Peer Teaching must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Peer Teaching must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Inherited Deja-Vu

**Purpose.** Experimental inheritance can transmit compressed predispositions or species culture to bootstrap learning without strict biological fidelity. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Inherited Deja-Vu must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Inherited Deja-Vu must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Topological Concept Ledger

**Purpose.** The concept ledger can later support nerve complexes, Morse-style summaries, curiosity gaps, and concept neighborhoods. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Topological Concept Ledger must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Topological Concept Ledger must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Debug and Inspector Tools

**Purpose.** Debugging tools must expose brain class, residency, chemistry, recent patches, teacher state, and Graphify links without altering behavior. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. Its implementation must follow ADR-024: production neural behavior is GPU-authoritative while cross-layer contracts stay explicit and versioned.

**Implementation boundary.** Define focused IDs, traits, configuration structs, documentation, and invariant tests. When this subsystem participates in cognition, production neural algorithms belong in reviewed WGSL pipelines and pure CPU neural helpers remain test-only or developer-only. Teacher LLM calls and SLM model loading stay behind their explicit optional boundaries.

**Data boundary.** Debug and Inspector Tools must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Debug and Inspector Tools must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs contract evidence plus the behavioral, hardware, save, soak, or performance evidence appropriate to its role. Tests must catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, scored world candidates, CPU shadow/fallback execution, direct hidden teacher injection, or unstructured actions. Behavioral tests include causal perturbations and ablations of internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.


---

# Appendix E: Code Skeleton Expansion Notes

The following module boundaries remain expected. Contract types belong in `alife_core`; production neural algorithms belong in focused `alife_gpu_backend` WGSL/pipeline modules. Implement each reviewed slice with invariant, behavioral, hardware, save, soak, and performance evidence rather than preserving empty stubs as architecture.

```text
crates/alife_core/src/
  ids.rs
  brain_class.rs
  lobe.rs
  genome.rs
  chemistry.rs
  experience.rs
  action.rs
  sensory_abi.rs
  action_abi.rs
  profiles.rs
  lineage.rs
  traits.rs

crates/alife_gpu_backend/src/
  backend.rs
  profile_planner.rs
  buffers.rs
  dispatch_batch.rs
  shader_manifest.rs

crates/alife_runtime/src/
  session.rs
  checkpoint.rs
  restore.rs

crates/alife_training/src/
  trainer.rs
  curriculum.rs
  evaluation.rs
  evolution.rs

crates/alife_archive/src/
  manifest.rs
  content_store.rs
  index.rs
  bundle.rs

crates/alife_school/src/
  teacher.rs
  lesson_api.rs
  verifier.rs
  curriculum.rs
  school_progress.rs

crates/alife_semantic/src/
  semantic_prior.rs
  speech_translation.rs
  lexicon_packet.rs
  providers/noop.rs
  providers/local_model.rs
```

Every file should start with a module-level comment that states whether it is contract-only, production runtime, test/developer-only, or research-gated future compatibility. This prevents Codex from mistaking a contract or research surface for production authority.
