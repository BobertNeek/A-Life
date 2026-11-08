# Creatures drive functions as a Qualia design target

The intended target is the useful C3/DS functional architecture, adapted to Qualia rather than cloned by chemical number. Distinct needs, grounded opportunities, learned relief, reversible priorities and healthy inactivity matter more than matching a table.

## Scope and evidence

- Qualia source inspected: commit **52a299a2b02adfe2168395c8e20fc2834d3ebfa3**. All Qualia paths below are repository-relative. This is a frozen source inspection, not a claim about a later integration tree.
- Read-only source analysis. No application execution, compilation, tests, native lives, or behavioral qualification was performed for this report. Only this report was written. No acquired creature state, checkpoints, credentials, or other private state is included.
- Creatures engine artifact inspected: archived source commit **321245bed27cbacb52421d95c9ff4f0bf67a49eb** in `videogamepreservation/creatures3`. It establishes the archived engine contract, not identical behavior across every C3/DS release or genome.

## Verified Creatures corrections

The supplied table is directionally useful, but two numeric ranges need correction:

| Item | Verified result | Source |
|---|---|---|
| Drive indices | 0–19, with the supplied name ordering | [Engine drive enum](https://github.com/videogamepreservation/creatures3/blob/321245bed27cbacb52421d95c9ff4f0bf67a49eb/engine/Creature/CreatureConstants.h#L58-L80) |
| Biological drive chemicals | Indices 0–14 map to chemicals **148–162** | [Explicit 20-entry mapping](https://github.com/videogamepreservation/creatures3/blob/321245bed27cbacb52421d95c9ff4f0bf67a49eb/engine/Catalogues/ChemicalNames.catalogue#L8-L32) |
| Navigation chemicals | Up=**199**, Down=**200**, Exit=**201**, Enter=**202**, Wait=**203** | Same explicit mapping |
| Backup chemicals | **131–145**, matching the supplied biological names | [Catalogue names](https://github.com/videogamepreservation/creatures3/blob/321245bed27cbacb52421d95c9ff4f0bf67a49eb/engine/Catalogues/ChemicalNames.catalogue#L193-L239) |
| Stress intermediates | **187–195**, with general Stress=**128**; not 168–178 | [Catalogue](https://github.com/videogamepreservation/creatures3/blob/321245bed27cbacb52421d95c9ff4f0bf67a49eb/engine/Catalogues/ChemicalNames.catalogue#L229-L287) |
| Environmental chemicals | 165–184 are CA channels; 165–167 are sound, light and heat. 163/164 are unnamed here. | Same catalogue |

The historical catalogue labels 193 as Stress(Sleep) and 194 as Stress(Tired), whereas the [current community chemical list](https://creatures.fandom.com/wiki/C3_Chemical_List) reverses those two. Resolve that detail against the specific chosen genome before implementing it.

The [archived developer explanation](https://lisdude.com/cdn/18.html) establishes the useful functional target: action competition combines need intensity, object accessibility and learned drive relief; attention and decision choose object/action; execution scripts handle navigation. Homesickness follows home scent. Wait supports lifts/doors, not an established generic social-pacing mechanism. The engine copies receptor drive levels into `driv` ([source](https://github.com/videogamepreservation/creatures3/blob/321245bed27cbacb52421d95c9ff4f0bf67a49eb/engine/Creature/SensoryFaculty.cpp#L257-L263)) and separately arbitrates involuntary versus voluntary actions ([source](https://github.com/videogamepreservation/creatures3/blob/321245bed27cbacb52421d95c9ff4f0bf67a49eb/engine/Creature/MotorFaculty.cpp#L140-L216)).

Backup conversion preserves deferred needs conceptually, but exact affected drives, thresholds and reactions are genome-specific. The [organ reference](https://creatures.fandom.com/wiki/C3_Organs) even documents a vanilla pain/protein-backup mistake. The [official Genetics Kit manual](https://cdn.cloudflare.steamstatic.com/steam/apps/1838430/manuals/Genetics_Kit_Manual.pdf?t=1640163808), pages 46–48, is a relevant conceptual reference; those pages were not independently re-read for this source follow-up. Precise food categories, toy effects, Grendel responses, circadian claims and territorial anger must not be treated as universal rules. Community CFE/CFF and later genomes are distinct from vanilla C3 and DS ChiChi.

## Operational fatigue and sleep findings

### Existing channels and authored coefficients

The calibrated founder path applies `with_waking_recovery` to both inherited alleles (`crates/alife_world/src/new_game.rs:458–478`). Its reference graph differs from the unmodified early-mammal graph.

| Property | Fatigue | SleepPressure |
|---|---|---|
| Chemical identity | Species 2; drive channel 1 | Species 18; endocrine channel 8 |
| Primary neural port | 259 | A hormone input, not one of the 11 drive ports |
| Calibrated waking source | `Awake`, authored gain **+0.00007** | `Awake`, authored gain **+0.00007** |
| Waking emitter floor | **0.0** developmental expression | **0.0** developmental expression |
| Authored retention | **0.99995** per elapsed tick, toward expressed baseline | **0.99995** per elapsed tick, toward expressed baseline |
| Recovery source | `SleepRecovery`, authored gain **−0.70** | `SleepRecovery`, authored gain **−0.75** |
| Recovery emitter floor | **0.0** | **0.0** |
| Other relevant effect | Fatigue reduces locomotor capacity through receptor nominal 1, gain −1 | SleepPressure feeds the neural Sleep receptor and consolidation trigger |

Sources: `crates/alife_core/src/biochemical_graph.rs:914–970,1135–1170,1944–1983,2018–2037`. The calibrated graph removes the old EnergyDeficit→Fatigue emitter and direct SleepRecovery→ATP boost. Thus this founder's fatigue is not currently a separately measured exertion accumulator.

These are **authored graph coefficients**, not universal final phenotype numbers. Chromosome expression scales hormone production, including SleepPressure emissions and baseline, and raises regulatory retention to a hormone-decay-derived power. Fatigue receptor threshold also receives a genetic adjustment. See `biochemical_graph.rs:630–715`.

For an emitter, the actual contribution is:

`response × expressed_gain × max(biochemical_expression, emitter_floor) × release_count`.

Emitters here have cadence 1, analogue response and threshold 0. `Awake` is `!body.sleeping`; `SleepRecovery` is the event dose. Event-backed recovery releases once per advance, whereas waking accumulation counts crossed cadence boundaries. Decay is separate and occurs first, toward the expressed baseline. Sources: `biochemical_graph.rs:1352–1420,1798–1816,1826–1844,1944–1957`.

Regulatory baselines scale by biochemical expression. Expression is 1 if the activation gate is zero; otherwise `clamp(maturation / activation_gate, 0, 1)`. At expression zero, these fatigue/sleep emitters contribute zero; seeded concentrations may still decay. The SleepState emitter is different: `Sleeping→species20`, gain 1 and floor **1.0**, with retention zero. A newborn body can therefore be marked sleeping without mature fatigue/sleep-pressure recovery. Sources: `biochemical_graph.rs:61–68,932–946`; `biochemistry.rs:569–604`.

### Stationary Rest really does provide bodily sleep recovery

The following is a source-supported causal chain, not a newly executed test:

1. Ordinary `Rest` produces `BodyEventDelta.sleep_recovery=1.0` (`crates/alife_world/src/headless.rs:8203–8226`).
2. Any realized nonzero displacement clears that dose, including offsetting path segments whose net displacement is zero (`headless.rs:8000–8010`). The composite path also cancels recovery on relevant damage/withdrawal (`headless.rs:4990–5029`; `headless/contact_withdrawal.rs:48–52`).
3. Body recovery is `event.sleep_recovery × metabolic_turnover`; `body.sleeping` becomes `recovery>0` (`crates/alife_core/src/biochemistry.rs:164–166,264–270`).
4. The updated body and same event feed the biochemical graph (`biochemistry.rs:924–975`). Therefore both fatigue and SleepPressure receive their sleep-recovery reductions, and `Awake` emission is off for that interval.
5. At expression 1 and centered trait scaling, a Rest interval can subtract 0.70 fatigue and 0.75 SleepPressure, subject to the preceding baseline decay and concentration clamping. This is much larger than their per-tick waking additions. At partial expression the emitter effects scale down.

The old `OutcomeProfile::rest` also contains nominal −0.35 fatigue and −0.10 SleepPressure observation fields. Do not confuse those descriptors with the authoritative graph recovery coefficients above: `finish_action` packages observations and the body event separately (`headless.rs:6425–6456`), and the composite biology advance uses the body event (`headless.rs:5033–5052`).

### Bodily sleep and memory SleepPhase are distinct

`WorldOrganismRecord` exposes both `body_sleeping` and a separate sealed sleep phase (`crates/alife_world/src/organism.rs:441–450,1081–1109`). There is no invariant equating them.

Consequently **stationary Rest can lower SleepPressure while memory SleepPhase remains Awake**, provided the independent phase thresholds and forced-recovery conditions do not trigger. The phase controller does not use `body.sleeping` as its entry criterion. Rest does not itself schedule replay/consolidation.

The exact threshold distinctions matter:

- `SleepConsolidationConfig::reference`: fatigue **0.9**, SleepPressure **0.85** (`crates/alife_core/src/sleep.rs:183–194`).
- The normal phenotype-derived runtime configuration keeps fatigue 0.9 but replaces SleepPressure's threshold with `cognitive_architecture.sleep_trigger_threshold()` (`crates/alife_runtime/src/canonical_preparation.rs:604–612`). The canonical architecture starts at **0.6** (`crates/alife_core/src/genome.rs:164–179`), then inherited architecture and the chemistry sleep-threshold adjustment determine the actual phenotype (`evolutionary_genetics.rs:1412–1426`). **Neither 0.85 nor 0.6 is universal.**
- `SleepController::evaluate_homeostasis` first handles seizure/catatonia recovery, then checks its configured fatigue/SleepPressure thresholds (`sleep.rs:249–281`). The separate `ChemistryPhenotype.fatigue_threshold` also influences receptor expression; it is not simply the consolidation entry threshold.
- CPU host evaluates that controller from current homeostasis before selecting awake actions; non-Awake phases inject sleep recovery and skip normal action selection (`crates/alife_training/src/cpu/host.rs:907–926,997–1016`). Low ATP affordability can force recovery independently.
- GPU scheduling may inject recovery when its prior phase is non-Awake **or** the prior body is sleeping, advances biology at most once, then checks homeostasis (`crates/alife_runtime/src/sleep_scheduler.rs:253–283`). The body flag is a recovery input here, not automatic phase entry. Already-advanced biology is not advanced twice (`crates/alife_world/src/organism.rs:1065–1078`).

No distinct voluntary `ActionKind::Sleep` exists. The action enum has Idle, Hold, Rest, Inspect, Move, Interact, Vocalize, Write, Gesture and Look (`crates/alife_core/src/action.rs:19–30`). Existing Rest is effectively a voluntary bodily-sleep/recovery opportunity. This does **not** prove that every bodily sleep interval should perform consolidation, or that the two clocks should be merged.

## Are richer truthful physiological inputs already available?

**Signed body temperature: no implemented signed body/core-temperature state found in this source.** Body and organ physiology store unsigned `temperature_stress`; the body update clamps it to [0,1] (`biochemistry.rs:49–56,84–104,167–191`). The thermal-contact candidate uses `abs(surface_temperature)` for harmful exposure (`crates/alife_world/src/headless/thermal_contact_candidate.rs:94–139`). Signed object surface temperatures and an atmospheric-temperature context field do exist (`grounded_sensing.rs:66,137,410`; `crates/alife_core/src/sensory_abi.rs:258–276`). They are environmental observations, not a measured signed internal temperature deficit. No warming-versus-cooling need should be fabricated from unsigned stress.

**Separate macronutrient stores: no protein/carbohydrate/fat reserve model found in the active reference physiology.** There are per-organ energy stores and a generic nutrient material chemical converted to ATP (`biochemistry.rs:84–93,220–245`; `biochemical_graph.rs:1110–1117`). Food/ecology carries scalar nutrition (`crates/alife_world/src/ecology.rs:118–147`; `headless.rs:8245–8271`). The sensory ABI includes a `blood_sugar_trend` field, but that name does not establish a glucose/glycogen reserve or three-way nutrient metabolism. Additional macro-hunger inputs require corresponding material stores, food composition and metabolism first.

## Smallest recommended implementation experiment

**Add one explicit awake-rest recovery opportunity in a fresh, opt-in physiology/action contract, alongside the existing Rest/body-sleep behavior.** Its authoritative event should relieve fatigue without supplying `sleep_recovery`, setting `body.sleeping`, or directly clearing SleepPressure. Keep the existing bodily-sleep route available and keep consolidation admission independent. This is the smallest additive experiment that demonstrates the useful C3 tired-versus-sleepy distinction without silently redefining old Rest actions, altering existing acquired creatures, or assuming a memory phase is the definition of bodily sleep.

This requires a distinct rest-recovery event/source and action identity; changing a label or conditionally clearing SleepPressure according to `SleepPhase` would not implement the distinction. No arbitrary coefficient is recommended without timescale measurements. A later explicit Sleep action/clearer naming and sustained bodily-sleep transition can replace the current overloaded terminology under a versioned contract.

Required controls before promotion:

1. **Matched outcomes:** Idle, awake rest, bodily sleep, rest-plus-motion, rest interrupted by damage; equal elapsed biological time, costs and initial reserves. Awake rest reduces fatigue only; bodily sleep remains able to reduce sleep pressure without requiring consolidation every interval. Neither produces nutrition/free ATP.
2. **Development and dose:** zero, partial and full biochemical expression; actual threshold crossings; cadence/catch-up behavior; multiple requests in one interval cannot duplicate recovery. Check actual phenotype coefficients, not only authored defaults.
3. **Independent state:** low-pressure bodily sleep with memory Awake remains valid; consolidation/forced recovery remain separately testable. Ensure awake rest cannot eliminate every reachable sleep opportunity or cause permanent sleep pressure accumulation.
4. **Integration and persistence:** CPU/GPU parity, authoritative motion/damage cancellation, exact save/restore and old-contract continuation; verify learned value uses measured outcomes, not stale nominal Rest descriptors. Include satiated healthy inactivity and hungry wake-to-feed controls, not a constant-activity target.

A separate important safeguard applies to any later C3-inspired crisis suppression: do not make underlying unmet drive concentrations disappear merely to gate attention. `crates/alife_core/src/chemistry.rs:57–80` rewards measured reductions, so suppressing hunger/boredom chemically could fabricate satisfied-need reward. Preserve deficits and separate temporary arbitration gating from relief/value.


## Separate executed physiology evidence and next implementation

The source-only findings above are distinct from the fixed canonical-body test in [PR #317](https://github.com/BobertNeek/A-Life/pull/317), head `264d01861a74c5e3d003d0347ed33e9eb4930aea`. That test supplies measured age 0/59/60/240 Rest-versus-Idle and movement controls plus exact developmental-boundary save continuation. Its 20 focused checks and strict lint passed locally; hosted checks were still running when this note was archived. It does not test voluntary neural policy or a distinct awake-rest action. Its measured body-sleep and memory-Awake coexistence supports keeping those concepts separate.

An additive awake-rest source/event candidate is now being implemented separately. Its intended contract retains all old Rest semantics and genome/save behavior, provides fatigue-only recovery through a fresh inherited biochemical recipe, pays ordinary awake upkeep, and leaves SleepPressure and memory-phase admission independent. This paragraph is a design status, not a compiled, executed or promoted result.

The new motor-lobe research candidate separately mediates eleven truthful existing drive inputs through matched references, need-pressure cells, a recovery/search/fear controller and dedicated motor execution pairs. It must not label existing unsigned thermal stress as hot/cold need, or fatigue as sleepiness. Existing Homeostasis hormone lane 19 already carries SleepPressure and is a suitable truthful future input under a revised encoder/recipe, without inventing another chemical. Actual native compilation and subsequent behavior evidence belong to that candidate's own reports.

## Method reference for the next iteration

Keep [Sparks of Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/) alongside this biological reference: experiment 4 targeted repair, experiment 5 whole-model construction, and experiment 6 iterative feedback. Use their intervention/control/inspection method, while recognizing that feedforward game controllers do not establish competence for a recurrent, biochemical, plastic creature. Preserve initial failures and meaningful revisions, distinguish authored coefficients from measured effects, and test actual action consequences before claiming learned self-care.
