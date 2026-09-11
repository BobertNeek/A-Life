# alife_core

Engine-independent cognitive contracts for A-Life.

This crate owns IDs, math/data primitives, brain class specs, lobe layouts, genome and endocrine skeletons, action and sensory ABI markers, experience headers, lineage manifests, and backend/provider traits. It must not depend on Bevy, wgpu, Avian, renderer types, OS windowing, Python runtimes, or LLM vendor SDKs.

Biochemical graphs validate and compile their species indexes, reaction coefficients,
and receptor groups when constructed or loaded. Saves contain genetic source data,
not compiled caches. Receptors sharing a target combine mean nominal, excitatory,
and inhibitory responses, with inherited analogue or digital thresholds and gains.
New founders use chemical receptors to regulate upkeep and repair across the existing
six organ roles. Repair consumes organ energy. Older graphs without these receptors
retain their previous upkeep and sleep-repair behavior.

The passive `MeanBrainAtp` metric retains the legacy `EnergyStability` wire name;
its meaning and accumulated values are unchanged.
