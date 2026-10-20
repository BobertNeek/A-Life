use alife_core::*;
fn frame() -> PerceptionFrame {
    let tick = Tick::new(7);
    let id = OrganismId(7);
    let candidate = ActionCandidate::new(
        0,
        ActionId(4),
        ActionKind::Inspect,
        CandidateActionFamily::Inspect,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(0.8).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(1),
    )
    .unwrap();
    PerceptionFrame::new(
        id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        SensorySnapshot::new(
            id,
            tick,
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(
            SensorProfile::PrivilegedAffordanceV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        Vec::new(),
    )
    .unwrap()
}
fn pre(cpu: bool, frame: PerceptionFrame) -> PreActionSnapshot {
    let class = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(42, class.id());
    let development = DevelopmentState::new(
        genome.id,
        frame.tick(),
        NormalizedScalar::new(0.35).unwrap(),
    );
    let ctor = if cpu {
        PreActionSnapshot::from_cpu_neural_frame
    } else {
        PreActionSnapshot::from_neural_frame
    };
    ctor(
        ExperienceSequenceId(99),
        class.id(),
        PhenotypeHash([11, 22, 33, 44]),
        genome.id,
        genome.schema_version,
        development,
        frame,
    )
    .unwrap()
}
fn decision(cpu: bool, f: &PerceptionFrame) -> DecisionSnapshot {
    let confidence = Confidence::new(0.8).unwrap();
    let command = f.candidates()[0]
        .to_command(OrganismId(7), confidence)
        .unwrap();
    let ctor = if cpu {
        DecisionSnapshot::from_cpu_neural_selection
    } else {
        DecisionSnapshot::from_neural_selection
    };
    ctor(
        ExperienceSequenceId(99),
        PhenotypeHash([11, 22, 33, 44]),
        7,
        1,
        f,
        NeuralActionSelection {
            candidate_index: 0,
            logit: 0.75,
            confidence,
            active_tiles: 12,
            active_synapses: 144,
        },
        command,
    )
    .unwrap()
}
fn outcome() -> PostActionOutcome {
    PostActionOutcome::new(
        OrganismId(7),
        ExperienceSequenceId(99),
        Tick::new(8),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::ZERO,
        NormalizedScalar::new(0.).unwrap(),
        NormalizedScalar::new(0.).unwrap(),
        SignedValence::ZERO,
        NormalizedScalar::new(0.).unwrap(),
    )
    .unwrap()
}
fn seal(cpu_pre: bool, cpu_decision: bool) -> Result<ExperiencePatch, ScaffoldContractError> {
    let f = frame();
    ExperiencePatchBuilder::new(ExperienceSequenceId(99))
        .record_pre_action(pre(cpu_pre, f.clone()))?
        .record_decision(decision(cpu_decision, &f))?
        .record_outcome(outcome())?
        .seal()
}
#[test]
fn explicit_cpu_evidence_roundtrips_and_keeps_gpu_wire_identity() {
    let f = frame();
    let gpu = pre(false, f.clone());
    let old_json = serde_json::to_string(&gpu).unwrap();
    assert!(old_json.contains("NeuralClosedLoopGpu"));
    let reread: PreActionSnapshot = serde_json::from_str(&old_json).unwrap();
    assert_eq!(reread, gpu);
    assert_eq!(gpu.policy_backend(), PolicyBackend::NeuralClosedLoopGpu);
    let cpu = pre(true, f.clone());
    let json = serde_json::to_string(&cpu).unwrap();
    assert!(json.contains("NeuralClosedLoopCpu"));
    assert_eq!(
        serde_json::from_str::<PreActionSnapshot>(&json).unwrap(),
        cpu
    );
    assert_eq!(cpu.policy_backend(), PolicyBackend::NeuralClosedLoopCpu);
    assert_eq!(PolicyBackend::default(), PolicyBackend::NeuralClosedLoopGpu);
    assert!(!PolicyBackend::NeuralClosedLoopCpu.requires_gpu());
    let d = decision(true, &f);
    assert_eq!(
        serde_json::from_slice::<DecisionSnapshot>(&serde_json::to_vec(&d).unwrap()).unwrap(),
        d
    );
}
#[test]
fn sealed_domains_cannot_mix_and_packed_cpu_tag_is_distinct() {
    assert!(seal(true, false).is_err());
    assert!(seal(false, true).is_err());
    let gpu = ExperiencePacker::default()
        .pack(&seal(false, false).unwrap())
        .unwrap();
    let cpu = ExperiencePacker::default()
        .pack(&seal(true, true).unwrap())
        .unwrap();
    assert_eq!(gpu.frame.flags & packed_log::PACKED_FLAG_CPU_NEURAL, 0);
    assert_ne!(cpu.frame.flags & packed_log::PACKED_FLAG_CPU_NEURAL, 0);
    assert_eq!(gpu.frame.schema_version, cpu.frame.schema_version);
    let reread: PackedExperienceRecord =
        serde_json::from_slice(&serde_json::to_vec(&gpu).unwrap()).unwrap();
    assert_eq!(gpu, reread);
}
