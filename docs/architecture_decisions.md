# A-Life Architecture Decisions

## ADR-001: Rust + Bevy + wgpu/WebGPU + WGSL

Decision: Use Rust, Bevy, wgpu/WebGPU, and WGSL. Unity, C#, and HLSL production shaders are out of scope.

## ADR-002: Scalable Brain Classes

Decision: Do not fix all brains at 2048 neurons. Use class-bucketed scalable brains. `Standard2048` is a reference tier only.

## ADR-003: Sparse Payload Pools

Decision: Do not allocate dense `[M, N, N]` neural buffers in production. Use sparse class-bucketed payload pools.

## ADR-004: Internal SLM vs External Teacher LLM

Decision: Internal SLM is a private subconscious semantic prior. External teacher LLM is an embodied/social teacher using normal perception channels.

## ADR-005: Multi-Pass GPU Pipeline

Decision: Start with separated WGSL passes. Do not implement a fused TileSpMV+Oja kernel in v0.

## ADR-006: Structured Actions

Decision: Use structured `ActionCommand`. Do not collapse output to a 1-byte token.

## ADR-007: Future AGI Language

Decision: Formal spec uses “grounded developmental generalist agent research direction.” AGI language appears only in non-requirements appendix.

## ADR-008: Graphify and DOX

Decision: Graphify and DOX are developer/agent tooling, not runtime dependencies. Graphify supplies queryable repo knowledge. DOX supplies AGENTS.md discipline.

## ADR-009: Engine-Neutral Sensory ABI and Optional Context Refs

Decision: Core sensory snapshots use stable IDs, core math primitives, fixed v1 channel groups, context stream metadata, and optional semantic/Gaussian references. They do not embed Bevy entities, renderer objects, SLM runtime objects, teacher internals, or hidden action hooks.

## ADR-010: Sealed Three-Phase ExperiencePatch

Decision: Runtime `ExperiencePatch` records are rich Rust data assembled through pre-action, decision, and post-action phases, then sealed before learning, memory, topology, or logging consumers can inspect them. Packed logging remains a separate P11 contract.

## ADR-011: Fixed Packed Logs with Side Buffers

Decision: Packed experience logs are versioned, fixed-size, intentionally lossy frames derived from sealed `ExperiencePatch` records, with variable-length payload summaries stored in deterministic side-buffer records. Packed logs are an export/replay boundary, not the canonical runtime cognition representation.

## ADR-012: Memory Recall as Bounded Expectancy Bias

Decision: Episodic associative memory stores sealed experience records in bounded deterministic banks and recalls `MemoryExpectancy` bias fields. Recall may bias valence, drive deltas, outcome summaries, affordances, danger/safety, social trust/fear, novelty, and curiosity, but it must not expose selected actions as replay commands.

## ADR-013: Bounded CPU Topological Concept Map

Decision: The P13 topological concept map is an engine-independent, bounded CPU-side ledger over sealed `ExperiencePatch` records. It creates `ConceptCell`, `CognitiveEdge`, `CognitiveSimplex`, and `UnresolvedGap` records plus curiosity bias metadata, but it does not use graph databases, GPU graph structures, renderer objects, engine entities, action-ID hint outputs, or direct public action-command output.

## ADR-014: Sleep Consolidation Preserves Genotype Boundaries

Decision: P16 sleep consolidation is a deterministic CPU sleep/offline phase that may drain `H_shadow`, stage `H_operational` and lifetime-layer updates, compress bounded memory/topology summaries, and emit structural edit candidates. It must not silently mutate `W_genetic_fixed`, and structural edits remain staged for sleep/offline compilation rather than active tick matrix resizing.

## ADR-015: GPU Upload Contracts Are Explicit Little-Endian Records

Decision: P24 GPU upload buffers are explicit little-endian, page-relative records translated from the P14 CPU sparse projection schema. The GPU backend must not use raw host pointers, unsafe transmute packing, or shader-invented parallel layouts for tile metadata, masks, packed indices, weight layers, activation ping-pong, accumulators, diagnostics, routing descriptors, or action-summary staging.

## ADR-016: Benchmark Tiers Are CPU-Smoke and GPU-Parity Gated

