# A-Life Master Specification

**Project:** A-Life  
**Stack:** Rust + Bevy + wgpu/WebGPU + WGSL  
**Spec revision:** 0.3 architecture-handoff  
**Date:** 2026-06-10  
**Status:** implementation scaffold target; not a complete runtime implementation  

This document is the controlling engineering specification for A-Life. It supersedes earlier fixed-2048, HLSL, dense-matrix, and single-pass-kernel drafts. It preserves the broad direction of the earlier Flat Sparse Tensor ALife design while correcting the implementation path: scalable brain classes instead of a fixed neuron count, class-bucketed sparse storage instead of dense `[M, N, N]` matrices, Rust/Bevy/wgpu/WebGPU/WGSL instead of Unity/HLSL, separated neural compute passes instead of fused TileSpMV+Oja passes, and explicit boundaries between internal subconscious semantic priors and external teacher agents.

The near-term goal is not to build the whole simulation. The near-term goal is to create a repository scaffold, documentation set, core Rust type contracts, toolchain, agent instructions, Graphify integration, DOX-style AGENTS.md hierarchy, and enough empty module structure that future coding sessions cannot accidentally hardcode the wrong architecture.

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
41. Required Files and Scaffolding
42. Glossary

---

## 1. Product Vision and Non-Negotiable Decisions

A-Life is a developmental artificial-life simulation game and research sandbox. The user-facing fantasy is an evolving ecosystem of creatures that learn, adapt, remember, reproduce, and eventually produce exceptional lineages that can be selected for deeper training. The engineering target is a modular sparse neural runtime that lets a population of organisms run on consumer hardware while preserving a path to larger single-agent and future research modes.

The controlling stack is Rust, Bevy, wgpu/WebGPU, and WGSL. Unity and C# are explicitly out of scope. HLSL may appear as a downstream native backend artifact of GPU drivers, but A-Life source shaders are authored in WGSL. CUDA, Triton, Vulkan-only, DirectX-only, TPU, and cluster runtimes are future research backends behind a compute abstraction. They are not initial implementation targets.

A-Life is not a conventional game AI planner. It is not a behavior tree system with learned parameters sprinkled on top. The organism brain is a sparse, plastic, metabolically constrained, genetically structured substrate. Local Hebbian/Oja updates, neurochemical modulators, sleep consolidation, and evolution over brain topology are first-class mechanics. The world exists to produce grounded sensorimotor experience, not just animation.

The core organism model must support scalable brains. The earlier 2048-neuron map is retained only as `Standard2048`, a reference class useful for examples, tests, and early profiling. It is not an invariant. The architecture must also support Nano/Small ecosystem creatures, large companion candidates, and future ascended classes. Scalable means class-bucketed and profile-gated, not dynamically resizing arbitrary matrices in the hot loop.

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

The repository must preserve these corrections as explicit architecture decisions in `docs/architecture_decisions.md` so future agent sessions do not regress.


## 3. Repository and Tooling Goals

The immediate Codex task is scaffolding, not runtime implementation. The repo should become a strong architecture container: workspaces, empty modules, type skeletons, docs, AGENTS.md guidance, Graphify integration notes, DOX hierarchy, and tests that assert constants and invariants. It should not attempt to implement real WGSL neural kernels yet.

The initial workspace should contain these crates:

- `alife_core`: engine-agnostic IDs, packed contracts, brain class specs, genome structures, ExperiencePatch, ActionCommand, memory profiles, and pure CPU reference math stubs.
- `alife_world`: Bevy-independent world concepts where practical: ecology, organisms, resources, drives, lesson-world APIs, and sensory extraction contracts.
- `alife_bevy_adapter`: Bevy-specific app, plugins, rendering, ECS integration, physics adapters, debug UI, and eventual demo scenes.
- `alife_gpu_backend`: wgpu resource planning, WGSL shader packaging, compute backend trait, buffer descriptors, and placeholder dispatch interfaces.
- `alife_school`: external teacher LLM roles, lesson API, verifier interfaces, curriculum definitions, and in-world teaching object contracts.
- `alife_semantic`: internal semantic prior provider interface and optional tiny local SLM provider stub.
- `alife_tools`: developer tooling hooks, graph integration helpers, docs validation, and spec consistency checks.

The repo should include `docs/master_spec.md`, `docs/future_research_compatibility.md`, `docs/schooling_and_teacher_architecture.md`, `docs/architecture_decisions.md`, and `docs/codex_handoff_prompt.md`. These docs are not decorative. They are the source of truth for agent work.

