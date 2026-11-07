use alife_core::*;

// Exercise the non-human-readable map visitor without claiming qualification
// of any extra binary codec. Values use existing serde_json machinery; only
// this top-level map is numeric-keyed and explicitly non-human-readable.
struct NumericBinaryDraftMap(Vec<(u64, serde_json::Value)>);
impl<'de> serde::Deserializer<'de> for NumericBinaryDraftMap {
    type Error = serde_json::Error;
    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_map(serde::de::value::MapDeserializer::new(self.0.into_iter()))
    }
    fn is_human_readable(&self) -> bool {
        false
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }
}
fn numeric_binary_draft_map(draft: &PerceptionFrameDraft) -> NumericBinaryDraftMap {
    // Literal original indices0..10 plus the new append-only index11. Do not
    // derive this order from the production Wire, whose regressions we test.
    let names = [
        "schema_version",
        "organism_id",
        "tick",
        "sensor_profile",
        "sensory",
        "body",
        "homeostasis",
        "candidates",
        "profile_provenance",
        "grounded_object_slots",
        "base_digest",
        "grip_observations",
    ];
    let mut value = serde_json::to_value(draft).unwrap();
    let object = value.as_object_mut().unwrap();
    let mut entries = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| object.remove(*name).map(|value| (index as u64, value)))
        .collect::<Vec<_>>();
    assert!(object.is_empty());
    // Numeric keys, not encounter order, must select the corresponding fields.
    entries.reverse();
    NumericBinaryDraftMap(entries)
}
fn assert_numeric_binary_draft_roundtrip(draft: &PerceptionFrameDraft) {
    let map = numeric_binary_draft_map(draft);
    let restored: PerceptionFrameDraft = serde::Deserialize::deserialize(map).unwrap();
    assert_eq!(&restored, draft);
}

fn graph(grip: bool) -> BrainPhenotype {
    let genome = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    if grip {
        PhenotypeCompiler::compile_n2048_grip_reader_graph(
            genome,
            development,
            SensorProfile::GroundedGripObjectSlotsV1,
        )
        .unwrap()
        .0
    } else {
        PhenotypeCompiler::compile_n2048_memory_gain_graph(
            genome,
            development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap()
        .0
    }
}

#[test]
fn new_interface_has_distinct_identity_and_exactly_sixty_four_declared_retargets() {
    let old = graph(false);
    let before = serde_json::to_vec(&old).unwrap();
    let new = graph(true);
    assert_eq!(old.synapses().len(), 32_768);
    assert_eq!(new.synapses().len(), 32_768);
    let mut changed = [0_u16; 8];
    for (old, new) in old.synapses().iter().zip(new.synapses()) {
        if old != new {
            let (CompiledSynapseKind::Decoder(a), CompiledSynapseKind::Decoder(b)) =
                (old.kind(), new.kind())
            else {
                panic!("only decoder coordinates may differ")
            };
            assert_eq!(a.head(), DecoderHeadKind::MemoryContext);
            assert_eq!(b.head(), a.head());
            assert_eq!(a.input_lane(), 31);
            assert_eq!(b.input_lane(), 53);
            assert_eq!(a.family(), b.family());
            assert_eq!(a.motor_index(), b.motor_index());
            assert_eq!(old.source(), new.source());
            assert_eq!(old.target(), new.target());
            assert_eq!(old.genetic_weight(), 0.0);
            assert_eq!(old.genetic_weight(), new.genetic_weight());
            assert_eq!(old.alpha(), new.alpha());
            assert_eq!(old.receptor_index(), new.receptor_index());
            changed[b.family().raw() as usize] += 1;
        }
    }
    assert_eq!(changed, [8; 8]);
    for family in 0..8 {
        for lane in 24..36 {
            assert!(new.synapses().iter().any(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c)
                if c.head() == DecoderHeadKind::MemoryContext && c.family().raw() == family && c.input_lane() == lane)));
        }
    }
    assert_eq!(old.projections(), new.projections());
    assert_eq!(old.neuron_dynamics(), new.neuron_dynamics());
    assert_eq!(old.plasticity_receptors(), new.plasticity_receptors());
    assert_eq!(old.candidate_decoder().flattened_input_lane_count(), 36);
    assert_eq!(new.candidate_decoder().flattened_input_lane_count(), 54);
    assert_eq!(
        new.candidate_decoder()
            .memory_channel()
            .unwrap()
            .schema_version(),
        3
    );
    assert!(new.cognitive_channel_plan().is_none());
    assert_ne!(old.phenotype_hash(), new.phenotype_hash());
    assert_ne!(
        old.foundation_abi().selector_digest(),
        new.foundation_abi().selector_digest()
    );
    let old_asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&old).unwrap();
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&new).unwrap();
    assert!(asset.is_n2048_grip_reader_candidate());
    assert!(!asset.is_n2048_memory_gain_candidate());
    assert!(N2048GripReaderCandidateV1::for_asset(&old_asset).is_err());
    assert!(N2048MemoryGainCandidateV1::for_asset(&asset).is_err());
    assert!(N2048GripReaderCandidateV1::fresh(SensorProfile::GroundedObjectSlotsV1).is_err());
    assert!(FoundationAbiBinding::canonical_for_foundation_asset(
        &BrainCapacityClass::n2048(),
        &asset
    )
    .is_err());
    assert_eq!(
        FoundationWeightAsset::decode_canonical(&asset.encode_canonical().unwrap()).unwrap(),
        asset
    );
    let old_selector =
        bincode::serde::encode_to_vec(old.foundation_abi(), bincode::config::standard()).unwrap();
    let selector =
        bincode::serde::encode_to_vec(new.foundation_abi(), bincode::config::standard()).unwrap();
    assert_eq!(old_selector[0], 6);
    assert_eq!(selector[0], 7);
    let mut wire = serde_json::to_value(&new).unwrap();
    wire["foundation_abi_selection"] = serde_json::to_value(old.foundation_abi()).unwrap();
    assert!(serde_json::from_value::<BrainPhenotype>(wire).is_err());
    assert_eq!(serde_json::to_vec(&graph(false)).unwrap(), before);
}