Decision: P20 benchmark tiers use deterministic CPU-reference headless scenarios for CI smoke at populations 1 and 10, while populations 50, 100, 250, and 500 remain manual expected-slow CPU measurements until GPU parity/runtime plans provide acceleration. Benchmark reports are generated under `target/artifacts/` and are not committed as baseline data.

## ADR-017: Super-Tile Culling Preserves CPU Oracle Behavior

Decision: P27 GPU supertile routing uses backend-owned active masks derived from core lobe/routing metadata as behavior-preserving early-exit data. The masks may skip inactive 16x16 microtiles inside 8x8/128x128 supertiles, but masked and unmasked execution must agree when skipped source regions are inactive. P27 counters are diagnostics/export metadata only; active gameplay APIs still do not require synchronous neural readback.

Dispatch-level scheduling and later structural GPU cleanup remain separate plans.

## ADR-018: Structural Recompaction Is Sleep/Offline Double-Buffered

Decision: P28 GPU structural recompaction compiles P16 sleep edit batches into
validated scratch upload buffers and swaps them all-or-nothing only at safe
sleep/offline boundaries. Active gameplay buffers are never mutated in place,
`W_genetic_fixed` remains immutable by default, and recompaction diagnostics are
sleep/offline/export metadata rather than active gameplay readback.

Dynamic shader-side allocation, GPU autophagy runtime passes, and no-readback
performance tier integration remain separate follow-up work.

## ADR-019: GPU Runtime Is Optional, Fallback-Capable, and No-Readback

Decision: P29 makes GPU static, GPU plastic, and GPU full backends selectable
runtime modes without replacing the CPU reference oracle. Unsupported hardware,
disabled features, validation failure, or unavailable full-runtime support fall
back to CPU with typed diagnostics.

Active gameplay does not expose synchronous bulk neural, per-synapse, per-lobe,
or weight readback. Diagnostics/export snapshots are frame, sleep, manual
validation, or performance-report boundary scoped. Runtime throttling preserves
sensory/motor and homeostatic priority while decimating non-essential cognitive
lobes first when GPU neural timing exceeds budget.

P29 performance reports must record unknown or missed targets honestly. P20 CPU
smoke data may be copied into P29 reports as fallback context, but it is not a
GPU performance claim.

## ADR-020: Portable Saves Use Stable IDs and Asset References

Decision: P34 save files, runtime configs, and asset manifests are explicitly
versioned JSON contracts. Portable saves store stable IDs and summaries plus
asset references/digests for generated weights, ETF prototypes, scenarios, and
world fixtures. They reject incompatible schemas unless a tested migration
exists.

Engine-local IDs such as Bevy `Entity`, Avian handles, wgpu handles, renderer
handles, and OS/window handles are not serialized directly. Bulk generated
tensors and large logs stay outside the main save file behind manifest entries.

Generated weight assets remain birth/initialization inputs. Lifetime learning,
H-traces, and consolidated habits remain separate from `W_genetic_fixed`.

## ADR-021: Playground Examples Are Headless-First and Optional-Feature Gated

Decision: P35 exposes the integrated playground through a headless CPU smoke
path by default, with Bevy/Avian, semantic provider, school, and GPU demos kept
as optional or manual paths where hardware or graphics support is required.

`alife_core` remains engine-independent, GPU runtime keeps CPU fallback, and
playground saves/configs consume P34 stable IDs and asset references instead of
engine-local handles. P36 owns release hardening, packaging, and soak gates.

## ADR-022: Release Gate Is Evidence-First And Manual Hardware Honest

Decision: P36 treats production readiness as a checklist-backed release gate.
Default validation, golden traces, scenarios, save/load, playground smoke,
benchmark smoke, core boundaries, and fast headless soak must pass before a
candidate can be called ready.

Slow soak, upper population benchmark tiers, GPU hardware parity, GPU
performance, and graphics smoke remain manual gates unless current hardware
evidence is recorded. Unknown GPU performance or missed targets must be reported
as unknown or missed, not inferred from CPU fallback data.

Windows validation uses the PowerShell Git Bash wrappers to avoid accidental WSL
invocation. The release gate does not create a P37 or new architecture lane;
future work is tracked as backlog notes or issues.

## ADR-023: Post-Seal Lifetime Deltas Are Core-Owned And Patch-Gated