Graphify should be treated as a query-first knowledge graph layer over the repository. DOX should be treated as an AGENTS.md discipline: local instructions near every subsystem, updated when meaningful structure changes.

Codex should be instructed to use `/goal` to hold the scaffold target in the active thread. Because the official Codex CLI docs limit `/goal` objectives to 4,000 characters, the `/goal` should be short and point at the docs rather than include the whole spec.


## 4. Runtime Layer Model

The runtime is divided into stable layers. Layer boundaries are more important than exact module names.

Layer A: Host world and ecology. The CPU owns world state, Bevy ECS entities, physics adapters, ecology, reproduction, death, lesson object placement, teacher avatars, persistence, and player interaction. The CPU also schedules brain residency and chooses which organisms are hot, warm, cold, sleeping, or dormant.

Layer B: Core cognitive contracts. This layer is pure Rust where possible and engine-agnostic. It defines IDs, packed ABI structs, brain class specs, lobe ranges, genome structures, ExperiencePatch, ActionCommand, sensory and action ABI versions, memory profile manifests, and deterministic CPU reference calculations. It must not import Bevy types.

Layer C: GPU neural backend. This layer owns wgpu devices, queues, bind groups, storage buffers, staging buffers, shader modules, compute pipelines, and dispatch batches. It accelerates sparse neural math and plasticity but is not the source of game truth. It is replaceable behind `NeuralComputeBackend`.

Layer D: Semantic prior layer. This is an optional internal private system attached to a creature/species/brain class. It produces bounded `LexiconModulationPacket`s from compressed sensory summaries, drives, and limited ExperiencePatch context. It does not act externally.

Layer E: School/teacher layer. This layer controls teacher avatars, curricula, verifiers, lesson APIs, blackboards, books, speech, gesture, demonstrations, praise, penalties, and assessment. It has private planning/evaluation state, but all instructional content perceived by a creature must pass through normal world channels.

Layer F: Tooling and documentation layer. Graphify maps code and docs into a queryable graph. DOX/AGENTS.md provides local agent instructions. CI verifies that scaffold invariants and docs references remain coherent.


## 5. Rust Workspace Layout

The repository should use a Cargo workspace. The initial work should prefer empty modules plus strongly named structs over partial runtime algorithms. Agents should compile as much as possible, but incomplete modules may be hidden behind feature flags until later.

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

Bevy is the host game framework, not the cognitive core. It provides rendering, app lifecycle, plugin structure, ECS scheduling, input, debug visualization, and later physics integration. World entities are not stored directly inside neural structs. Core contracts reference world objects by stable `WorldEntityId` wrappers.

The Bevy layer should expose systems for spawning organisms, resources, hazards, lesson objects, teacher avatars, blackboards, signs, books, toys, tools, and environmental state. It should also produce sensory packets. Sensory packets are not raw Bevy components; they are packed views according to a versioned sensory ABI.

The world layer owns action legality. A neural backend can propose `ActionCommand`s, but it cannot teleport, eat non-existent objects, reproduce without conditions, or override physics. If the brain proposes an impossible action, the world returns a failure or frustration outcome through ExperiencePatch.

The Bevy adapter eventually needs debug inspectors for brain class, hot/warm/cold residency, neurochemistry, recent ExperiencePatch entries, action arbitration, teacher lesson state, and Graphify/doc links. These are future UI features; the current scaffold only needs module boundaries.


## 7. Engine-Agnostic Cognitive Core

`alife_core` is the stable heart of the project. It must be usable without Bevy, without GPU access, and without any LLM. It defines data contracts and reference math. If `alife_core` starts depending on Bevy ECS or rendering, the architecture is drifting.

Core IDs should be newtype wrappers over integers. Packed vectors and quaternions should be defined in core rather than importing Bevy math types. All structs crossing CPU/GPU boundaries need explicit representation strategy. `repr(C)` can be used where appropriate, but the spec should avoid premature bytemuck claims until fields are verified as plain-old-data.

The cognitive core should expose:

- `BrainClassSpec`
- `BrainScaleTier`
- `LobeLayout`
- `BrainGenome`
- `EndocrineProfile`
- `ExperiencePatchHeader`
- `ExperiencePatchView`
- `ActionCommand`
- `SensoryAbiVersion`
- `ActionAbiVersion`
- `SemanticPriorProvider` trait
- `NeuralComputeBackend` trait
- `LineageExportManifest`

The core should include unit tests that assert lobe alignments, legal neuron counts, motor physical stride, brain class invariants, and action command packing expectations. These tests are cheap and prevent future accidental fixed-2048 regressions.


