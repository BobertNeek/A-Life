# P33 Evolution Genome Lab and EI0 Selection

Status: deterministic offline tooling foundation.

P33 retains a legacy `BrainGenome` smoke helper in
`alife_tools::p33_evolution`. It is not the EI0 managed-breeding substrate. EI0
evaluation lives in `alife_tools::p33_evaluation`; managed breeding lives in
`alife_tools::p33_selection`. Neither module owns runtime action, neural, world,
or reproduction policy.

## Intelligence evidence

`p33_evaluation` consumes validated `PackedExperienceRecord` traces. It keeps
learning, transfer, reversal, delayed memory, abstraction, social contribution,
and compute cost explicit. It also emits seven separate objectives:

- ecological;
- cognitive;
- social;
- group;
- stability;
- efficiency;
- diversity.

No weighted survival scalar replaces this vector. An unexposed measure is
`ScoreEstimate { value: None, samples: 0 }`, not a zero result.

The battery has permanent-anchor, procedural-breeding, and hidden-promotion
layers. A hidden trial must identify its hidden set and have zero prior exposure
and no teacher, player, semantic-prior, or hidden-reward assistance. It must also
carry nonempty source-run, foundation, adapter, backend, lineage, and compute
provenance. Promotion requires all hidden domains plus individual, persistent
pack, and randomized-team coverage. Any benchmark-gaming, fixed-answer, or
group-free-rider flag blocks promotion.

## Real fixture boundary

Run the committed scenario manifest with:

```powershell
cargo run -p alife_tools --bin p33_genome_lab -- evaluate-fixture `
  --fixture crates/alife_tools/tests/fixtures/p33_ei0_real_battery.json `
  --out crates/alife_tools/reports/ei0_real_fixture_report.json
```

Every manifest entry maps one actual `ScenarioFixture` run to the single phase
it honestly represents. The adapter does not duplicate one scenario and seed
under invented phase names. The committed run supplies nine real packed records
from all eight scenario families.

Its adapter backend is `HeuristicBaseline`. It proves deterministic fixture,
packed-log, report, and hostile-flag tooling. It is not GPU neural evidence and
cannot promote a foundation. Current fixtures lack genuine baseline/acquisition,
transfer, reversal, delayed recall, group removal, and replacement variants.
Learning, transfer, reversal, delayed memory, abstraction, social contribution,
and dependent cognitive/social/group objectives remain `UNKNOWN`.

## Managed composite-genome selection

`run_managed_selection` receives evaluation metadata beside the authoritative
`alife_core::CreatureGenome`. It validates managed candidates and excludes wild
genomes from every pairing while preserving exact wild genome records.

The retained pool is the union of:

- the Pareto frontier across all seven objectives;
- deterministic seeded lexicase order;
- one representative for each minority lineage;
- the strongest holder of each declared specialist role.

Pairing rejects equal lineages, direct descent, shared parents, shared known
ancestors, and incompatible foundation families or brain classes. Evaluation
ancestry metadata extends the core direct-parent IDs; it is not a second genome
contract.

## Authoritative offspring path

Managed breeding never calls legacy `p33_evolution::crossover_genomes` and
never crosses `BrainGenome` records. The only offspring path is:

1. Validate both composite parents through `CreatureGenome::validate_contract`.
2. Call `CreatureGenome::reproduce` with a deterministic nonzero conception seed.
3. Validate the returned genome and its `GeneticLineageProvenance`.
4. Call `CreatureGenome::express` and build its mature development state.
5. Compile the expressed brain through `PhenotypeCompiler`.
6. Record the core parent IDs, provenance, phenotype hash, neuron count, and
   synapse count.

A reproduction, expression, or compilation failure rejects the offspring.
Lifetime state is never copied into the genetic baseline.

## Cognitive introgression and probation

A high-cognition, low-ecology candidate may reproduce only with a robust,
unrelated mate. Two fragile candidates are never legal partners. The exception
generates at least two deterministic siblings. Each child receives elevated
probation metadata requiring cognition, ecology, transfer, stability/health,
and development checks. Its receipt names the other sibling and viable
non-parent population controls.

Persistent packs measure retained group coordination. Randomized teams measure
transferable social contribution. Removal and replacement evidence remains
required before group contribution is known; absent evidence stays unknown.
