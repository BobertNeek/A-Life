# Biological-drive role audit: prospective correction, no activation

Source-only audit, 2026-11-08 UTC. This source audit adds no code changes, native exposure or default activation. It is separate from the bounded V2/V3 runs; those frozen results remain unchanged. Line references below are to the hashed source snapshots listed at the end, not a claim that their containing checkout is a clean commit.

## Finding and erratum

**Drive lane 9 is boredom, not praise or a generic informational/reward lane, when the expressed graph contains `with_play_stimulation`.** That constructor maps chemical species 22 to Drive Extension0, adds Awake +0.0002 and PlayStimulation -0.2, and assigns biological value -0.08. Praise is separately **hormone lane 9**, Endocrine Extension0, species 21, PlayerReward +0.6 and hormone value +0.3. Praise clearance is deliberately not punishment. Sources: `biochemical_graph.rs:995–1074`; `chemistry.rs:60–75`.

V3's native recipe excludes boredom twice: its validator forbids nonzero magnitude[9], and its compiled pressure transform returns zero for lane 9. Changing only the coefficient cannot fix it. This is a mismatch in the experimental need port, not evidence that boredom is absent from organism physiology.

## Constructor scope: do not confuse reference, newborn and fixture

- **R: reference graph.** `BiochemicalPhenotype::early_mammal_reference` (`biochemical_graph.rs:1080–1243`) establishes species 1–19, ordinary drive receptors, reference emitters and default value profile. It does not itself install boredom, praise, waking recovery or health distress.
- **C: canonical newborn assembly.** `create_new_game_with_identity` constructs `CreatureGenome::early_mammal_founder`, adjusts metabolic turnover, calls `calibrate_inherited_founder_biochemistry` and `enable_inherited_newborn_nociception`, then expresses the genome (`alife_world/src/new_game.rs:182–254`). Calibration changes hunger emitter +0.18 to +0.03 and repair nominal to +0.02; installs waking recovery, health distress, player reward and play stimulation on both alleles (`:382–431`). Nociception sets the Damage→Pain developmental expression floor to 1 (`:434–463`). These are constructor transformations, not load-time rewrites or universal properties of all accepted/saved/evolved graphs.
- **F: V2/V3 source fixtures.** Both `crates/alife_training/tests/cpu_need_association.rs` call `create_canonical_new_game_with_n2048_candidate` twice (V2 lines 20/78; V3 lines 21/79). This routes through `create_n2048_candidate_inner` to the same canonical assembly (`new_game.rs:92–119`). The three inspected copies of `new_game.rs` have identical SHA-256. Thus these fixture constructors include C's additions; they are not bare R graphs. V3 then supplies an explicitly engineered neural asset and fresh scene. Its declared asset SHA-256 is `1d2978cbd857a7ca99dc1b2cf65be9f6ff417a6f78199282e0431b94332c9377` (`tests/cpu_need_association.rs:15,21–79`). This audit read the assertion/source; it did not regenerate or measure the asset.
- Actual lane concentrations depend on expression, chromosome modifiers, state and elapsed time. A reference baseline or emitter gain is not a measured initial concentration. Inspect and bind the **actual expressed phenotype** prospectively. Do not assume constructor names, species count or an extension index establish semantics.

## Lane/species/sign mapping

ABI: `chemistry.rs:106–150`; reference emitters/receptors: `biochemical_graph.rs:1080–1170`; species IDs: `:2017–2038`. Values are inherited change valuation, not all direct reward terms or all motor priorities.

| Lane | Role and reference species | Sources/signs, including scoped C changes | Biological value | Current V3 pressure |
|---|---|---|---|---|
| 0 Hunger | Excess need, species 1 | R EnergyDeficit +.18; Nutrition -.30. C replaces +.18 with +.03. | -1/7 | raw, magnitude 1/7 |
| 1 Fatigue | Excess need, species 2 | R EnergyDeficit +.12; SleepRecovery -.70. C removes deficit emitter, adds Awake +.00007, retention .99995. | -1/7 | raw, 1/7 |
| 2 Fear | Excess need, species 3 | R Damage +.35; inherited stress/hazard modifiers apply. | -1/7 | raw, 1/7 |
| 3 Pain | Excess need, species 4 | R Damage +.80; SleepRecovery -.45. C adds HealthDeficit +.10 above .4 and fully expressed acute damage. | -1/7 for relief; fresh injury separately valued | raw, 1/7 |
| 4 Loneliness | Excess need, species 5 | R SocialContact -.60; baseline/retention restores tone. C social-contact expression floor 1. | -1/7 | raw, 1/7 |
| 5 Curiosity | Regulatory/novelty signal, species 6 | C PerceivedNovelty +.04; Investigation -.08, full expression. | 0 | excluded deliberately |
| 6 Brain ATP | Availability, species 7 | R nutrient conversion supplies ATP; EnergyDeficit -.12, SleepRecovery +.30. C changes deficit to -.02 and removes sleep ATP emitter. | +1/7 | max(.75 - raw, 0), 1/7 |
| 7 Temperature stress | Excess need, species 8 | R body TemperatureStress +.30. | -1/7 | raw, 1/7 |
| 8 Reproductive drive | Appetitive opportunity/readiness, species 9 | R MatingOpportunity +.85. | 0 | excluded deliberately |
| 9 Extension0 | Boredom ONLY under verified play binding, species 22 | C Awake +.0002; PlayStimulation -.20; retention 1, full expression. | -.08 | **mistakenly excluded** |
| 10 Extension1 | Unbound in R/C | No reference/C drive receptor; derived frame starts at zero. Mutations may bind it. | 0 | excluded while unbound |