## 8. Scalable Brain Classes

A-Life does not use a globally fixed neuron count. Brains are assigned to discrete classes. Discrete classes are used instead of arbitrary neuron counts because they allow efficient batching, stable shader dispatch dimensions, predictable memory planning, and meaningful performance profiles.

Initial near-term classes:

- `Nano512`: simple organisms, insects, cheap background ecology, early tests.
- `Small1024`: small animals, early evolving creatures, memory-light agents.
- `Standard2048`: reference creature based on earlier docs.
- `Large4096`: focal creatures, richer association and memory.
- `Cognitive32768`: future advanced prototype, not v0.
- `Student131k`: first serious language/reasoning prototype target, future compatibility.
- `Ascended1M`: first serious school candidate, future compatibility.
- `Ascended5M`: high-end research/companion candidate, future compatibility.
- `ResearchCustom`: explicit opt-in custom class for future labs.

All classes obey these invariants:

- neuron count is at least 512.
- near-term GPU classes use counts aligned to 128.
- lobe starts and lengths align to 16.
- microtiles are 16x16.
- supertiles are 128x128.
- active-loop resizing is forbidden.
- dispatch batches are grouped by class.
- sparse payloads scale with active synapses, not N².

`Standard2048` should exist because it is useful for continuity, not because it is privileged. Tests should include `Nano512`, `Standard2048`, and `Large4096` to prove the architecture is not fixed.


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


## 10. Genome and Developmental Encoding

The genome is a structural controller. It is not merely a random seed for weights. It controls brain scale, lobe ratios, macro-connectome masks, sparse tile density priors, alpha plasticity masks, endocrine constants, drive thresholds, morphology, sensor layout, motor affordances, mutation rates, and developmental schedules.

A genome may encode an internal SLM/semantic-prior capacity gene. This lets species differ in subconscious semantic scaffolding. It also lets advanced or ascended lineages increase semantic prior bandwidth while simple organisms remain purely sensorimotor.

The genome should also encode developmental growth checkpoints. A creature may hatch as a small class and later migrate to a larger class at juvenile/adolescent/adult stages if its species supports it and the simulation profile has available compute. These migrations occur at safe synchronization points, not during active neural dispatch.

Inheritance should support biological and engineered modes. Pure biological mode inherits only genome plus cultural exposure. Practical A-Life mode may allow limited inherited deja-vu, species culture priors, and experimental Lamarckian carryover. This is intentional: the project is interested in smart creatures more than strict biological fidelity. All such inheritance mechanisms must be flagged and ablatable.

The genome should never directly store every learned synapse as heritable default. It should store compressed predispositions, structural tendencies, seed templates, and maybe distillation summaries. This keeps evolution from becoming a hidden supervised checkpoint copy mechanism.


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

Teacher feedback should generally enter through social/world channels: praise, tone, visible approval, access, reward objects, correction, demonstration, or task success. Limited hidden reward/plasticity injection is allowed only as an early bootstrapping or experimental mode and must be marked in logs so results are not confused with clean grounded learning.


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

The interface should be named `SemanticPriorProvider`, not `SlmHardcodedSystem`. The first implementation may be a tiny local SLM optional provider or a deterministic stub. The interface must support `NoSemanticPriorProvider` for ablation tests.

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

ExperiencePatch is the causal transaction between world and brain. It must be scalable by brain class and sensory ABI. Earlier fixed arrays like `[f32; 256]` are not acceptable as universal runtime contracts.

Use headers and offset/length references for packed logs. Debug views may use slices. The contract should capture:

- creature ID,
- brain class ID,
- sensory ABI version,
- action ABI version,
- tick/time,
- pre-action world state summary,
- sensory packet offsets,
- drive/endocrine values,
- memory/context references,
- decision candidates,
- selected ActionCommand,
- outcome deltas,
- reward/frustration/success signals,
- teacher/school episode references when applicable.

ExperiencePatch supports learning, debugging, sleep replay, curriculum assessment, and lineage export. It must record whether any hidden bootstrapping feedback was used. Clean-learning claims require logs that show what the creature actually perceived.

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

The speech ABI should support:

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

The teacher may write on boards, label objects, create books, draw maps, or arrange symbols. These are world objects. The creature reads them through perception. Hidden token-to-lexicon injection is forbidden for the teacher.


## 19. Action ABI and Motor Arbitration

Action ABI remains stable across brain classes. Internal motor rings can scale. The world still receives structured actions. Logical motor nodes and physical padded stride are distinct.

