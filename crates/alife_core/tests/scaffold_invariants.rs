use alife_core::{
    ActionCommand, ActionKind, BrainClassSpec, BrainGenome, BrainScaleTier, EndocrineProfile,
    ExperiencePatchHeader, GenomeId, LineageExportManifest, LineageId, LobeKind, LobeLayout,
    NeuralComputeBackend, SemanticPriorProvider, SemanticPriorRequest, TeacherPerceptionChannel,
    WorldEntityId,
};

#[test]
fn standard2048_is_one_scalable_reference_tier() {
    let nano = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let standard = BrainClassSpec::for_tier(BrainScaleTier::Standard2048);
    let large = BrainClassSpec::for_tier(BrainScaleTier::Large4096);

    assert_eq!(nano.neuron_count, 512);
    assert_eq!(standard.neuron_count, 2048);
    assert_eq!(large.neuron_count, 4096);
    assert!(standard.neuron_count > nano.neuron_count);
    assert!(large.neuron_count > standard.neuron_count);

    for spec in [nano, standard, large] {
        spec.validate().expect("reference tier should be valid");
        assert_eq!(spec.neuron_count % 128, 0);
        assert_eq!(spec.lobe_layout.total_neurons(), spec.neuron_count);
        assert!(spec.lobe_layout.regions_are_aligned(16));
    }
}

#[test]
fn lobe_layout_supports_absent_lobes_without_deleting_variants() {
    let layout = LobeLayout::with_disabled_lobe(
        BrainScaleTier::Nano512.neuron_count().unwrap(),
        LobeKind::GlyphVision,
    )
    .expect("disabled-lobe layout should still be valid");

    assert_eq!(layout.total_neurons(), 512);
    assert!(layout.contains_lobe(LobeKind::GlyphVision));
    assert_eq!(layout.region(LobeKind::GlyphVision).unwrap().len, 0);
    assert!(!layout.region(LobeKind::GlyphVision).unwrap().enabled);
}

#[test]
fn genome_and_endocrine_profile_reference_brain_class_without_runtime_weights() {
    let class = BrainClassSpec::for_tier(BrainScaleTier::Small1024);
    let genome = BrainGenome::scaffold(7, class.id);
    let endocrine = EndocrineProfile::baseline();

    assert_eq!(genome.species_seed, 7);
    assert_eq!(genome.brain_class_id, class.id);
    assert!(genome.genetic_prior_seed != 0);
    assert_eq!(endocrine.modulator_count(), 6);
}

#[test]
fn experience_patch_and_action_command_use_versioned_structured_contracts() {
    let patch = ExperiencePatchHeader::new(11, 22, 33);
    let action = ActionCommand::new(11, ActionKind::Interact, Some(WorldEntityId(44)), 0.75, 120);

    assert_eq!(patch.abi_version, ExperiencePatchHeader::ABI_VERSION);
    assert_eq!(patch.organism_id.0, 11);
    assert_eq!(action.abi_version, ActionCommand::ABI_VERSION);
    assert_eq!(action.target_entity, Some(WorldEntityId(44)));
    assert!(action.confidence > 0.0);
}

#[test]
fn semantic_prior_is_private_and_teacher_uses_perceptual_channels() {
    let request = SemanticPriorRequest::new(11, 22);
    let channels = TeacherPerceptionChannel::ALL;

    assert!(request.private_to_organism);
    assert!(channels.contains(&TeacherPerceptionChannel::Hearing));
    assert!(channels.contains(&TeacherPerceptionChannel::Vision));
    assert!(channels.contains(&TeacherPerceptionChannel::Writing));
    assert!(channels.contains(&TeacherPerceptionChannel::Gesture));
    assert!(channels.contains(&TeacherPerceptionChannel::Object));
}

#[test]
fn backend_and_semantic_traits_are_scaffold_interfaces_only() {
    fn assert_backend<T: NeuralComputeBackend>() {}
    fn assert_prior<T: SemanticPriorProvider>() {}

    struct TestBackend;
    impl NeuralComputeBackend for TestBackend {
        fn backend_name(&self) -> &'static str {
            "test-backend"
        }
    }

    struct TestPrior;
    impl SemanticPriorProvider for TestPrior {
        fn provider_name(&self) -> &'static str {
            "test-prior"
        }
    }

    assert_backend::<TestBackend>();
    assert_prior::<TestPrior>();
}

#[test]
fn lineage_export_manifest_keeps_migration_metadata_versioned() {
    let source = BrainClassSpec::for_tier(BrainScaleTier::Standard2048);
    let target = BrainClassSpec::for_tier(BrainScaleTier::Large4096);
    let manifest = LineageExportManifest {
        abi_version: LineageExportManifest::ABI_VERSION,
        lineage_id: LineageId(99),
        founder_genome_id: GenomeId(123),
        source_brain_class_id: source.id,
        target_brain_class_id: Some(target.id),
        exported_at_tick: 456,
    };

    assert_eq!(manifest.abi_version, 1);
    assert_eq!(manifest.source_brain_class_id, source.id);
    assert_eq!(manifest.target_brain_class_id, Some(target.id));
}
