//! Nine-arm native acquired-parent locomotion-readout corner tournament. No action forcing, private reset or new senses.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, FoundationTrainerCheckpoint, TrainingSequence};
use alife_world::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
enum Recipe {
    Candidate,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
 asset:PathBuf,genome:PathBuf,actor_checkpoint:PathBuf,founder_checkpoint:PathBuf,outdir:PathBuf,
 edits:Vec<(u32,u32)>,fixture_id:u8,horizon:usize,temperature:f32,observe:bool,
}
fn validate_temperature(temperature: f32) -> Result<()> {
    if temperature.to_bits() != 1_f32.to_bits() {
        return Err("temperature must be1".into());
    }
    Ok(())
}
fn bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}
fn identity(
    h: &CpuCanonicalHost,
    t: &CpuFoundationTrainer,
    g: &CreatureGenome,
    recipe: Recipe,
) -> Result<OrganismId> {
    let cp = h.checkpoint(t)?;
    let expected = if recipe == Recipe::Candidate {
        (18, Some(3), Some(14), 5, true)
    } else {
        (2, None, None, 4, false)
    };
    if (
        cp.schema_version,
        cp.candidate_opportunity_coverage2313,
        cp.joint_membership2177,
        t.phenotype().cognitive_architecture().schema_version(),
        t.phenotype().cognitive_architecture().grip_proprioception(),
    ) != expected
        || cp.residents.len() != 1
        || h.world().age_death_disabled()
    {
        return Err("recipe identity/normalage mismatch".into());
    }
    let record = h.world().organism_registry().iter().next().ok_or("actor")?;
    if bytes(record.genome())? != bytes(g)? {
        return Err("exactgenome mismatch".into());
    }
    if t.weights().len()!=32768 || t.weights().iter().any(|w|!w.is_finite()) {return Err("invalidpolicy".into());}
    Ok(record.organism_id())
}
fn trainer_value(t: &CpuFoundationTrainer) -> Result<u32> {
    let bits = t.weights()[30048].to_bits();
    if ![0, 0x3ecccccd].contains(&bits) {
        return Err("invalid Inspect policy value".into());
    }
    Ok(bits)
}
fn object(o: &WorldObject) -> Value {
    json!({"id":o.id,"label":o.label,"kind":o.kind,"organism_id":o.organism_id,"position":o.position,"body_yaw":o.body_yaw,"head_yaw":o.head_yaw,"optical_opacity":o.optical_opacity,"radius":o.radius,"nutrition":o.nutrition,"hazard_pain":o.hazard_pain,"token_id":o.token_id,"social_affinity":o.social_affinity,"teacher_channel":o.teacher_channel,"consumed":o.consumed,"carried_by":o.carried_by,"activity_hold":o.activity_hold,"local_steering":o.local_steering,"grounded_physical":o.grounded_physical,"tracking_provenance":o.tracking_provenance,"tracking_key":o.tracking_key})
}
fn objects(host: &CpuCanonicalHost) -> Vec<Value> {
    host.world().object_snapshots().iter().map(object).collect()
}
fn alive(host: &CpuCanonicalHost, own: OrganismId) -> bool {
    host.world()
        .organism_registry()
        .get(own)
        .is_some_and(|r| r.lifecycle().is_alive())
}
fn coordinates(p: &BrainPhenotype) -> Result<Vec<usize>> {
    let result = p.synapses().iter().enumerate().filter(|(_,s)|
        (s.kind()==CompiledSynapseKind::Recurrent && ((s.source()==20 && s.target()==908)||(s.source()==908 && s.target()==1823))) ||
        matches!(s.kind(),CompiledSynapseKind::Decoder(c) if s.source()==1823 && c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Contact)
    ).map(|(i,_)|i).collect::<Vec<_>>();
    if result.len() != 26 {
        return Err("grip observation requires exact26 coordinates".into());
    }
    Ok(result)
}
fn observe(
    host: &CpuCanonicalHost,
    trainer: &CpuFoundationTrainer,
    own: OrganismId,
    row: &CpuCollectedDecision,
    indices: &[usize],
) -> Result<Value> {
    let replay = trainer.evaluate_replay(&TrainingSequence {
        phenotype_hash: trainer.phenotype().phenotype_hash(),
        initial: row.initial.clone(),
        ticks: vec![row.replay_tick.clone()],
        burn_in_ticks: 0,
        memory_candidate_gain: trainer
            .phenotype()
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    })?;
    let cp = host.checkpoint(trainer)?;
    let neural = &cp
        .residents
        .iter()
        .find(|r| r.neural.organism_id == own)
        .ok_or("missing resident")?
        .neural;
    let frame = row.patch.pre_action().perception();
    let coeffs=indices.iter().map(|&i| {let s=&trainer.phenotype().synapses()[i];json!({"index":i,"source":s.source(),"target":s.target(),"kind":s.kind(),"genetic":s.genetic_weight(),"alpha":s.alpha(),"pre_effective_offset":row.replay_tick.effective_weight_offsets[i],"policy_weight":trainer.weights()[i],"pre_effective":trainer.weights()[i]+row.replay_tick.effective_weight_offsets[i],"post_fast":neural.fast[i],"post_lifetime":neural.lifetime[i],"post_eligibility":neural.eligibility[i]})}).collect::<Vec<_>>();
    let calibrated_pair=[26144usize,26146].map(|i|{let s=&trainer.phenotype().synapses()[i];json!({"index":i,"genetic_weight":s.genetic_weight(),"policy_weight":trainer.weights()[i],"policy_bits":trainer.weights()[i].to_bits(),"pre_effective_offset":row.replay_tick.effective_weight_offsets[i],"pre_effective":trainer.weights()[i]+row.replay_tick.effective_weight_offsets[i],"post_fast":neural.fast[i],"post_lifetime":neural.lifetime[i],"post_eligibility":neural.eligibility[i]})});
    let sampled = sample_cpu_joint_action(
        trainer.phenotype(),
        frame,
        &replay.candidate_logits[0],
        cp.sampling_seed,
        row.sampling_counter.ok_or("missing sample counter")?,
        row.action.temperature,
    )?;
    if bytes(&sampled)? != bytes(&row.action)? {
        return Err("native full sampler parity at logged temperature".into());
    }
    let mut value = json!({"candidate_logits":replay.candidate_logits[0],"decoder_candidates":row.replay_tick.candidates,"coordinates":coeffs,"calibrated_distance_pair":calibrated_pair,"grip_receipt":frame.grip_proprioception(),"tactile2":frame.sensory().channels.tactile_contact[2],"encoded20":row.replay_tick.encoded_inputs[20],"initial_neurons":([20,908,1823].map(|i|row.initial.activations[i])),"final_neurons":([20,908,1823].map(|i|row.final_features[i])),"sampling_seed":cp.sampling_seed,"sampling_counter_after":cp.sampling_counter});
    if let Some(receipt) = frame.candidate_opportunity_coverage() {
        value["candidate_opportunity_coverage"] = serde_json::to_value(receipt)?;
    }
    Ok(value)
}
fn save_boundary(
    out: &Path,
    name: &str,
    host: &CpuCanonicalHost,
    trainer: &CpuFoundationTrainer,
    temperature: f32,
) -> Result<()> {
    let path = out.join(name);
    fs::create_dir(&path)?;
    validate_temperature(temperature)?;
    fs::write(
        path.join("EXECUTION-POLICY.json"),
        bytes(&json!({"uniform_temperature":temperature,"required_on_every_restart":true}))?,
    )?;
    let cp = host.checkpoint(trainer)?;
    write_cpu_private_checkpoint(&path, &cp)?;
    let restored = read_cpu_private_checkpoint(&path)?;
    let original = bytes(&cp)?;
    if bytes(&restored)? != original {
        return Err("written boundary readback differs".into());
    }
    let mut cold = CpuCanonicalHost::from_checkpoint(restored, trainer)?;
    if bytes(&cold.checkpoint(trainer)?)? != original {
        return Err("native cold boundary differs".into());
    }
    let mut warm = host.snapshot();
    let a = warm.step(trainer, temperature)?;
    for row in &a.decisions {
        if row.action.temperature.to_bits() != temperature.to_bits() {
            return Err("native temperature mismatch".into());
        }
    }
    let b = cold.step(trainer, temperature)?;
    if bytes(&a.decisions)? != bytes(&b.decisions)?
        || bytes(&warm.checkpoint(trainer)?)? != bytes(&cold.checkpoint(trainer)?)?
    {
        return Err("boundary matched next-step differs".into());
    }
    if bytes(&host.checkpoint(trainer)?)? != original {
        return Err("boundary audit changed live host".into());
    }
    fs::write(
        path.join("NATIVE-AUDIT.json"),
        bytes(
            &json!({"cold_byteexact":true,"matched_next_step":true,"live_host_unchanged":true,"audit_clones_only":true}),
        )?,
    )?;
    Ok(())
}

