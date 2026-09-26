# Genetics repair — 2026-09-26

Scope: existing diploid CreatureGenome, ordinary world births, and projections of its traits.
Architecture: AOA-GEN-001 through AOA-GEN-008, AOA-GEN-010, AOA-BODY-003,
AOA-AUTH-001 through AOA-AUTH-005. This does not claim completion of de novo
structural evolution or promotion of larger brain classes.

## Inheritance and expression

The six chromosome families remain diploid: each child receives one recombined
homolog from each parent. They are six linkage groups, not one literal chromosome
pair. Numeric traits blend. Dominant/recessive category pairs express the dominant
value; equal-strength or explicitly codominant categories combine through actual
trait consumers: body-frame bulk averages, mate response averages, and starter
sound familiarity unions. Mutations assign the new category its declared default
dominance instead of accidentally retaining a different category's tag. Explicit
breed alleles can still specify their own dominance.

Matching biochemical parameters cross over with the same bounded linked-gamete
selector as the scalar chromosome genes. Species baseline/decay, reaction rate,
emitter threshold/gain/developmental floor, receptor threshold/gain/setpoint,
neural-emitter threshold/gain, and every valuation coefficient can mutate.
Unmatched topology remains one coherent structural allele; stoichiometry, species
bounds, discrete wiring, and cadence are preserved rather than randomly spliced.
Matching copies blend at expression. No live concentrations are inherited.

Compact brain architecture and learning parameters now reside as two homologs in
the existing Brain chromosome. Counts and rates, including chemical plasticity
receptor weights, inherit/recombine within class limits rather than resetting to
scaffold defaults at birth. Named action-credit policies remain coherent blocks.
They inherit from both homologs and express one whole allele using the stable
conception seed; exact foundation-specific overrides remain unchanged. Counts
cannot exceed their brain class. Construction parameters
are optional/defaulted in old JSON genomes and omitted when canonical; numerical
mutation addresses now support u16 loci, with legacy provenance digest bytes
preserved for old small addresses. The mutation receipt budget remains bounded.

## Cause and effect

| Traits | Actual consumer |
| --- | --- |
| Size, frame, hue | Size/load/body schema and existing renderer-neutral body-size/palette projection; frame alters bulk and resistance. |
| Metabolic efficiency, turnover | Food yield/reserves/upkeep; positive logarithmic turnover affects spending and repair. Legacy zero turnover alleles can now mutate too. |
| Movement efficiency, sensory acuity, reflex strength | Embodiment motor/sensory gain; movement efficiency also changes locomotor upkeep. Reflex affects controllability, not forced actions. |
| Lifespan, injury resistance, temperature tolerance | Age limit, injury/repair, thermal stress. |
| Stress baseline, hormone production/decay, bonding sensitivity | Baselines and production/decay in the existing biochemical graph, including neural emitters. Material reserves are not scaled as hormones. |
| Reward sensitivity | Chemical appetitive learning-receptor gain; no free reward for a hormonal rise. |
| Hunger/fatigue thresholds | Chemical drive-receptor thresholds. Higher threshold requires stronger chemical input. |
| Sleep threshold | Actual brain sleep-pressure trigger, independent of sleep maturation readiness. |
| Reproductive threshold | Existing physiological reproductive readiness. |
| Brain ATP efficiency | Neural-support upkeep and actual cognitive energy debit. |
| Food attraction, hazard aversion, social attention, novelty bias | Hunger/fear sensitivity, social chemical response, curiosity baseline. These influence motivation, not world candidate scores. |
| Starter vocabulary | Innate perceptual familiarity with heard token sounds; no inherited learned word meanings, memories, or forced speech. |
| Maturation, puberty, sensory/lobe/chemical activation | Existing developmental capability/expression gates. |
| Critical-period opening/closing | Derived window is bounded and at least one tick long despite adverse combinations. |
| Fertility | Deterministic probability of conception when either parent's reproductive cadence refreshes and all existing physiological/world gates pass. |
| Parental investment | Actual parental reserve transfer to offspring, with a parental reserve floor. This is birth provisioning, not an NPC care controller. |
| Mate preference | Chemical response to an encountered compatible mate; health/similarity/novelty combinations do not select an action. |
| Crossover probability/segment count and mutation controls | Existing linked-gamete crossover and bounded mutation, also used for chemistry and compact brain settings. |

Age limit and food-reserve horizon use an independent reference lifetime scale,
modified by lifespan/load genes. Maturation no longer shortens both by accident.
The default reference preserves founder economics; it is not a wall-time survival
countdown. Energy expenditure still runs through inherited turnover.

The previous `genetic_weight_bias` public field is named `genetic_weight_variation`
to describe its procedural-seed effect; its legacy JSON key still loads and saves.
For exact trained foundations this variation is inactive, as are N2048 lobe-ratio
and connectivity-layout modifiers. Brain class is compatibility constrained.
Migration checkpoint expresses readiness only: it cannot bypass N4096 promotion.
These are explicit restrictions, not claims that unused architectural evolution
has been implemented. Exact foundation weight bytes remain unchanged.

The legacy endocrine projection is diagnostic only. Live physiology has one
owner: the expressed biochemical graph. Scalar chromosome modifiers are centered
on the existing founder defaults and applied through bounded exponential gains.

## Birth failure and persistence

Nonviable/incompatible conception leaves the world clock and existing organisms
advancing. It allocates no child and charges no provision. Unexpected invariant
errors still abort the enclosing transaction. Late spawn/admission failure retains
existing transaction rollback. Child identities and fresh acquired state remain
owned by the existing newborn path.

Saved and newborn visual summaries project hue/size from authoritative biology,
while preserving separate existing cosmetic details. This repair does not convert
the legacy cosmetic bucket representation into a new diploid chromosome system.

Older JSON genomes still decode with default construction fields. This does not
prove every old world checkpoint resumes: cached phenotypes with nondefault genes
can differ after these expression repairs and fail strict snapshot validation.
No existing saves or training artifacts were rewritten or silently migrated.

The production build also exposed a missing `Look` action in the Bevy adapter.
It now preserves that action without locomotion; gaze remains world-owned.

## Verification

Use the existing genetics, valuation, development, and N2048 ABI test targets plus
the focused world-birth module. Two combined genetics regressions exercise trait
consumers and 32 maximally mutating, serialized offspring; one world regression
covers fertility, nonviable conception, and reserve transfer. Reuse the existing
birth rollback and offset-age tests. Compile the production feature path and run
the repository quick boundary/document checks. This is not a GPU breeding
campaign or a rendered visual playtest.

Recorded results: 46 core tests passed (genetics 16, valuation 4, development 11,
N2048 foundation ABI 15), all seven focused world-birth tests passed, and the
existing action-adapter regression passed with a new Look assertion. Only three
new test functions were added. The production voxel frontend library compiled;
four unrelated unused-import/dead-code warnings remain. Repository quick checks
passed both the core boundary check and all 77 documentation assertions.