For a class with `motor_logical_nodes`, reciprocal inhibition uses modulo wrapping over logical nodes. If the physical buffer stride is a power of two, bitwise masking may be used only for physical addressing, not for logical competition. This prevents the old 224-vs-256 bug.

Advanced/ascended creatures may have speech and writing motor systems. These are not special hidden channels. Speaking emits audible tokens/phonemes into the world. Writing creates visible glyph objects or modifies a writing surface. Teaching another creature uses the same action ABI as being taught.

Reasoning can override instinct only through arbitration. A creature may run toward danger to get food or protect a mate if the combined drives, learned values, and social motivations outweigh fear. Fear remains present. The motor system should represent conflict rather than deleting instinctive drives.


## 20. Memory Architecture

A-Life uses multiple memory forms. Synaptic memory stores habits and associations. Episodic memory stores events. Semantic prior memory supplies internal scaffolding. External world artifacts store culture. School records store curriculum progress. Save files store lineage.

Core memory layers:

- `W_genetic_fixed`: inherited immutable priors.
- `W_consolidated_habit`: durable lifetime learning consolidated during sleep.
- `H_operational`: live plastic trace updated online.
- `H_shadow`: higher-precision or staging trace for consolidation/rounding.
- episodic ledger: ExperiencePatch summaries and salient episodes.
- concept ledger: compressed conceptual/memory nodes.
- external artifacts: books, signs, maps, cultural records.
- school mastery ledger: concept-level test outcomes.

Memory must be ablatable. To prove grounded learning, tests should turn off teacher hints, reduce or disable internal SLM, test novel speakers/material, and test delayed recall after sleep.


## 21. Sparse Tensor Storage Model

Runtime storage is sparse and class-bucketed. Dense N² matrices are only conceptual and should not appear in production allocation plans.

For each `BrainClassBucket`, allocate:

- activation ping/pong buffers sized O(N * slots),
- accumulator buffers sized O(N * slots),
- compact action output buffers,
- sparse payload pools for genetic weights,
- sparse payload pools for consolidated habits,
- sparse payload pools for alpha masks,
- sparse payload pools for operational traces,
- sparse payload pools for shadow traces,
- sparse payload pools for wear/autophagy counters,
- microtile metadata buffers,
- supertile masks,
- replay/event buffers.

Genetics controls sparse structure. Evolution mutates topology and density. The GPU computes active paths, not dead weight lines. Memory budgets cap active synapses per brain class and per profile.

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

Migration inputs:

- genome,
- brain class,
- sensory ABI version,
- action ABI version,
- W_genetic_fixed sparse payloads,
- W_consolidated_habit sparse payloads,
- H_operational summaries,
- endocrine baseline,
- morphology/action map,
- memory/concept ledger,
- lineage metadata.

Migration process:

1. Freeze evolved core to preserve identity.
2. Allocate target class bucket.
3. Map old lobe ranges into target layout.
4. Preserve stable sensory/action ABI mappings.
5. Initialize expansion lobes as tabula-rasa or weakly seeded.
6. Shadow old core behavior before control handoff.
7. Gradually unfreeze selected pathways through gated integration.

A migrated creature should still feel like the same lineage at first. Expanded cortex can later influence behavior by weighing drives, feelings, memories, and goals through normal arbitration.


## 24. Learning Rules: Hebbian, Oja, and Modulators

The baseline plasticity rule is Oja-like local Hebbian learning modulated by lobe rate, endocrine state, reward/prediction context, alpha gates, and wear. It is not backpropagation through the world. Local rules enable continuous learning and biological-style adaptation.

A generic rule shape:

```text
delta_h = eta_lobe * xi_chemical * alpha_gate * y_post * (x_pre - y_post * h)
```

Additional terms may include anti-Hebbian inhibition, homeostatic scaling, reward modulation, novelty modulation, and sleep-only consolidation. These terms must be explicit and ablatable.

The internal SLM can weakly modulate attention/plasticity, but teacher evaluation cannot directly rewrite weights except under logged bootstrapping experiments. Exact verifiers grade math/logic/science; learning still happens through the creature's own sensory/action/outcome loop.


## 25. Weight Decomposition

The final weight formula is:

```text
W_effective = W_genetic_fixed + W_consolidated_habit + alpha_genome * H_operational
```

`W_genetic_fixed` is inherited and immutable during an organism's lifetime. `W_consolidated_habit` is durable learned habit. `H_operational` is live plastic trace. `H_shadow` supports higher-precision staging and stochastic rounding.