Reference drive receptors: threshold 0, gain +1, nominal 0 (`biochemical_graph.rs:1961–1971`). Chromosome traits can alter hunger/fatigue thresholds, hunger/fear gains and emitter gains (`:630–729`). Receptor groups aggregate positive and negative inputs with clamping (`:1762–1801`); semantic binding must cover actual receptors, not just species IDs. Derivation initializes unbound channels to zero (`:1473–1516`). Chemistry decays toward expressed baseline, not always zero (`:1380–1396`).

## Downstream relevance and legitimate rest

All 11 lanes have topology bindings (`topology.rs:3225–3244`); homeostasis enters interoception (`phenotype/io_compile.rs:653`). **This establishes input/binding presence only, not directed or effective recurrent reachability, useful weight signs, dynamic influence or learning.** The innate decoder shortcut and experimental auxiliary pressure route are distinct from recurrent coverage. The [earlier archived full graph audit](https://github.com/BobertNeek/A-Life/blob/c775c8c92e8f5e625f8a3e861ef79200199b54df/docs/experiments/weight-space-2026-11-07/README.md#L74-L92) found hunger neuron 256 → Reserve 1968–1983 → Temporal 1216–1231 ending in a terminal 33-neuron component despite apparent macro-lobe paths, because sampled tile boundaries did not overlap. That is archival evidence, not a repeated graph experiment in this source audit. A separate repaired 137-tile fresh graph must not be attributed to all defaults. Each evaluated founder requires its own neuron/edge-level directed reachability and perturbation/dynamics checks; this report does not conclude that all drives are correctly wired from lane presence. Curiosity still affects salience despite zero pressure weight (`chemistry.rs:730–739`). Reproductive drive participates in readiness gating (`biochemistry.rs:638–645`).

Reference innate priorities selectively prime hunger→approach/ingest, fear/pain/temperature→avoid, fatigue→rest; emergency drives suppress Idle/Inspect/Contact/Other (`phenotype/io_compile.rs:660–710`). They use urgency above .55 plus physical-cue/reach gates (`phenotype/decoder.rs:82–132`). These are not reflexes for every drive. Fatigue/pain/fear/ATP also modulate motor confidence, and low ATP/fatigue/sleep pressure trigger recovery (`chemistry.rs:741–780`). C waking recovery adds fatigue→locomotor capacity and sleeping→reduced metabolic rate (`biochemical_graph.rs:914–974`).

The native port adds bounded comparative own-history advantage to candidate logits, not an action command (`cpu/need_association.rs:541–555`). Preserve negative advantage, ties, unknown support and real Rest/NOOP alternatives. No activity quota or endless-action objective is justified.

**Cost boundary:** raw 11-drive deltas omit a distinct body/organ reserve-debit lane. The probe owner reports that final post-charge cognitive work debits body/organ reserve retained in full biology. Therefore flat drive axes are not evidence of cost-free action. Keep body/organ energy, injury and actual work receipts alongside drive changes in future controls; do not invent an unmeasured drive delta or conflate the pressure port with complete biological valuation.

## Prospective minimal versioned correction

Leave V2/V3 frozen. Introduce a separate recipe/port identity with a phenotype-validated role binding, for example `NeedAxisRole::{ExcessAbove{target}, DeficitBelow{target}, RegulatoryOnly, AppetitiveOpportunity, Unbound}` and `compile_for_phenotype(binding, recipe)`. Bind actual expressed graph/receptors, value profile, lane interpretation and targets into identity. Unsupported/mutated semantics fail closed or explicitly disable that lane, rather than silently assuming reference semantics.

For verified C/F biology only: preserve existing pressure magnitudes 1/7, add boredom increasing-pressure transform with magnitude .08, preserve curiosity/reproductive exclusions and unbound Extension1. .08 matches the existing biological coefficient (.56 of 1/7), a conservative starting scale, not an optimum. Preserve ATP target .75, knots/basis, gain/ceiling and normalization for a single-variable comparison unless independently versioned. ATP target .75 remains an experimental gene, not a target inferred from any positive value coefficient. An opportunity or generic reinforcement signal is not automatically shortage/excess pressure.

Coordinated V3 source sites for a future implementation: `need_association_native.rs:169–205` recipe/validator; `:359–367` pressure transform; `:320–324` fixed chemistry identity; `:846–858,1372–1374` scalar oracle and rejection test. The existing source-context phenotype hash (`cpu/need_association.rs:45–85`) protects history matching but does not validate recipe semantics.

## Required controls and evidence limits

0. **Recurrent coverage:** independently verify actual expressed neuron/edge-level paths from each input to relevant motor and learning targets, plus signed perturbation responses. Distinguish default graphs, the separately repaired graph, innate shortcuts and the auxiliary port. Topology registration alone does not pass this check.
1. **Unit/native operators:** one-axis signs, successor clipping, ATP target boundaries, equal-action/reference zero, boredom relief/worsening, no negative boredom; curiosity/opportunity/praise-only changes cannot become pressure relief; reject changed role bindings; preserve unknown/product-confidence gates and full/active operator parity. Match frozen normalization rather than simultaneously tuning gain.
2. **Physical:** actual awake/play boredom rise/relief; novelty/investigation dissociation; rest recovery under fatigue; nutrition/social/damage/thermal effects; expression and saturation; work costs and fresh injury. Test conflicting needs, not just each lane alone.
3. **Native:** exact requested/adapted/executed receipts, real stored/retrieved histories, supported zero versus unsupported zero, base logits/per-axis differences/final readout/action. Current observed candidate admission is Inspect plus Rest/NOOP (`need_association_source.rs:113–151`); Play requires truthful new action/receipt support and physical opportunity, not merely a coefficient.
4. **Behavioral limits:** The [bounded V2 result](https://github.com/BobertNeek/A-Life/blob/2afb42ca08d03aaeb584c6aa63eec458031bf753/docs/experiments/native-need-association-v2/RESULTS-v2.md) records 12 decisions and zero stored associations after binding rejection. The separate V3 role erratum records the curiosity/boredom-only changes. This source audit does not repeat those lives or treat absent pressure as learned-policy evidence. These facts diagnose what was omitted, not learned-policy competence. Frozen V3's reserved support/retrieval check is not broad competence or weight optimization. No claim here evaluates the external experiments 4/5/6 independently.

## Source provenance

The manifest below hashes exact source bytes read for this report. Base commit IDs identify checkout context only; V3 contains working-tree additions/changes. Accepted-main `10eecff4` adds separate retained-credit repairs; the exact inspected graph and V3 bytes are identified below rather than inferred from a checkout name.

### Accepted component source snapshot

Base commit: `ab69f2d32f9ef981862d610ea3450a66d1b1e199`.

- `crates/alife_core/src/chemistry.rs`: `9f7cb4b996c2e25a7e5bec8c948de07e81ff14bbc98dcb1f14a2bf8fbb29603c`
- `crates/alife_core/src/biochemical_graph.rs`: `ee739b3774c111f2633da570b9da10fd5a523affa771d01fdcb924b4003ff36f`
- `crates/alife_core/src/biochemistry.rs`: `0d5ff72f5c7e7305daf4b53f150ce116a9f85ce49738ad8746c4bbfc99f24f2b`
- `crates/alife_core/src/experience.rs`: `b97d3241ed0b07706854ced229ffc12abf3d4b75783f4abb7e666b546ae199d4`
- `crates/alife_core/src/topology.rs`: `16478d645fac7d8288cad98f4fd5b6d11af6c8933bcffd62395125c470ec6e8a`
- `crates/alife_core/src/phenotype/io_compile.rs`: `a51728c93c3419fdd798b092d4b1189c90e05a33f01e10d606c91830213e0de5`
- `crates/alife_core/src/phenotype/decoder.rs`: `7c2c035a87f0f79501343f006251c78108fe5df6c3d873320e85dda18552b543`
- `crates/alife_world/src/new_game.rs`: `a925cc546e267b83d52cd8c2a000bc41af030d8f404784fb04c981cd009fe2ee`

### V3 source snapshot

Base commit: `c775c8c92e8f5e625f8a3e861ef79200199b54df`.

- `crates/alife_core/src/need_association_native.rs`: `a487a016cd725052770cd934cc0ff9f400045dbbb244a8773b4269ddd1162729`
- `crates/alife_training/src/cpu/need_association.rs`: `63a59dbb7c6fcc441f17561398e2b560ed7814815c0ed16946acf009dc479be4`
- `crates/alife_training/src/cpu/need_association_source.rs`: `89cb73c3eb445ba162e17c3aa36eed15dd0d7b8a325cca95265ce463a509931c`
- `crates/alife_training/tests/cpu_need_association.rs`: `74f213501692c633ec15130a1754e51a0f1a813ef8d7546f0547fb0f7b76416d`
- `crates/alife_world/src/new_game.rs`: `a925cc546e267b83d52cd8c2a000bc41af030d8f404784fb04c981cd009fe2ee`

### native-need-association-reconstruction-v2

Base commit: `c775c8c92e8f5e625f8a3e861ef79200199b54df`.

- `crates/alife_training/tests/cpu_need_association.rs`: `667d219e684e75cb5f873e5e6a143de0c6e6c9e77898ed98f1f55616fc6f879d`
- `crates/alife_world/src/new_game.rs`: `a925cc546e267b83d52cd8c2a000bc41af030d8f404784fb04c981cd009fe2ee`