fn draft(
    profile: SensorProfile,
    ownership: GripOwnershipV1,
    op: GripOperationV1,
) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
    let slot = GroundedObjectSlotV1 {
        slot_index: 0,
        tracked_object_id: TrackedObjectId(77),
        bearing: [0., 1.],
        distance: 0.02,
        relative_velocity: [0.; 3],
        color: [0.2; 3],
        material: [0.1; 3],
        shape: [0.3; 3],
        chemical: [0.; 3],
        contact: 1.,
        proprioception: [0.; 2],
        temperature: 0.,
        terrain: [0.; 2],
        confidence: Confidence::new(1.)?,
    };
    let candidate = ActionCandidate::new(
        0,
        op.action_id(),
        ActionKind::Interact,
        op.family(),
        CandidateObservationRef::ObjectSlot(0),
        ActionTarget::new(Some(WorldEntityId(9)), None),
        slot.candidate_features()?,
        Confidence::new(1.)?,
        NormalizedScalar::new(0.2)?,
        DurationTicks::new(1),
        DurationTicks::new(1),
    )?;
    let tick = Tick::new(3);
    PerceptionFrameDraft::new_with_grip_observations(
        OrganismId(1),
        tick,
        profile,
        SensorySnapshot::new(
            OrganismId(1),
            tick,
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )?,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick)?,
        vec![slot],
        vec![GroundedGripSlotV1 {
            slot_index: 0,
            ownership,
            confidence: Confidence::new(1.)?,
        }],
    )
}

