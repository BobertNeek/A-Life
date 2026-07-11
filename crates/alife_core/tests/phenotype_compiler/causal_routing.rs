//! RED contract tests owned by Task 3 causal genome compilation and routing validation.

#[cfg(test)]
mod task3_causal_genome_and_routing_red_tests {
    use std::collections::BTreeSet;

    use alife_core::{
        ActivationFunction, ActiveTilePolicy, AlphaStoragePolicy, BiologicalPriority,
        BrainCapacityClass, BrainGenome, BrainPhenotype, CandidateActionFamily, CompiledProjection,
        CompiledSynapseKind, CriticalPeriod, DecoderHeadKind, DevelopmentStage, DevelopmentState,
        DriveThresholdKind, EndocrineConstantKind, LobeKind, LobeRatioOverride, LobeRatioPlan,
        LobeRatioRegistryRef, MacroConnectomeMask, MotorAffordanceGene, MotorAffordanceKind,
        NormalizedScalar, PhenotypeCompiler, ProjectionAlphaOverride, ProjectionKey,
        ProjectionType, ScaffoldContractError, SensorChannelGene, SensorChannelKind,
        SensorEncoderSourceGroup, SensorProfile, SparseDensityPrior, Tick, UpdateCadence,
        CANDIDATE_FEATURE_COUNT,
    };

    const TEST_SEED: u64 = 0xCA55_A11E;
    const BASE_MATURATION: f32 = 0.35;

    const SENSORY_TO_ASSOCIATION: ProjectionKey =
        ProjectionKey::new(LobeKind::SensoryGrounding, LobeKind::CoreAssociation);
    const ASSOCIATION_TO_MOTOR: ProjectionKey =
        ProjectionKey::new(LobeKind::CoreAssociation, LobeKind::MotorArbitration);

    fn fixture() -> (BrainGenome, DevelopmentState) {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(TEST_SEED, capacity.id());
        let development = DevelopmentState::new(
            genome.id,
            Tick::ZERO,
            NormalizedScalar::new(BASE_MATURATION).unwrap(),
        );
        (genome, development)
    }

    fn compile_result(
        genome: &BrainGenome,
        development: &DevelopmentState,
    ) -> Result<BrainPhenotype, ScaffoldContractError> {
        PhenotypeCompiler::compile(
            genome,
            &BrainCapacityClass::n512(),
            development,
            SensorProfile::PrivilegedAffordanceV1,
        )
    }

    fn compile_ok(genome: &BrainGenome, development: &DevelopmentState) -> BrainPhenotype {
        compile_result(genome, development).unwrap()
    }

    fn projection_for(
        phenotype: &BrainPhenotype,
        key: ProjectionKey,
    ) -> Option<&CompiledProjection> {
        phenotype.projections().iter().find(|projection| {
            projection.source_lobe() == key.source_lobe
                && projection.target_lobe() == key.target_lobe
        })
    }

    fn assignment_keys(phenotype: &BrainPhenotype) -> BTreeSet<(u16, u16, u32)> {
        phenotype
            .sensor_encoder()
            .assignments()
            .iter()
            .map(|assignment| {
                (
                    assignment.source_group().raw(),
                    assignment.source_index(),
                    assignment.target_neuron(),
                )
            })
            .collect()
    }

    fn added_assignment_keys(
        before: &BrainPhenotype,
        after: &BrainPhenotype,
    ) -> BTreeSet<(u16, u16, u32)> {
        assignment_keys(after)
            .difference(&assignment_keys(before))
            .copied()
            .collect()
    }

    fn decoder_coordinate_evidence(
        phenotype: &BrainPhenotype,
        expected_family: CandidateActionFamily,
    ) -> Vec<(u8, u8, u16, u16, u32)> {
        phenotype
            .synapses()
            .iter()
            .filter_map(|synapse| match synapse.kind() {
                CompiledSynapseKind::Decoder(coordinate)
                    if coordinate.family() == expected_family =>
                {
                    Some((
                        coordinate.head().raw(),
                        coordinate.family().raw(),
                        coordinate.input_lane(),
                        coordinate.motor_index(),
                        synapse.target(),
                    ))
                }
                _ => None,
            })
            .collect()
    }

