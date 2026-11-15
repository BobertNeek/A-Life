//! Public checked-in-pair native qualification. No private files or cohort inputs.
use super::*;
use crate::*;
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;

fn fixture(plan2: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    from_pair_with_repertoire(
        include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation"),
        include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation.genome.json"),
        plan2, 0.5)
}
fn collect(
    plan2: bool,
) -> (
    CpuCanonicalHost,
    CpuFoundationTrainer,
    Vec<CpuCollectedDecision>,
) {
    let (mut host, t) = fixture(plan2);
    let before = serde_json::to_vec(&t.checkpoint()).unwrap();
    let mut rows = vec![];
    for _ in 0..2 {
        rows.extend(host.step(&t, 1.).unwrap().decisions);
    }
    assert_eq!(rows.len(), 2);
    assert_eq!(before, serde_json::to_vec(&t.checkpoint()).unwrap());
    (host, t, rows)
}
fn value(t: &CpuFoundationTrainer) -> CpuValueHead {
    CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
        feature_count: t.phenotype().neuron_count(),
        optimizer_step: 0,
        parameters: vec![0.; (t.phenotype().neuron_count() as usize + 1) * 3],
        last_updated_policy_version: None,
    })
    .unwrap()
}
fn config() -> PpoConfig {
    PpoConfig {
        temperature: 1.,
        epochs: 1,
        target_kl: 100.,
        ..Default::default()
    }
}
fn windows(
    rows: &[CpuCollectedDecision],
    host: &CpuCanonicalHost,
    t: &CpuFoundationTrainer,
) -> Result<Vec<CpuResidualTrainingWindow>, TrainingError> {
    cpu_training_windows(
        rows,
        None,
        t,
        &value(t),
        host.world()
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .phenotype(),
        2242,
        config(),
        2,
        0,
    )
}
#[test]
fn plan2_native_rows_roundtrip_replay_probe_and_scratch_update() {
    let (host, t, rows) = collect(true);
    let before = serde_json::to_vec(&t.checkpoint()).unwrap();
    let bytes = serde_json::to_vec(&rows).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("memory_plan2_record_v1"));
    let restored: Vec<CpuCollectedDecision> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
    let a = windows(&rows, &host, &t).unwrap();
    let b = windows(&restored, &host, &t).unwrap();
    assert_eq!(a.len(), 1);
    let qa = &a[0].actor.sequence;
    let qb = &b[0].actor.sequence;
    assert_eq!(
        serde_json::to_vec(qa).unwrap(),
        serde_json::to_vec(qb).unwrap()
    );
    assert_eq!(
        t.evaluate_replay(qa).unwrap(),
        t.evaluate_replay(qb).unwrap()
    );
    let adjoints = vec![vec![1.; qa.ticks[0].candidates.len()]];
    assert_eq!(
        t.probe_replay_gradients(qa, &adjoints).unwrap(),
        t.probe_replay_gradients(qb, &adjoints).unwrap()
    );
    let mut checkpoints = vec![];
    for ws in [a, b] {
        let mut scratch = CpuFoundationTrainer::from_checkpoint(
            t.phenotype().clone(),
            t.source_foundation.clone(),
            t.checkpoint(),
        )
        .unwrap();
        let mut critic = value(&t);
        let receipt = train_cpu_residual_cohort(
            &mut scratch,
            &mut critic,
            ws.len(),
            1,
            |i| Ok(ws[i].clone()),
            config(),
            2242,
        )
        .unwrap();
        assert_eq!(receipt.completed_epochs, 1);
        assert!(!receipt.stopped_for_kl);
        assert_eq!(
            receipt.actor_optimizer_step,
            t.checkpoint().optimizer_step + 1
        );
        assert_eq!(receipt.value_optimizer_step, 1);
        checkpoints.push((
            serde_json::to_vec(&scratch.checkpoint()).unwrap(),
            serde_json::to_vec(&critic.checkpoint()).unwrap(),
        ));
    }
    assert_eq!(checkpoints[0], checkpoints[1]);
    assert_eq!(before, serde_json::to_vec(&t.checkpoint()).unwrap());
    let nested = CpuCollectorTrajectoryV1 {
        final_record: host
            .world()
            .organism_registry()
            .iter()
            .next()
            .unwrap()
            .clone(),
        decisions: restored,
    };
    let nested_bytes = serde_json::to_vec(&nested).unwrap();
    let decoded: CpuCollectorTrajectoryV1 = serde_json::from_slice(&nested_bytes).unwrap();
    assert_eq!(nested_bytes, serde_json::to_vec(&decoded).unwrap());
}
#[test]
fn plan2_rows_reject_mismatch_missing_and_malformed_markers() {
    let (host, t, rows) = collect(true);
    let (oldhost, old) = fixture(false);
    assert!(windows(&rows, &oldhost, &old).is_err());
    let mut missing = rows.clone();
    missing[0].memory_plan2242 = None;
    assert!(windows(&missing, &host, &t).is_err());
    let mut changed = rows.clone();
    changed[0].memory_plan2242.as_mut().unwrap().memory_bound = 1.;
    assert!(windows(&changed, &host, &t).is_err());
    let good = serde_json::to_value(&rows[0]).unwrap();
    for kind in 0..4 {
        let mut bad = good.clone();
        match kind {
            0 => {
                bad["memory_plan2_record_v1"]["plan"]["memory_bound"] = (-1.).into();
            }
            1 => {
                bad["memory_plan2_record_v1"]["plan"]["phenotype_hash"] =
                    serde_json::json!([0, 0, 0, 0]);
            }
            2 => {
                bad["organism_id"] = serde_json::to_value(rows[0].organism_id).unwrap();
            }
            _ => {
                bad["memory_plan2_record_v1"]["row"] = serde_json::Value::Null;
            }
        }
        assert!(serde_json::from_value::<CpuCollectedDecision>(bad).is_err());
    }
    let stripped = good["memory_plan2_record_v1"]["row"].clone();
    let stripped: CpuCollectedDecision = serde_json::from_value(stripped).unwrap();
    assert!(stripped.validate_memory_plan2242(t.phenotype()).is_err());
    assert!(stripped.validate_memory_plan2242(old.phenotype()).is_err());
}
fn legacy_flat_bytes(row: &CpuCollectedDecision) -> Vec<u8> {
    fn execution_work_is_zero(w: &u64) -> bool {
        *w == 0
    }
    #[derive(serde::Serialize)]
    struct Flat<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        joint_membership2177: &'a Option<JointMembershipLiveReceipt2177>,
        #[serde(skip_serializing_if = "Option::is_none")]
        executed_action_recall: &'a Option<Vec<Option<alife_core::ExecutedActionRecallV1>>>,
        #[serde(skip_serializing_if = "execution_work_is_zero")]
        executed_action_recall_work: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        memory_readout_research: &'a Option<CpuMemoryReadoutReceiptV1>,
        #[serde(skip_serializing_if = "Option::is_none")]
        terminal_context2135: &'a Option<TerminalContextReceipt2135>,
        organism_id: &'a OrganismId,
        #[serde(skip_serializing_if = "Option::is_none")]
        sampling_counter: &'a Option<u32>,
        initial: &'a crate::TrainingInitialState,
        replay_tick: &'a crate::TrainingReplayTick,
        action: &'a crate::PpoJointAction,
        final_features: &'a Vec<f32>,
        brain_work: &'a BrainWorkCounters,
        biological_reward: &'a f32,
        patch: &'a ExperiencePatch,
    }
    let flat = Flat {
        joint_membership2177: &row.joint_membership2177,
        executed_action_recall: &row.executed_action_recall,
        executed_action_recall_work: row.executed_action_recall_work,
        memory_readout_research: &row.memory_readout_research,
        terminal_context2135: &row.terminal_context2135,
        organism_id: &row.organism_id,
        sampling_counter: &row.sampling_counter,
        initial: &row.initial,
        replay_tick: &row.replay_tick,
        action: &row.action,
        final_features: &row.final_features,
        brain_work: &row.brain_work,
        biological_reward: &row.biological_reward,
        patch: &row.patch,
    };
    serde_json::to_vec(&flat).unwrap()
}
#[test]
fn legacy_rows_keep_original_flat_bytes() {
    let (host, t, rows) = collect(false);
    assert!(windows(&rows, &host, &t).is_ok());
    for row in &rows {
        assert!(row.memory_plan2242.is_none());
        let expected = legacy_flat_bytes(row);
        let actual = serde_json::to_vec(row).unwrap();
        assert_eq!(expected, actual);
        let restored: CpuCollectedDecision = serde_json::from_slice(&actual).unwrap();
        assert_eq!(actual, serde_json::to_vec(&restored).unwrap());
    }
}

