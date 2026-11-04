//! Explicit public-founder experiment; not a gameplay policy or checkpoint migration.
//! Teacher labels only enter detached native BPTT. Every collected transition,
//! reward, motor bundle and speech emission comes from the canonical CPU host.
use super::*;
use alife_core::phenotype::ExperimentalLanguageInputV1;
use alife_core::*;
use alife_semantic::{BoundedSpeechTranslator, TranslationAssistance};
use alife_training::{
    cpu::*, FoundationTrainerCheckpoint, ImitationExample, ImitationTarget, ReplaySpeechTarget,
};
use alife_world::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, process::Command};
mod curriculum;
#[cfg(test)]
mod provider_probe;
#[cfg(test)]
mod speech_gate_probe;
pub use curriculum::{
    continue_action_gated_language, continue_balanced_grounded_language,
    continue_contrastive_grounded_language, continue_embodied_language_curriculum,
    continue_factorial_grounded_language, continue_focused_grounded_language,
    continue_language_curriculum, continue_linear_grounded_language, refine_language_payload,
    refine_language_shared_gates,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LanguagePilotConfig {
    pub seed: u64,
    pub epochs: usize,
    pub rounds: usize,
    pub decisions: usize,
    pub llamacpp_translation: bool,
    pub experimental_word_bridge: bool,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn identity(value: &impl Serialize) -> Result<String> {
    Ok(hash(&serde_json::to_vec(value)?))
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    Ok(())
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum Goal {
    EatFood,
    GetToy,
}
impl Goal {
    fn word(self) -> u16 {
        match self {
            Self::EatFood => 8,
            Self::GetToy => 9,
        }
    }
    fn noun(self) -> u16 {
        match self {
            Self::EatFood => 1,
            Self::GetToy => 2,
        }
    }
    fn index(self) -> usize {
        match self {
            Self::EatFood => 0,
            Self::GetToy => 1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Arm {
    Grounded,
    Shuffled,
    Untreated,
    NoSpeech,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
struct Scene {
    prepared_tick: u64,
    maturation: f32,
    food: WorldEntityId,
    toy: WorldEntityId,
    distractor: WorldEntityId,
}
impl Scene {
    fn target(self, goal: Goal) -> WorldEntityId {
        match goal {
            Goal::EatFood => self.food,
            Goal::GetToy => self.toy,
        }
    }
}

fn teacher_input(slm: bool) -> Result<Vec<SpeechTranslationReceipt>> {
    ["eat food", "get toy"]
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let request = SpeechTranslationRequest::try_new(
                UtteranceId::new(i as u64 + 1)?,
                None,
                SpeechTranslationInput::PlayerText { text: text.into() },
                basic_vocabulary_bindings(),
            )?;
            let receipt = if slm {
                alife_semantic::LlamaCppSpeechTranslator::new(Default::default())?
                    .translate(&request)?
            } else {
                BoundedSpeechTranslator::new(
                    "bounded-pilot-teacher-v1",
                    TranslationAssistance::Disabled,
                )?
                .translate(&request)?
            };
            // This pilot's fixed distinction cannot silently change when a translator
            // omits or reorders a word. Preserve the actual delivery receipt.
            let expected = if i == 0 { vec![8, 1] } else { vec![9, 2] };
            if receipt
                .literal_tokens
                .iter()
                .map(|t| t.raw())
                .collect::<Vec<_>>()
                != expected
                || receipt.assisted != slm
            {
                return Err(
                    "teacher translation did not deliver this pilot's bounded lesson".into(),
                );
            }
            Ok(receipt)
        })
        .collect()
}

fn game(seed: u64, founder: u64, asset: &FoundationWeightAsset) -> Result<CanonicalNewGame> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    config.founder_seed_base = founder;
    Ok(create_canonical_new_game_with_n2048_candidate(
        &config, asset,
    )?)
}

fn compile_profile(
    seed: u64,
    asset: &FoundationWeightAsset,
    bridge: bool,
) -> Result<(BrainPhenotype, PhenotypeCompilerInputs)> {
    compile_input_profile(
        seed,
        asset,
        bridge.then_some(ExperimentalLanguageInputV1::TwoTokenReadoutBroadcastV1),
    )
}
fn compile_input_profile(
    seed: u64,
    asset: &FoundationWeightAsset,
    profile: Option<ExperimentalLanguageInputV1>,
) -> Result<(BrainPhenotype, PhenotypeCompilerInputs)> {
    let initial = game(seed, seed, asset)?;
    let record = initial
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("no founder")?;
    let genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = alife_runtime::canonical_preparation::foundation_construction_development(
        &genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    if let Some(profile) = profile {
        Ok(
            PhenotypeCompiler::compile_experimental_n2048_language_profile(
                genome,
                development,
                asset.clone(),
                profile,
            )?,
        )
    } else {
        Ok(PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?)
    }
}

#[cfg(test)]
fn new_actor(seed: u64, asset: &FoundationWeightAsset) -> Result<CpuFoundationTrainer> {
    new_actor_profile(seed, asset, false)
}
fn new_actor_profile(
    seed: u64,
    asset: &FoundationWeightAsset,
    bridge: bool,
) -> Result<CpuFoundationTrainer> {
    let (p, _) = compile_profile(seed, asset, bridge)?;
    let ids = p
        .synapses()
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match s.kind() {
            CompiledSynapseKind::Recurrent => Some(i as u32),
            CompiledSynapseKind::Decoder(c)
                if matches!(
                    c.head(),
                    DecoderHeadKind::ActionCandidate | DecoderHeadKind::SpeechPayload
                ) =>
            {
                Some(i as u32)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mask = StageTrainableMask::from_synapse_indices(&p, &ids)?;
    Ok(CpuFoundationTrainer::new(
        p,
        asset.clone(),
        mask,
        AdamWConfig {
            learning_rate: 0.003,
            ..Default::default()
        },
    )?)
}

fn arrange(world: &mut HeadlessWorld, layout: usize, evaluation: bool) -> Result<Scene> {
    ground_foundation_training_world(world)?;
    world.configure_ecology(EcologyState::default())?;
    let (id, body) = world.organism_entity_ids()[0];
    // Ordinary biological preparation, never a maturation/chemistry override.
    // Newborn association/speech gates cannot test retained language.
    let mut ready = None;
    for _ in 0..=2400 {
        let record = world
            .organism_registry()
            .get(id)
            .ok_or("missing prepared organism")?;
        let development = record
            .phenotype()
            .development_state_at(record.age_at(world.tick())?)?;
        if development
            .enabled_lobes
            .contains(&LobeKind::MultimodalAssociation)
            && development
                .active_sensor_channels
                .contains(&SensorChannelKind::Hearing)
            && [MotorAffordanceKind::Interact, MotorAffordanceKind::Vocalize]
                .iter()
                .all(|m| development.active_motor_affordances.contains(m))
        {
            if record.biochemistry().body.health < 0.95
                || record.biochemistry().body.energy <= 0.2
                || record.biochemistry().body.sleeping
            {
                return Err("ordinary language preparation is not healthy and awake".into());
            }
            ready = Some(development.maturation.raw());
            break;
        }
        world.try_advance_tick()?;
    }
    let maturation =
        ready.ok_or("ordinary language preparation did not reach native developmental gates")?;
    for object in world.object_snapshots() {
        if object.id != body {
            world.editor_remove_object(object.id)?;
        }
    }
    let body = world.entity(body).ok_or("missing body")?.clone();
    let distance = if evaluation {
        if layout == 0 {
            0.85
        } else {
            2.4
        }
    } else {
        0.65 + 0.04 * layout as f32
    };
    let side = if layout.is_multiple_of(2) { 1. } else { -1. };
    let offset = if evaluation { -side } else { side };
    let position = |bearing: f32, distance: f32| {
        Vec3f::new(
            body.position.x + distance * (body.body_yaw + bearing).cos(),
            body.position.y,
            body.position.z + distance * (body.body_yaw + bearing).sin(),
        )
    };
    let food = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "pilot-food".into(),
        kind: WorldObjectKind::Food,
        organism_id: None,
        position: position(offset * 0.58, distance),
        nutrition: 1.,
        hazard_pain: 0.,
        radius: 0.12,
        token_id: None,
    })?;
    let toy = world.spawn_toy("pilot-toy", position(-offset * 0.58, distance + 0.05), true)?;
    let distractor = world.editor_spawn_object(WorldEditorSpawnSpec {
        label: "pilot-visible-distractor".into(),
        kind: WorldObjectKind::Obstacle,
        organism_id: None,
        position: position(0., 3.5),
        nutrition: 0.,
        hazard_pain: 0.,
        radius: 0.15,
        token_id: None,
    })?;
    Ok(Scene {
        prepared_tick: world.tick().raw(),
        maturation,
        food,
        toy,
        distractor,
    })
}

fn host(
    seed: u64,
    founder_seed: u64,
    asset: &FoundationWeightAsset,
    actor: &CpuFoundationTrainer,
    layout: usize,
    evaluation: bool,
) -> Result<(CpuCanonicalHost, Scene)> {
    Ok(CpuCanonicalHost::new_prepared_cohort(
        game(seed, founder_seed, asset)?,
        actor,
        seed as u32,
        |world| arrange(world, layout, evaluation),
    )?)
}

fn speech_position(world: &HeadlessWorld) -> Result<Vec3f> {
    let entity = world
        .entity(world.organism_entity_ids()[0].1)
        .ok_or("no learner body")?;
    // A lateral speaker does not stand between the eye and either target.
    Ok(Vec3f::new(
        entity.position.x,
        entity.position.y,
        entity.position.z + 3.0,
    ))
}
fn no_teacher_or_prior(row: &CpuCollectedDecision) -> Result<()> {
    let frame = row.patch.pre_action().perception();
    if frame
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .any(|h| h.source_kind == UtteranceSourceKind::Teacher)
        || frame.sensory().semantic_context.is_some()
    {
        return Err("evaluation retained teacher speech or semantic prior".into());
    }
    Ok(())
}

struct Episode {
    rows: Vec<CpuCollectedDecision>,
    labels: Vec<ImitationTarget>,
    speech: Vec<Option<ReplaySpeechTarget>>,
}

fn candidate_label(
    row: &CpuCollectedDecision,
    chosen: Option<&ActionCandidate>,
) -> Result<ImitationTarget> {
    let mut motors = [None; 6];
    if let Some(chosen) = chosen {
        let enabled = row
            .action
            .motor_masks
            .iter()
            .enumerate()
            .fold(0, |bits, (slot, mask)| {
                bits | (u32::from(*mask != 0) << slot)
            });
        let demonstration =
            assemble_grounded_teacher(row.patch.pre_action().perception(), enabled, chosen)?;
        for (slot, &index) in demonstration.motor_indices.iter().enumerate() {
            if index == u16::MAX {
                continue;
            }
            if row.action.forced_slots.get(index as usize) != Some(&Some(slot as u8)) {
                return Err(
                    "teacher target does not match native representative arbitration".into(),
                );
            }
            // A representative may force this channel even when its conditional
            // sampling mask is empty. The existing objective marginalizes it.
            motors[slot] = Some(1u32 << index);
        }
    }
    Ok(ImitationTarget {
        representative: None,
        motors,
    })
}

fn action_label(
    row: &CpuCollectedDecision,
    goal: Goal,
    scene: Scene,
    narrate: bool,
) -> Result<ImitationTarget> {
    let frame = row.patch.pre_action().perception();
    let chosen = if narrate {
        frame
            .candidates()
            .iter()
            .find(|c| c.kind == ActionKind::Vocalize)
    } else {
        vocabulary_reception_choice(
            frame,
            Some(u32::from(goal.word())),
            Some(scene.target(goal)),
            None,
        )
    };
    candidate_label(row, chosen)
}

fn speech_label(
    row: &CpuCollectedDecision,
    previous: Option<&CpuCollectedDecision>,
    goal: Goal,
    scene: Scene,
) -> Option<ReplaySpeechTarget> {
    // Existing vocabulary production gates: a verb requires an actual earlier
    // successful command/consequence; hearing the request earns no verb label.
    grounded_speech_label(
        row.patch.pre_action().perception(),
        Some(goal.word()),
        Some(goal.noun()),
        Some(scene.target(goal)),
        previous.map(|r| &r.patch),
        None,
        true,
    )
}

fn collect(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    config: LanguagePilotConfig,
    arm: Arm,
    round: usize,
    translations: &[SpeechTranslationReceipt],
    traces: &mut impl Write,
) -> Result<(Vec<Episode>, Value)> {
    let mut episodes = Vec::new();
    let (mut heard, mut positive, mut labels, mut decisions, mut rewards) = (0, 0, 0, 0, 0.0f64);
    for episode in 0..8 {
        let goal = if episode % 2 == 0 {
            Goal::EatFood
        } else {
            Goal::GetToy
        };
        let seed = config.seed + 100 + (round * 8 + episode) as u64;
        let (mut host, scene) = host(seed, config.seed, asset, actor, episode / 2, false)?;
        let mut narration = None;
        let mut data = Episode {
            rows: Vec::new(),
            labels: Vec::new(),
            speech: Vec::new(),
        };
        for tick in 0..config.decisions {
            if matches!(arm, Arm::Grounded | Arm::Shuffled) && tick % 8 == 0 {
                // Four counterbalanced blocks: half aligned, half crossed in
                // the shuffled control, for each physical lesson and layout.
                let index = goal.index()
                    ^ usize::from(
                        arm == Arm::Shuffled && (episode / 2 + tick / 8).is_multiple_of(2),
                    );
                let position = speech_position(host.world())?;
                let id = host.world().organism_entity_ids()[0].0;
                alife_school::LanguageNursery::speak_in_world(
                    host.world_mut(),
                    id,
                    position,
                    translations[index].literal_tokens.clone(),
                )?;
            }
            let step = host.step(actor, 1.0).map_err(|e| {
                format!("native collection round {round} episode {episode} tick {tick}: {e}")
            })?;
            for row in step.decisions {
                let frame = row.patch.pre_action().perception();
                heard += usize::from(
                    frame
                        .sensory()
                        .language_context
                        .heard_tokens
                        .iter()
                        .flatten()
                        .any(|h| h.source_kind == UtteranceSourceKind::Teacher),
                );
                let speech = speech_label(&row, narration.as_ref(), goal, scene);
                let narrate = speech.is_some_and(|s| s.token.is_some());
                let label = action_label(&row, goal, scene, narrate)?;
                if narrate {
                    narration = None;
                }
                let (consumed, grabbed) = consequences(&row, scene);
                if (goal.index() == 0 && consumed)
                    || (goal.index() == 1
                        && grabbed
                        && host
                            .world()
                            .entity(scene.toy)
                            .is_some_and(|o| o.carried_by == Some(row.organism_id)))
                {
                    narration = Some(row.clone());
                }
                positive += usize::from(speech.is_some_and(|s| s.token.is_some()));
                labels += usize::from(label.motors.iter().any(Option::is_some));
                decisions += 1;
                rewards += f64::from(row.biological_reward);
                writeln!(
                    traces,
                    "{}",
                    json!({"phase":"training","arm":arm,"round":round,"episode":episode,"goal":goal,"scene":scene,"row":row,"label":{"representative":label.representative,"motors":label.motors},"speech_target":speech})
                )?;
                data.labels.push(label);
                data.speech.push(speech);
                data.rows.push(row);
            }
        }
        if data.rows.is_empty() {
            return Err("lesson collected no native decisions".into());
        }
        episodes.push(data);
    }
    Ok((
        episodes,
        json!({"round":round,"native_decisions":decisions,"teacher_heard_frames":heard,"action_label_rows":labels,"positive_speech_label_rows":positive,"sum_native_biological_reward":rewards}),
    ))
}

fn sequence(
    actor: &CpuFoundationTrainer,
    episode: &Episode,
    end: usize,
    start: usize,
) -> TrainingSequence {
    TrainingSequence {
        phenotype_hash: actor.phenotype().phenotype_hash(),
        initial: episode.rows[0].initial.clone(),
        ticks: episode.rows[..end]
            .iter()
            .map(|r| r.replay_tick.clone())
            .collect(),
        burn_in_ticks: start,
        memory_candidate_gain: actor
            .phenotype()
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    }
}

fn gradients(actor: &CpuFoundationTrainer, episodes: &[Episode]) -> Result<(Vec<f32>, Value)> {
    gradients_with_speech(actor, episodes, true)
}
fn gradients_with_speech(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
    supervise_speech: bool,
) -> Result<(Vec<f32>, Value)> {
    let mut sum = vec![0.; actor.weights().len()];
    let mut loss = 0.0f64;
    let mut windows = 0;
    for episode in episodes {
        for start in (0..episode.rows.len()).step_by(16) {
            let end = (start + 16).min(episode.rows.len());
            let sequence = sequence(actor, episode, end, start);
            let replay = actor.evaluate_replay(&sequence)?;
            let mut adjoints = Vec::new();
            for (i, logits) in replay
                .candidate_logits
                .iter()
                .enumerate()
                .take(end)
                .skip(start)
            {
                let action = &episode.rows[i].action;
                if episode.labels[i].representative.is_none()
                    && episode.labels[i].motors.iter().all(Option::is_none)
                {
                    // Absence of an observed actionable referent is absence of
                    // supervision, not an invented idle label or dropped row.
                    adjoints.push(vec![0.; logits.len()]);
                    continue;
                }
                let example = ImitationExample {
                    candidate_count: action.candidate_count,
                    representative_mask: action.representative_mask,
                    motor_masks: action.motor_masks,
                    forced_slots: action.forced_slots.clone(),
                    target: episode.labels[i].clone(),
                };
                let objective = cpu_imitation_objective(logits, &example, 1., 1.)?;
                loss +=
                    f64::from(objective.loss) / episode.rows.len() as f64 / episodes.len() as f64;
                adjoints.push(
                    objective
                        .logit_gradients
                        .into_iter()
                        .map(|g| g / (end - start) as f32)
                        .collect(),
                );
            }
            let mut speech = vec![None; start];
            if supervise_speech {
                speech.extend_from_slice(&episode.speech[start..end]);
            } else {
                speech.resize(end, None);
            }
            let probe = actor.probe_replay_gradients_with_speech(&sequence, &adjoints, &speech)?;
            let scale = (end - start) as f32 / episode.rows.len() as f32 / episodes.len() as f32;
            for (dst, src) in sum.iter_mut().zip(probe.gradients) {
                *dst += src * scale;
            }
            windows += 1;
        }
    }
    Ok((sum, json!({"mean_action_loss":loss,"bptt_windows":windows})))
}

fn word_path_probe(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    seed: u64,
) -> Result<Value> {
    let (base, scene) = host(seed + 40_000, seed, asset, actor, 0, false)?;
    let before = identity(&actor.checkpoint())?;
    let mut probes = Vec::new();
    for tokens in [vec![8, 1], vec![9, 2]] {
        let mut host = base.snapshot();
        let id = host.world().organism_entity_ids()[0].0;
        let position = speech_position(host.world())?;
        alife_school::LanguageNursery::speak_in_world(
            host.world_mut(),
            id,
            position,
            tokens
                .into_iter()
                .map(LanguageTokenId::new)
                .collect::<std::result::Result<_, _>>()?,
        )?;
        let mut rows = Vec::new();
        for _ in 0..8 {
            rows.extend(host.step(actor, 1.)?.decisions);
        }
        let labels = rows
            .iter()
            .map(|row| action_label(row, Goal::EatFood, scene, false))
            .collect::<Result<Vec<_>>>()?;
        let encoded = rows[0].replay_tick.encoded_inputs.clone();
        let rows_hash = identity(&rows)?;
        let speech = vec![None; rows.len()];
        let episode = Episode {
            rows,
            labels,
            speech,
        };
        let replay = actor.evaluate_replay(&sequence(actor, &episode, episode.rows.len(), 0))?;
        let (gradient, _) = gradients(actor, &[episode])?;
        probes.push((encoded, gradient, replay.candidate_logits, rows_hash));
    }
    if identity(&actor.checkpoint())? != before {
        return Err("word probe changed the actor".into());
    }
    let encoded = probes[0]
        .0
        .iter()
        .zip(&probes[1].0)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    let mask = actor.checkpoint().stage_mask;
    let changed = probes[0]
        .1
        .iter()
        .zip(&probes[1].1)
        .enumerate()
        .filter(|(i, (a, b))| mask.is_trainable(*i) && a.to_bits() != b.to_bits())
        .count();
    let logits = probes[0]
        .2
        .iter()
        .flatten()
        .zip(probes[1].2.iter().flatten())
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    Ok(
        json!({"native_world_steps":16,"prepared_tick":scene.prepared_tick,"maturation":scene.maturation,"changed_encoded_lanes":encoded,"changed_candidate_logits":logits,"changed_trainable_gradients":changed,"action_objective_word_influence_observed":changed>0,"demonstrates_retained_language":false,"actual_row_sha256":[probes[0].3,probes[1].3]}),
    )
}

fn preserve_frozen(
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
) -> Result<()> {
    for i in 0..before.weights.len() {
        if !before.stage_mask.is_trainable(i)
            && (before.weights[i].to_bits() != after.weights[i].to_bits()
                || before.first_moment[i].to_bits() != after.first_moment[i].to_bits()
                || before.second_moment[i].to_bits() != after.second_moment[i].to_bits()
                || before.update_ages[i] != after.update_ages[i])
        {
            return Err("offline pilot changed a frozen coordinate".into());
        }
    }
    Ok(())
}

fn export(actor: &mut CpuFoundationTrainer, seed: u64) -> Result<FoundationWeightAsset> {
    let before = actor.checkpoint();
    let asset = actor.export_candidate(TrainingStageManifest::new(1, 1, 1))?;
    let (p, _) = compile_input_profile(
        seed,
        &asset,
        actor.phenotype().experimental_language_input(),
    )?;
    actor.rebind_for_next_cohort(p, asset.clone())?;
    let after = actor.checkpoint();
    if before.weights != after.weights
        || before.first_moment != after.first_moment
        || before.second_moment != after.second_moment
        || before.update_ages != after.update_ages
        || before.optimizer_step != after.optimizer_step
        || before.stage_mask != after.stage_mask
    {
        return Err("founder export reset native Adam state".into());
    }
    Ok(asset)
}

fn physical_controls(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    seed: u64,
) -> Result<Vec<Value>> {
    let mut controls = Vec::new();
    for goal in [Goal::EatFood, Goal::GetToy] {
        let (mut host, scene) = host(
            seed + 30_000 + goal.index() as u64,
            seed,
            asset,
            actor,
            0,
            false,
        )?;
        let (id, body) = host.world().organism_entity_ids()[0];
        let mut consumed = false;
        let mut energy_gain = 0.0;
        let mut outcomes = Vec::new();
        for _ in 0..if goal.index() == 0 { 2 } else { 1 } {
            let world = host.world_mut();
            let before = *world
                .organism_registry()
                .get(id)
                .ok_or("no control organism")?
                .biochemistry();
            let frame = world.perception_frame(
                id,
                world.tick(),
                actor.phenotype().sensor_profile(),
                before.homeostasis,
            )?;
            if ![scene.food, scene.toy, scene.distractor]
                .into_iter()
                .all(|t| visible_target(&frame, t))
            {
                return Err("positive control lacks visible food, toy or distractor".into());
            }
            let primitive = if goal.index() == 0 {
                HeadlessActionIds::EAT
            } else {
                HeadlessActionIds::GRAB
            };
            let candidate = frame
                .candidates()
                .iter()
                .find(|c| c.action_id == primitive && c.target.entity == Some(scene.target(goal)))
                .ok_or("physical control lacks native candidate")?;
            let command = candidate.to_command(id, Confidence::new(1.)?)?;
            let receipt =
                world.apply_registered_command(&command, body, Tick(world.tick().raw() + 1))?;
            consumed |=
                receipt.action_result.execution.physical.contact == PhysicalContactKind::Consumed;
            energy_gain += receipt.biology_after.body.energy - before.body.energy;
            outcomes.push(json!({"command":command,"physical":receipt.action_result.execution.physical,"energy_delta":receipt.biology_after.body.energy-before.body.energy}));
            world.try_advance_tick()?;
        }
        let acquired = host
            .world()
            .entity(scene.toy)
            .is_some_and(|o| o.carried_by == Some(id));
        if (goal.index() == 0 && (!consumed || energy_gain <= 0.))
            || (goal.index() == 1 && !acquired)
        {
            return Err("native physical positive control failed".into());
        }
        controls.push(json!({"goal":goal,"scripted_not_learned":true,"consumed":consumed,"energy_gain":energy_gain,"acquired_toy":acquired,"receipts":outcomes}));
    }
    Ok(controls)
}

fn visible_target(frame: &PerceptionFrame, target: WorldEntityId) -> bool {
    frame.candidates().iter().any(|c| {
        c.target.entity == Some(target)
            && matches!(c.observation, CandidateObservationRef::ObjectSlot(_))
    })
}

fn consequences(row: &CpuCollectedDecision, scene: Scene) -> (bool, bool) {
    let bundle = row.patch.selected_bundle();
    let command = |primitive, target| {
        bundle.is_some_and(|b| {
            b.channels.iter().any(|c| {
                c.primitive == primitive && c.target.and_then(|t| t.entity) == Some(target)
            })
        })
    };
    let consumed = command(HeadlessActionIds::EAT, scene.food)
        && row.patch.outcome().joint.as_ref().is_some_and(|j| {
            j.channel_outcomes.iter().any(|o| {
                o.physical.target_entity == Some(scene.food)
                    && o.physical.contact == PhysicalContactKind::Consumed
            })
        });
    let grabbed = command(HeadlessActionIds::GRAB, scene.toy)
        && row.patch.outcome().joint.as_ref().is_some_and(|j| {
            j.channel_outcomes.iter().any(|o| {
                o.physical.target_entity == Some(scene.toy)
                    && o.physical.contact == PhysicalContactKind::Touch
            })
        });
    (consumed, grabbed)
}

fn evaluate(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    config: LanguagePilotConfig,
    arm: Arm,
    traces: &mut impl Write,
) -> Result<Vec<Value>> {
    let mut results = Vec::new();
    for layout in 0..2 {
        for goal in [Goal::EatFood, Goal::GetToy] {
            for cue in ["player", "silent", "swapped"] {
                let seed = config.seed + 10_000 + (layout * 2 + goal.index()) as u64;
                let (mut host, scene) = host(seed, config.seed, asset, actor, layout, true)?;
                let id = host.world().organism_entity_ids()[0].0;
                let mut previous = None;
                let (
                    mut meals,
                    mut nourished,
                    mut acquisitions,
                    mut utterances,
                    mut correct,
                    mut opportunities,
                    mut heard,
                    mut failures,
                    mut decisions,
                    mut sleeping,
                    mut consolidations,
                ) = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
                let (mut selected_eat, mut selected_grab) = (0, 0);
                let mut visible = Value::Null;
                for tick in 0..config.decisions {
                    if cue != "silent" && tick % 8 == 0 {
                        let words = if cue == "swapped" {
                            if goal.index() == 0 {
                                vec![9, 2]
                            } else {
                                vec![8, 1]
                            }
                        } else {
                            vec![goal.word(), goal.noun()]
                        };
                        let position = speech_position(host.world())?;
                        host.world_mut().emit_player_tokens(
                            Some(id),
                            position,
                            words
                                .into_iter()
                                .map(LanguageTokenId::new)
                                .collect::<std::result::Result<_, _>>()?,
                        )?;
                    }
                    let owned_before = host.world().entity(scene.toy).and_then(|o| o.carried_by);
                    let step = host.step(actor, 1.)?;
                    sleeping += step.sleeping_organisms;
                    consolidations += step.sleep_consolidations;
                    if host.world().entity_id("nursery-teacher").is_some() {
                        return Err("teacher actor in assessment".into());
                    }
                    for row in step.decisions {
                        no_teacher_or_prior(&row)?;
                        let frame = row.patch.pre_action().perception();
                        if visible.is_null() {
                            visible = json!({"food":visible_target(frame,scene.food),"toy":visible_target(frame,scene.toy),"distractor":visible_target(frame,scene.distractor)});
                        }
                        heard += usize::from(
                            frame
                                .sensory()
                                .language_context
                                .heard_tokens
                                .iter()
                                .flatten()
                                .any(|h| h.source_kind == UtteranceSourceKind::Player),
                        );
                        if let Some(bundle) = row.patch.selected_bundle() {
                            selected_eat += bundle
                                .channels
                                .iter()
                                .filter(|c| {
                                    c.primitive == HeadlessActionIds::EAT
                                        && c.target.and_then(|t| t.entity) == Some(scene.food)
                                })
                                .count();
                            selected_grab += bundle
                                .channels
                                .iter()
                                .filter(|c| {
                                    c.primitive == HeadlessActionIds::GRAB
                                        && c.target.and_then(|t| t.entity) == Some(scene.toy)
                                })
                                .count();
                        }
                        let (meal, grab) = consequences(&row, scene);
                        meals += usize::from(meal);
                        nourished +=
                            usize::from(meal && row.patch.outcome().energy_delta.raw() > 0.);
                        acquisitions += usize::from(
                            grab && owned_before != Some(id)
                                && host
                                    .world()
                                    .entity(scene.toy)
                                    .is_some_and(|o| o.carried_by == Some(id)),
                        );
                        let label = speech_label(&row, previous.as_ref(), goal, scene)
                            .filter(|s| s.token.is_some());
                        opportunities += usize::from(label.is_some());
                        let emitted = host
                            .world()
                            .audible_utterances()
                            .into_iter()
                            .filter(|u| {
                                u.source_kind == UtteranceSourceKind::Creature
                                    && u.speaker_id == Some(id)
                                    && u.emitted_tick == frame.tick()
                            })
                            .collect::<Vec<_>>();
                        utterances += emitted.len();
                        for utterance in &emitted {
                            if let Some(label) = label {
                                let expected = std::iter::once(label.token.unwrap())
                                    .chain(label.continuation.into_iter().take_while(|t| *t != 0))
                                    .collect::<Vec<_>>();
                                correct += usize::from(
                                    utterance.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>()
                                        == expected,
                                );
                            }
                        }
                        failures += usize::from(!row.patch.outcome().success);
                        decisions += 1;
                        writeln!(
                            traces,
                            "{}",
                            json!({"phase":"evaluation","arm":arm,"layout":layout,"goal":goal,"cue":cue,"scene":scene,"row":row,"emitted":emitted,"grounded_speech_target":label})
                        )?;
                        if label.is_some() {
                            previous = None;
                        }
                        if (goal.index() == 0 && meal)
                            || (goal.index() == 1
                                && grab
                                && owned_before != Some(id)
                                && host
                                    .world()
                                    .entity(scene.toy)
                                    .is_some_and(|o| o.carried_by == Some(id)))
                        {
                            previous = Some(row);
                        }
                    }
                }
                results.push(json!({"arm":arm,"layout":if layout==0{"relocated_near"}else{"relocated_far"},"goal":goal,"cue":cue,"seed":seed,"prepared_tick":scene.prepared_tick,"maturation":scene.maturation,"native_decisions":decisions,"teacher_present":false,"prior_enabled":false,"initial_visible":visible,"player_heard_frames":heard,"selected_food_eat_commands":selected_eat,"selected_toy_grab_commands":selected_grab,"consumptions":meals,"nourished_meals":nourished,"toy_acquisitions":acquisitions,"correct_physical":if goal.index()==0{nourished}else{acquisitions},"utterances":utterances,"grounded_utterance_opportunities":opportunities,"correct_grounded_utterances":correct,"joint_failures":failures,"sleeping_ticks":sleeping,"sleep_consolidations":consolidations}));
            }
        }
    }
    Ok(results)
}

#[derive(Serialize, Deserialize)]
struct Recovery {
    kind: String,
    inputs: PhenotypeCompilerInputs,
    foundation: Vec<u8>,
    actor: FoundationTrainerCheckpoint,
    host: CpuCanonicalCheckpoint,
    next_gradient: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assessment: Option<RecoveredAssessment>,
}
#[derive(Serialize, Deserialize)]
struct RecoveredAssessment {
    config: LanguagePilotConfig,
    arm: Arm,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    curriculum: Option<curriculum::AssessmentPlan>,
}
struct HashedTrace<W> {
    inner: W,
    digest: Sha256,
}
impl<W: Write> Write for HashedTrace<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.digest.update(&bytes[..written]);
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
fn recovered_assessment(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    request: RecoveredAssessment,
    traces: impl Write,
) -> Result<Value> {
    let mut traces = HashedTrace {
        inner: traces,
        digest: Sha256::new(),
    };
    let evaluation = if let Some(plan) = request.curriculum {
        curriculum::evaluate(actor, asset, request.config, request.arm, plan, &mut traces)?
    } else {
        evaluate(actor, asset, request.config, request.arm, &mut traces)?
    };
    Ok(
        json!({"evaluation":evaluation,"native_trace_sha256":format!("{:x}",traces.digest.finalize())}),
    )
}
fn reload(recovery: Recovery, directory: Option<&Path>) -> Result<Value> {
    if recovery.kind != "disposable-retained-language-pilot-v1" {
        return Err("not an experimental pilot recovery".into());
    }
    let p = PhenotypeCompiler::compile_validated(&recovery.inputs, &BrainCapacityClass::n2048())?;
    let source = FoundationWeightAsset::decode_canonical(&recovery.foundation)?;
    let mut actor = CpuFoundationTrainer::from_checkpoint(p, source.clone(), recovery.actor)?;
    let mut host = CpuCanonicalHost::from_checkpoint(recovery.host, &actor)?;
    let before_host = identity(&host.checkpoint(&actor)?)?;
    let before_actor = identity(&actor.checkpoint())?;
    // The child restores the full actor before constructing assessment worlds.
    // No update, forced action, teacher actor or semantic prior enters assessment.
    let assessment = if let Some(request) = recovery.assessment {
        Some(if let Some(directory) = directory {
            let file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join("reloaded-evaluation.jsonl.zst"));
            match file {
                Ok(file) => {
                    let mut traces = zstd::stream::write::Encoder::new(file, 3)?;
                    let result = recovered_assessment(&actor, &source, request, &mut traces)?;
                    traces.finish()?;
                    result
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    // A repeat cold restore must preserve prior evidence. Verify
                    // its complete native trace against the regenerated one.
                    let result = recovered_assessment(&actor, &source, request, std::io::sink())?;
                    let mut saved = zstd::stream::read::Decoder::new(fs::File::open(
                        directory.join("reloaded-evaluation.jsonl.zst"),
                    )?)?;
                    let mut digest = HashedTrace {
                        inner: std::io::sink(),
                        digest: Sha256::new(),
                    };
                    std::io::copy(&mut saved, &mut digest)?;
                    if result["native_trace_sha256"] != format!("{:x}", digest.digest.finalize()) {
                        return Err("existing assessment trace differs from cold replay".into());
                    }
                    result
                }
                Err(error) => return Err(error.into()),
            }
        } else {
            recovered_assessment(&actor, &source, request, std::io::sink())?
        })
    } else {
        None
    };
    let mut decisions = Vec::new();
    for _ in 0..8 {
        decisions.extend(host.step(&actor, 1.)?.decisions);
    }
    let after_host = identity(&host.checkpoint(&actor)?)?;
    let rows = identity(&decisions)?;
    actor.apply_gradients(&recovery.next_gradient)?;
    let mut result = json!({"before_host":before_host,"before_actor":before_actor,"after_host":after_host,"decisions":rows,"after_native_adam_update":identity(&actor.checkpoint())?});
    if let Some(assessment) = assessment {
        result["assessment"] = assessment;
    }
    Ok(result)
}

/// Only the experimental runner uses this path; it is not a native joint-PPO
/// handoff and makes no critic/version continuity claim.
fn read_recovery(directory: &Path) -> Result<Recovery> {
    let bytes = fs::read(directory.join("experimental-recovery.json"))?;
    let expected = fs::read_to_string(directory.join("experimental-recovery.sha256"))?;
    if hash(&bytes) != expected {
        return Err("experimental recovery checksum mismatch".into());
    }
    let mut de = serde_json::Deserializer::from_slice(&bytes);
    de.disable_recursion_limit();
    Ok(Recovery::deserialize(&mut de)?)
}
pub fn reload_language_pilot_child(directory: &Path) -> Result<Value> {
    reload(read_recovery(directory)?, Some(directory))
}

fn exact_reload(
    directory: &Path,
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    config: LanguagePilotConfig,
    arm: Arm,
    gradient: Vec<f32>,
) -> Result<Value> {
    exact_reload_plan(
        directory,
        actor,
        asset,
        RecoveredAssessment {
            config,
            arm,
            curriculum: None,
        },
        gradient,
    )
}
fn exact_reload_plan(
    directory: &Path,
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    assessment: RecoveredAssessment,
    gradient: Vec<f32>,
) -> Result<Value> {
    fs::create_dir(directory)?;
    let seed = assessment.config.seed;
    let (mut host, _) = host(seed + 20_000, seed, asset, actor, 0, true)?;
    for _ in 0..4 {
        host.step(actor, 1.)?;
    }
    let (_, inputs) =
        compile_input_profile(seed, asset, actor.phenotype().experimental_language_input())?;
    let recovery = Recovery {
        kind: "disposable-retained-language-pilot-v1".into(),
        inputs,
        foundation: asset.encode_canonical()?,
        actor: actor.checkpoint(),
        host: host.checkpoint(actor)?,
        next_gradient: gradient,
        assessment: Some(assessment),
    };
    let bytes = serde_json::to_vec(&recovery)?;
    fs::write(directory.join("experimental-recovery.sha256"), hash(&bytes))?;
    fs::write(directory.join("experimental-recovery.json"), bytes)?;
    let expected = reload(recovery, None)?;
    let child = Command::new(std::env::current_exe()?)
        .arg("--experimental-reload-child")
        .arg(directory)
        .output()?;
    if !child.status.success() {
        return Err(format!(
            "fresh-process pilot restore failed: {}",
            String::from_utf8_lossy(&child.stderr)
        )
        .into());
    }
    let actual: Value = serde_json::from_slice(&child.stdout)?;
    if actual != expected {
        return Err("fresh-process native continuation differs".into());
    }
    Ok(json!({"fresh_process_exact":true,"native_private_and_optimizer_hashes":actual}))
}

pub fn run_retained_language_pilot(output: &Path, config: LanguagePilotConfig) -> Result<Value> {
    if config.seed == 0
        || !(1..=64).contains(&config.epochs)
        || !(1..=4).contains(&config.rounds)
        || !(4..=64).contains(&config.decisions)
    {
        return Err(
            "pilot bounds: nonzero seed, epochs 1..64, rounds 1..4, decisions 4..64".into(),
        );
    }
    fs::create_dir(output)?;
    let started = Instant::now();
    let translations = teacher_input(config.llamacpp_translation)?;
    write_json(&output.join("teacher-deliveries.json"), &translations)?;
    let source = initial_n2048_care_asset(config.seed)?;
    let control_actor = new_actor_profile(config.seed, &source, config.experimental_word_bridge)?;
    let controls = physical_controls(&control_actor, &source, config.seed)?;
    let initial = control_actor.checkpoint();
    let initial_word_probe = word_path_probe(&control_actor, &source, config.seed)?;
    let trace_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("native-trace.jsonl.zst"))?;
    let mut traces = zstd::stream::write::Encoder::new(trace_file, 3)?;
    let mut arms = Vec::new();
    for arm in [
        Arm::Grounded,
        Arm::Shuffled,
        if config.experimental_word_bridge {
            Arm::NoSpeech
        } else {
            Arm::Untreated
        },
    ] {
        let mut asset = source.clone();
        let mut actor = new_actor_profile(config.seed, &asset, config.experimental_word_bridge)?;
        if actor.checkpoint() != initial {
            return Err("matched actors differ before treatment".into());
        }
        let mut collections = Vec::new();
        let mut updates = Vec::new();
        let mut next_gradient = None;
        for round in 0..config.rounds {
            eprintln!("language pilot {arm:?} round {round}: native collection");
            let (episodes, receipt) = collect(
                &actor,
                &asset,
                config,
                arm,
                round,
                &translations,
                &mut traces,
            )?;
            collections.push(receipt);
            if arm != Arm::Untreated {
                for epoch in 0..config.epochs {
                    let (gradient, objective) = gradients(&actor, &episodes)?;
                    let before = actor.checkpoint();
                    let norm = actor.apply_gradients(&gradient)?;
                    let after = actor.checkpoint();
                    preserve_frozen(&before, &after)?;
                    let recurrent = actor
                        .phenotype()
                        .synapses()
                        .iter()
                        .enumerate()
                        .filter(|(i, s)| {
                            s.kind() == CompiledSynapseKind::Recurrent
                                && before.weights[*i].to_bits() != after.weights[*i].to_bits()
                        })
                        .count();
                    updates.push(json!({"round":round,"epoch":epoch,"step":actor.optimizer_step(),"objective":objective,"gradient_norm":norm,"changed_recurrent_weights":recurrent,"frozen_coordinates_exact":true}));
                    next_gradient = Some(gradient);
                }
                asset = export(&mut actor, config.seed)?;
            }
        }
        eprintln!("language pilot {arm:?}: teacher-absent evaluation");
        let final_word_probe = word_path_probe(&actor, &asset, config.seed)?;
        let continuity = if let Some(gradient) = next_gradient {
            exact_reload(
                &output.join(format!("recovery-{arm:?}")),
                &actor,
                &asset,
                config,
                arm,
                gradient,
            )?
        } else {
            Value::Null
        };
        let evaluation = if continuity.is_null() {
            json!(evaluate(&actor, &asset, config, arm, &mut traces)?)
        } else {
            continuity["native_private_and_optimizer_hashes"]["assessment"]["evaluation"].clone()
        };
        arms.push(json!({"arm":arm,"collections":collections,"updates":updates,"actor_step":actor.optimizer_step(),"final_word_path_probe":final_word_probe,"actor_sha256":identity(&actor.checkpoint())?,"evaluation":evaluation,"continuity":continuity}));
    }
    traces.finish()?;
    let report = json!({"kind":"experimental-retained-grounded-language-v3","execution_domain":"native-cpu","config":config,"teacher_translation_deliveries":translations,"semantic_prior_deliveries":0,"semantic_prior_enabled":false,"initial_actor_sha256":identity(&initial)?,"initial_word_path_probe":initial_word_probe,"scripted_world_positive_controls":controls,"arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"limitations":["Fixed small pilot, not standard CFF ChiChi competence or a general language model.","Experimental word bridge is explicit in compiler inputs/phenotype; it copies raw bits to shared native readouts, not token meanings. Foundation assets alone do not carry this profile; no promotion or existing-brain migration is supported.","Actor-only supervised foundation learning across fresh cohorts; no same-individual optimizer rebase or critic update.","Recorded lifetime/fast offsets, memory and route context remain fixed during conditional replay; recurrent prefixes are rebuilt under current weights for every update.","Subjects pass native hearing/association/motor developmental gates through ordinary world ticks before neural capture. No natural-sleep retention trial; sleeping ticks within ordinary assessment are reported without forcing sleep.","Default uses a deterministic bounded teacher translator; a real SLM delivery is claimed only with confirmed optional llama.cpp receipts.","Fresh-process reload covers exact private state, next eight native steps and the next native Adam update using a retained real gradient; it does not retrain the gradient after reload."]});
    write_json(&output.join("results.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embodied_profile_forks_new_founder_without_resetting_optimizer_or_migrating_private_life() {
        let seed = 847_501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let legacy = new_actor_profile(seed, &asset, false).unwrap();
        let mut actor = new_actor_profile(seed, &asset, true).unwrap();
        assert_eq!(
            identity(legacy.phenotype()).unwrap(),
            "7b7aca53bb50bdefd162c0b88ea990026004192fec2d85848c2fde16ecf9a308"
        );
        assert_eq!(
            identity(actor.phenotype()).unwrap(),
            "a57d7dbbdaa174d3c02988822c4a549616d2fae387adc69b51d4698a26fea054"
        );
        let (_, ordinary_inputs) = compile_profile(seed, &asset, false).unwrap();
        assert!(legacy
            .fork_embodied_language_input_experiment(&ordinary_inputs)
            .is_err());
        let (mut subject, scene) = host(seed, seed, &asset, &actor, 0, false).unwrap();
        let id = subject.world().organism_entity_ids()[0].0;
        let pos = speech_position(subject.world()).unwrap();
        subject
            .world_mut()
            .emit_player_tokens(
                Some(id),
                pos,
                vec![
                    LanguageTokenId::new(8).unwrap(),
                    LanguageTokenId::new(1).unwrap(),
                ],
            )
            .unwrap();
        let row = subject.step(&actor, 1.).unwrap().decisions.remove(0);
        let label = action_label(&row, Goal::EatFood, scene, false).unwrap();
        let episode = Episode {
            speech: vec![Some(ReplaySpeechTarget {
                token: None,
                continuation: [0; 5],
                act: SpeechActKind::Declare,
                weight: 0.25,
            })],
            labels: vec![label],
            rows: vec![row],
        };
        let (gradient, _) = gradients(&actor, &[episode]).unwrap();
        actor.apply_gradients(&gradient).unwrap();
        let asset = export(&mut actor, seed).unwrap();
        let (_, inputs) = compile_profile(seed, &asset, true).unwrap();
        let (resident, _) = host(seed, seed, &asset, &actor, 0, false).unwrap();
        let private = resident.checkpoint(&actor).unwrap();
        let source = actor.checkpoint();
        let source_hash = identity(&source).unwrap();
        let (fork, new_inputs) = actor
            .fork_embodied_language_input_experiment(&inputs)
            .unwrap();
        assert_eq!(identity(&actor.checkpoint()).unwrap(), source_hash);
        let copied = fork.checkpoint();
        assert_eq!(copied.weights, source.weights);
        assert_eq!(copied.first_moment, source.first_moment);
        assert_eq!(copied.second_moment, source.second_moment);
        assert_eq!(copied.update_ages, source.update_ages);
        assert_eq!(copied.optimizer_step, source.optimizer_step);
        assert_eq!(copied.stage_mask, source.stage_mask);
        assert!(source.update_ages.iter().any(|a| *a > 0));
        assert_eq!(fork.phenotype().synapses(), actor.phenotype().synapses());
        assert_ne!(
            fork.phenotype().phenotype_hash(),
            actor.phenotype().phenotype_hash()
        );
        assert_eq!(
            new_inputs.experimental_language_input(),
            Some(ExperimentalLanguageInputV1::TwoTokenAndEmbodiedSpeechV2)
        );
        assert_eq!(
            fork.phenotype().sensor_encoder().assignments().len(),
            actor.phenotype().sensor_encoder().assignments().len() + 30
        );
        for (group, index) in [
            (SensorEncoderSourceGroup::SensoryChannel, 34),
            (SensorEncoderSourceGroup::SensoryChannel, 37),
            (SensorEncoderSourceGroup::Homeostasis, 0),
        ] {
            assert!(fork
                .phenotype()
                .sensor_encoder()
                .assignments()
                .iter()
                .any(|a| a.source_group() == group
                    && a.source_index() == index
                    && (1856..1888).contains(&a.target_neuron())));
        }
        assert!(CpuCanonicalHost::from_checkpoint(private.clone(), &fork).is_err());
        assert!(CpuCanonicalHost::from_checkpoint(private, &actor).is_ok());
        let (fresh, _) = host(seed, seed, &asset, &fork, 0, false).unwrap();
        assert!(CpuCanonicalHost::from_checkpoint(fresh.checkpoint(&fork).unwrap(), &fork).is_ok());
    }

    #[test]
    #[ignore = "explicit native causal capture; requires a new audit output directory"]
    fn capture_native_language_causal_source() {
        let directory =
            std::path::PathBuf::from(std::env::var("ALIFE_LANGUAGE_AUDIT_DIR").unwrap());
        fs::create_dir(&directory).unwrap();
        let seed = 847_501;
        let source = initial_n2048_care_asset(seed).unwrap();
        let bridge = std::env::var("ALIFE_LANGUAGE_AUDIT_BRIDGE").as_deref() == Ok("1");
        let actor = new_actor_profile(seed, &source, bridge).unwrap();
        write_json(&directory.join("phenotype.json"), actor.phenotype()).unwrap();
        let (base, scene) = host(seed + 40_000, seed, &source, &actor, 0, false).unwrap();
        for (name, words) in [("eat", vec![8, 1]), ("get", vec![9, 2])] {
            let mut subject = base.snapshot();
            let id = subject.world().organism_entity_ids()[0].0;
            let position = speech_position(subject.world()).unwrap();
            alife_school::LanguageNursery::speak_in_world(
                subject.world_mut(),
                id,
                position,
                words
                    .into_iter()
                    .map(|w| LanguageTokenId::new(w).unwrap())
                    .collect(),
            )
            .unwrap();
            let mut rows = Vec::new();
            for _ in 0..8 {
                rows.extend(subject.step(&actor, 1.).unwrap().decisions);
            }
            let labels = rows
                .iter()
                .map(|r| action_label(r, Goal::EatFood, scene, false).unwrap())
                .collect();
            let speech = vec![None; rows.len()];
            let episode = Episode {
                rows,
                labels,
                speech,
            };
            if name == "eat" {
                let base = sequence(&actor, &episode, episode.rows.len(), 0);
                let evaluate = |q: &TrainingSequence| {
                    let evaluated = actor.evaluate_replay(q).unwrap();
                    let adjoints = evaluated
                        .candidate_logits
                        .iter()
                        .enumerate()
                        .map(|(i, logits)| {
                            let action = &episode.rows[i].action;
                            let example = ImitationExample {
                                candidate_count: action.candidate_count,
                                representative_mask: action.representative_mask,
                                motor_masks: action.motor_masks,
                                forced_slots: action.forced_slots.clone(),
                                target: episode.labels[i].clone(),
                            };
                            if example.target.motors.iter().all(Option::is_none)
                                && example.target.representative.is_none()
                            {
                                return vec![0.; logits.len()];
                            }
                            cpu_imitation_objective(logits, &example, 1., 1.)
                                .unwrap()
                                .logit_gradients
                        })
                        .collect::<Vec<_>>();
                    let gradient = actor.probe_replay_gradients(q, &adjoints).unwrap();
                    json!({"logits":evaluated.candidate_logits,"activations":evaluated.final_activations,"gradient":gradient.gradients})
                };
                let mut word_pulse = base.clone();
                let mut readout_pulse = base.clone();
                for tick in &mut word_pulse.ticks {
                    for n in [643, 872, 875] {
                        tick.encoded_inputs[n] += 0.75;
                    }
                }
                for tick in &mut readout_pulse.ticks {
                    for n in 1824..1840 {
                        tick.encoded_inputs[n] += 0.75;
                    }
                }
                write_json(&directory.join("diagnostic-interventions.json"), &json!({"not_behavior_or_learning_claim":true,"baseline":evaluate(&base),"word_neuron_pulse":evaluate(&word_pulse),"ingest_readout_pulse":evaluate(&readout_pulse)})).unwrap();
            }
            let evaluation = actor
                .evaluate_replay(&sequence(&actor, &episode, episode.rows.len(), 0))
                .unwrap();
            let (gradient, objective) = gradients(&actor, std::slice::from_ref(&episode)).unwrap();
            write_json(&directory.join(format!("{name}-rows.json")), &episode.rows).unwrap();
            write_json(&directory.join(format!("{name}-evaluation.json")),&json!({"logits":evaluation.candidate_logits,"activations":evaluation.final_activations,"gradient":gradient,"objective":objective})).unwrap();
        }
    }

    #[test]
    fn experimental_word_profile_opens_native_objective_and_preserves_legacy_identity() {
        let seed = 847_501;
        let source = initial_n2048_care_asset(seed).unwrap();
        let (legacy, inputs) = compile_profile(seed, &source, false).unwrap();
        let legacy_bytes = serde_json::to_vec(&legacy).unwrap();
        let encoded = serde_json::to_vec(&inputs).unwrap();
        assert!(!String::from_utf8_lossy(&encoded).contains("experimental_language_input"));
        let old_inputs: PhenotypeCompilerInputs = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            serde_json::to_vec(
                &PhenotypeCompiler::compile_validated(&old_inputs, &BrainCapacityClass::n2048())
                    .unwrap()
            )
            .unwrap(),
            legacy_bytes
        );
        let (experimental, experimental_inputs) = compile_profile(seed, &source, true).unwrap();
        assert_eq!(legacy.synapses(), experimental.synapses());
        assert_ne!(legacy.phenotype_hash(), experimental.phenotype_hash());
        assert_eq!(
            experimental.sensor_encoder().assignments().len(),
            legacy.sensor_encoder().assignments().len() + 160
        );
        let mut stripped = serde_json::to_value(&experimental_inputs).unwrap();
        stripped
            .as_object_mut()
            .unwrap()
            .remove("experimental_language_input");
        assert!(serde_json::from_value::<PhenotypeCompilerInputs>(stripped).is_err());
        let mut phenotype_stripped = serde_json::to_value(&experimental).unwrap();
        phenotype_stripped
            .as_object_mut()
            .unwrap()
            .remove("experimental_language_input");
        assert!(serde_json::from_value::<BrainPhenotype>(phenotype_stripped).is_err());
        let actor = new_actor_profile(seed, &source, true).unwrap();
        let report = word_path_probe(&actor, &source, seed).unwrap();
        assert!(report["changed_candidate_logits"].as_u64().unwrap() > 0);
        assert!(report["changed_trainable_gradients"].as_u64().unwrap() > 0);
        let ordinary = new_actor(seed, &source).unwrap();
        let (mut host, _) = host(seed, seed, &source, &actor, 0, false).unwrap();
        host.step(&actor, 1.).unwrap();
        let saved = host.checkpoint(&actor).unwrap();
        assert!(CpuCanonicalHost::from_checkpoint(saved.clone(), &ordinary).is_err());
        let restored = CpuCanonicalHost::from_checkpoint(
            serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap(),
            &actor,
        )
        .unwrap();
        assert_eq!(
            identity(&restored.checkpoint(&actor).unwrap()).unwrap(),
            identity(&saved).unwrap()
        );
    }

    #[test]
    fn native_world_controls_consume_nourish_and_acquire_distinct_visible_objects() {
        let seed = 847_511;
        let source = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor(seed, &source).unwrap();
        let before = actor.checkpoint();
        let controls = physical_controls(&actor, &source, seed).unwrap();
        assert_eq!(controls.len(), 2);
        assert_eq!(controls[0]["consumed"], true);
        assert!(controls[0]["energy_gain"].as_f64().unwrap() > 0.);
        assert_eq!(controls[1]["acquired_toy"], true);
        assert_eq!(actor.checkpoint(), before);
    }

    #[test]
    fn ordinary_spatial_teacher_is_not_a_retention_result_and_targets_stay_offline() {
        let seed = 847_512;
        let source = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor(seed, &source).unwrap();
        let (mut host, scene) = host(seed, seed, &source, &actor, 0, false).unwrap();
        let id = host.world().organism_entity_ids()[0].0;
        let position = speech_position(host.world()).unwrap();
        let translations = teacher_input(false).unwrap();
        alife_school::LanguageNursery::speak_in_world(
            host.world_mut(),
            id,
            position,
            translations[0].literal_tokens.clone(),
        )
        .unwrap();
        let row = host.step(&actor, 1.).unwrap().decisions.remove(0);
        assert!(scene.prepared_tick > 0 && scene.maturation > 0.);
        assert!(no_teacher_or_prior(&row).is_err());
        assert!(speech_label(&row, None, Goal::EatFood, scene)
            .unwrap()
            .token
            .is_none());
        let label = action_label(&row, Goal::EatFood, scene, false).unwrap();
        assert!(label.motors.iter().flatten().any(|mask| *mask != 0));
        let action = &row.action;
        assert_eq!(
            action.motor_masks[2], 0,
            "reference founder has one sampled head"
        );
        let eat = row
            .patch
            .pre_action()
            .perception()
            .candidates()
            .iter()
            .find(|c| c.action_id == HeadlessActionIds::EAT && c.target.entity == Some(scene.food))
            .unwrap();
        let forced_label = candidate_label(&row, Some(eat)).unwrap();
        assert_eq!(forced_label.motors[2], Some(1u32 << eat.candidate_index));
        ImitationExample {
            candidate_count: action.candidate_count,
            representative_mask: action.representative_mask,
            motor_masks: action.motor_masks,
            forced_slots: action.forced_slots.clone(),
            target: forced_label,
        }
        .validate()
        .unwrap();
        ImitationExample {
            candidate_count: action.candidate_count,
            representative_mask: action.representative_mask,
            motor_masks: action.motor_masks,
            forced_slots: action.forced_slots.clone(),
            target: label,
        }
        .validate()
        .unwrap();
    }

    #[test]
    fn reports_unresolved_word_objective_path_without_confusing_hearing_with_learning() {
        let seed = 847_514;
        let source = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor(seed, &source).unwrap();
        let report = word_path_probe(&actor, &source, seed).unwrap();
        assert!(report["changed_encoded_lanes"].as_u64().unwrap() > 0);
        // A measured limitation of this public founder, not a passing language
        // capability test. Keep the negative visible until the path is repaired.
        assert_eq!(report["changed_trainable_gradients"], 0);
        assert_eq!(report["demonstrates_retained_language"], false);
    }

    #[test]
    fn native_recurrent_update_and_export_keep_frozen_coordinates_and_all_adam_history() {
        let seed = 847_513;
        let source = initial_n2048_care_asset(seed).unwrap();
        let mut actor = new_actor(seed, &source).unwrap();
        let (mut host, scene) = host(seed, seed, &source, &actor, 0, false).unwrap();
        let rows = (0..4)
            .flat_map(|_| host.step(&actor, 1.).unwrap().decisions)
            .collect::<Vec<_>>();
        let labels = rows
            .iter()
            .map(|r| action_label(r, Goal::EatFood, scene, false).unwrap())
            .collect();
        let speech = rows.iter().map(|_| None).collect();
        let mut episode = Episode {
            rows,
            labels,
            speech,
        };
        let observed_labels = episode.labels.clone();
        episode.labels.fill(ImitationTarget {
            representative: None,
            motors: [None; 6],
        });
        let (unsupervised_gradient, _) = gradients(&actor, std::slice::from_ref(&episode)).unwrap();
        assert!(unsupervised_gradient.iter().all(|g| *g == 0.));
        episode.labels = observed_labels;
        let (gradient, _) = gradients(&actor, &[episode]).unwrap();
        assert!(actor
            .phenotype()
            .synapses()
            .iter()
            .enumerate()
            .any(|(i, s)| s.kind() == CompiledSynapseKind::Recurrent && gradient[i] != 0.));
        let before = actor.checkpoint();
        actor.apply_gradients(&gradient).unwrap();
        preserve_frozen(&before, &actor.checkpoint()).unwrap();
        assert_eq!(actor.optimizer_step(), 1);
        export(&mut actor, seed).unwrap();
        assert_eq!(actor.optimizer_step(), 1);
        assert!(actor.checkpoint().first_moment.iter().any(|m| *m != 0.));
        assert!(actor.checkpoint().update_ages.contains(&1));
    }
}
