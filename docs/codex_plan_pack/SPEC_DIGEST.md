# Source spec digest for Codex

This digest condenses the uploaded A-Life specs into implementation constraints. It is not a replacement for the specs, but it gives Codex the non-negotiable architecture in one file.

## Current scaffold assumption

The current repository is assumed to contain a Rust workspace scaffold with crates equivalent to `alife_core`, `alife_world`, `alife_gpu_backend`, `alife_bevy_adapter`, `alife_school`, `alife_semantic`, and `alife_tools`. The previous review found that the scaffold mostly defines module boundaries and shallow invariant tests, not a complete runtime. It also identified likely cleanup items: a duplicate nested spec pack, a non-portable Codex hook, thin `ExperiencePatch` and `ActionCommand` contracts, and missing validation evidence. Plan P00 and P01 force Codex to re-audit this instead of trusting stale assumptions.

## Cognitive-first CPU/GPU split

The CPU/Rust side owns authoritative world orchestration, ECS/world integration, object affordance perception, associative memory, topological cognitive maps, sleep consolidation, genome compilation, and action arbitration. The GPU/WGSL side accelerates sparse lobe projections, active tile masks, fixed-point accumulation/finalization, and H-trace/plasticity math. GPU math must not introduce behavioral semantics that do not exist in the CPU reference.

## Performance target

The target runtime is 60 FPS under an approximate 2GB-4GB VRAM budget, scaling through benchmark tiers: 1, 10, 50, 100, 250, and 500 agents. The architecture must include a benchmark harness and explicit throttling/recovery behavior rather than hoping the final implementation fits.

## Creature cognition model

A creature has sensory inputs, association lobes, hormones/drives, motor proposal lobes, and an explicit action arbitration layer. Memory and topology are not optional add-ons; they are part of the causal learning loop. The CPU creates, validates, seals, and distributes experience patches to memory, topology, endocrine updates, learning traces, and logs.

## Genome and learning split

The genome defines brain scale, lobe topology, lobe ratios, macro-connectome masks, sparse density priors, alpha/plasticity masks, endocrine constants, drive thresholds, sensor layout, motor affordances, mutation rates, and developmental schedules. Effective weight should be treated as:

`W_effective = W_genetic_fixed + W_lifetime_consolidated + alpha * H_operational`

The older shorthand `W_fixed + alpha * H` is acceptable only inside low-level examples where `W_fixed` is clearly not being mutated by lifetime learning.

## Drives and hormones

Drives and endocrine vectors dynamically scale thresholds, learning rates, motor confidence, and salience. Hunger, fatigue, fear, pain, loneliness, curiosity, dopamine, cortisol, adrenaline, oxytocin, serotonin, BrainATP, seizure/catatonia controls, and related fields need explicit bounds and validation. Failures should enter safe idle/sleep/recovery modes rather than corrupting learning.

## MemoryExpectancy

Associative memory is content-addressable and episodic. Recall inputs are current sensory-hormonal context. Recall outputs are expectancy bias fields: expected valence, predicted outcome/drive deltas, affordance bias, danger/safety bias, social trust/fear bias, and salience suggestions. It must not return raw historical action replay.

## Topological cognitive map

The cognitive map contains `ConceptCell`, `CognitiveEdge`, `CognitiveSimplex`, and `UnresolvedGap`. Edges include predicts, causes, satisfies_drive, belongs_to, socially_liked, socially_feared, contradicts, and related causal/semantic relations. Contradictions and prediction errors create unresolved gaps that bias curiosity.

## Action contract

The motor layer must output a structured `ActionCommand`, not a one-byte token in core. At minimum, command fields include action identity, optional target entity, optional target position, intensity, duration, confidence, drive-source mask, optional teacher/lesson response metadata, and optional speech/writing motor payload reference. GPU/lateral ring tokens are internal acceleration details and must be decoded into the structured action contract before world execution.

## ExperiencePatch contract

The runtime patch is rich and causal. It is assembled from:

1. `PreActionSnapshot`: body, drives, hormones, sensory frame, memory expectancy, optional Gaussian/semantic context, social/language context.
2. `DecisionSnapshot`: proposals, selected command, rejected top proposal, arbitration trace.
3. `PostActionOutcome`: collision/physical result, drive deltas, hormone deltas, success/failure, reward/valence, frustration/pain/energy deltas, concept/memory hints.

A sealed `ExperiencePatch` may then feed memory, topology, endocrine updates, learning traces, and log packing. A separate `PackedExperienceFrame` handles fixed-size export, while side buffers hold variable-length visible entities, touched entities, heard tokens, cluster salience, memory links, and concept links.

## Sparse GPU execution

The GPU backend uses a strict multi-pass pipeline: clear accumulators, SpMV projection with fixed-point atomics, activation finalization, and plasticity update. WGSL barriers are workgroup-scoped, so single-pass read/write designs are rejected. The sparse schema uses 16x16 microtiles and 8x8 microtile supertiles (128x128 macro grid) with hierarchical active masks. The runtime must support static GPU forward parity first, then plasticity, then super-tile culling, then structural recompaction.

## Low-precision constraints

GPU accumulation uses scaled integer atomics or equivalent safe fixed-point policy. Activations are clamped. Overflow detection must be explicit. INT8/INT16 traces with stochastic rounding and 32-bit intermediate accumulation are allowed only with parity tests and long-run saturation checks.

## Flat tensor restrictions

No host-side graph interlocks during active gameplay. No dynamic allocation or resizing inside active compute loops. SLM token embeddings enter only as modulatory lexicon inputs at slow cadence and cannot bypass grounding or arbitration. Compute thread dimensions and shared-memory padding must be deliberately chosen and tested.

## Sleep consolidation

Sleep handles synaptic compression, pruning, structural synaptogenesis, episodic indexing, H-trace drain, shadow registry decay, concept simplification, and optional `W_lifetime_consolidated` update. It must not mutate inherited genetic weights unless an explicit Lamarckian experiment is enabled.

## Offline tools

Offline Python/Rust tools ingest packed logs, cluster behavior, profile genomes, analyze representation geometry, compute ETF/neural-collapse metrics, and optionally generate initial weights through D2NWG. These tools are never required for the real-time runtime.