Sleep may consolidate stable traces into `W_consolidated_habit`. It must not silently bake lifetime learning into `W_genetic_fixed` unless an explicit experimental Lamarckian/species-prior mode is enabled. Offspring may inherit genetic predispositions, species culture, limited deja-vu priors, or experimental distillations depending on settings.

This separation is central to evolution. Genes optimize brain structure and priors; experience optimizes lifetime behavior; school/culture provides curriculum.


## 26. GPU Compute Pipeline

The production neural tick is multi-pass:

0. Clear accumulators.
1. Sparse projection over active microtiles/supertiles.
2. Activation finalization into ping-pong buffers.
3. Oja/Hebbian plasticity and wear update.
4. Optional local autophagy/pruning pass.
5. Motor arbitration/action output pass.

Passes may later be fused after correctness and profiling, but initial implementation must keep them separate. This prevents write-after-read hazards, makes debugging possible, and preserves WebGPU portability.

The GPU backend must batch by brain class. A dispatch batch contains class spec ID, slot range, active tile range, sensory buffer offsets, activation buffers, payload pool references, and output buffer offsets.

The CPU may read small action output buffers each tick. Large neural buffers should not be synchronously read back in normal play.


## 27. WGSL Authoring Rules

All production shaders are WGSL. Do not create HLSL source files unless explicitly labelled as non-authoritative pseudocode. WGSL modules should be small and testable.

Initial placeholder modules:

- `clear_accumulators.wgsl`
- `spmv_projection.wgsl`
- `activation_finalize.wgsl`
- `oja_update.wgsl`
- `wear_autophagy.wgsl`
- `motor_arbitration.wgsl`
- `sleep_consolidation.wgsl`

WebGPU limitations are design constraints. Floating-point atomics should not be assumed. Use scaled integer accumulators where needed. Subgroup features should have portable fallbacks. Workgroup sizes and buffer layouts must be explicit.

Any future CUDA/Triton/TPU backend must implement the same `NeuralComputeBackend` contract and must not change organism semantics.


## 28. Sleep, Replay, and Consolidation

Sleep is not a decorative animation. It is a compute and memory phase. Organisms sleep because brains need consolidation, metabolic recovery, and structural cleanup. Larger brains require more sleep.

Sleep jobs:

- replay salient ExperiencePatches,
- drain shadow traces,
- consolidate habits,
- prune fatigued low-salience pathways,
- update concept/episodic ledgers,
- test memory stability,
- prepare developmental migration if scheduled.

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
- Do not implement runtime kernels until docs/scaffolding are stable.
- Teacher LLM and internal SLM are separate.
- Use Graphify for architecture queries when installed.
- Keep docs synchronized.

Child AGENTS.md files should exist in `crates/alife_core`, `crates/alife_gpu_backend`, `crates/alife_school`, `crates/alife_semantic`, and `docs`.


## 32. Codex Operating Rules

Use `/goal` to set a persistent objective for the task. Because official Codex docs state `/goal` objectives must be non-empty and no more than 4,000 characters, use a compact goal and point at `docs/codex_handoff_prompt.md` for details.

Codex should begin in planning mode. It should inspect existing files, report divergences, then scaffold. It should not implement neural runtime algorithms. It should create docs, type skeletons, module stubs, scripts, and tests for invariants.

Codex should make small commits if working in a repo. It should run `cargo fmt`, `cargo check --workspace`, and any docs checks. It should not install random dependencies without explaining why.

Codex should initialize DOX hierarchy and add Graphify instructions. It should not require Graphify to be installed for the project to build.


## 33. Testing and Validation Strategy

Initial tests are structural, not behavioral. They should assert:

- brain classes have legal neuron counts,
- lobe ranges are aligned and non-overlapping,
- motor logical nodes and physical stride obey rules,
- `Standard2048` reproduces the old reference layout only as a class,
- action command structs are stable,
- ExperiencePatch headers use offsets rather than fixed sensory arrays,
- genome fields include topology, chemistry, morphology, plasticity, and development,
- `NoSemanticPriorProvider` exists,
- teacher interfaces do not use internal SLM hooks.

Behavioral tests come later: food seeking, avoidance, simple word grounding, SLM-off transfer, teacher ablations, and sleep recall.


## 34. Determinism and Reproducibility

A-Life needs deterministic seeds for credible evolution and debugging. Organism birth, genome mutation, tile initialization, lesson generation, replay selection, and stochastic rounding should be seeded and reproducible where possible.