    type FixtureMutation = fn(&mut BrainGenome, &mut DevelopmentState);
    type EvidenceAssertion = fn(&BrainPhenotype, &BrainPhenotype);

    struct CausalCase {
        name: &'static str,
        prepare: FixtureMutation,
        mutate: FixtureMutation,
        assert_expected_change: EvidenceAssertion,
    }

    fn no_fixture_change(_: &mut BrainGenome, _: &mut DevelopmentState) {}

    fn mutate_lobe_override(genome: &mut BrainGenome, _: &mut DevelopmentState) {
        genome.lobe_ratios = LobeRatioPlan::InlineOverrides(vec![LobeRatioOverride {
            lobe: LobeKind::CoreAssociation,
            ratio: NormalizedScalar::new(0.50).unwrap(),
        }]);
    }

    fn assert_lobe_override_change(before: &BrainPhenotype, after: &BrainPhenotype) {
        let before_region = before
            .lobe_layout()
            .region(LobeKind::CoreAssociation)
            .unwrap();
        let after_region = after
            .lobe_layout()
            .region(LobeKind::CoreAssociation)
            .unwrap();
        assert!(after_region.enabled);
        assert!(after_region.len > before_region.len);
        assert_ne!(
            (before_region.start, before_region.len),
            (after_region.start, after_region.len),
        );
    }

    fn mutate_macro_disable(genome: &mut BrainGenome, _: &mut DevelopmentState) {
        genome
            .macro_connectome_masks
            .iter_mut()
            .find(|mask| mask.projection == SENSORY_TO_ASSOCIATION)
            .unwrap()
            .enabled = false;
        genome
            .sparse_density_priors
            .retain(|prior| prior.projection != SENSORY_TO_ASSOCIATION);
    }

    fn assert_macro_disable_change(before: &BrainPhenotype, after: &BrainPhenotype) {
        assert!(projection_for(before, SENSORY_TO_ASSOCIATION).is_some());
        assert!(projection_for(after, SENSORY_TO_ASSOCIATION).is_none());
        assert_eq!(before.projections().len(), after.projections().len() + 1);
    }

    fn mutate_route_density(genome: &mut BrainGenome, _: &mut DevelopmentState) {
        genome
            .sparse_density_priors
            .iter_mut()
            .find(|prior| prior.projection == ASSOCIATION_TO_MOTOR)
            .unwrap()
            .density = NormalizedScalar::new(0.08).unwrap();
    }

    fn assert_route_density_change(before: &BrainPhenotype, after: &BrainPhenotype) {
        let before_count = projection_for(before, ASSOCIATION_TO_MOTOR)
            .unwrap()
            .synapse_range()
            .1;
        let after_count = projection_for(after, ASSOCIATION_TO_MOTOR)
            .unwrap()
            .synapse_range()
            .1;
        assert!(after_count > before_count);
        assert!(after.synapses().len() > before.synapses().len());
    }

    fn mutate_projection_alpha(genome: &mut BrainGenome, _: &mut DevelopmentState) {
        genome
            .alpha_mask
            .projection_overrides
            .push(ProjectionAlphaOverride {
                projection: SENSORY_TO_ASSOCIATION,
                alpha: NormalizedScalar::new(0.75).unwrap(),
            });
    }

