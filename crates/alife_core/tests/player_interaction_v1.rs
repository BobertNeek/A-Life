use alife_core::*;

fn founder(opt_in: bool) -> CreaturePhenotype {
    let mut genome = CreatureGenome::early_mammal_founder(
        2116,
        FoundationGeneticIdentity::new(20, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    if opt_in {
        for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
            genome.chemistry.graph = genome
                .chemistry
                .graph
                .clone()
                .with_social_disapproval_v1(side)
                .unwrap();
        }
    }
    genome.express().unwrap()
}

#[test]
fn social_disapproval_is_explicit_and_not_damage_or_nutrition() {
    let p = founder(true);
    let state = BiochemistryState::new(&p, Tick(0)).unwrap();
    let control = state.advance(Tick(1), BodyEventDelta::zero(), &p).unwrap();
    let event = BodyEventDelta {
        social_disapproval_v1: 1.0,
        ..BodyEventDelta::zero()
    };
    let scold = state.advance(Tick(1), event, &p).unwrap();
    assert_eq!(scold.body, control.body);
    assert_eq!(
        scold.homeostasis.drives.pain,
        control.homeostasis.drives.pain
    );
    assert!(scold.homeostasis.drives.fear > control.homeostasis.drives.fear);
    assert!(scold.homeostasis.hormones.cortisol > control.homeostasis.hormones.cortisol);
    let later = scold.advance(Tick(2), BodyEventDelta::zero(), &p).unwrap();
    assert!(later.homeostasis.drives.fear < scold.homeostasis.drives.fear);
}

#[test]
fn absent_inherited_source_is_not_silently_installed() {
    let p = founder(false);
    let state = BiochemistryState::new(&p, Tick(0)).unwrap();
    let control = state.advance(Tick(1), BodyEventDelta::zero(), &p).unwrap();
    let scold = state
        .advance(
            Tick(1),
            BodyEventDelta {
                social_disapproval_v1: 1.0,
                ..BodyEventDelta::zero()
            },
            &p,
        )
        .unwrap();
    assert_eq!(control, scold);
    let json = serde_json::to_string(&state).unwrap();
    let restored: BiochemistryState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, state);
}

#[test]
fn legacy_event_wire_and_source_ordinals_stay_compatible() {
    let zero = BodyEventDelta::zero();
    let json = serde_json::to_value(zero).unwrap();
    assert!(json.get("social_disapproval_v1").is_none());
    assert_eq!(
        serde_json::from_value::<BodyEventDelta>(json).unwrap(),
        zero
    );
    assert_eq!(
        bincode::serde::encode_to_vec(
            BiochemicalSourceLocus::AwakeRestRecoveryV1,
            bincode::config::legacy()
        )
        .unwrap(),
        15_u32.to_le_bytes()
    );
    assert_eq!(
        bincode::serde::encode_to_vec(
            BiochemicalSourceLocus::SocialDisapprovalV1,
            bincode::config::legacy()
        )
        .unwrap(),
        16_u32.to_le_bytes()
    );
}

#[test]
fn inherited_development_gate_is_not_bypassed() {
    let mut genome = CreatureGenome::early_mammal_founder(
        2117,
        FoundationGeneticIdentity::new(20, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome
            .chemistry
            .graph
            .clone()
            .with_social_disapproval_v1(side)
            .unwrap();
    }
    let expressed = genome.express().unwrap();
    let indices = expressed
        .chemistry
        .biochemical
        .emitters()
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            (row.source == BiochemicalSourceLocus::SocialDisapprovalV1).then_some(i)
        })
        .collect::<Vec<_>>();
    for side in [AlleleSide::Maternal, AlleleSide::Paternal] {
        for &index in &indices {
            genome.chemistry.graph = genome
                .chemistry
                .graph
                .clone()
                .with_emitter_expression_floor(side, index, 0.0)
                .unwrap();
        }
    }
    let p = genome.express().unwrap();
    let state = BiochemistryState::new(&p, Tick(0)).unwrap();
    let control = state.advance(Tick(1), BodyEventDelta::zero(), &p).unwrap();
    let scold = state
        .advance(
            Tick(1),
            BodyEventDelta {
                social_disapproval_v1: 1.0,
                ..BodyEventDelta::zero()
            },
            &p,
        )
        .unwrap();
    assert_eq!(scold, control);
}
