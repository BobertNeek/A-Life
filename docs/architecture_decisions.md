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