pub(super) fn fixture_bound2266(bound: f32) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    from_pair_with_repertoire(
        include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation"),
        include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation.genome.json"),
        true, bound)
}

fn from_pair_with_repertoire(
    asset_bytes: &[u8],
    genome_bytes: &[u8],
    plan2: bool,
    bound: f32,
) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let mut asset = FoundationWeightAsset::decode_canonical(asset_bytes).unwrap();
    let mut genome: CreatureGenome = serde_json::from_slice(genome_bytes).unwrap();
    if plan2 {
        for architecture in [
            &mut genome.brain.construction.maternal_architecture,
            &mut genome.brain.construction.paternal_architecture,
        ] {
            *architecture = architecture.with_memory_candidate_bound_v4(bound).unwrap();
        }
    }
    assert!(
        genome
            .n2048_foundation_candidate
            .as_ref()
            .unwrap()
            .encode_canonical()
            .unwrap()
            == asset_bytes
    );
    if plan2 {
        let expressed = genome.express().unwrap();
        let development = foundation_construction_development(
            &expressed.brain_genome,
            &BrainCapacityClass::n2048(),
            &expressed.development_state_at(Tick::ZERO).unwrap(),
        )
        .unwrap();
        let (fresh, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
            expressed.brain_genome.clone(),
            development,
            asset.manifest().sensor_profile(),
        )
        .unwrap();
        let rebound = FoundationWeightAsset::from_trained_weights(
            &fresh,
            asset.weights().to_vec(),
            asset.manifest().training_stage(),
        )
        .unwrap();
        assert_eq!(asset.weights().len(), 32768);
        assert!(asset
            .weights()
            .iter()
            .zip(rebound.weights())
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        assert_eq!(
            asset.manifest().address_map_digest(),
            rebound.manifest().address_map_digest()
        );
        genome = genome
            .with_n2048_foundation_candidate(rebound.clone())
            .unwrap();
        asset = rebound;
    }
    let mut config = CanonicalNewGameConfig::phase3(2026111701, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset).unwrap();
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let expressed = genome.express().unwrap();
    let development = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    assert_eq!(
        p.candidate_decoder().factorized_motor_channels(&p).unwrap(),
        vec![MotorChannel::Locomotion, MotorChannel::Manipulation]
    );
    assert_eq!(
        p.candidate_decoder()
            .memory_channel()
            .unwrap()
            .schema_version(),
        if plan2 { 2 } else { 1 }
    );
    assert!(p
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    assert_eq!(
        p.candidate_decoder()
            .memory_channel()
            .unwrap()
            .max_candidate_gain(),
        if plan2 { bound } else { 0.5 }
    );
    let chemistry = *old.biochemistry();
    game.world
        .replace_organism_record_exact(
            WorldOrganismRecord::new(
                old.organism_id(),
                old.world_entity_id(),
                genome.clone(),
                expressed,
                chemistry,
                old.birth_tick(),
            )
            .unwrap(),
        )
        .unwrap();
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(&genome).unwrap()).0;
    let mask = StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 42).unwrap(),
        trainer,
    )
}