Decision: live lifetime/H_shadow application is allowed only through
`alife_core` owned, versioned, validated delta batches derived from sealed
`ExperiencePatch` evidence. External GPU code may convert validated plasticity
results into the core batch format, but it may not expose raw GPU buffers,
wgpu resources, engine IDs, Bevy types, or renderer handles to `alife_core`.

`CreatureMind` applies these batches only after patch sealing and rejects wrong
organism/tick/sequence, replay, NaN/Inf, out-of-range values, duplicate targets,
failed CPU shadow parity, non-H_shadow layers, and any claimed mutation of
`W_genetic_fixed`, lifetime-consolidated weights, or H_operational. CPU fallback
and CPU shadow parity remain authoritative.

## ADR-024: Closed-Loop Neural Cognition Is GPU-Authoritative

Decision: The production neural policy gathers current perception and unscored
world candidates before dispatch, then performs encoding, recurrent dynamics,
candidate scoring, winner selection, waking plasticity, and sleep
consolidation through WGSL pipelines. Production does not run a live CPU neural
shadow, parity-gated duplicate brain, or automatic CPU neural fallback.

`HeuristicBaseline` remains explicit and separately labelled. GPU unavailability
returns a typed unavailable result. N512, N1024, and N2048 are the initial
production neural capacity classes; larger classes remain research-gated.

This decision supersedes the CPU consolidation authority in ADR-014, the P14
CPU-schema ownership clause in ADR-015, GPU parity gating in ADR-016, CPU
fallback in ADR-019 and ADR-021, and the CPU-shadow/parity authority clauses in
ADR-023. Their save-safety, sparse-layout, world-authority, and sealed-patch
boundaries remain in force where they do not conflict with ADR-024.

## ADR-025: Candidate-Conditional Memory and Grounded Profiles

Decision: Episodic recall uses a versioned state-action-target query and returns
bounded per-candidate latent/value context. Production consumes that context
only through explicit GPU candidate-decoder channels; no memory context is
pooled into recurrent/global inputs, and no candidate-invariant memory or
topology scalar is added to action scores. Queries bind the pre-context
perception base digest; GPU dispatch and sealing bind the separately computed
final frame digest after consume-once context finalization.

`PrivilegedAffordanceV1` and `GroundedObjectSlotsV1` are separately provenanced.
Grounded slots contain physical observations and no semantic class labels.
Topology is a bounded diagnostic sidecar over sealed patches. Memory and
topology saturation deterministically merge, evict, compact, or summarize and
cannot abort a valid neural tick. Persistent bindings use tracked-object or
episodic IDs rather than raw world entity IDs. Tracker, memory, compaction, and
topology owners are portable organism IDs rather than GPU handles.

## ADR-026: GPU Closed-Loop Scaling Is Evidence- and Budget-Gated

Decision: N512, N1024, and N2048 are the only initial production neural
capacity classes. Each class is promoted independently only when one canonical
manifest binds complete A/B/C/D evidence from the same source tree and Vulkan
adapter: causal GPU authority and replay; immediate learning, sleep, and
restore; separate privileged and grounded memory/saturation evidence; logical
and VRAM budgets, admission, BrainATP/throttling, save migration, both-profile
10,240-tick soak and replay; all 12 populated benchmark rows; and the exact
global source, docs, boundary, and authority gates.

Completed benchmark rows may promote. Honest `Missed` or `Unavailable` rows
remain valid evidence but block only their class. Runtime population limits
come from an explicit neural-heap profile, not the capacity class. N4096 and
larger legacy tiers remain inspection/export or research-only. No CPU neural
shadow, parity gate, automatic CPU neural fallback, configuration override, or
unbound benchmark result is a promotion mechanism.

ADR-026 supersedes the benchmark/fallback portions of ADR-016, ADR-019,
ADR-021, and ADR-022 where they conflict with ADR-024 through ADR-026. Their
renderer fallback and evidence-honesty decisions remain in force.

## ADR-027: Curated Foundations Use Baldwinian Inheritance

Decision: N2048 is the first trained brain class. A versioned immutable curated
foundation supplies perception, proprioception, movement, eating, resting,
survival instincts, content-neutral memory mechanics, and language mechanics.
The production genetic payload is composed deterministically as
`W_genetic = foundation + compiled genome deltas`. Curated gradient training is
followed by evolutionary hardening in the exact production GPU runtime.

