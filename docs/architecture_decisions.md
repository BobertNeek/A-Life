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