// A non-human-readable serializer probe, not a claim about a particular binary codec.
// It delegates values to JSON's value serializer while exercising the binary gate.
struct NonHumanProbe;
impl serde::Serializer for NonHumanProbe {
    type Ok = serde_json::Value;
    type Error = serde_json::Error;
    type SerializeSeq = <serde_json::value::Serializer as serde::Serializer>::SerializeSeq;
    type SerializeTuple = <serde_json::value::Serializer as serde::Serializer>::SerializeTuple;
    type SerializeTupleStruct =
        <serde_json::value::Serializer as serde::Serializer>::SerializeTupleStruct;
    type SerializeTupleVariant =
        <serde_json::value::Serializer as serde::Serializer>::SerializeTupleVariant;
    type SerializeMap = <serde_json::value::Serializer as serde::Serializer>::SerializeMap;
    type SerializeStruct = <serde_json::value::Serializer as serde::Serializer>::SerializeStruct;
    type SerializeStructVariant =
        <serde_json::value::Serializer as serde::Serializer>::SerializeStructVariant;
    fn is_human_readable(&self) -> bool {
        false
    }
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_bool(serde_json::value::Serializer, v)
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_i8(serde_json::value::Serializer, v)
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_i16(serde_json::value::Serializer, v)
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_i32(serde_json::value::Serializer, v)
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_i64(serde_json::value::Serializer, v)
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_u8(serde_json::value::Serializer, v)
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_u16(serde_json::value::Serializer, v)
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_u32(serde_json::value::Serializer, v)
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_u64(serde_json::value::Serializer, v)
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_f32(serde_json::value::Serializer, v)
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_f64(serde_json::value::Serializer, v)
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_char(serde_json::value::Serializer, v)
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_str(serde_json::value::Serializer, v)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_bytes(serde_json::value::Serializer, v)
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_none(serde_json::value::Serializer)
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_unit(serde_json::value::Serializer)
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_unit_struct(serde_json::value::Serializer, name)
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_unit_variant(
            serde_json::value::Serializer,
            name,
            index,
            variant,
        )
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        serde::Serializer::serialize_seq(serde_json::value::Serializer, len)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        serde::Serializer::serialize_tuple(serde_json::value::Serializer, len)
    }
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        serde::Serializer::serialize_tuple_struct(serde_json::value::Serializer, name, len)
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        serde::Serializer::serialize_tuple_variant(
            serde_json::value::Serializer,
            name,
            index,
            variant,
            len,
        )
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        serde::Serializer::serialize_map(serde_json::value::Serializer, len)
    }
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        serde::Serializer::serialize_struct(serde_json::value::Serializer, name, len)
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        serde::Serializer::serialize_struct_variant(
            serde_json::value::Serializer,
            name,
            index,
            variant,
            len,
        )
    }
    fn serialize_some<T: ?Sized + serde::Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_some(serde_json::value::Serializer, value)
    }
    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_newtype_struct(serde_json::value::Serializer, name, value)
    }
    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        serde::Serializer::serialize_newtype_variant(
            serde_json::value::Serializer,
            name,
            index,
            variant,
            value,
        )
    }
}
#[test]
fn plan2_rejects_nonhuman_wire_while_legacy_keeps_flat_serialization() {
    use serde::Serialize;
    let (_, _, old) = collect(false);
    let (_, _, new) = collect(true);
    assert_eq!(
        old[0].serialize(NonHumanProbe).unwrap(),
        serde_json::to_value(&old[0]).unwrap()
    );
    let error = new[0].serialize(NonHumanProbe).unwrap_err();
    assert!(error.to_string().contains("versioned JSON envelope"));
}