GPU determinism is difficult across devices. The architecture should define deterministic CPU reference paths for small tests and bounded acceptance metrics for GPU runs. Exact bitwise determinism across all hardware is a long-term goal, not an immediate guarantee.

All experiments involving hidden bootstrapping, teacher feedback, SLM support, or inherited deja-vu must be logged. Reproducing a creature requires genome, seed lineage, brain class, ABI versions, profile settings, and migration history.


## 35. Performance and Profiling Plan

Performance scales through sparsity, residency, brain class batching, and profile caps. The initial scaffolding should not chase micro-optimizations. But the docs should define future metrics:

- hot-brain tick throughput,
- SpMV tiles per second,
- active synapses per class,
- memory bandwidth,
- action latency,
- sleep jobs per second,
- replay throughput,
- schooling lessons per simulated hour,
- Graphify/doc agent overhead.

Large brains slow population growth. Ascended classes are special modes. A single 1M-neuron school candidate is not expected to coexist with hundreds of hot ecosystem brains on low hardware.


## 36. Data Persistence, Saves, and Lineage Export

Saves must be versioned. They should include world state, organisms, genomes, brain class specs, ABI versions, memory snapshots, consolidated habits, episodic summaries, school progress, and lineage metadata.

Lineage export supports ascension. It should store enough to recreate the creature later, migrate it to a larger class, and test whether its identity survived migration.

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


## 37. Schooling and Curriculum Interfaces

The master spec defines boundaries; detailed schooling lives in `docs/schooling_and_teacher_architecture.md`. Schooling is staged by utility and reasoning capacity.

Stages:

1. Preschool grounding: objects, colors, food, danger, simple commands.
2. Language bootstrapping: roles, negation, sequence, requests, social phrases.
3. Writing: reports, descriptions, explanations.
4. Math: manipulable quantities first, symbols later, exact verifiers always.
5. History: simulated world history first, real human history later.
6. Science: experiments, measurement, prediction, repeatability.
7. Social reasoning: promises, obligations, trust, teaching others.
8. Independent exams: teacher off, internal SLM reduced/off, novel environment, delayed recall.

Teacher LLM roles use world-authorized lesson APIs to spawn/arrange objects. The private evaluator can grade and select lessons. Direct hidden reward/plasticity injection is allowed only in limited, logged bootstrapping mode.


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

Compatibility does not imply implementation. v0/v1 stays focused on scaffolding, core contracts, scalable brain classes, genetics/chemistry/evolution data models, and clean module boundaries.

Speculative AGI language belongs only in non-requirements appendix text. Formal documents should use “grounded developmental generalist agent research direction.”


## 39. Non-Goals for v0/v1

Do not build:

- Unity integration.
- HLSL production kernels.
- CUDA/Triton/TPU backends.
- 1M-neuron runtime.
- Teacher LLM runtime.
- Internal SLM model loading.
- D2NWG training.
- full topological Morse implementation.
- real neural GPU kernels.
- AGI claims.
- arbitrary dynamic brain resizing.

Do build:

- repo scaffold,
- docs,
- workspace crates,
- type skeletons,
- invariant tests,
- scripts,
- AGENTS.md/DOX hierarchy,
- Graphify instructions,
- Codex handoff prompt.


## 40. Implementation Milestones

Milestone 0: Documentation and agent rules. Add master docs, architecture decisions, future compatibility, schooling doc, Codex prompt, AGENTS.md hierarchy, and Graphify/DOX notes.

Milestone 1: Workspace scaffold. Add Cargo workspace and empty crates. Ensure `cargo check --workspace` passes.

Milestone 2: Core type skeletons. Add IDs, packed math types, brain classes, genome structs, ExperiencePatch headers, ActionCommand, memory profiles, traits.

Milestone 3: Invariant tests. Assert scalable classes and no fixed-2048 regressions.

Milestone 4: Bevy shell. Add minimal app plugin and window only.

Milestone 5: CPU reference toy brain. Later.

Milestone 6: wgpu placeholder backend. Later.

Milestone 7: actual WGSL neural passes. Later.


## 41. Required Files and Scaffolding

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

Required crate stubs:

- `crates/alife_core`
- `crates/alife_world`
- `crates/alife_gpu_backend`
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

`W_genetic_fixed`: inherited immutable weight prior.

`W_consolidated_habit`: lifetime learning consolidated during sleep.

`H_operational`: online plastic trace.

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

The following checks should eventually exist as tests or docs checks:

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

# Appendix C: Print/Page Estimate

