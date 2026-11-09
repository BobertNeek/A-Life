//! Authored bounded native/world controls, NOT executed in source-only preparation.
//! Fixed disposable fixtures are not autonomous learning or scientific life evidence.
use super::*;
use crate::{AdamWConfig, StageTrainableMask};
use alife_world::*;
#[allow(dead_code)]
#[path = "grouped_host_fixture.rs"]
mod recipe;
// Fixed mnemonic control seed, declared before any execution. No seed search.
const FIXTURE_SEED: u64 = 0x004f_5054_564f_4331;
fn build<P: GroupedPolicyStrategy>() -> (CpuGroupedM2HostV1, PhenotypeCompilerInputs) {
    eprintln!("DUAL_COUNT recipe OPTVOC1 preparation=0");
    let recipe = recipe::build(FIXTURE_SEED, true).unwrap();
    let mut founder = CpuFoundationTrainer::new(
        recipe.phenotype.clone(),
        recipe.asset.clone(),
        StageTrainableMask::recurrent_only(&recipe.phenotype).unwrap(),
        AdamWConfig::default(),
    )
    .unwrap();
    // Disposable optimizer sentinels, not trained or imported acquired state.
    let mut optimizer = founder.checkpoint();
    optimizer.optimizer_step = 23;
    optimizer.update_ages[0] = 23;
    optimizer.first_moment[0] = 0.03125;
    optimizer.second_moment[0] = 0.0625;
    founder =
        CpuFoundationTrainer::from_checkpoint(recipe.phenotype.clone(), recipe.asset, optimizer)
            .unwrap();
    // This fixture must retain the audited zero payload head; no token seeding.
    assert!(founder.phenotype().synapses().iter().enumerate().filter(|(_,s)|
        matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload))
        .all(|(i,_)|founder.weights()[i].to_bits()==0_f32.to_bits()));
    let mut game =
        create_grounded_drive_research_new_game(&recipe.config, &recipe.genotype).unwrap();
    let newborn = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let mut blank = HeadlessScenarioBuilder::new(FIXTURE_SEED)
        .agent("learner", newborn.organism_id(), Vec3f::ZERO)
        .build()
        .unwrap();
    blank
        .register_grounded_drive_research_record(newborn)
        .unwrap();
    game.world = blank;
    let inputs = recipe.inputs;
    let (host, ()) = CpuGroupedM2HostV1::new_prepared_policy::<P, _, String>(
        game,
        founder,
        inputs.clone(),
        (FIXTURE_SEED ^ (FIXTURE_SEED >> 32)) as u32,
        MediationModeV1::Full,
        |_| Ok(()),
    )
    .unwrap();
    (host, inputs)
}
fn fresh() -> (CpuOptionalVocalM2HostV1, PhenotypeCompilerInputs) {
    let (inner, inputs) = build::<OptionalVocalPolicy>();
    (CpuOptionalVocalM2HostV1 { inner }, inputs)
}
fn frozen(host: &CpuOptionalVocalM2HostV1) -> Vec<u8> {
    let r = host.inner.host.residents.values().next().unwrap();
    let n = r.neural.checkpoint().unwrap();
    serde_json::to_vec(&(
        host.inner.founder.checkpoint(),
        n.lifetime,
        n.fast,
        n.eligibility,
        n.weight_generation,
        n.neural.dendrites,
        r.v11.checkpoint(),
    ))
    .unwrap()
}
#[cfg(any())]
#[test]
fn optional_host_source_once_quiet_physics_and_exact_next_boundary() {
    let (mut warm, inputs) = fresh();
    let frozen_before = frozen(&warm);
    let first = warm.step(0.73).unwrap();
    assert_eq!(first.decisions.len(), 1);
    let d = &first.decisions[0];
    assert_eq!(first.tick_after.raw(), first.tick_before.raw() + 1);
    assert_eq!(d.motor.native.core.source_execution_count, 1);
    assert_eq!(d.motor.native.core.final_generation, 1);
    assert_eq!(d.action.counter(), d.sampling_counter);
    let scores = warm.last_selection_scores(d.organism_id).unwrap();
    assert!(!scores.source().payload_present());
    scores
        .validate_sampling_context(
            &d.action,
            d.action.seed(),
            d.sampling_counter,
            f64::from(0.73_f32),
        )
        .unwrap();
    assert_eq!(scores.physical_selection(&d.action).unwrap().1[3], None);
    assert_eq!(
        d.action.motors()[3],
        crate::cpu::OptionalVocalChoiceV1::Quiet
    );
    assert!(d
        .patch
        .selected_bundle()
        .unwrap()
        .channels
        .iter()
        .all(|c| c.channel != MotorChannel::Vocal));
    assert_eq!(d.physical.outcome_tick.raw(), first.tick_after.raw());
    assert_eq!(
        warm.world()
            .organism_registry()
            .get(d.organism_id)
            .unwrap()
            .biochemistry()
            .tick,
        first.tick_after
    );
    assert_eq!(warm.world().audible_utterances().len(), 0);
    let charged = d.patch.decision().cognitive_work.as_ref().unwrap();
    assert_eq!(charged.learning_ops, 0);
    assert_eq!(charged.neural_updates, d.brain_work.neuron_updates);
    assert_eq!(charged.synapses_evaluated, d.brain_work.synapse_ops);
    let extra = GroupedMotorStateV1::append_work(warm.inner.founder.phenotype()).unwrap();
    assert_eq!(
        charged.neural_updates,
        2048 * u64::from(d.brain_work.microsteps) + extra.neuron_updates
    );
    assert!(d.motor.applied_body_energy_debit > 0.);
    assert_eq!(frozen(&warm), frozen_before);
    let wire = warm.checkpoint().unwrap().encode().unwrap();
    let mut cold = CpuOptionalVocalM2HostV1::restore(
        CpuOptionalVocalM2HostCheckpointV1::decode(&wire).unwrap(),
        &inputs,
        MediationModeV1::Full,
    )
    .unwrap();
    assert_eq!(cold.checkpoint().unwrap().encode().unwrap(), wire);
    let a = warm.step(0.73).unwrap();
    let b = cold.step(0.73).unwrap();
    assert_eq!(a.decisions.len(), b.decisions.len());
    for (x, y) in a.decisions.iter().zip(&b.decisions) {
        assert_eq!(
            serde_json::to_vec(&x.action).unwrap(),
            serde_json::to_vec(&y.action).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&x.patch).unwrap(),
            serde_json::to_vec(&y.patch).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&x.motor).unwrap(),
            serde_json::to_vec(&y.motor).unwrap()
        );
    }
    assert_eq!(
        warm.checkpoint().unwrap().encode().unwrap(),
        cold.checkpoint().unwrap().encode().unwrap()
    );
    assert_eq!(frozen(&warm), frozen_before);
    assert_eq!(frozen(&cold), frozen_before);
}
#[cfg(any())]
#[test]
fn optional_host_rejects_opposite_policy_and_bad_temperature_transactionally() {
    let (mut new, _) = fresh();
    let (old, _) = build::<LegacyGroupedPolicy>();
    let new_wire = new.checkpoint().unwrap().encode().unwrap();
    let old_wire = old.checkpoint().unwrap().encode().unwrap();
    assert!(CpuGroupedM2HostCheckpointV1::decode(&new_wire).is_err());
    assert!(CpuOptionalVocalM2HostCheckpointV1::decode(&old_wire).is_err());
    assert!(new.inner.checkpoint().is_err());
    assert!(new
        .inner
        .host
        .grouped_step(&new.inner.founder, 0.73)
        .is_err());
    assert_eq!(new.checkpoint().unwrap().encode().unwrap(), new_wire);
    assert!(new.step(f32::NAN).is_err());
    assert_eq!(new.checkpoint().unwrap().encode().unwrap(), new_wire);
    assert!(new.actor_optimization().is_err());
    assert!(new.waking_synaptic_plasticity().is_err());
    assert!(new.synaptic_sleep_consolidation().is_err());
    assert_eq!(new.checkpoint().unwrap().encode().unwrap(), new_wire);
    let old_json: serde_json::Value =
        serde_json::from_slice(&old_wire[super::super::MAGIC.len()..]).unwrap();
    assert!(old_json["controllers"]
        .as_object()
        .unwrap()
        .values()
        .all(|r| r.get("optional_vocal").is_none()));
}
#[cfg(any())]
#[test]
fn quiet_does_not_change_real_empty_payload_or_cooldown_failure() {
    let (host, _) = fresh();
    let id = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let command = ActionCommand::structured(
        id,
        ActionId(400),
        ActionKind::Vocalize,
        ActionTarget::NONE,
        Intensity::new(1.).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    // Deliberate fixture commands bypass the new sampler, not the physical guard.
    let mut empty = host.world().clone();
    let failed = empty.apply_neural_command(&command, None, false).unwrap();
    assert!(!failed.execution.succeeded);
    assert_eq!(
        failed.execution.failure,
        Some(ReferenceActionFailure::ActionRejected)
    );
    assert!(failed.emitted_utterance.is_none());
    assert!(empty.audible_utterances().is_empty());
    let payload = SpeechMotorPayload::try_new(
        SpeechActKind::Declare,
        vec![LanguageTokenId::new(1).unwrap()],
        Confidence::new(1.).unwrap(),
    )
    .unwrap();
    let mut speech = host.world().clone();
    let succeeded = speech
        .apply_neural_command(&command, Some(payload.clone()), false)
        .unwrap();
    assert!(succeeded.execution.succeeded);
    assert!(succeeded.emitted_utterance.is_some());
    let cooldown = speech
        .apply_neural_command(&command, Some(payload), false)
        .unwrap();
    assert!(!cooldown.execution.succeeded);
    assert_eq!(
        cooldown.execution.failure,
        Some(ReferenceActionFailure::ActionRejected)
    );
    assert!(cooldown.emitted_utterance.is_none());
    assert_eq!(speech.audible_utterances().len(), 1);
}
// Test-only adapters deliberately cross the pending owner, never exposed publicly.
struct RejectLegacyStage;
struct RejectLegacyClose;
macro_rules! optional_strategy_constants {
    () => {
        type Action = OptionalVocalActionV1;
        const OPTIONAL: bool = true;
        const MAGIC: &'static [u8] = OPTIONAL_MAGIC;
        const LINEAGE: &'static str = OPTIONAL_LINEAGE;
        const POLICY: &'static str = OPTIONAL_VOCAL_POLICY_V1;
    };
}
impl GroupedPolicyStrategy for RejectLegacyStage {
    optional_strategy_constants!();
    fn select(
        f: &CpuFoundationTrainer,
        r: &mut Resident,
        p: &PerceptionFrame,
        o: &CpuDecisionOutput,
        s: u32,
        c: u32,
        t: f64,
    ) -> Result<Selection<Self::Action>, CpuHostError> {
        OptionalVocalPolicy::select(f, r, p, o, s, c, t)
    }
    fn stage(
        r: &mut Resident,
        p: &BrainPhenotype,
        a: &Self::Action,
        b: CpuDecisionBinding,
    ) -> Result<(), CpuHostError> {
        let scores = &r
            .grouped
            .as_ref()
            .unwrap()
            .motor
            .last_receipt
            .as_ref()
            .unwrap()
            .native
            .scores;
        let legacy = scores.sample(a.seed(), a.counter(), a.temperature())?;
        assert!(r
            .neural
            .stage_grouped_closed_selection(p, &legacy, scores, b)
            .is_err());
        Err(CpuHostError::State(
            "fixed legacy stage rejection after optional forward",
        ))
    }
    fn close(
        _: &mut CpuOrganismState,
        _: &CpuFoundationTrainer,
        _: &ExperiencePatch,
        _: &NeuralReceptorFrame,
    ) -> Result<(), CpuHostError> {
        panic!("stage must reject before physical execution")
    }
}
impl GroupedPolicyStrategy for RejectLegacyClose {
    optional_strategy_constants!();
    fn select(
        f: &CpuFoundationTrainer,
        r: &mut Resident,
        p: &PerceptionFrame,
        o: &CpuDecisionOutput,
        s: u32,
        c: u32,
        t: f64,
    ) -> Result<Selection<Self::Action>, CpuHostError> {
        OptionalVocalPolicy::select(f, r, p, o, s, c, t)
    }
    fn stage(
        r: &mut Resident,
        p: &BrainPhenotype,
        a: &Self::Action,
        b: CpuDecisionBinding,
    ) -> Result<(), CpuHostError> {
        OptionalVocalPolicy::stage(r, p, a, b)
    }
    fn close(
        n: &mut CpuOrganismState,
        f: &CpuFoundationTrainer,
        p: &ExperiencePatch,
        r: &NeuralReceptorFrame,
    ) -> Result<(), CpuHostError> {
        assert!(n.close_grouped_outcome(f, p, r).is_err());
        Err(CpuHostError::State(
            "fixed legacy close rejection after optional physical transaction",
        ))
    }
}
#[cfg(any())]
#[test]
fn optional_pending_owner_rejects_legacy_stage_and_close() {
    let (mut host, _) = fresh();
    let before = host.checkpoint().unwrap().encode().unwrap();
    assert!(matches!(
        host.inner
            .host
            .grouped_policy_step::<RejectLegacyStage>(&host.inner.founder, 0.73),
        Err(CpuHostError::State(
            "fixed legacy stage rejection after optional forward"
        ))
    ));
    assert_eq!(host.checkpoint().unwrap().encode().unwrap(), before);
    assert!(matches!(
        host.inner
            .host
            .grouped_policy_step::<RejectLegacyClose>(&host.inner.founder, 0.73),
        Err(CpuHostError::State(
            "fixed legacy close rejection after optional physical transaction"
        ))
    ));
    assert_eq!(host.checkpoint().unwrap().encode().unwrap(), before);
}

#[cfg(any())]
#[test]
fn optional_extracted_inner_source_and_canonical_owners_reject_old_loaders() {
    let (host, _) = fresh();
    let checkpoint = host.checkpoint().unwrap();
    let wire = checkpoint.encode().unwrap();
    let metadata: serde_json::Value =
        serde_json::from_slice(&wire[OPTIONAL_MAGIC.len()..]).unwrap();
    // Exactly the serialized inner state, not a privately accessible wrapper.
    let canonical: CpuCanonicalCheckpoint =
        serde_json::from_value(metadata["canonical"].clone()).unwrap();
    let source: CpuOrganismCheckpoint =
        serde_json::from_value(metadata["canonical"]["residents"][0]["neural"].clone()).unwrap();
    assert_eq!(canonical.schema_version, OPTIONAL_VOCAL_CANONICAL_SCHEMA);
    assert_eq!(
        source.schema_version,
        crate::cpu::organism::OPTIONAL_VOCAL_NEURAL_SCHEMA
    );
    assert!(CpuOrganismState::from_checkpoint(host.inner.founder.phenotype(), source).is_err());
    assert!(CpuCanonicalHost::from_checkpoint(canonical.clone(), &host.inner.founder).is_err());
    let research = CpuGroundedDriveResearchCheckpoint {
        schema_version: 1,
        recipe_digest: N2048GroundedDriveBasisV1::recipe_digest(),
        policy: BrainActivityPolicyV1::grounded_drive_research_v1(),
        source_asset_digest: host.inner.founder.source_foundation.digest(),
        checkpoint: canonical,
    };
    assert!(CpuCanonicalHost::from_grounded_drive_research_checkpoint(
        research,
        &host.inner.founder
    )
    .is_err());
    for variant in 0..3 {
        let mut tampered = metadata.clone();
        match variant {
            0 => tampered["canonical"]["schema_version"] = serde_json::json!(2),
            1 => {
                tampered["canonical"]["residents"][0]["neural"]["schema_version"] =
                    serde_json::json!(2)
            }
            _ => {
                for state in tampered["controllers"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                {
                    state.as_object_mut().unwrap().remove("optional_vocal");
                }
            }
        }
        let mut bytes = OPTIONAL_MAGIC.to_vec();
        bytes.extend(serde_json::to_vec(&tampered).unwrap());
        assert!(CpuOptionalVocalM2HostCheckpointV1::decode(&bytes).is_err());
    }
    let mut foreign = metadata.clone();
    foreign["source_lineage"] = serde_json::json!(super::super::LINEAGE);
    foreign["policy"] = serde_json::json!(super::super::POLICY);
    let mut disguised = super::super::MAGIC.to_vec();
    disguised.extend(serde_json::to_vec(&foreign).unwrap());
    assert!(CpuGroupedM2HostCheckpointV1::decode(&disguised).is_err());
    // Even explicit raw marker stripping cannot silently discard the proof
    // through the old host: world brain authority still commits to it.
    let mut stripped: CpuCanonicalCheckpoint =
        serde_json::from_value(metadata["canonical"].clone()).unwrap();
    stripped.schema_version = 2;
    for r in &mut stripped.residents {
        r.neural.schema_version = 2;
    }
    let envelope = CpuGroundedDriveResearchCheckpoint {
        schema_version: 1,
        recipe_digest: N2048GroundedDriveBasisV1::recipe_digest(),
        policy: BrainActivityPolicyV1::grounded_drive_research_v1(),
        source_asset_digest: host.inner.founder.source_foundation.digest(),
        checkpoint: stripped,
    };
    assert!(CpuCanonicalHost::from_grounded_drive_research_checkpoint(
        envelope,
        &host.inner.founder
    )
    .is_err());
    assert_eq!(host.checkpoint().unwrap().encode().unwrap(), wire);
}

// Private future qualification overlay; not part of production candidate.
// Append inside optional_vocal_host_tests.rs. Substitute CpuOptionalVocalM2HostV1 only.
fn composition_step_bytes<A: serde::Serialize>(s: &CpuGroupedM2HostStepV1<A>) -> Vec<u8> {
    serde_json::to_vec(&(
        s.tick_before, s.tick_after, s.sleeping_organisms, s.sleep_consolidations,
        s.decisions.iter().map(|d| (
            d.organism_id, &d.action, d.sampling_counter, &d.source_diagnostic_logits,
            &d.biological_emissions, &d.brain_work, &d.patch, &d.motor,
            &d.physical, d.sleep_phase, d.sleep_trigger,
        )).collect::<Vec<_>>(),
    )).unwrap()
}
#[test]
fn dual_policy_cache_complete_warm_cold_observer_rollback_witness() {
    use sha2::{Digest, Sha256};
    let (mut warm, inputs) = fresh();
    let frozen_before = frozen(&warm); // Includes nonzero optimizer sentinels.
    let mut witness = Sha256::new();
    let mut absorb = |b: Vec<u8>| { crate::qualification_sink::write_frame(&b); witness.update((b.len() as u64).to_le_bytes()); witness.update(b); };
    absorb(warm.checkpoint().unwrap().encode().unwrap());
    let first = warm.step(0.73).unwrap();
    absorb(composition_step_bytes(&first));
    let checkpoint = warm.checkpoint().unwrap();
    let wire = checkpoint.encode().unwrap();
    absorb(wire.clone());
    // In-process checkpoint clones carry warm transient caches until restore.
    let mut cold = CpuOptionalVocalM2HostV1::restore(checkpoint.clone(), &inputs, MediationModeV1::Full).unwrap();
    let mut observed_warm = CpuOptionalVocalM2HostV1::restore(checkpoint.clone(), &inputs, MediationModeV1::Full).unwrap();
    let mut observed_cold = CpuOptionalVocalM2HostV1::restore(checkpoint, &inputs, MediationModeV1::Full).unwrap();
    let a = warm.step(0.73).unwrap();
    let b = cold.step(0.73).unwrap();
    let c = observed_warm.step_research_observed(0.73).unwrap();
    let d = observed_cold.step_research_observed(0.73).unwrap();
    let output = composition_step_bytes(&a);
    assert_eq!(output, composition_step_bytes(&b));
    assert_eq!(output, composition_step_bytes(&c.step));
    assert_eq!(output, composition_step_bytes(&d.step));
    assert_eq!(c.world_receipts, d.world_receipts);
    assert_eq!(receipt_complete_bytes(&c.world_receipts), receipt_complete_bytes(&d.world_receipts));
    absorb(output);
    absorb(receipt_complete_bytes(&c.world_receipts));
    let after = warm.checkpoint().unwrap().encode().unwrap();
    absorb(after.clone());
    for h in [&warm, &cold, &observed_warm, &observed_cold] {
        assert_eq!(h.checkpoint().unwrap().encode().unwrap(), after);
        assert_eq!(frozen(h), frozen_before);
    }
    // Fixed late sampler failure, not a search or arbitrary fault schedule.
    assert!(warm.step(f32::NAN).is_err());
    assert!(cold.step(f32::NAN).is_err());
    assert!(observed_warm.step_research_observed(f32::NAN).is_err());
    assert!(observed_cold.step_research_observed(f32::NAN).is_err());
    for h in [&warm, &cold, &observed_warm, &observed_cold] {
        assert_eq!(h.checkpoint().unwrap().encode().unwrap(), after);
        assert_eq!(frozen(h), frozen_before);
    }
    absorb(warm.checkpoint().unwrap().encode().unwrap());
    drop(absorb);
    println!("DUAL_POLICY_FULL_OUTPUT_SHA256={:x}", witness.finalize());
}

use serde_json::{json, Value};
fn transfer(t: &GripTransfer) -> Value {
    json!({"object":t.object,"kind":format!("{:?}",t.kind),"previous_owner":t.previous_owner,"new_owner":t.new_owner,"previous_position":t.previous_position,"new_position":t.new_position})
}
fn world_receipt(r: &HeadlessMotorTransactionReceipt) -> Value {
    let channels=r.channel_receipts.iter().map(|c|json!({"command":c.command,"phase":format!("{:?}",c.phase),"grip_transfers":c.grip_transfers.iter().map(transfer).collect::<Vec<_>>(),"play_stimulation":c.play_stimulation,"investigation":c.investigation,"observation":c.observation,"physical":c.physical})).collect::<Vec<_>>();
    json!({"bundle":r.bundle,"outcome_tick":r.outcome_tick,"succeeded":r.succeeded,"joint":r.joint,"channels":channels,"grip_transfers":r.grip_transfers.iter().map(transfer).collect::<Vec<_>>(),"body_event":r.body_event,"biology_before":r.biology_before,"biology_after":r.biology_after})
}

fn receipt_body_bits(b: &BodyEventDelta) -> Vec<u32> {
    vec![b.energy, b.damage, b.temperature_stress, b.nutrition, b.social_contact,
        b.player_reward, b.play_stimulation, b.perceived_novelty, b.investigation,
        b.sleep_recovery, b.mating_opportunity].into_iter().map(f32::to_bits).collect()
}
fn receipt_keys(value: &Value, names: &[&str]) {
    let actual: std::collections::BTreeSet<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(actual, names.iter().copied().collect());
}
fn receipt_complete_bytes(receipts: &[HeadlessMotorTransactionReceipt]) -> Vec<u8> {
    let originals = receipts.iter().map(world_receipt).collect::<Vec<_>>();
    for (r, j) in receipts.iter().zip(&originals) {
        receipt_keys(j, &["bundle", "outcome_tick", "succeeded", "joint", "channels", "grip_transfers", "body_event", "biology_before", "biology_after"]);
        assert_eq!(j["channels"].as_array().unwrap().len(), r.channel_receipts.len());
        for c in j["channels"].as_array().unwrap() {
            receipt_keys(c, &["command", "phase", "grip_transfers", "play_stimulation", "investigation", "observation", "physical"]);
        }
        for t in r.grip_transfers.iter().chain(r.channel_receipts.iter().flat_map(|c| &c.grip_transfers)) {
            receipt_keys(&transfer(t), &["object", "kind", "previous_owner", "new_owner", "previous_position", "new_position"]);
        }
    }
    // Retain exact historical codec bytes as an opaque byte array, not parsed/reencoded.
    // Debug is supplemental, not a replacement codec: derived Debug includes private
    // nested fields skipped by serde. BodyEventDelta raw bits explicitly bind -0 skips.
    serde_json::to_vec(&(
        serde_json::to_vec(&originals).unwrap(),
        format!("{:?}", receipts),
        receipts.iter().map(|r| receipt_body_bits(&r.body_event)).collect::<Vec<_>>(),
    )).unwrap()
}
#[cfg(any())]
#[test]
fn dual_policy_receipt_codec_all_fields_null_signed_zero_and_framing() {
    // Pure value/encoding controls only. No phenotype, source, biology, world,
    // host, founder, recipe, or native-forward constructor is called.
    for kind in [GripTransferKind::Acquired, GripTransferKind::Released, GripTransferKind::Placed, GripTransferKind::Consumed] {
        let t = GripTransfer { object: WorldEntityId(7), previous_owner: None,
            new_owner: Some(OrganismId(9)), previous_position: Vec3f::new(-0.0, 0.25, -0.5),
            new_position: Vec3f::new(0.0, -0.25, 0.5), kind };
        let j = transfer(&t);
        receipt_keys(&j, &["object", "kind", "previous_owner", "new_owner", "previous_position", "new_position"]);
        assert_eq!(j["object"], json!(7)); assert_eq!(j["previous_owner"], Value::Null);
        assert_eq!(j["new_owner"], json!(9)); assert_eq!(j["kind"], json!(format!("{:?}", kind)));
        let mut reverse = t;
        reverse.previous_owner = Some(OrganismId(3)); reverse.new_owner = None;
        let reversed = transfer(&reverse);
        assert_eq!(reversed["previous_owner"], json!(3)); assert!(reversed["new_owner"].is_null());
        assert_ne!(serde_json::to_vec(&j).unwrap(), serde_json::to_vec(&reversed).unwrap());
        let bytes = serde_json::to_vec(&j).unwrap();
        let decoded: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!((decoded["previous_position"]["x"].as_f64().unwrap() as f32).to_bits(), (-0.0_f32).to_bits());
        assert_eq!((decoded["new_position"]["x"].as_f64().unwrap() as f32).to_bits(), 0.0_f32.to_bits());
        assert_eq!(decoded["previous_position"], json!(t.previous_position));
        assert_eq!(decoded["new_position"], json!(t.new_position));
    }
    assert_ne!(serde_json::to_vec(&Value::Null).unwrap(), serde_json::to_vec(&json!([])).unwrap());
    assert!(!receipt_complete_bytes(&[]).is_empty());
    let positive = BodyEventDelta::zero();
    let mut negative = positive;
    negative.player_reward = -0.0; negative.play_stimulation = -0.0;
    negative.perceived_novelty = -0.0; negative.investigation = -0.0;
    assert_eq!(serde_json::to_vec(&positive).unwrap(), serde_json::to_vec(&negative).unwrap());
    assert_ne!(receipt_body_bits(&positive), receipt_body_bits(&negative));
    assert_ne!(format!("{:?}", positive), format!("{:?}", negative));
    assert_eq!(&receipt_body_bits(&negative)[5..9], &[0x80000000; 4]);
    assert_eq!(receipt_body_bits(&negative).len(), 11);
    let transaction = json!({"bundle":{},"outcome_tick":7,"succeeded":false,"joint":{},
        "channels":[{"command":{},"phase":"Attempt","grip_transfers":[],"play_stimulation":-0.0,
            "investigation":0.0,"observation":null,"physical":{}}],"grip_transfers":[],
        "body_event":{},"biology_before":{},"biology_after":{}});
    receipt_keys(&transaction, &["bundle","outcome_tick","succeeded","joint","channels","grip_transfers","body_event","biology_before","biology_after"]);
    receipt_keys(&transaction["channels"][0], &["command","phase","grip_transfers","play_stimulation","investigation","observation","physical"]);
    let json_bytes = serde_json::to_vec(&transaction).unwrap();
    let roundtrip: Value = serde_json::from_slice(&json_bytes).unwrap();
    assert!(roundtrip["channels"][0]["observation"].is_null());
    assert_eq!((roundtrip["channels"][0]["play_stimulation"].as_f64().unwrap() as f32).to_bits(), 0x80000000);
    let payloads = vec![Vec::new(), (0..=255).collect::<Vec<u8>>(), json_bytes,
        serde_json::to_vec(&receipt_body_bits(&negative)).unwrap(), b"null\0[]\n-0.0".to_vec()];
    let mut expected = Vec::new();
    for bytes in &payloads {
        expected.extend_from_slice(&(bytes.len() as u64).to_le_bytes()); expected.extend_from_slice(bytes);
        crate::qualification_sink::write_frame(bytes);
    }
    let actual = std::fs::read(std::env::var("DUAL_WITNESS_PATH").unwrap()).unwrap();
    assert_eq!(actual, expected);
    let mut cursor = 0;
    for bytes in &payloads {
        let length = u64::from_le_bytes(actual[cursor..cursor+8].try_into().unwrap()) as usize; cursor += 8;
        assert_eq!(length, bytes.len()); assert_eq!(&actual[cursor..cursor+length], bytes); cursor += length;
    }
    assert_eq!(cursor, actual.len());
    eprintln!("DUAL_CODEC_CONTROL pure=true complete_frames=5 source_calls=0 world_calls=0");
}