    fn assert_projection_alpha_change(before: &BrainPhenotype, after: &BrainPhenotype) {
        let before_projection = projection_for(before, SENSORY_TO_ASSOCIATION).unwrap();
        let after_projection = projection_for(after, SENSORY_TO_ASSOCIATION).unwrap();
        assert_eq!(
            before_projection.synapse_range().1,
            after_projection.synapse_range().1,
        );

        let before_alphas: Vec<_> = before
            .synapses()
            .iter()
            .filter(|synapse| synapse.route_index() == before_projection.route_index())
            .map(|synapse| synapse.alpha())
            .collect();
        let after_alphas: Vec<_> = after
            .synapses()
            .iter()
            .filter(|synapse| synapse.route_index() == after_projection.route_index())
            .map(|synapse| synapse.alpha())
            .collect();
        assert_eq!(before_alphas.len(), after_alphas.len());
        assert!(before_alphas
            .iter()
            .all(|alpha| (*alpha - 0.25).abs() < f32::EPSILON));
        assert!(after_alphas
            .iter()
            .all(|alpha| (*alpha - 0.75).abs() < f32::EPSILON));
    }

    fn mutate_sensor_gene(genome: &mut BrainGenome, _: &mut DevelopmentState) {
        genome.sensor_layout.channels.push(SensorChannelGene {
            kind: SensorChannelKind::Proprioception,
            receptor_count: 13,
            target_lobe: LobeKind::SensoryGrounding,
            enabled_at_maturation: 0,
        });
    }

    fn assert_sensor_gene_change(before: &BrainPhenotype, after: &BrainPhenotype) {
        let added = added_assignment_keys(before, after);
        assert_eq!(added.len(), 13);
        let expected_source_indices: BTreeSet<_> = (0_u16..13).collect();
        let actual_source_indices: BTreeSet<_> = added
            .iter()
            .map(|(source_group, source_index, target)| {
                assert_eq!(*source_group, SensorEncoderSourceGroup::Body.raw());
                let sensory = after
                    .lobe_layout()
                    .region(LobeKind::SensoryGrounding)
                    .unwrap();
                assert!(sensory.contains_neuron(*target));
                *source_index
            })
            .collect();
        assert_eq!(actual_source_indices, expected_source_indices);
    }

    fn mutate_motor_gene(genome: &mut BrainGenome, _: &mut DevelopmentState) {
        genome.motor_affordances.push(MotorAffordanceGene {
            kind: MotorAffordanceKind::Eat,
            enabled: true,
            motor_lobe_units: 1,
            enabled_at_maturation: 0,
        });
    }

    fn assert_motor_gene_change(before: &BrainPhenotype, after: &BrainPhenotype) {
        assert!(decoder_coordinate_evidence(before, CandidateActionFamily::Ingest).is_empty());
        let coordinates = decoder_coordinate_evidence(after, CandidateActionFamily::Ingest);
        assert_eq!(coordinates.len(), CANDIDATE_FEATURE_COUNT);

        let expected_input_lanes: BTreeSet<_> = (0_u16..CANDIDATE_FEATURE_COUNT as u16).collect();
        let input_lanes: BTreeSet<_> = coordinates
            .iter()
            .map(|(head, family, input_lane, motor_index, target)| {
                assert_eq!(*head, DecoderHeadKind::ActionCandidate.raw());
                assert_eq!(*family, CandidateActionFamily::Ingest.raw());
                assert_eq!(
                    *target,
                    after.candidate_decoder().motor_start() + u32::from(*motor_index),
                );
                *input_lane
            })
            .collect();
        assert_eq!(input_lanes, expected_input_lanes);
        assert_eq!(
            coordinates
                .iter()
                .map(|(_, _, _, motor_index, _)| *motor_index)
                .collect::<BTreeSet<_>>()
                .len(),
            1,
        );
    }

    fn prepare_maturation_gate(genome: &mut BrainGenome, _: &mut DevelopmentState) {
        genome.sensor_layout.channels.push(SensorChannelGene {
            kind: SensorChannelKind::Hearing,
            receptor_count: 8,
            target_lobe: LobeKind::AuditorySpeech,
            enabled_at_maturation: 60,
        });
    }

    fn cross_maturation_gate(_: &mut BrainGenome, development: &mut DevelopmentState) {
        development.maturation = NormalizedScalar::new(0.75).unwrap();
    }