This master specification is intentionally long. When rendered as a conventional technical document with code blocks and headings, the combined docs in this pack should exceed the previous 50-page architecture draft. The master spec should be treated as the controlling implementation envelope; future compatibility and schooling docs are required companions, not optional essays.


---

# Appendix D: Subsystem Deep Dives for Codex

This appendix is intentionally detailed. It gives future Codex sessions enough context to create the scaffold without inventing incompatible abstractions. These sections are not requests to implement runtime behavior now; they are contract-level notes.

### BrainScaleClass Registry

**Purpose.** The registry is the authoritative table of supported brain classes, including consumer ecosystem classes and future ascension classes. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** BrainScaleClass Registry must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** BrainScaleClass Registry must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Lobe Ratio Genome

**Purpose.** Lobe ratios let evolution allocate neural budget among perception, drives, memory, language, motor control, and future reasoning systems. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Lobe Ratio Genome must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Lobe Ratio Genome must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Macro-Connectome Genome

**Purpose.** Macro-connectome genes control which lobe-to-lobe pathways exist, which supertiles are enabled, and how sparse routing capacity is distributed. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Macro-Connectome Genome must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Macro-Connectome Genome must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Sparse Payload Pools

**Purpose.** Sparse payload pools store active synapse data without dense N squared allocation and allow each brain class to have its own budgets. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Sparse Payload Pools must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Sparse Payload Pools must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### GPU Profile Planner

**Purpose.** The profile planner maps available memory and compute into hot, warm, cold, sleep, and dormant brain residency budgets. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** GPU Profile Planner must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** GPU Profile Planner must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Brain Residency Scheduler

**Purpose.** The scheduler decides which organisms receive full neural ticks and which are time-sliced, compressed, sleeping, or dormant. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Brain Residency Scheduler must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Brain Residency Scheduler must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### ExperiencePatch Ledger

**Purpose.** The ledger records causal pre-action, decision, and outcome phases for learning, replay, debug, and school assessment. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** ExperiencePatch Ledger must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** ExperiencePatch Ledger must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### ActionCommand Router

**Purpose.** The action router transforms structured neural proposals into host-authoritative world actions with legality checks and failure feedback. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** ActionCommand Router must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** ActionCommand Router must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Internal Semantic Prior Provider

**Purpose.** The internal semantic prior substitutes for some deep evolved instinct and language bias while remaining private, bounded, and ablatable. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Internal Semantic Prior Provider must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Internal Semantic Prior Provider must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### External Teacher Controller

**Purpose.** The external teacher controls avatars and lesson objects while all instructional content enters through hearing, vision, writing, gesture, or demonstration. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** External Teacher Controller must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** External Teacher Controller must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Lesson World API

**Purpose.** The lesson API lets a curriculum planner arrange objects, boards, maps, tools, rewards, and exams without bypassing creature perception. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Lesson World API must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Lesson World API must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Exact Verifier Interface

**Purpose.** The verifier grades math, logic, and science tasks deterministically so LLM explanations are not confused with truth. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Exact Verifier Interface must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Exact Verifier Interface must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Speech and Hearing ABI

**Purpose.** The speech ABI provides developmental progression from clean token/phoneme streams to more realistic acoustic perception. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Speech and Hearing ABI must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Speech and Hearing ABI must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Glyph and Writing ABI

**Purpose.** The writing ABI treats text as visible world symbols or simplified glyph sensors rather than hidden token injection. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Glyph and Writing ABI must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Glyph and Writing ABI must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Neurochemistry Vector

**Purpose.** The neurochemistry vector mediates fear, hunger, curiosity, social attachment, pain, fatigue, reward, and sleep pressure. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Neurochemistry Vector must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Neurochemistry Vector must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Drive Arbitration Model

**Purpose.** Drive arbitration forces advanced reasoning to weigh motivations against instincts and feelings rather than deleting those signals. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Drive Arbitration Model must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Drive Arbitration Model must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Sleep Replay Engine

**Purpose.** Sleep replay consolidates habits, prunes wear, restores energy, tests memory stability, and can prepare safe brain migration. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Sleep Replay Engine must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Sleep Replay Engine must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Brain Migration Protocol

**Purpose.** Migration preserves an evolved core while adding larger cortex capacity through freeze, map, shadow, and gated unfreeze steps. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Brain Migration Protocol must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Brain Migration Protocol must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Ascended School Candidate

**Purpose.** The first serious school candidate is a future one-million-neuron class, while 131k is the first serious prototype. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Ascended School Candidate must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Ascended School Candidate must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Lineage Export Manifest

