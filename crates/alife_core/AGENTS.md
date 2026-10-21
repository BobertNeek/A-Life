# alife_core

Own engine-independent organism, genome, phenotype, chemistry, learning, language,
and persistence contracts. Keep Bevy, wgpu, UI, and provider integration outside
this crate. Brain classes scale; `Standard2048` is a reference class.

Preserve stable logical identity across storage/compiler changes. Packed GPU
offsets are runtime lookups, not durable identity. Genetic birth does not inherit
lifetime weights, memories, learned language, eligibility, or transient state.
Loading chemical state must not reinitialize stored concentrations.

Use the existing foundation and checkpoint codecs. Keep genetic assets separate
from acquired state, and preserve native founder reconstruction and save/load.
World actions remain subject to world validation; diagnostic memory/topology
sidecars do not select actions.