struct Trace {
    encoder: Option<zstd::stream::write::Encoder<'static, fs::File>>,
    hash: blake3::Hasher,
    length: u64,
    rows: usize,
}
impl Trace {
    fn new(path: &Path) -> Result<Self> {
        Ok(Self {
            encoder: Some(zstd::stream::write::Encoder::new(
                fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(path)?,
                3,
            )?),
            hash: blake3::Hasher::new(),
            length: 0,
            rows: 0,
        })
    }
    fn row(&mut self, value: &Value) -> Result<()> {
        let mut b = bytes(value)?;
        b.push(b'\n');
        self.encoder.as_mut().ok_or("closed trace")?.write_all(&b)?;
        self.encoder.as_mut().ok_or("closed trace")?.flush()?;
        self.hash.update(&b);
        self.length += b.len() as u64;
        self.rows += 1;
        Ok(())
    }
    fn finish(mut self, path: &Path) -> Result<Value> {
        use std::io::Read;
        let expected = self.hash.finalize();
        self.encoder
            .take()
            .ok_or("closed trace")?
            .finish()?
            .sync_all()?;
        let mut reader = zstd::stream::read::Decoder::new(fs::File::open(path)?)?;
        let mut hash = blake3::Hasher::new();
        let mut length = 0u64;
        let mut rows = 0;
        let mut buf = [0u8; 65536];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            length += n as u64;
            rows += buf[..n].iter().filter(|&&x| x == b'\n').count();
        }
        if hash.finalize() != expected || length != self.length || rows != self.rows {
            return Err("stream readback mismatch".into());
        }
        Ok(
            json!({"lossless":true,"blake3":expected.to_hex().to_string(),"decoded_bytes":length,"rows":rows,"compressed_bytes":fs::metadata(path)?.len()}),
        )
    }
}
impl Drop for Trace {
    fn drop(&mut self) {
        // Best-effort frame closure preserves already-written rows after a later
        // native/I/O error. Only explicit finish/readback certifies completeness.
        if let Some(encoder) = self.encoder.take() {
            if let Ok(file) = encoder.finish() {
                let _ = file.sync_all();
            }
        }
    }
}
fn capacity(host: &CpuCanonicalHost, own: OrganismId) -> Result<Value> {
    let r = host.world().organism_registry().get(own).ok_or("actor")?;
    let b = r.biochemistry();
    let p = r.phenotype();
    let chemical = b
        .graph_state()
        .locomotor_capacity(&p.chemistry.biochemical)?;
    let organ = b.body.organ(OrganKind::Locomotor);
    let reserve = (organ.energy / (0.5 + 0.5 * p.body.metabolic_efficiency)).clamp(0., 1.);
    Ok(
        json!({"chemical_locomotor_capacity":chemical,"locomotor_reserve":reserve,"locomotor_integrity":organ.integrity,"actual_locomotor_capacity":chemical.map(|v|v*reserve*organ.integrity),"biology":b,"lifecycle":r.lifecycle(),"sleep_phase":r.sleep_phase(),"sleep_phase_tick":r.sleep_phase_tick(),"sleep_work_units_cumulative":r.sleep_work_units(),"last_cognitive_work":r.cognitive_work(),"last_cognitive_energy_debit":r.cognitive_energy_debit()}),
    )
}
fn run_inner(
    host: &mut CpuCanonicalHost,
    t: &CpuFoundationTrainer,
    g: &CreatureGenome,
    out: &Path,
    intervals: usize,
    observation: bool,
    temperature: f32,
    recipe: Recipe,
    targets: &[WorldEntityId],
    stop_on_target: bool,
    praise: bool,
) -> Result<Value> {
    let own = identity(host, t, g, recipe)?;
    let founder = bytes(&t.checkpoint())?;
    let indices = coordinates(t.phenotype())?;
    // Parent entrypoint owns create-only output directory.

    let result = (|| -> Result<Value> {
        fs::write(out.join("FOUNDER-ADAM.json"), &founder)?;
        save_boundary(out, "ORIGIN", host, t, temperature)?;
        let origin = host.checkpoint(t)?;
        fs::write(
            out.join("EXECUTION-POLICY.json"),
            bytes(
                &json!({"uniform_temperature":temperature,"required_on_every_restart":true,"checkpoint_identity_does_not_choose_temperature":true,"genotype_weights_adam_private_world_unchanged":true,"temperature_unchanged_policy_readout_calibration_not_inherited_or_acquired_skill":true}),
            )?,
        )?;
        save_boundary(out, "START", host, t, temperature)?;
        if bytes(&host.checkpoint(t)?)? != bytes(&origin)? {
            return Err("startup altered saved individual".into());
        }
        let trace_path = out.join("steps.jsonl.zst");
        let mut trace = Trace::new(&trace_path)?;
        let mut completed = 0;
        let mut all_meals = Vec::new();
        let mut designated = Vec::new();
        let mut public_events = Vec::new();
        for local in 0..intervals {
            if !alive(host, own) {
                break;
            }
            if praise && [0, 2, 4, 6].contains(&local) {
                let record = host.world().organism_registry().get(own).ok_or("actor")?;
                let source = host
                    .world()
                    .entity(record.world_entity_id())
                    .ok_or("body")?
                    .position;
                let receipt = host.world_mut().queue_player_interaction_v1(
                    own,
                    source,
                    PlayerInteractionV1::Praise,
                )?;
                public_events.push(json!({"local":local,"receipt":receipt}));
            }
            let before_objects = objects(host);
            let physiology_before = capacity(host, own)?;
            let sleep_before = host
                .world()
                .organism_registry()
                .get(own)
                .ok_or("actor")?
                .sleep_work_units();
            let step = host.step(t, temperature)?;
            let record = host.world().organism_registry().get(own).ok_or("actor")?;
            let sleep_after = record.sleep_work_units();
            let sleep_delta = sleep_after
                .checked_sub(sleep_before)
                .ok_or("sleepworkdecreased")?;
            let decision_work = step
                .decisions
                .iter()
                .map(|r| r.patch.cognitive_work().map_or(0, |w| w.weighted_total))
                .sum::<u64>();
            let mixed = sleep_delta > 0 && !step.decisions.is_empty();
            let sleep_receipt = if sleep_delta > 0 && step.decisions.is_empty() {
                Some(record.cognitive_work().clone())
            } else {
                None
            };
            let work_observation = json!({"decision_work_sum":decision_work,"sleep_work_before":sleep_before,"sleep_work_after":sleep_after,"sleep_work_delta":sleep_delta,"new_sleep_charge_receipt":sleep_receipt,"unexpected_mixed_sleep_decision_charge":mixed,"last_receipt_not_a_cumulative_total":true});
            completed += 1;
            let mut decisions = Vec::new();
            for row in &step.decisions {
                if row.action.temperature.to_bits() != temperature.to_bits() {
                    return Err("decision not at declared temperature".into());
                }
                let frame = row.patch.pre_action().perception();
                if intervals == 600 && frame.candidates().iter().any(|c| c.target.entity.is_some())
                {
                    return Err("preparation observed target".into());
                }
                decisions.push(json!({"sequence":row.patch.pre_action().sequence_id,"sampling_counter":row.sampling_counter,"action":row.action,"candidates":frame.candidates(),"sensory":frame.sensory(),"grounded_object_slots":frame.grounded_object_slots(),"body":frame.body(),"homeostasis":frame.homeostasis(),"brain_work":row.brain_work,"cognitive_context":row.patch.pre_action().cognitive_context,"outcome":row.patch.outcome(),"reward":row.biological_reward,"work":row.patch.cognitive_work(),"patch_causal_digest":row.patch.causal_digest()?,"base_digest":frame.base_digest(),"frame_digest":frame.frame_digest(),"joint":row.joint_membership2177,"memory_plan":row.memory_plan2242,"executed_action_recall":row.executed_action_recall,"executed_action_recall_work":row.executed_action_recall_work,"approach_attention_feedback2325":row.approach_attention_feedback2325,"active_approach_opportunity2327":row.active_approach_opportunity2327,"observation":if observation{Some(observe(host,t,own,row,&indices)?)}else{None}}));
            }
            let mut consumed = Vec::new();
            for old in &before_objects {
                if old["consumed"] == false {
                    let id = WorldEntityId(old["id"].as_u64().ok_or("objectid")?);
                    if host
                        .world()
                        .entity(id)
                        .is_some_and(|o| o.consumed && o.nutrition > 0.)
                    {
                        let witnessed = step.decisions.iter().any(|r| {
                            r.patch.outcome().joint.as_ref().is_some_and(|j| {
                                j.channel_outcomes.iter().any(|ch| {
                                    ch.physical.contact == PhysicalContactKind::Consumed
                                        && ch.physical.target_entity == Some(id)
                                        && ch.execution_evidence.as_ref().is_some_and(|e| {
                                            e.succeeded && e.body_inputs.nutrition > 0.
                                        })
                                })
                            })
                        });
                        if !witnessed {
                            return Err("unwitnessed consumption".into());
                        }
                        consumed.push(id);
                        all_meals.push(json!({"id":id,"interval":completed}));
                    }
                }
            }
            let mut designated_this = Vec::new();
            for target in targets {
                for row in &step.decisions {
                    if row.patch.outcome().joint.as_ref().is_some_and(|j| {
                        j.channel_outcomes.iter().any(|c| {
                            c.physical.contact == PhysicalContactKind::Consumed
                                && c.physical.target_entity == Some(*target)
                                && c.execution_evidence.as_ref().is_some_and(|e| e.succeeded)
                        })
                    }) {
                        let event = json!({"id":target,"interval":completed,"nutrition":host.world().entity(*target).ok_or("target")?.nutrition});
                        designated_this.push(event.clone());
                        designated.push(event);
                    }
                }
            }
            trace.row(&json!({"interval":completed,"local":local,"tick_before":step.tick_before,"tick_after":step.tick_after,"sleeping":step.sleeping_organisms,"sleep_consolidations":step.sleep_consolidations,"work_observation":work_observation,"objects_before":before_objects,"objects_after":objects(host),"physiology_before":physiology_before,"physiology_after":capacity(host,own)?,"nutritious_consumption_transitions":consumed,"designated_consumptions":designated_this,"decisions":decisions}))?;
            if completed % if intervals > 128 { 128 } else { 32 } == 0 {
                save_boundary(
                    out,
                    &format!("INTERVAL-{completed:04}"),
                    host,
                    t,
                    temperature,
                )?;
            }
            if stop_on_target && !designated_this.is_empty() {
                break;
            }
        }
        let stream = trace.finish(&trace_path)?;
        fs::write(out.join("STREAM-AUDIT.json"), bytes(&stream)?)?;
        identity(host, t, g, recipe)?;
        if bytes(&t.checkpoint())? != founder {
            return Err("founderchanged".into());
        }
        save_boundary(out, "FINAL", host, t, temperature)?;
        let summary = json!({"schema":1,"phase":out.file_name().unwrap().to_string_lossy(),"designated_targets":targets,"designated_consumptions":designated,"public_events":public_events,"requested_intervals":intervals,"completed_intervals":completed,"alive":alive(host,own),"offers":[],"nutritious_consumption_transitions":all_meals,"final_physiology":capacity(host,own)?,"founder_adam_unchanged":true,"genotype_unchanged":true,"policy_bits":[t.weights()[26144].to_bits(),t.weights()[26146].to_bits()],"native_sleep_only":true,"counterfactual_age_policy_research":host.world().age_death_disabled(),"uniform_sampling_temperature":temperature,"experimental_active_approach_opportunity":recipe==Recipe::Candidate,"approach_nomination_retained":recipe==Recipe::Candidate,"stream":stream});
        fs::write(
            out.join("SUMMARY.json"),
            serde_json::to_vec_pretty(&summary)?,
        )?;
        Ok(summary)
    })();
    if let Err(e) = &result {
        let checkpoint_error = save_boundary(out, "ERROR-LAST-VALID", host, t, temperature)
            .err()
            .map(|e| e.to_string());
        fs::write(
            out.join("ERROR.json"),
            bytes(
                &json!({"error":e.to_string(),"checkpoint_error":checkpoint_error,"automatic_retry":false}),
            )?,
        )?;
    }
    result
}

