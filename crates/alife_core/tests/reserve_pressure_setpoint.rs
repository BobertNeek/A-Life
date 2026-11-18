//! Explicit inherited reserve threshold. No production default or policy change.
use alife_core::*;
use serde_json::json;

const TARGET: f32 = 0.8;
const RESERVES: [f32; 7] = [1.0, 0.9, 0.8, 0.7925, 0.65, 0.4, 0.1];

fn founder() -> CreatureGenome {
    CreatureGenome::early_mammal_founder(
        921412,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
}

fn indices(genome: &CreatureGenome) -> Vec<usize> {
    genome
        .chemistry
        .graph
        .expressed()
        .emitters()
        .iter()
        .enumerate()
        .filter_map(|(index, emitter)| {
            (emitter.source == BiochemicalSourceLocus::EnergyDeficit).then_some(index)
        })
        .collect()
}

fn candidate(mut genome: CreatureGenome) -> CreatureGenome {
    let indices = indices(&genome);
    assert_eq!(indices.len(), 3);
    for index in indices {
        let emitter = genome.chemistry.graph.expressed().emitters()[index];
        assert_eq!(emitter.response, EmitterResponse::Analogue);
        assert!(!emitter.inverted);
        for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
            genome.chemistry.graph = genome
                .chemistry
                .graph
                .clone()
                .with_emitter_threshold(side, index, 1.0 - TARGET)
                .unwrap();
        }
    }
    genome
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 2e-6, "{actual} != {expected}");
}

fn delta(after: [f32; 11], before: [f32; 11]) -> [f32; 11] {
    std::array::from_fn(|i| after[i] - before[i])
}

#[test]
fn inherited_threshold_is_validated_diploid_and_default_preserving() {
    let original = founder();
    let original_wire = serde_json::to_value(&original).unwrap();
    let mut changed_wire = serde_json::to_value(candidate(original.clone())).unwrap();
    for side in ["maternal", "paternal"] {
        for index in indices(&original) {
            assert_eq!(
                changed_wire["chemistry"]["graph"][side]["emitters"][index]["threshold"],
                json!(1.0 - TARGET)
            );
            changed_wire["chemistry"]["graph"][side]["emitters"][index]["threshold"] = json!(0.0);
        }
    }
    assert_eq!(changed_wire, original_wire);
    assert_eq!(serde_json::to_value(founder()).unwrap(), original_wire);
    for threshold in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        assert!(original
            .chemistry
            .graph
            .clone()
            .with_emitter_threshold(AlleleSide::Maternal, 0, threshold)
            .is_err());
    }
    assert!(original
        .chemistry
        .graph
        .clone()
        .with_emitter_threshold(AlleleSide::Maternal, usize::MAX, 0.2)
        .is_err());
    let candidate = candidate(original.clone());
    let restored: CreatureGenome =
        serde_json::from_slice(&serde_json::to_vec(&candidate).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(&candidate).unwrap()
    );
    let mut one_side = original.clone();
    one_side.chemistry.graph = one_side
        .chemistry
        .graph
        .with_emitter_threshold(AlleleSide::Maternal, 0, 1.0 - TARGET)
        .unwrap();
    close(
        one_side.express().unwrap().chemistry.biochemical.emitters()[0].threshold,
        (1.0 - TARGET) * 0.5,
    );
    let legacy = original.express().unwrap();
    let protected = candidate.express().unwrap();
    assert_eq!(
        legacy.chemistry.biochemical.species(),
        protected.chemistry.biochemical.species()
    );
    assert_eq!(
        legacy.chemistry.biochemical.receptors(),
        protected.chemistry.biochemical.receptors()
    );
    assert_eq!(
        legacy.chemistry.biochemical.reactions(),
        protected.chemistry.biochemical.reactions()
    );
    assert_eq!(
        legacy.chemistry.biochemical.neuroemitters(),
        protected.chemistry.biochemical.neuroemitters()
    );
    assert_eq!(
        legacy.chemistry.biochemical.value_profile(),
        protected.chemistry.biochemical.value_profile()
    );
    let initial = BiochemistryState::new(&protected, Tick(600)).unwrap();
    let restored: BiochemistryState =
        serde_json::from_slice(&serde_json::to_vec(&initial).unwrap()).unwrap();
    assert_eq!(restored, initial);
}