#[test]
fn ownership_is_typed_and_bound_without_repurposing_object_lanes() {
    let held = draft(
        SensorProfile::GroundedGripObjectSlotsV1,
        GripOwnershipV1::SelfHeld,
        GripOperationV1::Release,
    )
    .unwrap();
    let unheld = draft(
        SensorProfile::GroundedGripObjectSlotsV1,
        GripOwnershipV1::NotSelfHeld,
        GripOperationV1::Close,
    )
    .unwrap();
    assert_eq!(held.grounded_object_slots(), unheld.grounded_object_slots());
    assert_eq!(
        held.candidates()[0].features,
        unheld.candidates()[0].features
    );
    assert_ne!(held.base_digest(), unheld.base_digest());
    for (ownership, operation) in [
        (GripOwnershipV1::NotSelfHeld, GripOperationV1::Release),
        (GripOwnershipV1::SelfHeld, GripOperationV1::Close),
    ] {
        assert!(draft(
            SensorProfile::GroundedGripObjectSlotsV1,
            ownership,
            operation
        )
        .is_err());
    }
    assert!(draft(
        SensorProfile::GroundedObjectSlotsV1,
        GripOwnershipV1::SelfHeld,
        GripOperationV1::Release
    )
    .is_err());
    let mut tampered = serde_json::to_value(&held).unwrap();
    tampered["grip_observations"][0]["ownership"] = serde_json::json!("NotSelfHeld");
    assert!(serde_json::from_value::<PerceptionFrameDraft>(tampered).is_err());
    let mut missing = serde_json::to_value(&held).unwrap();
    missing.as_object_mut().unwrap().remove("grip_observations");
    assert!(serde_json::from_value::<PerceptionFrameDraft>(missing).is_err());
    let bytes = bincode::serde::encode_to_vec(&held, bincode::config::standard()).unwrap();
    // Literal append-only sequence: all eleven legacy fields precede the new
    // extension, including the old digest at index10.
    let append_only_sequence = (
        held.schema_version(),
        held.organism_id(),
        held.tick(),
        held.sensor_profile(),
        held.sensory(),
        held.body(),
        held.homeostasis(),
        held.candidates(),
        held.profile_provenance(),
        held.grounded_object_slots(),
        held.base_digest(),
        held.grip_observations(),
    );
    assert_eq!(
        bytes,
        bincode::serde::encode_to_vec(&append_only_sequence, bincode::config::standard()).unwrap()
    );
    let (restored, used): (PerceptionFrameDraft, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(restored, held);
    assert_eq!(used, bytes.len());
    let nested = (held.clone(), 0x1122_3344_5566_7788_u64);
    let nested_bytes = bincode::serde::encode_to_vec(&nested, bincode::config::standard()).unwrap();
    let (copy, consumed): ((PerceptionFrameDraft, u64), usize) =
        bincode::serde::decode_from_slice(&nested_bytes, bincode::config::standard()).unwrap();
    assert_eq!(copy, nested);
    assert_eq!(consumed, nested_bytes.len());
    assert_numeric_binary_draft_roundtrip(&held);
    let mut missing_extension = numeric_binary_draft_map(&held);
    missing_extension.0.retain(|(index, _)| *index != 11);
    assert!(<PerceptionFrameDraft as serde::Deserialize>::deserialize(missing_extension).is_err());
}

#[test]
fn legacy_draft_wire_sequence_and_json_are_unchanged() {
    // Literal pre-extension field order. Its binary output is the old codec;
    // this comparison catches both inserted fields and changed optional tags.
    #[derive(serde::Serialize)]
    struct LegacyWire<'a> {
        schema_version: u16,
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: &'a SensorySnapshot,
        body: BodySnapshot,
        homeostasis: &'a HomeostaticSnapshot,
        candidates: &'a [ActionCandidate],
        profile_provenance: SensorProfileProvenance,
        grounded_object_slots: &'a [GroundedObjectSlotV1],
        base_digest: PerceptionBaseDigest,
    }
    let tick = Tick::ZERO;
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let candidate = ActionCandidate::new(
        0,
        ActionKind::Idle.canonical_id(),
        ActionKind::Idle,
        CandidateActionFamily::Idle,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(1.).unwrap(),
        NormalizedScalar::new(0.).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(1),
    )
    .unwrap();
    let old = PerceptionFrameDraft::new(
        OrganismId(1),
        tick,
        profile,
        SensorySnapshot::new(
            OrganismId(1),
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
        SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
        Vec::new(),
    )
    .unwrap();
    let legacy = LegacyWire {
        schema_version: old.schema_version(),
        organism_id: old.organism_id(),
        tick: old.tick(),
        sensor_profile: old.sensor_profile(),
        sensory: old.sensory(),
        body: old.body(),
        homeostasis: old.homeostasis(),
        candidates: old.candidates(),
        profile_provenance: old.profile_provenance(),
        grounded_object_slots: old.grounded_object_slots(),
        base_digest: old.base_digest(),
    };
    let bytes = bincode::serde::encode_to_vec(&legacy, bincode::config::standard()).unwrap();
    assert_eq!(
        bytes,
        bincode::serde::encode_to_vec(&old, bincode::config::standard()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&legacy).unwrap(),
        serde_json::to_vec(&old).unwrap()
    );
    let (restored, used): (PerceptionFrameDraft, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(restored, old);
    assert_eq!(used, bytes.len());
    assert_numeric_binary_draft_roundtrip(&old);
}

#[test]
fn old_profile_arbitrary_operation_numbers_keep_their_old_meaning() {
    for raw in [2201, 2202] {
        let profile = SensorProfile::GroundedObjectSlotsV1;
        let tick = Tick::ZERO;
        let candidate = ActionCandidate::new(
            0,
            ActionId(raw),
            ActionKind::Interact,
            CandidateActionFamily::Contact,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero(),
            Confidence::new(1.).unwrap(),
            NormalizedScalar::new(0.).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap();
        let old = PerceptionFrameDraft::new(
            OrganismId(1),
            tick,
            profile,
            SensorySnapshot::new(
                OrganismId(1),
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
            SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
            Vec::new(),
        )
        .unwrap();
        let json = serde_json::to_vec(&old).unwrap();
        assert!(!String::from_utf8_lossy(&json).contains("grip_observations"));
        assert_eq!(
            serde_json::from_slice::<PerceptionFrameDraft>(&json).unwrap(),
            old
        );
        assert_numeric_binary_draft_roundtrip(&old);
        // The old human-readable positional form has exactly the same eleven
        // fields, with no trailing grip extension. Both numeric action IDs
        // retain their original meaning through that representation too.
        let mut old_fields = numeric_binary_draft_map(&old).0;
        old_fields.sort_by_key(|(index, _)| *index);
        assert_eq!(old_fields.len(), 11);
        assert!(old_fields
            .iter()
            .enumerate()
            .all(|(i, (index, _))| *index == i as u64));
        let positional = old_fields
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>();
        let positional_json = serde_json::to_vec(&positional).unwrap();
        let restored: PerceptionFrameDraft = serde_json::from_slice(&positional_json).unwrap();
        assert_eq!(restored, old);
        assert_eq!(serde_json::to_vec(&restored).unwrap(), json);
        let nested = (old, 0x1122_3344_5566_7788_u64);
        let bytes = bincode::serde::encode_to_vec(&nested, bincode::config::standard()).unwrap();
        let (copy, used): ((PerceptionFrameDraft, u64), usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
        assert_eq!(copy, nested);
        assert_eq!(used, bytes.len());
    }
}

#[test]
fn retarget_mapping_and_physical_order_tampering_are_rejected() {
    let p = graph(true);
    let index = p
        .synapses()
        .iter()
        .position(|s| {
            matches!(s.kind(), CompiledSynapseKind::Decoder(c)
        if c.head() == DecoderHeadKind::MemoryContext && c.input_lane() == 53)
        })
        .unwrap();
    let mut wire = serde_json::to_value(&p).unwrap();
    wire["synapses"][index]["kind"]["Decoder"]["input_lane"] = serde_json::json!(31);
    assert!(serde_json::from_value::<BrainPhenotype>(wire).is_err());
    let mut wire = serde_json::to_value(&p).unwrap();
    wire["synapses"]
        .as_array_mut()
        .unwrap()
        .swap(index, index + 1);
    assert!(serde_json::from_value::<BrainPhenotype>(wire).is_err());
    let mut wire = serde_json::to_value(&p).unwrap();
    wire["synapses"][index]["source"] = serde_json::json!(p.synapses()[index].source() + 1);
    assert!(serde_json::from_value::<BrainPhenotype>(wire).is_err());
}

#[test]
fn operation_port_requires_supported_recall_and_is_zero_for_play_vocal_and_empty_memory() {
    let frame = draft(
        SensorProfile::GroundedGripObjectSlotsV1,
        GripOwnershipV1::SelfHeld,
        GripOperationV1::Release,
    )
    .unwrap()
    .finalize(PerceptionContextBlock::empty())
    .unwrap();
    let c = &frame.candidates()[0];
    let mut memory = CandidateMemoryContextV1 {
        candidate_index: 0,
        target_latent: [0.; 8],
        family_value: [0.; 4],
        target_confidence: Confidence::new(0.).unwrap(),
        family_confidence: Confidence::new(0.).unwrap(),
        target_source_count: 0,
        family_source_count: 0,
        best_target_source: None,
        best_family_source: None,
    };
    assert_eq!(
        grip_research::operation_value_input(&frame, c, &memory).unwrap(),
        0.
    );
    // Missing evidence may not become a release signal, even if a caller inserts valence.
    memory.family_value[0] = -1.;
    assert!(grip_research::operation_value_input(&frame, c, &memory).is_err());
    memory.family_source_count = 1;
    memory.best_family_source = Some(MemoryId(1));
    assert!(grip_research::operation_value_input(&frame, c, &memory).is_err());
    memory.family_confidence = Confidence::new(0.25).unwrap();
    assert_eq!(
        grip_research::operation_value_input(&frame, c, &memory).unwrap(),
        -0.25
    );
    for (action, kind) in [
        (ActionId(212), ActionKind::Interact),
        (ActionKind::Vocalize.canonical_id(), ActionKind::Vocalize),
    ] {
        let mut ordinary = *c;
        ordinary.action_id = action;
        ordinary.kind = kind;
        ordinary.family = CandidateActionFamily::Other;
        assert_eq!(
            grip_research::operation_value_input(&frame, &ordinary, &memory).unwrap(),
            0.
        );
    }
    let mut wrong_target = *c;
    wrong_target.target.entity = Some(WorldEntityId(99));
    assert!(grip_research::operation_value_input(&frame, &wrong_target, &memory).is_err());
    let mut missing_target = serde_json::to_value(&frame).unwrap()["base"].clone();
    missing_target["candidates"][0]["target"]["entity"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<PerceptionFrameDraft>(missing_target).is_err());
}
