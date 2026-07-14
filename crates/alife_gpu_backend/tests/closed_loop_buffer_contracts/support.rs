use std::ops::Range;

use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainCapacityClass,
    BrainClassId, BrainGenome, BrainPhenotype, CandidateActionFamily, CandidateFeatureVector,
    CandidateObservationRef, CompiledSynapseKind, Confidence, DevelopmentState, DurationTicks,
    HomeostaticSnapshot, NormalizedScalar, OrganismId, PerceptionFrame, PhenotypeCompiler, Pose,
    SensorProfile, SensoryChannels, SensorySnapshot, Tick, Vec3f, Velocity, WorldEntityId,
};

pub fn compile(class_id: BrainClassId, seed: u64) -> BrainPhenotype {
    let capacity = BrainCapacityClass::production_for_id(class_id).unwrap();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap()
}

pub fn production_phenotypes() -> [BrainPhenotype; 3] {
    [
        compile(BrainCapacityClass::N512_ID, 41),
        compile(BrainCapacityClass::N1024_ID, 41),
        compile(BrainCapacityClass::N2048_ID, 41),
    ]
}

pub fn recurrent_count(phenotype: &BrainPhenotype) -> usize {
    phenotype
        .synapses()
        .iter()
        .filter(|row| matches!(row.kind(), CompiledSynapseKind::Recurrent))
        .count()
}

pub fn ranges_are_disjoint(left: Range<u32>, right: Range<u32>) -> bool {
    left.end <= right.start || right.end <= left.start
}

pub fn perception_fixture() -> PerceptionFrame {
    let organism_id = OrganismId(7);
    let tick = Tick::new(0x1_0000_0007);
    let sensory = SensorySnapshot::new(
        organism_id,
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let mut first_features = CandidateFeatureVector::zero();
    let mut second_features = CandidateFeatureVector::zero();
    for (index, value) in first_features.0.iter_mut().enumerate() {
        *value = index as f32 / 32.0;
    }
    for (index, value) in second_features.0.iter_mut().enumerate() {
        *value = -(index as f32) / 32.0;
    }
    let candidates = vec![
        ActionCandidate::new(
            0,
            ActionId(4),
            ActionKind::Inspect,
            CandidateActionFamily::Inspect,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            first_features,
            Confidence::new(0.8).unwrap(),
            NormalizedScalar::new(0.1).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap(),
        ActionCandidate::new(
            1,
            ActionId(101),
            ActionKind::Move,
            CandidateActionFamily::Approach,
            CandidateObservationRef::ObjectSlot(3),
            ActionTarget::new(Some(WorldEntityId(55)), Some(Vec3f::new(1.0, 0.0, 2.0))),
            second_features,
            Confidence::new(0.9).unwrap(),
            NormalizedScalar::new(0.2).unwrap(),
            DurationTicks::new(2),
            DurationTicks::new(4),
        )
        .unwrap(),
    ];
    PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        candidates,
    )
    .unwrap()
}
