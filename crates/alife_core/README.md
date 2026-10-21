# alife_core

Engine-independent cognition and biology for A-Life.

This crate implements genomes, phenotype compilation, biochemistry, learning,
memory, language, and portable brain data. It also defines stable IDs, math
primitives, action and sensory ABIs, and private semantic-prior messages.
World simulation, graphics, GPU execution, and external providers live in other
crates.

Biochemical graphs validate and compile their species indexes, reaction coefficients,
and receptor groups when constructed or loaded. Saves contain genetic source data,
not compiled caches. Receptors sharing a target combine mean nominal, excitatory,
and inhibitory responses, with inherited analogue or digital thresholds and gains.
New founders use chemical receptors to regulate upkeep and repair across the existing
six organ roles. Repair consumes organ energy. Older graphs without these receptors
retain their previous upkeep and sleep-repair behavior.

The passive `MeanBrainAtp` metric retains the legacy `EnergyStability` wire name;
its meaning and accumulated values are unchanged.