    fn assert_maturation_gate_change(before: &BrainPhenotype, after: &BrainPhenotype) {
        let added = added_assignment_keys(before, after);
        assert_eq!(added.len(), 8);
        let expected_hearing_lanes: BTreeSet<_> = (16_u16..24).collect();
        let actual_hearing_lanes: BTreeSet<_> = added
            .iter()
            .map(|(source_group, source_index, target)| {
                assert_eq!(
                    *source_group,
                    SensorEncoderSourceGroup::SensoryChannel.raw(),
                );
                let auditory = after
                    .lobe_layout()
                    .region(LobeKind::AuditorySpeech)
                    .unwrap();
                assert!(auditory.contains_neuron(*target));
                *source_index
            })
            .collect();
        assert_eq!(actual_hearing_lanes, expected_hearing_lanes);
    }

    #[test]
    fn every_accepted_genome_family_changes_its_owned_phenotype_evidence() {
        let cases = [
            CausalCase {
                name: "lobe allocation override",
                prepare: no_fixture_change,
                mutate: mutate_lobe_override,
                assert_expected_change: assert_lobe_override_change,
            },
            CausalCase {
                name: "macro connectome disable",
                prepare: no_fixture_change,
                mutate: mutate_macro_disable,
                assert_expected_change: assert_macro_disable_change,
            },
            CausalCase {
                name: "route density",
                prepare: no_fixture_change,
                mutate: mutate_route_density,
                assert_expected_change: assert_route_density_change,
            },
            CausalCase {
                name: "projection alpha",
                prepare: no_fixture_change,
                mutate: mutate_projection_alpha,
                assert_expected_change: assert_projection_alpha_change,
            },
            CausalCase {
                name: "sensor gene",
                prepare: no_fixture_change,
                mutate: mutate_sensor_gene,
                assert_expected_change: assert_sensor_gene_change,
            },
            CausalCase {
                name: "motor gene",
                prepare: no_fixture_change,
                mutate: mutate_motor_gene,
                assert_expected_change: assert_motor_gene_change,
            },
            CausalCase {
                name: "maturation gate",
                prepare: prepare_maturation_gate,
                mutate: cross_maturation_gate,
                assert_expected_change: assert_maturation_gate_change,
            },
        ];

        for case in cases {
            let (mut genome, mut development) = fixture();
            (case.prepare)(&mut genome, &mut development);
            let before = compile_ok(&genome, &development);

            let mut mutated_genome = genome.clone();
            let mut mutated_development = development.clone();
            (case.mutate)(&mut mutated_genome, &mut mutated_development);
            let after = compile_ok(&mutated_genome, &mutated_development);

            assert_ne!(
                before.phenotype_hash(),
                after.phenotype_hash(),
                "{} changed no canonical phenotype identity",
                case.name,
            );
            (case.assert_expected_change)(&before, &after);
        }
    }

    type InvalidGenomeMutation = fn(&mut BrainGenome);

    struct InvalidGenomeCase {
        name: &'static str,
        mutate: InvalidGenomeMutation,
    }

    fn duplicate_macro_key(genome: &mut BrainGenome) {
        genome
            .macro_connectome_masks
            .push(genome.macro_connectome_masks[0]);
    }

    fn duplicate_density_key(genome: &mut BrainGenome) {
        genome
            .sparse_density_priors
            .push(genome.sparse_density_priors[0]);
    }

    fn leave_density_without_mask(genome: &mut BrainGenome) {
        let orphaned_key = genome.sparse_density_priors[0].projection;
        genome
            .macro_connectome_masks
            .retain(|mask| mask.projection != orphaned_key);
    }

    fn enable_mask_on_disabled_lobe(genome: &mut BrainGenome) {
        genome.lobe_ratios = LobeRatioPlan::InlineOverrides(vec![LobeRatioOverride {
            lobe: LobeKind::CoreAssociation,
            ratio: NormalizedScalar::new(0.0).unwrap(),
        }]);
    }

