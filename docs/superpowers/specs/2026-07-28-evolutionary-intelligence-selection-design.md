# Evolutionary Intelligence, Artificial Selection, and Creature Biology Design

**Status:** Approved design

**Date:** 2026-07-28

**Controlling architecture:** `docs/master_spec.md` and ADR-024 through ADR-030

**ADR impact:** No new ADR is required. This design applies ADR-002, ADR-007,
ADR-024, and ADR-027 through ADR-030 to the post-foundation program.

## 1. Purpose

A-Life will develop intelligence through embodied life, inheritance, learning,
social transmission, and deliberate artificial selection. The near-term target
is not AGI. It is a fun creature game whose best lineages approach lab-rat-level
adaptive intelligence in simplified worlds and exceed Norn-style basic speech.

Later eras may develop pack behavior, hunter-gatherer-style social structures,
formal schooling, abstract reasoning, and rare ascended individuals. These are
promotion-gated research directions. They are not claims about current ability.

## 2. Core principles

1. Intelligence must control behavior through the GPU-authoritative neural loop.
2. Selection rewards behavior, learning, transfer, and social contribution. It
   does not reward a hidden intelligence label.
3. Artificial selection may accelerate evolution, but ecological competence,
   provenance, diversity, and held-out evaluation remain visible.
4. The player controls habitats, breeding, tests, education, and selection.
   The player does not secretly choose a creature's neural actions.
5. Ordinary offspring inherit DNA and the current species foundation. They do
   not inherit personal memories or lifetime weights.
6. Foundations may distill audited population improvements between eras. This
   is an explicit Baldwinian update, not silent Lamarckian inheritance.
7. Brain capacity increases only after the current class reaches a measured
   behavioral ceiling. Population falls according to measured hardware cost.
8. Wild and managed creatures use identical cognition.

## 3. Evolutionary eras and scaling

Each era has one promoted brain class, a measured population budget, ecological
goals, intelligence tests, and a promotion gate. Promotion requires stable gains
across unseen worlds and diverse lineages. One remarkable creature does not
promote an entire era.

At population promotion:

1. Select several strong and genetically diverse lineages.
2. Create a breeding population in the larger class.
3. Preserve genome traits, foundation provenance, and lineage identity.
4. Keep ordinary offspring free of parental lifetime memories.
5. Transfer culture through parenting, peers, schools, and world artifacts.
6. Set the new population cap from measured GPU and simulation cost.
7. Archive the prior population for later comparison or reintroduction.

Late-game individual ascension is separate. A selected living creature may
receive function-preserving brain growth, retain its learned identity, and enter
advanced education. Ascension remains rare, expensive, and promotion-gated.

## 4. Creature DNA

`CreatureGenome` composes stable, versioned chromosome groups:

- body genes: size, metabolism, senses, movement, fertility, lifespan, injury
  resistance, temperature tolerance, and appearance;
- brain genes: class, lobe ratios, connectivity, density, neuron dynamics,
  receptors, inherited weight biases, and plasticity;
- chemistry genes: hormone baselines, production, sensitivity, decay, drive
  thresholds, reward sensitivity, stress, bonding, sleep, and reproduction;
- development genes: maturation, puberty, lobe and sensor activation, critical
  periods, and safe migration checkpoints;
- reproduction genes: fertility, mate preferences, crossover, mutation, and
  parental-investment tendencies;
- predisposition genes: starter vocabulary, reflexes, attraction, aversion,
  social attention, and other trainable biases.

The initial species-wide vocabulary is deliberately small. It may include name,
attention, food, danger, come, stop, give, rest, and help. Creatures ground richer
meanings during life. Populations may later develop aliases and dialects.

The genome uses two alleles per locus. Continuous traits blend both alleles.
Discrete traits use dominant, recessive, or codominant expression. Sexual
reproduction performs bounded crossover, allele selection, and mutation before
phenotype compilation. Structural mutation may later duplicate, disable, or
alter compatible modules.

Epigenetic state may affect development within one life. Heritable epigenetics
is optional, provenance-tagged, ablatable, and disabled by default.

Genes bias development and neural learning. They never issue action commands.

## 5. Biochemistry

Biochemistry has three layers.

### 5.1 Body state

Body state records physical truth such as energy, nutrition, hydration,
temperature, damage, toxins, fertility, pregnancy, and neural energy cost.

### 5.2 Drives

Drives expose bodily needs to the brain. Initial channels include hunger,
fatigue, fear, pain, loneliness, curiosity, BrainATP, temperature stress, and
reproductive drive.

### 5.3 Hormones

Hormones control slower global modes. Initial channels include adrenaline,
cortisol, dopamine, oxytocin, serotonin, acetylcholine, learning modulation,
developmental hormone, and sleep pressure.

The causal loop is:

```text
world event -> body change -> chemistry -> neural perception
            -> GPU decision -> action -> measured outcome
            -> hormone response -> Hebbian/Oja learning
```

Chemistry changes salience, thresholds, motivation, attention, and plasticity.
It does not select actions. Fast signals update often. Metabolism, development,
and reproduction use slower bounded cadences.

## 6. Training across timescales

Different neural material learns at different timescales:

| Material | Change boundary | Mechanism |
|---|---|---|
| Fast weights | experience | modulated Hebbian/Oja plasticity |
| Lifetime weights | sleep | replay and consolidation |
| Developmental circuits | critical periods | scheduled high plasticity |
| Genetic structure | reproduction | mutation, crossover, and selection |
| Foundation weights | era boundary | audited GPU curriculum and promotion |
| Added capacity | promotion or ascension | migration, gated unfreezing, education |

A dormant region needs a valid route and nonzero plasticity before experience
can train it. Structural mutations create new routes between generations or at
safe offline boundaries. Active neural dispatch never resizes its own topology.

Each new era may receive a curated foundation distilled from audited prior-era
populations. Ordinary births still use genome plus starter foundation.

## 7. Selection and breeding

Selection keeps separate score vectors for:

- ecological fitness;
- individual cognition;
- learning speed and transfer;
- social intelligence;
- group contribution;
- health and developmental stability;
- compute efficiency;
- genetic novelty and lineage value.

Pareto or lexicase selection preserves different strengths. Natural breeding
continues where habitat rules allow it. The player may assign artificial
breeding rights in managed populations.

### 7.1 Cognitive introgression

A high-intelligence, low-survival creature may reproduce through a controlled
exception lane:

1. Pair it with a robust, unrelated survivor.
2. Never pair two low-survival exceptions.
3. Place offspring in a probation cohort.
4. Test cognition, survival, transfer, health, and development.
5. Compare them with sibling and population controls.
6. Return successful descendants to the breeding population.
7. Archive failed combinations with complete provenance.

Only corrupt genomes, invalid phenotypes, or failure to complete reproduction
are absolute disqualifiers. Poor ordinary survival is not.

### 7.2 Group intelligence

Persistent packs measure culture, loyalty, roles, and long-term coordination.
Randomized teams measure transferable social intelligence. Removal and
replacement trials estimate individual contribution. Selection may reward
teachers, coordinators, specialists, leaders, and successful group lineages.

## 8. Intelligence evaluation

The intelligence system has three test layers:

- permanent anchor tests measure historical progress;
- procedural tests influence ordinary breeding;
- hidden held-out tests control era promotion.

Tests measure learning speed, reversal learning, delayed memory, navigation,
causal inference, transfer, tool use, imitation, communication, cooperation,
teaching, and unfamiliar-problem performance. Test exposure and assistance are
recorded. Fixed tests never control breeding alone.

Artificial selection retains an ecological context. Novel environments,
changing rewards, energy costs, social competition, and held-out transfer make
flexible intelligence cheaper than brittle memorization.

## 9. Player and habitat modes

The player acts as breeder, teacher, scientist, and ecosystem god. Habitats,
not entire saves, define management mode:

- **Wild:** creatures choose mates, groups, territories, and migration.
- **Reserve:** creatures live mostly wild; the player may tag, test, capture,
  breed, and reintroduce selected individuals.
- **Managed:** the player controls breeding, testing, education, and membership.
- **School:** selected creatures receive structured developmental education.

All modes share one cognition implementation. Transfers record quarantine,
assistance, foundation, possession, and selection provenance.

Later player possession may provide embodied demonstration. The creature still
perceives and learns from the results, but the session is marked assisted.

## 10. First era: Norn-plus early mammal

The first evolutionary program targets flexible early-mammal behavior with
low-cost grounded speech present from birth.

Required abilities:

- self-maintenance and basic survival;
- learned food, danger, and object affordances;
- spatial navigation and delayed location memory;
- simple multi-step problems;
- reward reversal and generalization;
- individual recognition and observational learning;
- inherited starter vocabulary;
- acquisition of new grounded words;
- post-sleep retention.

Stage 1 excludes formal packs, coordinated hunting, complex tools,
hunter-gatherer economies, advanced schooling, ascension, possession, and
open-ended natural language.

## 11. System authority

Six systems remain separate:

- genome system: inheritance, mutation, expression, and phenotype compilation;
- body system: metabolism, injury, reproduction, and physical state;
- chemistry system: drives, hormones, decay, and development;
- GPU brain: perception, learning, memory, speech, and action selection;
- selection system: tests, rankings, mating permissions, and promotion;
- lineage archive: ancestry, provenance, evidence, and checkpoints.

The selection system never injects actions, thoughts, rewards, or hidden
semantic answers.

## 12. Failure containment and evidence

- Invalid genomes become recorded nonviable conceptions.
- Invalid phenotypes fail before birth.
- Dangerous chemistry produces biological distress, recovery, or death.
- One failed creature cannot corrupt the population.
- Wild and managed creatures use identical neural execution.
- Every enabled gene family must cause a relevant phenotype difference.
- Hormones must alter motivation or learning without selecting actions.
- Offspring recombination must be reproducible from recorded seeds.
- Ordinary births must clear lifetime memories and weights.
- Selection gains must transfer to unseen tests and later generations.
- Ablations must attribute gains to genes, learning, sleep, language, or social
  exposure.
- Long simulations must remain bounded on the user's machine.

## 13. Deferred scope

The full post-foundation program is recorded in
`docs/creatures_agi_roadmap_pack/EVOLUTIONARY_INTELLIGENCE_ROADMAP.md`.
Only the first approved era becomes an implementation plan next.