Foundation sections declare `Fixed`, `Slow`, or `Fast` lifetime-plasticity
bands. Fixed sections do not learn during life. Slow sections can adapt without
quickly erasing inherited skills. Fast sections support personal association,
episodic, working-memory, and lexicon learning. Promotion may distill audited
population improvements into a future foundation version, but an individual
parent's acquired weights are not silently copied into offspring.

At genetic birth, the child inherits foundation identity, compatible structural
genes, endocrine traits, and sparse genetic deltas. Lifetime weights, episodic
or semantic memories, learned language bindings, eligibility, and transient
state are not inherited. Cross-foundation mating is allowed only inside an
explicit compatibility family, using persistent logical addresses and a
declared child foundation. This is the default Baldwinian boundary; experimental
Lamarckian modes require separate, visible provenance and are not Foundation V1.

## ADR-028: Grounded Language and Narration Remain Neural

Decision: N2048 uses `LanguageCodebookV1`, a limited compositional vocabulary of
256 stable logical codes that is independent of neuron indices and packed GPU
offsets. The codebook supplies pronounceable symbols and grammatical roles, not
inherited object meanings. Ecological nouns, action words, names, aliases, and
dialects are grounded during life through ordinary perception, action,
demonstration, and sealed outcomes.

Player, creature, and teacher utterances are spatial world events. Player text
may be normalized into at most 16 existing or novel tokens, but it never creates
an action score, target instruction, or reward. A named utterance is addressed;
an unnamed utterance is heard only by creatures in physical range, subject to
noise, attention, and hearing ability.

Self-narration is an authentic neural act. The world exposes one unscored
`Vocalize` opportunity when legal. Only after it wins does the GPU speech head
select the speech act and up to six tokens. The world validates, charges, and
broadcasts that literal payload. Other creatures hear the raw token sequence
selected by the speaker, never polished SLM output.

The local SLM has separate developmental-prior and translation request schemas.
It may weakly scaffold learning or map human surface text to already bounded
tokens, and may render raw creature tokens for the player with uncertainty. It
may not author creature thought or speech content, issue actions, choose world
targets, inject reward, or activate hidden concepts. Unaided and SLM-assisted
language evidence remain separate.

## ADR-029: Persistent Neural Identity Enables Function-Preserving Growth

Decision: Every phenotype publishes persistent neuron, projection, synapse, and
decoder addresses. A neuron is addressed by lobe identity and ordinal, while
projection and synapse addresses derive from stable logical endpoints and route
identity. The canonical address map is bound by BLAKE3-256. Packed GPU offsets
remain runtime-local.

`GeneticRebuild`, `DurableLearnedFounder`, and `ExactResume` are distinct
checkpoint modes. Durable founder cloning preserves consolidated learning,
long-term semantic/episodic content, learned language, and provenance while
clearing activations, eligibility, working memory, injuries, age, current
targets, and world-local bindings.

Research growth appends capacity after each preserved lobe prefix. Migration
occurs only at a sealed boundary, completes pending consolidation exactly once,
creates a rollback checkpoint, maps all old state by persistent address,
initializes expansion neurons and bridges dormant, and requires same-adapter
selection identity with logit delta at most `1e-6` before atomic handoff.
`N4096Research` remains unpromoted.

## ADR-030: Durable Creature Archives Precede Retirement

Decision: Every creature receives an immutable genetic archive before GPU
insertion. The archive binds genome, foundation assets, ABI and codebook
identities, lineage, provenance, and passive life statistics in a content-
addressed profile-local store. Selected elites may additionally receive
quota-bound durable learned checkpoints; pinned checkpoints are never
automatically evicted.

A dying creature is archived before its GPU handle is scrubbed or its world
entity despawns. Final outcome sealing, life-stat commit, optional learned-state
capture, and retirement receipt all precede handle retirement. A crash-rebuilt
index must recover the immutable manifests and content-addressed assets.

Founder creation supports genetic founders by default, explicit learned
mind-state clones for selected elites, and mutated genetic offspring. Cross-run
ranking uses exposure-aware passive statistics plus active survival, reversal,
retention, grounding, narration, communication, and generalization challenges.
Missing exposure is `Unknown`, never a zero score. Export/import bundles are
digest-checked, bounded, traversal-safe, and preserve complete founder
provenance across saves.