fn expected_next(previous: [f32; 11], pressure: f32, sleep: f32) -> [f32; 11] {
    let mut next = previous;
    next[0] = (0.25 + (previous[0] - 0.25) * 0.96 + 0.18 * pressure).clamp(0.0, 1.0);
    next[1] = ((0.2 + (previous[1] - 0.2) * 0.96 + 0.12 * pressure).clamp(0.0, 1.0) - 0.7 * sleep)
        .clamp(0.0, 1.0);
    next[6] = ((0.75 + (previous[6] - 0.75) * 0.96 - 0.12 * pressure).clamp(0.0, 1.0)
        + 0.3 * sleep)
        .clamp(0.0, 1.0);
    next
}

fn equilibrium(pressure: f32, sleep: f32) -> [f32; 3] {
    // Inherited declaration order clips energy-deficit release BEFORE sleep.
    // Thus sleep imposes a fatigue ceiling and ATP floor even at low reserve.
    [
        (0.25 + 0.18 * pressure / 0.04).clamp(0.0, 1.0),
        (0.2 + (0.12 * pressure - 0.7 * sleep) / 0.04).clamp(0.0, 1.0 - 0.7 * sleep),
        (0.75 + (-0.12 * pressure + 0.3 * sleep) / 0.04).clamp(0.3 * sleep, 1.0),
    ]
}