    fn add_noncanonical_route_key(genome: &mut BrainGenome) {
        let key = ProjectionKey::new(LobeKind::SensoryGrounding, LobeKind::MotorArbitration);
        genome.macro_connectome_masks.push(MacroConnectomeMask {
            projection: key,
            enabled: true,
            structural_growth_allowed: false,
        });
        genome.sparse_density_priors.push(SparseDensityPrior {
            projection: key,
            density: NormalizedScalar::new(0.02).unwrap(),
            max_active_synapse_share: NormalizedScalar::new(0.10).unwrap(),
        });
    }

    fn enable_structural_growth(genome: &mut BrainGenome) {
        genome.macro_connectome_masks[0].structural_growth_allowed = true;
    }

    fn use_registry_lobe_ratios(genome: &mut BrainGenome) {
        genome.lobe_ratios = LobeRatioPlan::RegistryRef(LobeRatioRegistryRef {
            registry_id: 7,
            version: 1,
        });
    }

    fn use_dense_debug_alpha(genome: &mut BrainGenome) {
        genome.alpha_mask.storage_policy = AlphaStoragePolicy::DenseDebugReference;
        genome.alpha_mask.dense_reference_opt_in = true;
    }

    #[test]
    fn invalid_genome_routing_and_storage_forms_are_rejected() {
        let cases = [
            InvalidGenomeCase {
                name: "duplicate macro-connectome key",
                mutate: duplicate_macro_key,
            },
            InvalidGenomeCase {
                name: "duplicate density key",
                mutate: duplicate_density_key,
            },
            InvalidGenomeCase {
                name: "density without a macro mask",
                mutate: leave_density_without_mask,
            },
            InvalidGenomeCase {
                name: "enabled mask on a disabled lobe",
                mutate: enable_mask_on_disabled_lobe,
            },
            InvalidGenomeCase {
                name: "noncanonical route key",
                mutate: add_noncanonical_route_key,
            },
            InvalidGenomeCase {
                name: "structural growth in Slice A",
                mutate: enable_structural_growth,
            },
            InvalidGenomeCase {
                name: "unresolved lobe-ratio registry reference",
                mutate: use_registry_lobe_ratios,
            },
            InvalidGenomeCase {
                name: "dense debug alpha in production",
                mutate: use_dense_debug_alpha,
            },
        ];

        for case in cases {
            let (mut genome, development) = fixture();
            (case.mutate)(&mut genome);
            assert!(
                compile_result(&genome, &development).is_err(),
                "{} was silently accepted",
                case.name,
            );
        }
    }

    #[test]
    fn unsupported_neutral_evolution_field_returns_phenotype_compile() {
        let (mut genome, development) = fixture();
        genome.plasticity_mask.oja_enabled = false;

        assert_eq!(
            compile_result(&genome, &development).unwrap_err(),
            ScaffoldContractError::PhenotypeCompile,
        );
    }

    #[test]
    fn zero_density_or_zero_share_enabled_routes_are_rejected() {
        for zero_density in [true, false] {
            let (mut genome, development) = fixture();
            if zero_density {
                genome.sparse_density_priors[0].density = NormalizedScalar::new(0.0).unwrap();
            } else {
                genome.sparse_density_priors[0].max_active_synapse_share =
                    NormalizedScalar::new(0.0).unwrap();
            }
            assert_eq!(
                compile_result(&genome, &development).unwrap_err(),
                ScaffoldContractError::PhenotypeCompile,
            );
        }
    }

    #[test]
    fn open_critical_periods_are_rejected_until_they_are_causal() {
        let (genome, mut development) = fixture();
        development.open_critical_periods.push(CriticalPeriod {
            lobe: LobeKind::CoreAssociation,
            opens_at: Tick::ZERO,
            closes_at: Tick(10),
            plasticity_bias: NormalizedScalar::new(0.5).unwrap(),
        });
        assert_eq!(
            compile_result(&genome, &development).unwrap_err(),
            ScaffoldContractError::PhenotypeCompile,
        );
    }

