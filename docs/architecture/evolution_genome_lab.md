# P33 Evolution Genome Lab

Status: v0 offline tooling contract.

P33 adds deterministic offline breeding and mutation over valid `BrainGenome`
records. It is not part of active gameplay by default. The implementation lives
in `alife_tools::p33_evolution`, consumes `alife_core` contracts, and keeps
runtime crates independent from offline research/tooling code.

## Genotype

The genotype is the inherited `BrainGenome`: brain class, lobe ratio plan,
macro-connectome masks, sparse density priors, alpha/plasticity masks,
endocrine constants, drive thresholds, sensor layout, motor affordances,
mutation rates, crossover policy, developmental schedule, and inheritance
policy.

Mutation operators must validate the child immediately. A mutation that would
create invalid lobe ratios, invalid alpha bounds, non-finite endocrine values,
or dynamic active-loop resizing pressure is rejected instead of emitted for
later repair.

## Phenotype

The phenotype is the running creature assembled from the genotype plus birth
initialization, world state, sensory history, learned lifetime layers, memory,
topology, and homeostatic state. P33 selection only edits inherited genome
contracts. It does not copy active creature state into genetic baseline.

## Lifetime Consolidation

Lifetime learning remains in `W_lifetime_consolidated`, `H_operational`, and
`H_shadow`. P33 lineage records include `lifetime_state_inherited: false` by
default, and smoke tests assert the selection lab does not leak lifetime state
into offspring genetic priors.

The existing Lamarckian flags in `InheritancePolicy` remain explicit opt-ins.
P33 does not enable an experimental Lamarckian mode; if a later plan enables
one, it must add separate tests proving the mode is explicit, logged, and
ablatable.

## Fitness From Packed Logs

The P33 fitness summary reads versioned P11 packed-log records and computes:

- survival time from packed tick span
- energy stability from signed energy deltas
- food success from successful packed actions
- pain avoidance from packed pain deltas
- curiosity resolution from prediction error
- social/word task score from heard-token and teacher/school side records
- optional teacher verifier score from teacher/school side records

Packed logs remain export/replay data. They do not replace sealed runtime
`ExperiencePatch` records.

## Optional P32 Weight Assets

P32 is developed concurrently, so P33 does not import or guess P32-specific
types. The genome lab accepts a generic `BirthWeightInitializerRef` containing
an asset id and schema version. It must be `birth_only: true`. These references
are lineage metadata for birth initialization only; they are not runtime
dependencies and do not authorize inheriting lifetime-learned state.

## Determinism

The offline lab uses a small deterministic seed mixer and no external RNG
dependency. Same seed, same parent set, and same fitness summaries produce the
same survivor ordering and offspring records.
