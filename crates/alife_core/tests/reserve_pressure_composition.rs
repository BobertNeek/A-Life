//! Zero-advance recipe composition contracts. The sealed physiology grid is separate.
use alife_core::*;

const THRESHOLD: f32 = 1.0 - 0.8;

fn founder() -> CreatureGenome {
    CreatureGenome::early_mammal_founder(
        921412,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
}

fn waking(mut genome: CreatureGenome) -> CreatureGenome {
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome.chemistry.graph.with_waking_recovery(side).unwrap();
    }
    genome
}

fn play(mut genome: CreatureGenome) -> CreatureGenome {
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome.chemistry.graph.with_play_stimulation(side).unwrap();
    }
    genome
}

// This is a construction recipe for the symmetric public-genome fixtures, not
// an adapter that edits a live graph or assumes arbitrary homologs share indices.
fn reserve(mut genome: CreatureGenome) -> CreatureGenome {
    let original = serde_json::to_value(&genome).unwrap();
    assert_eq!(
        original["chemistry"]["graph"]["maternal"]["emitters"],
        original["chemistry"]["graph"]["paternal"]["emitters"],
        "this construction helper requires matched homologous emitter arrays"
    );
    let graph = genome.chemistry.graph.expressed();
    let targets: Vec<_> = graph
        .receptors()
        .iter()
        .filter_map(|r| match r.target {
            BiochemicalTargetLocus::Drive(
                BiochemicalDriveChannel::Hunger
                | BiochemicalDriveChannel::Fatigue
                | BiochemicalDriveChannel::BrainAtp,
            ) => Some(r.source),
            _ => None,
        })
        .collect();
    let indices: Vec<_> = graph
        .emitters()
        .iter()
        .enumerate()
        .filter_map(|(index, e)| {
            (e.source == BiochemicalSourceLocus::EnergyDeficit && targets.contains(&e.target))
                .then_some(index)
        })
        .collect();
    assert!(matches!(indices.len(), 2 | 3));
    for index in indices.iter().copied() {
        let emitter = genome.chemistry.graph.expressed().emitters()[index];
        assert_eq!(emitter.response, EmitterResponse::Analogue);
        assert!(!emitter.inverted);
        for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
            genome.chemistry.graph = genome
                .chemistry
                .graph
                .with_emitter_threshold(side, index, THRESHOLD)
                .unwrap();
        }
    }
    // Restore ONLY the edited threshold fields in a comparison copy. Everything
    // else, including missing rows and intentional gains, must match exactly.
    let mut restored_wire = serde_json::to_value(&genome).unwrap();
    for side in ["maternal", "paternal"] {
        for index in indices.iter().copied() {
            restored_wire["chemistry"]["graph"][side]["emitters"][index]["threshold"] =
                original["chemistry"]["graph"][side]["emitters"][index]["threshold"].clone();
        }
    }
    assert_eq!(restored_wire, original);
    genome.validate_contract().unwrap();
    genome
}

fn assert_same(a: &CreatureGenome, b: &CreatureGenome) {
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    assert_eq!(
        a.express().unwrap().chemistry.biochemical,
        b.express().unwrap().chemistry.biochemical
    );
}

fn assert_waking_preserved(genome: &CreatureGenome) {
    let graph = genome.chemistry.graph.expressed();
    let source = |channel| {
        graph
            .receptors()
            .iter()
            .find(|r| r.target == BiochemicalTargetLocus::Drive(channel))
            .unwrap()
            .source
    };
    let fatigue = source(BiochemicalDriveChannel::Fatigue);
    let atp = source(BiochemicalDriveChannel::BrainAtp);
    assert!(!graph.emitters().iter().any(|e| {
        (e.source == BiochemicalSourceLocus::EnergyDeficit && e.target == fatigue)
            || (e.source == BiochemicalSourceLocus::SleepRecovery && e.target == atp)
    }));
    let atp_emitter = graph
        .emitters()
        .iter()
        .find(|e| e.source == BiochemicalSourceLocus::EnergyDeficit && e.target == atp)
        .unwrap();
    assert_eq!(atp_emitter.gain, -0.02);
    assert_eq!(atp_emitter.threshold, THRESHOLD);
    assert!(graph.emitters().iter().any(|e| {
        e.source == BiochemicalSourceLocus::Awake && e.target == fatigue && e.gain == 0.00007
    }));
}

#[test]
fn reserve_and_waking_order_preserves_removed_couplings_and_atp_gain() {
    let before = waking(reserve(founder()));
    let after = reserve(waking(founder()));
    assert_same(&before, &after);
    assert_waking_preserved(&after);
    println!("composition waking: two orders equal; fatigue deficit and sleep ATP absent; ATP gain -0.02; zero advances");
}

#[test]
fn reserve_and_play_order_preserves_boredom_curiosity_and_valuation() {
    let before = play(reserve(founder()));
    let after = reserve(play(founder()));
    assert_same(&before, &after);
    let graph = after.chemistry.graph.expressed();
    assert!(graph.receptors().iter().any(|r| {
        r.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Extension0)
    }));
    assert_eq!(graph.value_profile().drives[9], -0.08);
    for (source, gain) in [
        (BiochemicalSourceLocus::PerceivedNovelty, 0.04),
        (BiochemicalSourceLocus::Investigation, -0.08),
        (BiochemicalSourceLocus::PlayStimulation, -0.2),
    ] {
        assert!(graph
            .emitters()
            .iter()
            .any(|e| e.source == source && e.gain == gain));
    }
    println!(
        "composition play: two orders equal; boredom/curiosity/valuation preserved; zero advances"
    );
}

#[test]
fn reserve_position_preserves_each_fixed_waking_play_recipe_order() {
    let waking_then_play = [
        play(waking(reserve(founder()))),
        play(reserve(waking(founder()))),
        reserve(play(waking(founder()))),
    ];
    let play_then_waking = [
        waking(play(reserve(founder()))),
        waking(reserve(play(founder()))),
        reserve(waking(play(founder()))),
    ];
    for variants in [&waking_then_play, &play_then_waking] {
        for pair in variants.windows(2) {
            assert_same(&pair[0], &pair[1]);
        }
        for genome in variants {
            assert_waking_preserved(genome);
        }
    }
    println!("composition waking+play: three reserve positions equal within each of two fixed recipe orders; zero advances");
}

#[test]
fn reserve_preserves_explicitly_retuned_existing_emitter_gains() {
    let mut genome = waking(founder());
    let indices: Vec<_> = genome
        .chemistry
        .graph
        .expressed()
        .emitters()
        .iter()
        .enumerate()
        .filter_map(|(index, e)| {
            (e.source == BiochemicalSourceLocus::EnergyDeficit).then_some((index, e.gain))
        })
        .collect();
    assert_eq!(indices.len(), 2);
    for (index, gain) in indices {
        let new_gain = if gain > 0.0 { 0.11 } else { -0.013 };
        for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
            genome.chemistry.graph = genome
                .chemistry
                .graph
                .with_emitter_gain(side, index, new_gain)
                .unwrap();
        }
    }
    let changed = reserve(genome);
    let gains: Vec<_> = changed
        .chemistry
        .graph
        .expressed()
        .emitters()
        .iter()
        .filter(|e| e.source == BiochemicalSourceLocus::EnergyDeficit)
        .map(|e| e.gain)
        .collect();
    assert_eq!(gains, [0.11, -0.013]);
    println!("composition custom gains: 0.11/-0.013 preserved; zero advances");
}