    #[test]
    fn gpu_facing_enum_raw_mappings_are_stable_total_and_checked() {
        for (value, raw) in [
            (ActivationFunction::Identity, 0),
            (ActivationFunction::Relu, 1),
            (ActivationFunction::Tanh, 2),
            (ActivationFunction::Logistic, 3),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(ActivationFunction::try_from_raw(raw).unwrap(), value);
        }
        assert!(ActivationFunction::try_from_raw(4).is_err());

        for (value, raw) in [
            (LobeKind::SensoryGrounding, 1),
            (LobeKind::MetabolicDrive, 2),
            (LobeKind::AuditorySpeech, 3),
            (LobeKind::GlyphVision, 4),
            (LobeKind::LexiconConcept, 5),
            (LobeKind::CoreAssociation, 6),
            (LobeKind::EpisodicMemory, 7),
            (LobeKind::WorkingMemory, 8),
            (LobeKind::MotorArbitration, 9),
            (LobeKind::HomeostaticRegulation, 10),
            (LobeKind::LanguageExpansion, 11),
            (LobeKind::MathQuantity, 12),
            (LobeKind::NarrativeHistory, 13),
            (LobeKind::SocialReasoning, 14),
            (LobeKind::SelfCriticUncertainty, 15),
            (LobeKind::PlanningDream, 16),
            (LobeKind::SpeechWritingMotor, 17),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(LobeKind::try_from_raw(raw).unwrap(), value);
        }
        assert!(LobeKind::try_from_raw(0).is_err());
        assert!(LobeKind::try_from_raw(18).is_err());

        for (value, raw) in [
            (ProjectionType::FeedForward, 0),
            (ProjectionType::Feedback, 1),
            (ProjectionType::Recurrent, 2),
            (ProjectionType::Modulatory, 3),
            (ProjectionType::MotorProposal, 4),
            (ProjectionType::Homeostatic, 5),
            (ProjectionType::LateralInhibition, 6),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(ProjectionType::try_from_raw(raw).unwrap(), value);
        }
        assert!(ProjectionType::try_from_raw(7).is_err());

        for (value, raw) in [
            (ActiveTilePolicy::EssentialReservation, 0),
            (ActiveTilePolicy::SalienceGated, 1),
            (ActiveTilePolicy::Decimated, 2),
            (ActiveTilePolicy::SleepQueued, 3),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(ActiveTilePolicy::try_from_raw(raw).unwrap(), value);
        }
        assert!(ActiveTilePolicy::try_from_raw(4).is_err());

        for (value, raw) in [
            (UpdateCadence::Hot60Hz, 0),
            (UpdateCadence::Hot15To60Hz, 1),
            (UpdateCadence::Hot10To30Hz, 2),
            (UpdateCadence::Hot5To15Hz, 3),
            (UpdateCadence::Hot1To5Hz, 4),
            (UpdateCadence::SleepOrOffline, 5),
            (UpdateCadence::Disabled, 6),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(UpdateCadence::try_from_raw(raw).unwrap(), value);
        }
        assert!(UpdateCadence::try_from_raw(7).is_err());

        for (value, raw) in [
            (BiologicalPriority::Essential, 0),
            (BiologicalPriority::High, 1),
            (BiologicalPriority::Normal, 2),
            (BiologicalPriority::NonEssential, 3),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(BiologicalPriority::try_from_raw(raw).unwrap(), value);
        }
        assert!(BiologicalPriority::try_from_raw(4).is_err());

        for (value, raw) in [
            (SensorEncoderSourceGroup::SensoryChannel, 1),
            (SensorEncoderSourceGroup::Body, 2),
            (SensorEncoderSourceGroup::Homeostasis, 3),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(SensorEncoderSourceGroup::try_from_raw(raw).unwrap(), value);
        }
        assert!(SensorEncoderSourceGroup::try_from_raw(0).is_err());
        assert!(SensorEncoderSourceGroup::try_from_raw(4).is_err());

        for (value, raw) in [
            (DecoderHeadKind::ActionCandidate, 1),
            (DecoderHeadKind::MemoryContext, 2),
        ] {
            assert_eq!(value.raw(), raw);
            assert_eq!(DecoderHeadKind::try_from_raw(raw).unwrap(), value);
        }
        assert!(DecoderHeadKind::try_from_raw(0).is_err());
        assert!(DecoderHeadKind::try_from_raw(3).is_err());
    }

