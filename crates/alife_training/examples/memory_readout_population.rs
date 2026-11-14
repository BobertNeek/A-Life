//! LOCAL-only existing MemoryContext family-energy population assay.
//! Fresh genetic lives; diagnostics never enter acquired creatures.
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask, TrainingSequence};
use alife_world::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
#[path = "support/terminal_evidence2135.rs"]
mod evidence;
use evidence::LocalEvidence;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn bytes<T: Serialize>(x: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(x)?)
}
fn hash(x: &[u8]) -> String {
    format!("{:x}", Sha256::digest(x))
}
const POPULATION: [f32; 5] = [-16., -4., 0., 4., 16.];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
    seed: u64,
    gain: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prepared_readout_coefficient: Option<f32>,
    swap_nutrition: bool,
    reverse_exposure: bool,
    reverse_creation: bool,
    good_left: bool,
    angle: f32,
    distance: f32,
    appearance: u8,
    #[serde(default)]
    verify_cues: bool,
}
fn physiology(h: &CpuCanonicalHost, id: OrganismId) -> Value {
    let b = h
        .world()
        .organism_registry()
        .get(id)
        .unwrap()
        .biochemistry();
    json!({"tick":b.tick,"homeostasis":b.homeostasis,"body":b.body,"development":b.development})
}
fn config(asset: &FoundationWeightAsset) -> Result<CanonicalNewGameConfig> {
    let mut c = CanonicalNewGameConfig::phase3(2026111701, 1)?;
    c.brain_class = BrainScaleTier::Standard2048;
    c.sensor_profile = asset.manifest().sensor_profile();
    Ok(c)
}
// Load the exported construction genome directly. No new chromosome mutation,
// weight rebinding or reconstruction of a different founder is performed.
fn prepare_parent(
    asset: &FoundationWeightAsset,
    genome: &CreatureGenome,
) -> Result<(BrainPhenotype, CanonicalNewGame)> {
    let embedded = genome
        .n2048_foundation_candidate
        .as_ref()
        .ok_or("parent genome lacks foundation")?;
    if embedded.encode_canonical()? != asset.encode_canonical()? {
        return Err("genome/asset pair mismatch".into());
    }
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config(asset)?, asset)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let expressed = genome.express()?;
    // Verify that carrying the canonical newborn biology changes no biology.
    assert_eq!(
        bytes(record.biochemistry())?,
        bytes(&BiochemistryState::new_with_age(
            &expressed,
            record.birth_tick(),
            Tick::ZERO
        )?)?
    );
    let dev = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO)?,
    )?;
    let (compiled, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        dev,
        asset.clone(),
    )?;
    if !compiled
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits())
    {
        return Err("compiled parent weights changed".into());
    }
    game.world
        .replace_organism_record_exact(WorldOrganismRecord::new(
            record.organism_id(),
            record.world_entity_id(),
            genome.clone(),
            expressed,
            *record.biochemistry(),
            record.birth_tick(),
        )?)?;
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(genome)?).0;
    Ok((compiled, game))
}