#[test]
#[ignore = "fixed 3626-call core-only grid; see frozen plan"]
fn fixed_reserve_setpoint_grid() {
    let legacy = founder().express().unwrap();
    let protected = candidate(founder()).express().unwrap();
    let mut calls = 0_u64;
    let mut total_work = 0_u64;
    let mut account = |work: BiochemicalWorkReceipt| {
        calls += 1;
        total_work += work.total();
        assert_eq!(
            (
                work.species_updates,
                work.reaction_evaluations,
                work.emitter_evaluations,
                work.receptor_evaluations,
                work.neural_emitter_evaluations
            ),
            (19, 1, 22, 26, 0)
        );
    };
    for sleep in [0.0, 0.1] {
        let mut previous_pressure = [0.0; 2];
        for reserve in RESERVES {
            let mut finals = [[0.0; 11]; 2];
            for (variant, phenotype) in [&legacy, &protected].into_iter().enumerate() {
                let initial = BiochemistryState::new(phenotype, Tick(600)).unwrap();
                let initial_drives = initial.homeostasis.drives.to_array();
                let mut body = initial.body;
                body.set_energy(reserve).unwrap();
                body.sleeping = sleep > 0.0;
                let threshold = if variant == 0 { 0.0 } else { 1.0 - TARGET };
                let pressure = (1.0 - body.energy - threshold).max(0.0);
                assert!(pressure >= previous_pressure[variant]);
                previous_pressure[variant] = pressure;
                let mut graph = *initial.graph_state();
                let mut previous = initial_drives;
                let mut first = initial_drives;
                for step in 1..=128 {
                    let (next, work) = graph
                        .advance(
                            Tick(600 + step),
                            body,
                            BodyEventDelta {
                                sleep_recovery: sleep,
                                ..BodyEventDelta::zero()
                            },
                            None,
                            &phenotype.chemistry.biochemical,
                            1.0,
                        )
                        .unwrap();
                    account(work);
                    graph = next;
                    let drives = graph
                        .derive_homeostasis(&phenotype.chemistry.biochemical)
                        .unwrap()
                        .drives
                        .to_array();
                    for (actual, expected) in drives
                        .into_iter()
                        .zip(expected_next(previous, pressure, sleep))
                    {
                        close(actual, expected);
                    }
                    if step == 1 {
                        first = drives;
                    }
                    previous = drives;
                }
                finals[variant] = previous;
                let steady = equilibrium(pressure, sleep);
                println!(
                    "setpoint {}",
                    json!({"label":"fixed_reserve_grid", "variant":variant,
                    "target":if variant == 0 {1.0} else {TARGET}, "reserve":body.energy,
                    "sleep_dose_per_step":sleep, "pressure":pressure, "steps":128,
                    "initial":initial_drives, "first":first, "final":previous,
                    "raw":delta(previous,initial_drives), "equilibrium_h_f_atp":steady})
                );
                for (index, steady) in [0, 1, 6].into_iter().zip(steady) {
                    assert!((previous[index] - steady).abs() < 0.006);
                }
                if variant == 1 && reserve >= TARGET && sleep == 0.0 {
                    assert_eq!(pressure, 0.0);
                    assert_eq!(previous, initial_drives);
                }
                if reserve == 0.1 && sleep == 0.0 {
                    assert_eq!((previous[0], previous[1], previous[6]), (1.0, 1.0, 0.0));
                }
            }
            let contrast = delta(finals[1], finals[0]);
            assert!(contrast[0] <= 0.0 && contrast[1] <= 0.0 && contrast[6] >= 0.0);
            for index in [2, 3, 4, 5, 7, 8, 9, 10] {
                assert_eq!(contrast[index], 0.0);
            }
            println!(
                "setpoint {}",
                json!({"label":"matched_contrast", "reserve":reserve,
                "sleep_dose_per_step":sleep, "candidate_minus_control":contrast})
            );
        }
    }
    for reserve in [1.0, 0.65, 0.1] {
        for (variant, phenotype) in [&legacy, &protected].into_iter().enumerate() {
            let initial = BiochemistryState::new(phenotype, Tick(600)).unwrap();
            let mut body = initial.body;
            body.set_energy(reserve).unwrap();
            body.sleeping = true;
            let (next, work) = initial
                .graph_state()
                .advance(
                    Tick(601),
                    body,
                    BodyEventDelta {
                        sleep_recovery: 1.0,
                        ..BodyEventDelta::zero()
                    },
                    None,
                    &phenotype.chemistry.biochemical,
                    1.0,
                )
                .unwrap();
            account(work);
            let drives = next
                .derive_homeostasis(&phenotype.chemistry.biochemical)
                .unwrap()
                .drives
                .to_array();
            println!(
                "setpoint {}",
                json!({"label":"full_sleep_single_dose", "reserve":reserve,
                "variant":variant, "raw":delta(drives,initial.homeostasis.drives.to_array()), "drives":drives})
            );
        }
    }
    for reserve in [0.8, 0.1] {
        for (variant, phenotype) in [&legacy, &protected].into_iter().enumerate() {
            let initial = BiochemistryState::new_with_age(phenotype, Tick(600), Tick(0)).unwrap();
            let mut body = initial.body;
            body.set_energy(reserve).unwrap();
            let (next, work) = initial
                .graph_state()
                .advance(
                    Tick(601),
                    body,
                    BodyEventDelta::zero(),
                    None,
                    &phenotype.chemistry.biochemical,
                    0.0,
                )
                .unwrap();
            account(work);
            let drives = next
                .derive_homeostasis(&phenotype.chemistry.biochemical)
                .unwrap()
                .drives
                .to_array();
            assert_eq!(drives, initial.homeostasis.drives.to_array());
            println!(
                "setpoint {}",
                json!({"label":"expression_zero", "reserve":reserve,
                "variant":variant, "drives":drives})
            );
        }
    }
    for (variant, phenotype) in [&legacy, &protected].into_iter().enumerate() {
        let mut state = BiochemistryState::new(phenotype, Tick(600)).unwrap();
        let initial = state.homeostasis.drives.to_array();
        let mut before_sleep = None;
        for step in 1..=16 {
            state = state
                .advance(
                    Tick(600 + step),
                    BodyEventDelta {
                        sleep_recovery: if step > 8 { 0.1 } else { 0.0 },
                        ..BodyEventDelta::zero()
                    },
                    phenotype,
                )
                .unwrap();
            account(state.biochemical_work());
            if step == 8 {
                before_sleep = Some(state);
            }
            if [1, 8, 9, 16].contains(&step) {
                println!(
                    "setpoint {}",
                    json!({"label":"core_awake_then_rest", "variant":variant,
                    "step":step, "reserve":state.body.energy, "drives":state.homeostasis.drives.to_array(),
                    "raw_from_initial":delta(state.homeostasis.drives.to_array(),initial)})
                );
            }
        }
        let before_sleep = before_sleep.unwrap();
        assert!(state.body.energy < before_sleep.body.energy);
        assert!(state.homeostasis.drives.fatigue < before_sleep.homeostasis.drives.fatigue);
        assert!(state.homeostasis.drives.brain_atp > before_sleep.homeostasis.drives.brain_atp);
    }
    assert_eq!(calls, 3626);
    assert_eq!(total_work, 3626 * 68);
    println!(
        "setpoint {}",
        json!({"completed_advance_calls":calls, "work_total":total_work,
        "world_intervals":0, "neural_emissions":0})
    );
}