    #[test]
    fn compiler_input_enum_raw_mappings_are_stable_total_and_checked() {
        macro_rules! assert_mapping {
            ($ty:ty, [$(($variant:path, $raw:expr)),+ $(,)?], $invalid:expr) => {{
                $(
                    assert_eq!($variant.raw(), $raw);
                    assert_eq!(<$ty>::try_from_raw($raw).unwrap(), $variant);
                )+
                assert!(<$ty>::try_from_raw($invalid).is_err());
            }};
        }
        assert_mapping!(
            EndocrineConstantKind,
            [
                (EndocrineConstantKind::DopamineBaseline, 0),
                (EndocrineConstantKind::SerotoninBaseline, 1),
                (EndocrineConstantKind::CortisolBaseline, 2),
                (EndocrineConstantKind::OxytocinBaseline, 3),
                (EndocrineConstantKind::AdrenalineBaseline, 4),
                (EndocrineConstantKind::AcetylcholineBaseline, 5),
                (EndocrineConstantKind::BrainAtpBaseline, 6),
                (EndocrineConstantKind::DevelopmentHormoneBaseline, 7),
            ],
            8
        );
        assert_mapping!(
            DriveThresholdKind,
            [
                (DriveThresholdKind::Hunger, 0),
                (DriveThresholdKind::Fatigue, 1),
                (DriveThresholdKind::Fear, 2),
                (DriveThresholdKind::Pain, 3),
                (DriveThresholdKind::Loneliness, 4),
                (DriveThresholdKind::Curiosity, 5),
                (DriveThresholdKind::Reproduction, 6),
            ],
            7
        );
        assert_mapping!(
            SensorChannelKind,
            [
                (SensorChannelKind::Vision, 0),
                (SensorChannelKind::Hearing, 1),
                (SensorChannelKind::Touch, 2),
                (SensorChannelKind::Smell, 3),
                (SensorChannelKind::Taste, 4),
                (SensorChannelKind::Proprioception, 5),
                (SensorChannelKind::Interoception, 6),
                (SensorChannelKind::GlyphVision, 7),
            ],
            8
        );
        assert_mapping!(
            MotorAffordanceKind,
            [
                (MotorAffordanceKind::Move, 0),
                (MotorAffordanceKind::Turn, 1),
                (MotorAffordanceKind::Eat, 2),
                (MotorAffordanceKind::Rest, 3),
                (MotorAffordanceKind::Interact, 4),
                (MotorAffordanceKind::Vocalize, 5),
                (MotorAffordanceKind::Write, 6),
                (MotorAffordanceKind::Gesture, 7),
                (MotorAffordanceKind::Reproduce, 8),
            ],
            9
        );
        assert_mapping!(
            DevelopmentStage,
            [
                (DevelopmentStage::Hatchling, 0),
                (DevelopmentStage::Juvenile, 1),
                (DevelopmentStage::Adolescent, 2),
                (DevelopmentStage::Adult, 3),
                (DevelopmentStage::Elder, 4),
            ],
            5
        );
    }

    #[test]
    fn slice_a_compiles_only_zero_delay_routes_and_rejects_tampering() {
        let (genome, development) = fixture();
        let phenotype = compile_ok(&genome, &development);
        assert!(!phenotype.projections().is_empty());
        assert!(phenotype
            .projections()
            .iter()
            .all(|projection| projection.delay_microsteps() == 0),);

        let mut wire = serde_json::to_value(&phenotype).unwrap();
        wire["projections"][0]["delay_microsteps"] = serde_json::json!(1);
        assert!(serde_json::from_value::<BrainPhenotype>(wire).is_err());
    }
}