fn basis(p: &BrainPhenotype) -> Vec<usize> {
    let lane = p
        .candidate_decoder()
        .memory_channel()
        .unwrap()
        .family_value_lane_start()
        + 3;
    p.synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c)
  if c.head()==DecoderHeadKind::MemoryContext&&c.family()==CandidateActionFamily::Ingest&&u32::from(c.input_lane())==lane).then_some(i)).collect()
}
// These same-input coordinates form one aggregate forward readout direction.
// They remain separate native coordinates for ordered f32 sums and plasticity.
fn candidate(
    base: &FoundationWeightAsset,
    genome: &CreatureGenome,
    gain: f32,
) -> Result<(
    FoundationWeightAsset,
    CreatureGenome,
    BrainPhenotype,
    CanonicalNewGame,
)> {
    let (p, _) = prepare_parent(base, genome)?;
    let ids = basis(&p);
    assert_eq!(ids.len(), 40);
    let mut weights = base.weights().to_vec();
    for &i in &ids {
        weights[i] += gain / ids.len() as f32;
    }
    let asset = if gain == 0. {
        base.clone()
    } else {
        FoundationWeightAsset::from_trained_weights(
            &p,
            weights.clone(),
            base.manifest().training_stage(),
        )?
    };
    let child = genome
        .clone()
        .with_n2048_foundation_candidate(asset.clone())?;
    let mut without = child.clone();
    without.n2048_foundation_candidate = genome.n2048_foundation_candidate.clone();
    assert_eq!(&without, genome);
    let (compiled, game) = prepare_parent(&asset, &child)?;
    for (i, (a, b)) in p.synapses().iter().zip(compiled.synapses()).enumerate() {
        assert_eq!(
            (a.source(), a.target(), a.route_index(), a.kind(), a.alpha()),
            (b.source(), b.target(), b.route_index(), b.kind(), b.alpha())
        );
        if !ids.contains(&i) {
            assert_eq!(a.genetic_weight().to_bits(), b.genetic_weight().to_bits());
        }
    }
    assert_eq!(
        compiled
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect::<Vec<_>>(),
        weights
    );
    Ok((asset, child, compiled, game))
}
fn setup(
    base: &FoundationWeightAsset,
    genome: &CreatureGenome,
    q: &Request,
) -> Result<(
    CpuCanonicalHost,
    CpuFoundationTrainer,
    OrganismId,
    FoundationWeightAsset,
    CreatureGenome,
)> {
    let (asset, child, p, mut game) = candidate(base, genome, q.gain)?;
    let trainer = CpuFoundationTrainer::new(
        p,
        asset.clone(),
        StageTrainableMask::recurrent_only(&prepare_parent(&asset, &child)?.0)?,
        AdamWConfig::default(),
    )?;
    let old = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .clone();
    let id = old.organism_id();
    let mut world = HeadlessScenarioBuilder::new(q.seed)
        .agent("subject", id, Vec3f::ZERO)
        .build()?;
    world.register_organism_record(WorldOrganismRecord::new(
        id,
        world.entity_id("subject").unwrap(),
        child.clone(),
        old.phenotype().clone(),
        *old.biochemistry(),
        old.birth_tick(),
    )?)?;
    game.world = world;
    // Fresh pre-decision biological preparation, not an acquired-state reset.
    let (h, _) = CpuCanonicalHost::new_prepared_cohort(game, &trainer, q.seed as u32, |w| {
        for _ in 0..600 {
            w.try_advance_tick()?;
        }
        Ok::<_, ScaffoldContractError>(())
    })?;
    Ok((h, trainer, id, asset, child))
}
// Explicit physical stimulus parameters in this local experiment.
#[allow(clippy::too_many_arguments)]
fn spawn(
    h: &mut CpuCanonicalHost,
    id: OrganismId,
    label: &str,
    appearance: u8,
    nutrition: f32,
    x: f32,
    z: f32,
    angle: f32,
    variant: u8,
) -> Result<WorldEntityId> {
    let body = h
        .world()
        .entity(
            h.world()
                .organism_registry()
                .get(id)
                .unwrap()
                .world_entity_id(),
        )
        .unwrap()
        .position;
    let (s, c) = angle.sin_cos();
    let target = h.world_mut().editor_spawn_object(WorldEditorSpawnSpec {
        label: label.into(),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: Vec3f::new(body.x + c * x - s * z, body.y, body.z + s * x + c * z),
        nutrition,
        hazard_pain: 0.,
        radius: 0.3,
        token_id: None,
    })?;
    let color = match (variant, appearance) {
        (0, 0) => [0.2, 0.7, 0.2],
        (0, _) => [0.8, 0.2, 0.7],
        (_, 0) => [0.2, 0.3, 0.9],
        (_, _) => [0.8, 0.7, 0.15],
    };
    h.world_mut().set_grounded_physical_properties(
        target,
        GroundedPhysicalProperties {
            velocity: Vec3f::ZERO,
            color,
            material: [0.4; 3],
            shape: [0.6, 0.4, 0.5],
            chemical: [0.8, 0., 0.25],
            surface_temperature: 0.,
            terrain: [0.5; 2],
        },
    )?;
    Ok(target)
}
fn recall(
    row: &CpuCollectedDecision,
    pre: &CpuCanonicalCheckpoint,
) -> Result<(EpisodicRetrievalContextV1, MemoryRecallReceipt)> {
    let f = row.patch.pre_action().perception();
    let draft = PerceptionFrameDraft::new(
        f.organism_id(),
        f.tick(),
        f.sensor_profile(),
        f.sensory().clone(),
        f.body(),
        *f.homeostasis(),
        f.candidates().to_vec(),
        f.profile_provenance(),
        f.grounded_object_slots().to_vec(),
    )?;
    assert_eq!(draft.base_digest(), f.base_digest());
    let r = &pre.residents[0];
    let bank = MemorySidecarState::restore_portable(
        r.memory.profile,
        r.memory_compaction,
        r.memory.clone(),
        r.staged_memory.clone(),
    )?;
    let found = bank.recall_frame(&draft)?;
    for (c, m) in row
        .replay_tick
        .candidates
        .iter()
        .zip(&found.context().candidates)
    {
        for j in 0..8 {
            assert_eq!(
                c.decoder_inputs[24 + j],
                m.target_latent[j] * m.target_confidence.raw()
            );
        }
        for j in 0..4 {
            assert_eq!(
                c.decoder_inputs[32 + j],
                m.family_value[j] * m.family_confidence.raw()
            );
        }
    }
    Ok((found.context().clone(), found.receipt().clone()))
}
fn run(
    base: &FoundationWeightAsset,
    genome: &CreatureGenome,
    q: &Request,
    e: &mut LocalEvidence,
) -> Result<Value> {
    if !POPULATION.contains(&q.gain) || q.distance <= 0. || q.distance > 4. || q.appearance > 1 {
        return Err("outside frozen bounded population/fixture".into());
    }
    if q.prepared_readout_coefficient
        .is_some_and(|w| ![-4., 0., 4.].contains(&w) || q.gain != 0.)
    {
        return Err(
            "prepared readout requires unchanged parent and declared signed weights".into(),
        );
    }
    if q.verify_cues {
        let mut evidence = Vec::new();
        for nutrition in [0.0_f32, 0.6_f32] {
            let (mut diagnostic, founder, own, _, diagnostic_genome) = setup(base, genome, q)?;
            if let Some(coefficient) = q.prepared_readout_coefficient {
                diagnostic.enable_frozen_memory_readout(
                    FrozenMemoryReadoutConstructionV1 {
                        schema_version: 1,
                        readout_abi_version: 1,
                        work_policy_version: 1,
                        parent_genome: diagnostic_genome,
                        source_family: CandidateActionFamily::Ingest,
                        destination_family: CandidateActionFamily::Approach,
                        coefficient,
                    },
                    &founder,
                )?;
            }
            let target = spawn(
                &mut diagnostic,
                own,
                "nutrition-current-cue-counterfactual",
                0,
                nutrition,
                1.5,
                0.5,
                q.angle,
                q.appearance,
            )?;
            let step = diagnostic.step(&founder, 1.0)?;
            assert!(!diagnostic.world().entity(target).unwrap().consumed);
            assert_eq!(step.decisions.len(), 1);
            e.save(
                &format!("NUTRITION-CUE-{nutrition}.rows.json"),
                &bytes(&step.decisions)?,
            )?;
            evidence.push(step.decisions[0].clone());
        }
        assert_eq!(evidence[0].initial, evidence[1].initial);
        assert_eq!(evidence[0].replay_tick, evidence[1].replay_tick);
        assert_eq!(evidence[0].action, evidence[1].action);
        assert_eq!(evidence[0].final_features, evidence[1].final_features);
        e.save("NUTRITION-CUE-CHECK.json", &bytes(&json!({"passed":true,
            "fresh_counterfactual_food_nutrition":[0.0,0.6],"actual_current_encoded_and_candidate_inputs_exact":true,
            "no_object_consumed":true,"not_population_outcomes":true}))?)?;
    }
    let (mut h, t, id, asset, child) = setup(base, genome, q)?;
    if let Some(coefficient) = q.prepared_readout_coefficient {
        let construction = FrozenMemoryReadoutConstructionV1 {
            schema_version: 1,
            readout_abi_version: 1,
            work_policy_version: 1,
            parent_genome: child.clone(),
            source_family: CandidateActionFamily::Ingest,
            destination_family: CandidateActionFamily::Approach,
            coefficient,
        };
        e.save("READOUT-CONSTRUCTION.json", &bytes(&construction)?)?;
        h.enable_frozen_memory_readout(construction, &t)?;
    }
    e.save("REQUEST.json", &bytes(q)?)?;
    e.save("CANDIDATE.alife-foundation", &asset.encode_canonical()?)?;
    e.save("GENOME.json", &bytes(&child)?)?;
    let adam = bytes(&t.checkpoint())?;
    e.save("FOUNDER-ADAM-BIRTH.json", &adam)?;
    let birth = bytes(&h.checkpoint(&t)?)?;
    e.save("PREPARED-BIRTH.cp.json", &birth)?;
    e.remember_valid(birth);
    let p = t.phenotype();
    let ids = basis(p);
    e.save("PROVENANCE.json",&bytes(&json!({"source_commit":option_env!("ALIFE_ASSAY_GIT_COMMIT"),"source_sha256":hash(include_bytes!("memory_readout_population.rs")),
  "binary_sha256":hash(&std::fs::read(std::env::current_exe()?)?),"asset_sha256":hash(&asset.encode_canonical()?),"genome_sha256":hash(&bytes(&child)?),"gain":q.gain,"population":POPULATION,"prepared_readout_coefficient":q.prepared_readout_coefficient,
  "basis":if q.prepared_readout_coefficient.is_some(){"CPU research prepared Ingest energy to Approach; separate frozen config; no legacy coefficient edits"}else{"Ingest MemoryContext family_value_lane_start + 3; positive uniform L1-normalized coefficient"},"indices":if q.prepared_readout_coefficient.is_some(){Vec::<usize>::new()}else{ids.clone()},"cap":p.candidate_decoder().memory_channel().unwrap().max_candidate_gain(),"terminal":"Off","backend":"native-cpu"}))?)?;
    let graph = p
        .synapses()
        .iter()
        .map(|s| match s.kind() {
            CompiledSynapseKind::Recurrent => json!([
                s.source(),
                s.target(),
                s.route_index(),
                0,
                0,
                0,
                0,
                s.alpha()
            ]),
            CompiledSynapseKind::Decoder(c) => json!([
                s.source(),
                s.target(),
                s.route_index(),
                c.head().raw(),
                c.family().raw(),
                c.input_lane(),
                0,
                s.alpha()
            ]),
        })
        .collect::<Vec<_>>();
    e.save("GRAPH-LOCAL.json",&bytes(&json!({"weights":t.weights(),"edges":graph,"dynamics":p.neuron_dynamics(),"cadence":p.projections().iter().map(|r|r.update_cadence().raw()).collect::<Vec<_>>(),"biases":p.candidate_decoder().families().iter().map(|f|(f.family().raw(),f.bias())).collect::<Vec<_>>(),"microsteps":p.microstep_count()}))?)?;
    let mut twin = h.snapshot();
    let mut targets = Vec::new();
    let mut probe = Vec::new();
    let mut trace = Vec::new();
    let mut sources = Vec::new();
    let mut meals = Vec::new();
    let mut first_choice = None;
    let mut sleeping = 0;
    let initial = physiology(&h, id);
    for j in 0..150 {
        if [0, 24, 48, 72].contains(&j) {
            let k = j / 24;
            let app = ([0, 1, 1, 0][k] as u8) ^ u8::from(q.reverse_exposure);
            let nutrition = if (app == 0) ^ q.swap_nutrition {
                0.6
            } else {
                0.
            };
            let label = format!("exposure-{k}");
            let a = spawn(
                &mut h,
                id,
                &label,
                app,
                nutrition,
                1.5,
                0.5,
                q.angle,
                q.appearance,
            )?;
            assert_eq!(
                a,
                spawn(
                    &mut twin,
                    id,
                    &label,
                    app,
                    nutrition,
                    1.5,
                    0.5,
                    q.angle,
                    q.appearance
                )?
            );
            targets.push((a, app, nutrition));
        }
        if j == 102 {
            let good_app = u8::from(q.swap_nutrition);
            let order = if q.reverse_creation { [1, 0] } else { [0, 1] };
            for app in order {
                let good = app == good_app;
                let left = if good { q.good_left } else { !q.good_left };
                let z = if left {
                    -0.5 * q.distance
                } else {
                    0.5 * q.distance
                };
                let x = (q.distance * q.distance - z * z).sqrt();
                let nutrition = if good { 0.6 } else { 0. };
                let label = format!("probe-{app}");
                let a = spawn(
                    &mut h,
                    id,
                    &label,
                    app,
                    nutrition,
                    x,
                    z,
                    q.angle,
                    q.appearance,
                )?;
                assert_eq!(
                    a,
                    spawn(
                        &mut twin,
                        id,
                        &label,
                        app,
                        nutrition,
                        x,
                        z,
                        q.angle,
                        q.appearance
                    )?
                );
                probe.push((a, app, nutrition));
            }
        }
        let pre = h.checkpoint(&t)?;
        let before = physiology(&h, id);
        let was = targets
            .iter()
            .chain(&probe)
            .map(|(a, _, _)| h.world().entity(*a).unwrap().consumed)
            .collect::<Vec<_>>();
        if q.prepared_readout_coefficient.is_some() || j % 24 == 0 || j == 102 || j == 149 {
            let b = bytes(&pre)?;
            e.save(&format!("STEP-{j:03}-PRE.cp.json"), &b)?;
            e.remember_valid(b);
        }
        let step = h.step(&t, 1.)?;
        e.save(&format!("STEP-{j:03}.rows.json"), &bytes(&step.decisions)?)?;
        let post = h.checkpoint(&t)?;
        if q.prepared_readout_coefficient.is_some() {
            let b = bytes(&post)?;
            e.save(&format!("STEP-{j:03}-POST.cp.json"), &b)?;
            e.remember_valid(b);
        }
        twin.step(&t, 1.)?;
        sleeping += step.sleeping_organisms;
        for (k, (a, app, nutrition)) in targets.iter().chain(&probe).enumerate() {
            if !was[k] && h.world().entity(*a).unwrap().consumed {
                let event = json!({"tick":step.tick_after,"target":a,"appearance":app,"nutrition":nutrition,"probe":k>=targets.len()});
                if k >= targets.len() && first_choice.is_none() {
                    first_choice = Some(event.clone());
                }
                meals.push(event);
            }
        }
        for (row_idx, row) in step.decisions.iter().enumerate() {
            if j >= 102 || [5, 29, 53, 77].contains(&j) {
                let (context, receipt) = recall(row, &pre)?;
                e.save(
                    &format!("STEP-{j:03}-{row_idx}-RECALL.json"),
                    &bytes(&json!({"context":context,"receipt":receipt}))?,
                )?;
                let mut candidates = Vec::new();
                for (i, (c, m)) in row
                    .replay_tick
                    .candidates
                    .iter()
                    .zip(&context.candidates)
                    .enumerate()
                {
                    let native_candidate = &row.patch.pre_action().perception().candidates()[i];
                    let memory = p
                        .synapses()
                        .iter()
                        .enumerate()
                        .filter_map(|(k, s)| match s.kind() {
                            CompiledSynapseKind::Decoder(d)
                                if d.head() == DecoderHeadKind::MemoryContext
                                    && d.family() == c.family =>
                            {
                                Some(
                                    c.decoder_inputs[d.input_lane() as usize]
                                        * (t.weights()[k]
                                            + row.replay_tick.effective_weight_offsets[k]),
                                )
                            }
                            _ => None,
                        })
                        .sum::<f32>();
                    let binding = c.memory_readout.as_ref().map_or(0., |b| b.contribution());
                    let total_memory = memory + binding;
                    candidates.push(json!({"ordinary_memory_raw":memory,"prepared_binding_contribution":binding,"prepared_memory_total":total_memory,"prepared_memory_capped":total_memory.clamp(-0.5,0.5),"index":i,"family":c.family,"target":native_candidate.target,"hunger_delta":m.target_latent[0],"confidence":m.target_confidence.raw(),"weighted_hunger":c.decoder_inputs[24],"source_count":m.target_source_count,"best_source":m.best_target_source,"family_source_count":m.family_source_count,"best_family_source":m.best_family_source,"family_value":m.family_value,"family_confidence":m.family_confidence.raw(),"memory_raw":memory,"memory_capped":memory.clamp(-0.5,0.5)}));
                }
                let seq = TrainingSequence {
                    phenotype_hash: p.phenotype_hash(),
                    initial: row.initial.clone(),
                    ticks: vec![row.replay_tick.clone()],
                    burn_in_ticks: 0,
                    memory_candidate_gain: 0.5,
                };
                let native = t.evaluate_replay(&seq)?;
                assert_eq!(native.final_activations[0], row.final_features);
                assert_eq!(
                    sample_cpu_joint_action(
                        p,
                        row.patch.pre_action().perception(),
                        &native.candidate_logits[0],
                        q.seed as u32,
                        row.sampling_counter.unwrap(),
                        1.
                    )?,
                    row.action
                );
                if [102, 108, 110].contains(&j) && q.prepared_readout_coefficient.is_some() {
                    let mut zeroed = seq.clone();
                    for candidate in &mut zeroed.ticks[0].candidates {
                        if let Some(term) = &mut candidate.memory_readout {
                            term.coefficient = 0.;
                        }
                    }
                    let ablated = t.evaluate_replay(&zeroed)?;
                    let selected = sample_cpu_joint_action(
                        p,
                        row.patch.pre_action().perception(),
                        &ablated.candidate_logits[0],
                        q.seed as u32,
                        row.sampling_counter.unwrap(),
                        1.,
                    )?;
                    e.save(&format!("STEP-{j:03}-{row_idx}-PREPARED-READOUT-COPY.json"),&bytes(&json!({"native":seq,"zeroed_frozen_coefficient":zeroed,"native_logits":native.candidate_logits[0],"zeroed_logits":ablated.candidate_logits[0],"zeroed_action":selected,"trajectory_intervention":false}))?)?;
                }
                if [102, 108, 110].contains(&j) && q.prepared_readout_coefficient.is_none() {
                    let mut trials = Vec::new();
                    for gain in POPULATION {
                        let mut changed = seq.clone();
                        for &index in &ids {
                            changed.ticks[0].effective_weight_offsets[index] +=
                                (gain - q.gain) / ids.len() as f32;
                        }
                        let evaluated = t.evaluate_replay(&changed)?;
                        let selected = sample_cpu_joint_action(
                            p,
                            row.patch.pre_action().perception(),
                            &evaluated.candidate_logits[0],
                            q.seed as u32,
                            row.sampling_counter.unwrap(),
                            1.0,
                        )?;
                        let mut no_source = changed.clone();
                        for c in &mut no_source.ticks[0].candidates {
                            if c.family == CandidateActionFamily::Ingest {
                                c.decoder_inputs[35] = 0.;
                            }
                        }
                        let masked = t.evaluate_replay(&no_source)?;
                        trials.push(json!({"gain":gain,"sequence":changed,"logits":evaluated.candidate_logits[0],
                            "source_masked_sequence":no_source,"source_masked_logits":masked.candidate_logits[0],"resampled_action":selected}));
                    }
                    e.save(&format!("STEP-{j:03}-{row_idx}-COEFFICIENT-COPY.json"),&bytes(&json!({"actual_gain":q.gain,"trials":trials,"trajectory_intervention":false}))?)?;
                }
                e.save(&format!("STEP-{j:03}-{row_idx}-TAPE.json"),&bytes(&json!({"sequence":seq,"native_logits":native.candidate_logits[0],"native_final":native.final_activations[0]}))?)?;
                sources.push(json!({"interval":j,"tick":step.tick_before,"candidates":candidates}));
            }
        }
        if q.prepared_readout_coefficient.is_some()
            || j % 24 == 0
            || j == 102
            || j == 149
            || was
                .iter()
                .zip(targets.iter().chain(&probe))
                .any(|(was, (a, _, _))| !was && h.world().entity(*a).unwrap().consumed)
        {
            let b = bytes(&post)?;
            assert_eq!(b, bytes(&twin.checkpoint(&t)?)?);
            if q.prepared_readout_coefficient.is_none() {
                e.save(&format!("STEP-{j:03}-POST.cp.json"), &b)?;
                e.remember_valid(b);
            }
        }
        trace.push(json!({"interval":j,"tick":step.tick_before,"before":before,"after":physiology(&h,id),"actions":step.decisions.iter().map(|r|&r.action).collect::<Vec<_>>(),"rewards":step.decisions.iter().map(|r|r.biological_reward).collect::<Vec<_>>() }));
    }
    let final_cp = bytes(&h.checkpoint(&t)?)?;
    e.save("FINAL.cp.json", &final_cp)?;
    e.remember_valid(final_cp.clone());
    assert_eq!(
        final_cp,
        bytes(
            &CpuCanonicalHost::from_checkpoint(serde_json::from_slice(&final_cp)?, &t)?
                .checkpoint(&t)?
        )?
    );
    assert_eq!(adam, bytes(&t.checkpoint())?);
    e.save("FOUNDER-ADAM-FINAL.json", &adam)?;
    Ok(
        json!({"request":q,"initial":initial,"final":physiology(&h,id),"meals":meals,"first_probe_consumption":first_choice,"sleeping_steps":sleeping,
 "exposure_consumed":targets.iter().filter(|(a,_,_)|h.world().entity(*a).unwrap().consumed).count(),"probe_consumed":probe.iter().filter(|(a,_,_)|h.world().entity(*a).unwrap().consumed).count(),
 "sources":sources,"trace":trace,"observer_full_cp_selected_boundaries":true,"cold_exact":true,"founder_adam_unchanged":true}),
    )
}
fn main() -> Result<()> {
    use std::io::{BufRead, Write};
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: memory_readout_population LOCAL_ROOT < requests.jsonl".into());
    }
    let parent="docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation";
    let ab = std::fs::read(parent)?;
    let gb = std::fs::read(format!("{parent}.genome.json"))?;
    assert_eq!(
        hash(&ab),
        "d83a995e802e41b8e17799b0871f75d9ede35f95850049777323112e636654d1"
    );
    assert_eq!(
        hash(&gb),
        "557da394c045d0b4353009c842825677aa7505f636d6252b20f67bfe0c3a5189"
    );
    let base = FoundationWeightAsset::decode_canonical(&ab)?;
    let genome: CreatureGenome = serde_json::from_slice(&gb)?;
    for line in std::io::stdin().lock().lines() {
        let q: Request = serde_json::from_str(&line?)?;
        let mut e = LocalEvidence::begin(Path::new(&args[1]), &q.id)?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&base, &genome, &q, &mut e)
        }));
        match result.unwrap_or_else(|_| {
            Err("assertion/panic; retained local state; no automatic retry".into())
        }) {
            Ok(s) => {
                e.save("SUMMARY.json", &serde_json::to_vec_pretty(&s)?)?;
                println!(
                    "{}",
                    json!({"completed":true,"id":q.id,"evidence":e.directory(),"first_probe_consumption":s["first_probe_consumption"],"exposure_consumed":s["exposure_consumed"]})
                );
            }
            Err(err) => {
                let retained = e.retain_error_checkpoint();
                e.save("ERROR.json",&bytes(&json!({"error":err.to_string(),"retained":format!("{retained:?}"),"automatic_retry":false}))?)?;
                println!("{}", json!({"error":err.to_string(),"id":q.id}));
            }
        }
        std::io::stdout().flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parent() -> (FoundationWeightAsset, CreatureGenome) {
        let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation");
        (
            FoundationWeightAsset::decode_canonical(&std::fs::read(&root).unwrap()).unwrap(),
            serde_json::from_slice(
                &std::fs::read(format!("{}.genome.json", root.display())).unwrap(),
            )
            .unwrap(),
        )
    }
    #[test]
    fn genetic_candidate_roundtrip_preserves_unrelated_weights_and_genome() {
        let (base, genome) = parent();
        let (p, _) = prepare_parent(&base, &genome).unwrap();
        let selected = basis(&p);
        for gain in POPULATION {
            let (asset, child, _, _) = candidate(&base, &genome, gain).unwrap();
            let loaded =
                FoundationWeightAsset::decode_canonical(&asset.encode_canonical().unwrap())
                    .unwrap();
            let reloaded: CreatureGenome = serde_json::from_slice(&bytes(&child).unwrap()).unwrap();
            assert_eq!(
                reloaded
                    .n2048_foundation_candidate
                    .as_ref()
                    .unwrap()
                    .encode_canonical()
                    .unwrap(),
                loaded.encode_canonical().unwrap()
            );
            let (phenotype, _) = prepare_parent(&loaded, &reloaded).unwrap();
            let mut changed = 0;
            for (i, synapse) in phenotype.synapses().iter().enumerate() {
                if synapse.genetic_weight().to_bits() != base.weights()[i].to_bits() {
                    assert!(selected.contains(&i));
                    changed += 1;
                }
            }
            assert_eq!(changed, if gain == 0. { 0 } else { 40 });
            let mut restored = reloaded;
            restored.n2048_foundation_candidate = genome.n2048_foundation_candidate.clone();
            assert_eq!(restored, genome);
        }
    }
    #[test]
    fn decoder_only_basis_requires_source_and_retains_the_native_cap() {
        let (base, genome) = parent();
        let mut no_source_reference = None;
        for gain in [-16., 0., 16.] {
            let (asset, _, p, _) = candidate(&base, &genome, gain).unwrap();
            let mask = StageTrainableMask::recurrent_only(&p).unwrap();
            let n = p.neuron_count() as usize;
            let t = CpuFoundationTrainer::new(p, asset, mask, AdamWConfig::default()).unwrap();
            let mut q = TrainingSequence {
                phenotype_hash: t.phenotype().phenotype_hash(),
                initial: alife_training::TrainingInitialState {
                    activations: vec![0.; n],
                    activity_ema: vec![0.; n],
                    metabolic_load: vec![0.; n],
                    dendrites: Default::default(),
                },
                ticks: vec![alife_training::TrainingReplayTick {
                    encoded_inputs: vec![0.; n],
                    projection_gain: 1.,
                    local_threshold_shift: 0.,
                    microstep_count: 3,
                    enabled_routes: vec![true; t.phenotype().projections().len()],
                    effective_weight_offsets: vec![0.; t.weights().len()],
                    structural_synapses: vec![],
                    candidates: vec![alife_training::TrainingReplayCandidate {
                        memory_readout: None,
                        family: CandidateActionFamily::Ingest,
                        decoder_inputs: [0.; 54],
                        innate_bias: 0.,
                    }],
                }],
                burn_in_ticks: 0,
                memory_candidate_gain: 0.5,
            };
            let absent = t.evaluate_replay(&q).unwrap();
            if let Some(reference) = &no_source_reference {
                assert_eq!(&absent, reference);
            } else {
                no_source_reference = Some(absent.clone());
            }
            q.ticks[0].candidates[0].decoder_inputs[35] = 1.;
            let present = t.evaluate_replay(&q).unwrap();
            assert_eq!(present.final_activations, absent.final_activations);
            let difference = present.candidate_logits[0][0] - absent.candidate_logits[0][0];
            assert_eq!(
                difference,
                if gain < 0. {
                    -0.5
                } else if gain > 0. {
                    0.5
                } else {
                    0.
                }
            );
        }
    }
}