**Purpose.** Lineage export packages genome, memories, ABI versions, class ID, chemistry, and migration history for future ascension or analysis. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Lineage Export Manifest must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Lineage Export Manifest must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Graphify Workflow

**Purpose.** Graphify creates a queryable knowledge graph so agents can understand the repository without reading every file blindly. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Graphify Workflow must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Graphify Workflow must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### DOX Hierarchy

**Purpose.** DOX-style AGENTS.md files give agents local instructions and require docs updates after meaningful changes. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** DOX Hierarchy must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** DOX Hierarchy must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Codex Goal Workflow

**Purpose.** The /goal command maintains a persistent task objective while details live in files that avoid the 4000 character goal limit. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Codex Goal Workflow must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Codex Goal Workflow must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Future Backend Interface

**Purpose.** The backend trait keeps future multi-GPU, cluster, or research hardware possible without changing organism semantics. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Future Backend Interface must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Future Backend Interface must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Ablation Framework

**Purpose.** Ablations prove whether a behavior is learned internally or only appears when the teacher or internal semantic prior is active. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Ablation Framework must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Ablation Framework must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Curriculum Progression

**Purpose.** Curriculum moves from grounded objects and actions to language, writing, math, history, science, tool use, and teaching others. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Curriculum Progression must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Curriculum Progression must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Peer Teaching

**Purpose.** Advanced creatures should eventually teach less advanced creatures through the same speech, writing, gesture, and demonstration channels. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Peer Teaching must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Peer Teaching must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Inherited Deja-Vu

**Purpose.** Experimental inheritance can transmit compressed predispositions or species culture to bootstrap learning without strict biological fidelity. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Inherited Deja-Vu must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Inherited Deja-Vu must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Topological Concept Ledger

**Purpose.** The concept ledger can later support nerve complexes, Morse-style summaries, curiosity gaps, and concept neighborhoods. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Topological Concept Ledger must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Topological Concept Ledger must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.

### Debug and Inspector Tools

**Purpose.** Debugging tools must expose brain class, residency, chemistry, recent patches, teacher state, and Graphify links without altering behavior. exists so A-Life can evolve from a consumer-scale ecosystem game into a larger developmental research platform without rewriting the core contracts. This subsystem must be named and bounded early, but it must not be over-implemented during the scaffold phase. The immediate job for Codex is to create stable modules, data structures, and tests that prevent future architectural drift.

**Near-term scaffold.** The near-term implementation should define IDs, traits, configuration structs, documentation, and invariant tests. Runtime algorithms should be represented by placeholder traits or `todo!()` stubs only when necessary. If a feature requires real GPU kernels, teacher LLM calls, SLM model loading, or complex simulation logic, it belongs in a later milestone. The scaffold should compile, document intent, and make wrong implementations obvious.

**Data boundary.** Debug and Inspector Tools must communicate through explicit contracts rather than implicit global state. Every cross-layer message should carry version identifiers where the format may evolve. CPU world state, GPU neural buffers, semantic priors, and teacher systems should be connected through narrow interfaces. This is especially important for experiments: if a creature learns a word, we need to know whether the information came through hearing, writing, teacher feedback, internal semantic bias, inheritance, or hidden bootstrapping.

**Evolution and scaling.** Debug and Inspector Tools must scale with brain class and compute profile. Nano ecosystem creatures should be able to omit advanced capacity. Ascended classes should be able to add capacity without changing the ABI. The genome should be able to tune participation in this subsystem through lobe ratios, gates, drives, developmental schedules, or feature flags. Larger versions should cost more compute, energy, sleep, and reproductive opportunity.

**Testing requirement.** Each subsystem needs at least one scaffold-level test or docs check. Tests should catch regression to Unity, HLSL, fixed 2048 brains, dense matrices, direct hidden teacher injection, or unstructured actions. Later behavioral tests should include ablations that disable internal semantic priors, external teacher help, hidden reward injection, and curriculum hints.


---

# Appendix E: Code Skeleton Expansion Notes

The following skeletons are expected eventually. Codex should create the files and minimal types, but not fill in algorithms beyond invariants.

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

crates/alife_school/src/
  teacher.rs
  lesson_api.rs
  verifier.rs
  curriculum.rs
  school_progress.rs

crates/alife_semantic/src/
  semantic_prior.rs
  lexicon_packet.rs
  providers/noop.rs
  providers/tiny_local_stub.rs
```

Every file should start with a module-level comment that states whether it is v0 scaffold, future compatibility, or later runtime implementation. This prevents Codex from mistaking an interface stub for an algorithm request.