fn compile(asset: &FoundationWeightAsset, genome: &CreatureGenome) -> Result<BrainPhenotype> {
    let e = genome.express()?;
    let dev = foundation_construction_development(
        &e.brain_genome,
        &BrainCapacityClass::n2048(),
        &e.development_state_at(Tick::ZERO)?,
    )?;
    Ok(
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            e.brain_genome,
            dev,
            asset.clone(),
        )?
        .0,
    )
}
fn sha(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
fn load(r:&Request)->Result<(CpuCanonicalHost,CpuFoundationTrainer,CreatureGenome,Value)> {
 validate_temperature(r.temperature)?;if !r.observe||r.horizon!=1024||r.fixture_id>1{return Err("requires1024/T1/observe/fixture0or1".into());}
 let a=FoundationWeightAsset::decode_canonical(&fs::read(&r.asset)?)?;let g:CreatureGenome=serde_json::from_slice(&fs::read(&r.genome)?)?;
 if g.n2048_foundation_candidate.as_ref().ok_or("asset")?.encode_canonical()?!=a.encode_canonical()?{return Err("pairmismatch".into());}
 let p=compile(&a,&g)?;let f:FoundationTrainerCheckpoint=serde_json::from_slice(&fs::read(&r.founder_checkpoint)?)?;let original=bytes(&f)?;let t=CpuFoundationTrainer::from_checkpoint(p,a,f)?;
 if bytes(&t.checkpoint())?!=original{return Err("founderrestore".into());}
 for(i,(w,s))in t.weights().iter().zip(t.phenotype().synapses()).enumerate(){let b=if i==26144{0x80000000}else if i==26146{0}else if(30048..30088).contains(&i){0x3ecccccd}else{s.genetic_weight().to_bits()};if w.to_bits()!=b{return Err(format!("commonbaseweight{i}").into());}}
 let cp=read_cpu_private_checkpoint(&r.actor_checkpoint)?;let raw=bytes(&cp)?;let h=CpuCanonicalHost::from_checkpoint(cp,&t)?;let own=identity(&h,&t,&g,Recipe::Candidate)?;
 if h.world().tick().raw()!=626||h.checkpoint(&t)?.sampling_counter!=626||!alive(&h,own)||bytes(&h.checkpoint(&t)?)?!=raw{return Err("requireshealthyacquired626cold".into());}
 let rec=h.world().organism_registry().get(own).ok_or("actor")?;let b=rec.biochemistry();
 if b.body.health<0.99||b.body.injury>0.001{return Err("unhealthymazeparent".into());}
 let pins=json!({"asset":{"path":r.asset,"sha256":sha(&r.asset)?},"genome":{"path":r.genome,"sha256":sha(&r.genome)?},"actor":{"path":r.actor_checkpoint.join("collector-checkpoint.json.zst"),"sha256":sha(&r.actor_checkpoint.join("collector-checkpoint.json.zst"))?},"founder":{"path":r.founder_checkpoint,"sha256":sha(&r.founder_checkpoint)?}});Ok((h,t,g,pins))
}
fn policy(original:&CpuCanonicalHost,base:&CpuFoundationTrainer,g:&CreatureGenome,edits:&[(u32,u32)])->Result<(CpuCanonicalHost,CpuFoundationTrainer,Value)>{
 let before=base.checkpoint();let source=original.checkpoint(base)?;let mut used=std::collections::BTreeSet::new();let mut e=Vec::new();
 for &(i,bits)in edits{let w=f32::from_bits(bits);let s=base.phenotype().synapses().get(i as usize).ok_or("index")?;if !used.insert(i)||!w.is_finite()||!matches!(s.kind(),CompiledSynapseKind::Decoder(c)if c.head()==DecoderHeadKind::ActionCandidate&&matches!(c.family(),CandidateActionFamily::Approach|CandidateActionFamily::Avoid)){return Err("editsmustuniqueApproachAvoidActionCandidate".into());}if base.weights()[i as usize].to_bits()!=bits{e.push((i,w));}}
 let next=if e.is_empty(){CpuFoundationTrainer::from_checkpoint(base.phenotype().clone(),g.n2048_foundation_candidate.clone().ok_or("asset")?,before.clone())?}else{base.calibrated_readout_copy(&e)?};let mut expected=before.clone();for&(i,w)in&e{expected.weights[i as usize]=w;}if bytes(&next.checkpoint())?!=bytes(&expected)?{return Err("otherfounderAdamchange".into());}
 let mut h=original.snapshot();if !e.is_empty(){h.rebase_readout_policy(base,&next)?;}let after=h.checkpoint(&next)?;let mut expectedcp=source.clone();expectedcp.founder_weight_digest=after.founder_weight_digest;if bytes(&after)?!=bytes(&expectedcp)?{return Err("otherprivateworldchange".into());}
 let cold=CpuCanonicalHost::from_checkpoint(after.clone(),&next)?;if bytes(&cold.checkpoint(&next)?)?!=bytes(&after)?{return Err("policycold".into());}
 Ok((h,next,json!({"changed_coordinates":e.len(),"requested_edits":edits,"only_founder_weights_and_host_digest":true,"all_private_genotype_Adam_world_preserved":true,"cold_exact":true,"operator_policy_not_acquired_skill":true})))
}
fn fixture(h:&mut CpuCanonicalHost,t:&CpuFoundationTrainer,g:&CreatureGenome,id:u8)->Result<(WorldEntityId,Value)>{
 let own=identity(h,t,g,Recipe::Candidate)?;let before=h.checkpoint(t)?;let old=objects(h);let actor=h.world().entity(h.world().organism_registry().get(own).ok_or("actor")?.world_entity_id()).ok_or("body")?;let pos=actor.position;let yaw=actor.body_yaw;let sign=if id==0{1.}else{-1.};let mut events=Vec::new();let mut walls=Vec::new();let specs=[(1.2,-1.65),(1.2,-1.10),(1.2,-0.55),(1.2,0.),(1.2,0.55),(1.75,-1.65),(2.30,-1.65)];
 for(i,(forward,lateral))in specs.into_iter().enumerate(){let l=sign*lateral;let p=Vec3f::new(pos.x+forward*yaw.cos()-l*yaw.sin(),pos.y,pos.z+forward*yaw.sin()+l*yaw.cos());let o=h.world_mut().editor_spawn_object(WorldEditorSpawnSpec{label:format!("2341-corner-wall-{i}"),kind:WorldObjectKind::Obstacle,organism_id:None,position:p,nutrition:0.,hazard_pain:0.,radius:0.35,token_id:None})?;walls.push(o);events.push(object(h.world().entity(o).ok_or("wall")?));}
 let p=Vec3f::new(pos.x+2.6*yaw.cos(),pos.y,pos.z+2.6*yaw.sin());let goal=h.world_mut().editor_spawn_object(WorldEditorSpawnSpec{label:"2341-corner-goal".into(),kind:WorldObjectKind::Food,organism_id:None,position:p,nutrition:0.6,hazard_pain:0.,radius:0.3,token_id:None})?;let mut props=h.world().entity(goal).ok_or("goal")?.grounded_physical;props.chemical=[0.;3];h.world_mut().set_grounded_physical_properties(goal,props)?;events.push(object(h.world().entity(goal).ok_or("goal")?));
 for o in&old{let n=h.world().entity(WorldEntityId(o["id"].as_u64().ok_or("id")?)).ok_or("oldobject")?;if bytes(o)?!=bytes(&object(n))?{return Err("fixturechangedoldobject".into());}}
 let after=h.checkpoint(t)?;let mut a=serde_json::to_value(&before)?;let mut b=serde_json::to_value(&after)?;
 // Exact old organism/private/sampling and all nonpublic world fields remain byteequal.
 for key in ["next_entity_id","next_spawn_sequence"]{if b["world"]["world"][key].as_u64()!=a["world"]["world"][key].as_u64().map(|n|n+8){return Err("fixtureallocatornot+8".into());}}
 let refs=b["world"]["world"]["voxel_backend"]["world_resource_refs"].as_array().ok_or("refs")?;let fresh=refs.iter().filter(|x|x["stable_id"].as_u64()==Some(goal.raw())).collect::<Vec<_>>();if fresh.len()!=1||fresh[0]["kind"]!="Food"{return Err("goalref".into());}let tile=VoxelTileCoord::new(p.x.round()as i32,p.z.round()as i32);let size=b["world"]["world"]["voxel_backend"]["profile_budget"]["chunk_tile_size"].as_u64().ok_or("size")?;let chunk=VoxelChunkCoord::for_tile(u16::try_from(size)?,tile);if fresh[0]["tile"]!=serde_json::to_value(tile)?||fresh[0]["chunk"]!=serde_json::to_value(chunk)?{return Err("derivedgoalref".into());}
 b["world"]["world"]["voxel_backend"]["world_resource_refs"]=Value::Array(refs.iter().filter(|x|x["stable_id"].as_u64()!=Some(goal.raw())).cloned().collect());
 let active=h.world().object_snapshots().iter().filter(|o|o.kind==WorldObjectKind::Food&&!o.consumed).count();
 if b["world"]["world"]["ecology"]["metrics"]["active_resources"].as_u64()!=Some(active as u64){return Err("resourcecount".into());}b["world"]["world"]["ecology"]["metrics"]["active_resources"]=a["world"]["world"]["ecology"]["metrics"]["active_resources"].clone();
 for k in ["objects","next_entity_id","next_spawn_sequence","last_touched_entities"]{a["world"]["world"].as_object_mut().ok_or("world")?.remove(k);b["world"]["world"].as_object_mut().ok_or("world")?.remove(k);}
 if bytes(&a)?!=bytes(&b)?{return Err("fixturechangedprivateorotherworld".into());}
 Ok((goal,json!({"fixture_id":id,"actor_origin":pos,"actor_yaw":yaw,"native_locomotion_body_radius":h.world().terrain().map_or(LocomotionLimits::default().body_radius,|t|t.limits().body_radius),"walls":walls,"goal":goal,"objects":events,"old_objects_and_private_RNG_unchanged":true,"public_zero_pain_solid_obstacles":true,"goal_visibility_not_assumed_hidden":true,"no_route_waypoint_or_action_forcing":true})))
}
fn execute(r:&Request)->Result<Value>{
 if r.outdir.exists(){return Err("createonlyoutdir".into());}let(original,base,g,inputs)=load(r)?;let originalcp=bytes(&original.checkpoint(&base)?)?;let(mut h,t,transition)=policy(&original,&base,&g,&r.edits)?;fs::create_dir(&r.outdir)?;save_boundary(&r.outdir,"SOURCE-ORIGIN",&original,&base,1.)?;save_boundary(&r.outdir,"POLICY-START",&h,&t,1.)?;fs::write(r.outdir.join("FOUNDER-BEFORE.json"),bytes(&base.checkpoint())?)?;fs::write(r.outdir.join("TRANSITION.json"),bytes(&transition)?)?;fs::write(r.outdir.join("INPUT-PINS.json"),bytes(&inputs)?)?;
 let(goal,events)=fixture(&mut h,&t,&g,r.fixture_id)?;fs::write(r.outdir.join("PUBLIC-EVENTS.json"),bytes(&events)?)?;let result=run_inner(&mut h,&t,&g,&r.outdir,r.horizon,true,1.,Recipe::Candidate,&[goal],false,false)?;
 if bytes(&original.checkpoint(&base)?)?!=originalcp{return Err("originalchanged".into());}for key in["asset","genome","actor","founder"]{if sha(Path::new(inputs[key]["path"].as_str().ok_or("path")?))?!=inputs[key]["sha256"].as_str().ok_or("sha")?{return Err("inputchanged".into());}}
 fs::write(r.outdir.join("COMPLETE.json"),bytes(&json!({"complete":true,"all_original_inputs_unchanged":true,"fixture_id":r.fixture_id,"edits":r.edits}))?)?;Ok(result)
}
fn main()->Result<()>{let a=std::env::args().collect::<Vec<_>>();if a.len()!=2{return Err("usage request.json".into());}let r:Request=serde_json::from_slice(&fs::read(&a[1])?)?;println!("{}",execute(&r)?);Ok(())}
#[cfg(test)]mod tests;
