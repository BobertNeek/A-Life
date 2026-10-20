//! Fifty organism slots in one ordinary world/GPU transaction, one offline policy.
//! The production class-arena kernels carry the hardware row dimension. Only
//! replay is streamed; collecting fifty separate serial sessions is not used.
use super::*;
use crate::{FoundationReplayRecordRef, FoundationReplaySource};
use alife_core::OrganismId;
use std::{collections::BTreeMap, path::PathBuf};
mod admission;
#[cfg(test)]
mod checkpoint_tests;

pub const FOUNDATION_TRAINING_GROUP_SIZE: usize = 50;

// Diagnostic only: each completed pair measures host wall time, including any
// driver work/waits in the enclosed call. Never adds GPU polls or changes state.
struct GroupProgress {
    started: Instant,
    next_phase: std::cell::Cell<u64>,
}

struct GroupPhase<'a> {
    progress: &'a GroupProgress,
    id: u64,
    name: String,
    started: Instant,
}

impl GroupProgress {
    fn emit(&self, event: &str, mut fields: serde_json::Value) {
        use std::io::Write;
        fields["kind"] = serde_json::json!("group50_progress");
        fields["event"] = serde_json::json!(event);
        fields["elapsed_seconds"] = serde_json::json!(self.started.elapsed().as_secs_f64());
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{fields}");
        let _ = stderr.flush();
    }

    fn counts(&self, phase: &str, counts: serde_json::Value) {
        self.emit(
            "progress",
            serde_json::json!({"phase":phase,"counts":counts}),
        );
    }

    fn begin(&self, name: impl Into<String>) -> GroupPhase<'_> {
        let id = self.next_phase.get();
        self.next_phase.set(id + 1);
        let phase = GroupPhase {
            progress: self,
            id,
            name: name.into(),
            started: Instant::now(),
        };
        phase.emit("begin");
        phase
    }
}

impl GroupPhase<'_> {
    fn emit(&self, event: &str) {
        self.progress.emit(
            event,
            serde_json::json!({"phase":self.name,"phase_id":self.id,
                "phase_seconds":self.started.elapsed().as_secs_f64()}),
        );
    }

    fn complete(self) {
        self.emit("complete");
    }
}

pub struct FoundationGroupRequest<'a> {
    pub previous: &'a Path,
    pub output: &'a Path,
    pub seed_history: &'a Path,
    pub seed: u64,
    pub decisions_per_organism: usize,
    /// Explicit incomplete-history opt-in. It never asserts historical freshness.
    pub known_exclusions: bool,
    pub replay_device_limit_bytes: u64,
    pub replay_host_limit_bytes: u64,
    pub sampling_temperature: f32,
}

struct Life {
    id: OrganismId,
    phenotype: alife_core::CreaturePhenotype,
    directory: PathBuf,
    writer: FoundationReplayWriter,
    references: Vec<FoundationReplayRecordRef>,
    features: Vec<Vec<f32>>,
    death: Option<(u64, alife_core::BiochemistryState)>,
    initial_energy: f32,
    minimum_energy: f32,
    last_captured_energy: f32,
    initial_sensory_sha256: Option<String>,
    successes: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Copy)]
struct Span {
    life: usize,
    burn: usize,
    start: usize,
    end: usize,
}

fn truncated(captured_rows: usize, requested_rows: usize) -> bool {
    captured_rows > requested_rows
}

fn spans(
    life: usize,
    references: &[FoundationReplayRecordRef],
    rows: usize,
    width: usize,
) -> Vec<Span> {
    let mut result = Vec::new();
    let mut segment_start = 0;
    while segment_start < rows {
        let segment = references[segment_start].segment;
        let end = references[segment_start..rows]
            .iter()
            .position(|r| r.segment != segment)
            .map_or(rows, |offset| segment_start + offset);
        for start in (segment_start..end).step_by(width) {
            result.push(Span {
                life,
                burn: start.saturating_sub(128).max(segment_start),
                start,
                end: (start + width).min(end),
            });
        }
        segment_start = end;
    }
    result
}

fn admitted_actor_rows(spans: &[Span], updates: u32, lives: usize) -> (Vec<usize>, u64) {
    let batches = spans.len().div_ceil(50);
    let mut seen = vec![false; spans.len()];
    let mut exposures = 0;
    if batches > 0 {
        for update in 0..updates as usize {
            let start = update % batches * 50;
            let end = (start + 50).min(spans.len());
            for index in start..end {
                seen[index] = true;
                exposures += (spans[index].end - spans[index].start) as u64;
            }
        }
    }
    let mut counts = vec![0; lives];
    for (span, seen) in spans.iter().zip(seen) {
        if seen {
            counts[span.life] += span.end - span.start;
        }
    }
    (counts, exposures)
}

fn group_source(previous: &Path) -> Result<SealedCycleContinuation> {
    if !previous.join("group50.json").is_file() {
        let source = load_cycle_continuation(previous, None)?;
        let receipt: FoundationCycleReceipt =
            serde_json::from_slice(&std::fs::read(previous.join("cycle.json"))?)?;
        let phenotype = crate::foundation_archived_success_replay::compile_source(
            source.asset.clone(),
            receipt.seed,
            source.founder_seed_base,
        )?;
        source
            .actor
            .validate_for(&phenotype, source.asset.digest())?;
        PpoTrainingState::from_checkpoint(source.value.clone())?;
        if source.value.feature_count != phenotype.neuron_count() {
            return Err("native cycle critic feature coordinates changed".into());
        }
        return Ok(source);
    }
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(previous.join("group50.json"))?)?;
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
        previous.join("trained.alife-foundation"),
    )?)?;
    let actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&std::fs::read(previous.join("actor-checkpoint.json"))?)?;
    let value: alife_training::PpoValueHeadCheckpoint =
        serde_json::from_slice(&std::fs::read(previous.join("value-checkpoint.json"))?)?;
    let policy = receipt["policy_version"]
        .as_u64()
        .ok_or("missing group policy version")?;
    PpoTrainingState::from_checkpoint(value.clone())?;
    if receipt["schema"] != 1
        || receipt["complete"] != true
        || receipt["mode"] != "group50_shared_habitat"
        || receipt["population"] != 50
        || receipt["biological_objective_version"] != BIOLOGICAL_OBJECTIVE_VERSION
        || receipt["critic_objective_version"] != critic_residual::OBJECTIVE_VERSION
        || receipt["next_cohort_optimizer_rebound"] != true
        || receipt["new_asset_digest"] != digest(&asset)
        || actor.source_foundation_digest != asset.digest()
        || receipt["actor_optimizer_step"].as_u64() != Some(u64::from(actor.optimizer_step))
        || receipt["value_optimizer_step"].as_u64() != Some(u64::from(value.optimizer_step))
        || value.last_updated_policy_version != Some(policy)
        || actor.weights.len() != asset.weights().len()
        || actor
            .weights
            .iter()
            .zip(asset.weights())
            .any(|(a, b)| a.to_bits() != b.to_bits())
        || receipt["actor_checkpoint_sha256"]
            != admission::hash(&std::fs::read(previous.join("actor-checkpoint.json"))?)
        || receipt["value_checkpoint_sha256"]
            != admission::hash(&std::fs::read(previous.join("value-checkpoint.json"))?)
    {
        return Err("group checkpoint is not an exact complete native handoff".into());
    }
    let founder_seed = receipt["founder_seed_base"]
        .as_u64()
        .filter(|s| *s > 0)
        .ok_or("missing founder seed")?;
    let mut config = alife_world::CanonicalNewGameConfig::phase3(
        receipt["seed"].as_u64().ok_or("missing source seed")?,
        1,
    )?;
    config.founder_seed_base = founder_seed;
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("missing source group founder")?;
    let genome = record.phenotype().brain_genome.clone();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &genome,
        &alife_core::BrainCapacityClass::n2048(),
        &record
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)?,
    )?;
    let (phenotype, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        asset.clone(),
    )?;
    actor.validate_for(&phenotype, asset.digest())?;
    if value.feature_count != phenotype.neuron_count() {
        return Err("group critic feature coordinates changed".into());
    }
    Ok(SealedCycleContinuation {
        asset,
        actor,
        value,
        policy_version: policy.checked_add(1).ok_or("policy overflow")?,
        founder_seed_base: receipt["founder_seed_base"]
            .as_u64()
            .filter(|s| *s > 0)
            .ok_or("missing founder seed")?,
        objective_transition_from: None,
    })
}

/// Read-only native handoff validation. It allocates no seed and opens no GPU.
/// Source admission does not assert historical seed freshness or restore a life.
pub fn validate_foundation_training_group_source(previous: &Path) -> Result<serde_json::Value> {
    let source = group_source(previous)?;
    Ok(
        serde_json::json!({"schema":1,"native_checkpoint_valid":true,
        "next_policy_version":source.policy_version,
        "biological_objective_version":BIOLOGICAL_OBJECTIVE_VERSION,
        "critic_objective_version":critic_residual::OBJECTIVE_VERSION,
        "objective_state_reset":false,"founder_seed_base":source.founder_seed_base,
        "foundation_digest":digest(&source.asset),"learned_weight_count":source.actor.weights.len(),
        "actor_optimizer_step":source.actor.optimizer_step,
        "actor_update_age_min":source.actor.update_ages.iter().min(),
        "actor_update_age_max":source.actor.update_ages.iter().max(),
        "actor_checkpoint_sha256":admission::hash(&std::fs::read(previous.join("actor-checkpoint.json"))?),
        "value_optimizer_step":source.value.optimizer_step,
        "value_checkpoint_sha256":admission::hash(&std::fs::read(previous.join("value-checkpoint.json"))?),
        "seed_reserved":false,"historical_freshness":"not_asserted","personal_lifetime_restored":false}),
    )
}

// Conservative replay peak: old/new packed sets can overlap during a shape
// change. The reserve covers the persistent PPO objective, actor accumulation,
// checkpoint/rollback/readback and bounded critic feature scratch. Runtime
// class arenas are reported separately. This is a bound, not measured VRAM.
fn replay_auxiliary_bound_bytes(neurons: u64, synapses: u64) -> u64 {
    // Persistent accumulation + packed checkpoint readback, PPO objective/
    // rollback, metrics readback, and bounded residual feature/head scratch.
    synapses * 24
        + neurons * 257 * 8
        + (neurons + 1) * 64
        + alife_training::MAX_TRAINING_SEQUENCE_TICKS as u64 * (64 + 45 + 8) * 4
        + 2 * 1024 * 1024
}
fn replay_peak_bytes(
    phenotype: &alife_core::BrainPhenotype,
    sequence: &alife_training::TrainingSequence,
) -> Result<u64> {
    Ok(
        FoundationTrainer::replay_allocation_bytes(phenotype, sequence)? * 2
            + replay_auxiliary_bound_bytes(
                u64::from(phenotype.neuron_count()),
                phenotype.synapses().len() as u64,
            ),
    )
}

#[derive(Clone, Copy)]
struct ReplayDeviceLimits {
    peak: u64,
    binding: u64,
}

fn load(
    life: &Life,
    span: Span,
    source: &FoundationReplaySource,
    phenotype: &alife_core::BrainPhenotype,
    budget: FoundationReplayBudget,
    limits: ReplayDeviceLimits,
) -> Result<crate::FoundationReplayWindow> {
    let replay = load_foundation_replay_window(
        &life.directory,
        &life.references[span.burn..span.end],
        span.start - span.burn,
        source,
        phenotype,
        budget,
    )?;
    let allocation = replay_peak_bytes(phenotype, &replay.sequence)?;
    if allocation > limits.peak
        || FoundationTrainer::replay_storage_binding_bytes(phenotype, &replay.sequence)?
            > limits.binding
    {
        return Err(
            "replay exceeds peak allocation or measured adapter storage binding limit".into(),
        );
    }
    Ok(replay)
}

/// Native continuation with no optimizer reset or historical rollout injection.
/// Seeds are reserved before any world/device operation, including failed runs.
pub fn run_foundation_training_group(
    request: FoundationGroupRequest<'_>,
) -> Result<serde_json::Value> {
    if request.seed == 0
        || !(1..=2048).contains(&request.decisions_per_organism)
        || request.replay_device_limit_bytes < 16 * 1024 * 1024
        || request.replay_device_limit_bytes > 2 * 1024 * 1024 * 1024
    {
        return Err("group50 needs a nonzero seed, 1..=2048 decisions per organism and 16..=2048 MiB replay budget".into());
    }
    if request.output.exists() {
        return Err("group output must be a new directory".into());
    }
    if request.replay_host_limit_bytes == 0
        || request.replay_host_limit_bytes > 8 * 1024 * 1024 * 1024
    {
        return Err("group replay host ceiling must be in1..=8192 MiB".into());
    }
    cycle_ppo_config(request.sampling_temperature)?;
    let admitted = group_source(request.previous)?;
    let provenance = admission::admit(
        request.previous,
        request.seed_history,
        request.output,
        request.seed,
        request.known_exclusions,
    )?;
    let lane = request
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(lane)?;
    let lock_path = lane.join(".eat-held-food-owner.lock");
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)?;
    struct Lock(PathBuf, Option<std::fs::File>);
    impl Drop for Lock {
        fn drop(&mut self) {
            drop(self.1.take());
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _lock = Lock(lock_path, Some(lock));
    // Recheck under the shared lane lock; no stale snapshot can authorize a seed.
    let provenance_locked = admission::admit(
        request.previous,
        request.seed_history,
        request.output,
        request.seed,
        request.known_exclusions,
    )?;
    if provenance != provenance_locked {
        return Err("seed evidence changed before reservation".into());
    }
    // Immutable, importable exclusion receipt. Creating this file is the seed
    // reservation; a failed or uncertain attempt never removes it.
    let ledger_path = lane.join(format!(".group50-seed-{}.json", request.seed));
    let ledger = serde_json::json!({"schema":1,"kind":"grab-food-seed-ledger",
        "training_seed_ranges":[[request.seed,request.seed]],"assessment_seeds":[],"allocations":[],
        "group50_reservation":{"output":request.output,"world_seed":request.seed,"admission":provenance}});
    let mut reservation = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ledger_path)?;
    use std::io::Write;
    reservation.write_all(&serde_json::to_vec(&ledger)?)?;
    reservation.sync_all()?;
    crate::foundation_training_output::in_new_directory(
        request.output,
        || run_owned(&request, admitted, &provenance),
        |error| {
            let _=std::fs::write(request.output.join("failure.json"),serde_json::to_vec_pretty(&serde_json::json!({"error":error.to_string(),"seed_admission":provenance,"seed_consumed":true})).unwrap_or_default());
        },
    )
}

fn run_owned(
    request: &FoundationGroupRequest<'_>,
    admitted: SealedCycleContinuation,
    provenance: &serde_json::Value,
) -> Result<serde_json::Value> {
    let output = request.output;
    let SealedCycleContinuation {
        asset,
        policy_version,
        founder_seed_base,
        actor,
        value: old_value,
        ..
    } = admitted;
    let started = Instant::now();
    let progress = GroupProgress {
        started,
        next_phase: std::cell::Cell::new(0),
    };
    let setup_phase = progress.begin("source_copy_and_world_setup");
    let budget = FoundationReplayBudget {
        host_limit_bytes: request.replay_host_limit_bytes,
    };
    budget.check_additional(256 * 1024 * 1024)?;
    std::fs::write(
        output.join("seed-provenance.json"),
        serde_json::to_vec_pretty(provenance)?,
    )?;
    std::fs::create_dir(output.join("source-receipts"))?;
    for entry in std::fs::read_dir(request.previous)? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && (entry.path().extension().is_some_and(|e| e == "json")
                || entry.file_name() == "trained.alife-foundation")
        {
            std::fs::copy(
                entry.path(),
                output.join("source-receipts").join(entry.file_name()),
            )?;
        }
    }
    for (name, expected) in provenance["source_sha256"]
        .as_object()
        .ok_or("missing native source bindings")?
    {
        if admission::hash(&std::fs::read(output.join("source-receipts").join(name))?)
            != expected.as_str().ok_or("bad source hash")?
        {
            return Err("source changed between admission and immutable copy".into());
        }
    }
    let copied = group_source(request.previous)?;
    if copied.asset != asset
        || copied.actor != actor
        || copied.value != old_value
        || copied.policy_version != policy_version
        || copied.founder_seed_base != founder_seed_base
    {
        return Err("copied source differs from admitted native optimizer state".into());
    }
    std::fs::write(
        output.join("initial.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    let mut game = alife_world::create_canonical_foundation_training_group(
        request.seed,
        founder_seed_base,
        &asset,
    )?;
    crate::foundation_training::ground_foundation_training_world(&mut game.world)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
    let population = game
        .world
        .organism_registry()
        .iter()
        .map(|r| {
            (
                r.organism_id(),
                r.phenotype().clone(),
                r.biochemistry().body.energy,
            )
        })
        .collect::<Vec<_>>();
    if population.len() != FOUNDATION_TRAINING_GROUP_SIZE {
        return Err("group admission did not create fifty private organisms".into());
    }
    std::fs::write(
        output.join("scenario.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"world_seed":request.seed,"founder_seed_base":founder_seed_base,
        "habitat":"shared_canonical_world","population":50,"founders":game.receipt.founders.iter().map(|f|serde_json::json!({"organism_id":f.organism_id.raw(),
            "position":game.world.entity(f.world_entity_id).map(|e|e.position.to_array())})).collect::<Vec<_>>(),"age_death_disabled":true}),
        )?,
    )?;
    setup_phase.complete();
    let phase = progress.begin("live_backend_create");
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    phase.complete();
    let phase = progress.begin("live_runtime_create");
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        backend,
        game.world,
        request.seed,
        BrainScaleTier::Standard2048,
        asset.manifest().sensor_profile(),
        alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
        format!("n2048-group50-{}", request.seed),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        cycle_sampling_config(request.seed, policy_version, request.sampling_temperature)?,
    )?;
    phase.complete();
    let phase = progress.begin("initial_checkpoint_prepare");
    let asset_root = output.join("world-assets");
    std::fs::create_dir(&asset_root)?;
    let save_path = output.join("world.json");
    let mut config = alife_world::RuntimeConfig::deterministic_default(
        request.seed,
        BrainScaleTier::Standard2048,
    );
    config.features.gpu_backend_enabled = true;
    let base = alife_world::PortableSaveFile::from_headless_world(
        format!("n2048-group50-{}", request.seed),
        runtime.world(),
        config,
        alife_world::AssetManifest::empty(),
        game.creatures,
    )?;
    base.validate_with_asset_root(&asset_root)?;
    phase.complete();
    let phase = progress.begin("initial_boundary_attach");
    runtime.attach_durable_checkpoint_boundary(&save_path, &asset_root, base)?;
    phase.complete();
    let phase = progress.begin("initial_checkpoint_capture");
    let exact = runtime.capture_portable_checkpoint()?;
    phase.complete();
    let phase = progress.begin("initial_snapshot_publish");
    let published = GpuDurableSaveManifest::publish_snapshot(&save_path, &asset_root, &exact)?;
    if published.save != exact {
        return Err("group exact initial save changed".into());
    }
    phase.complete();
    let phase = progress.begin("durable_boundary_rebind");
    runtime.rebind_durable_checkpoint_boundary(&save_path, &asset_root, &exact)?;
    phase.complete();
    runtime.set_performance_measurement_enabled(true);
    let collection_started = Instant::now();
    let phase = progress.begin("first_runtime_tick");
    runtime.tick()?;
    phase.complete();
    let mut batch_available_seconds = collection_started.elapsed().as_secs_f64();
    let phase = progress.begin("first_capture_check");
    let first = runtime.take_foundation_training_steps();
    if first.len() != 50 {
        return Err(format!(
            "group initial tick expected50 GPU decisions, got{}",
            first.len()
        )
        .into());
    }
    let phenotype = first[0].before.phenotype.as_ref().clone();
    phase.complete();
    let phase = progress.begin("trainer_staging_create");
    let staging = runtime.new_staging_like_live()?;
    phase.complete();
    let phase = progress.begin("initial_trainer_memory_check");
    let adapter_limits = staging.offline_training_device_queue()?.0.limits();
    let limits = ReplayDeviceLimits {
        peak: request.replay_device_limit_bytes,
        binding: adapter_limits
            .max_storage_buffer_binding_size
            .min(adapter_limits.max_buffer_size),
    };
    let initial_sequence = crate::foundation_replay_sequence(&first[..1], 0)?;
    if replay_peak_bytes(&phenotype, &initial_sequence)? > limits.peak
        || FoundationTrainer::replay_storage_binding_bytes(&phenotype, &initial_sequence)?
            > limits.binding
    {
        return Err("initial trainer exceeds replay allocation or adapter binding limit".into());
    }
    phase.complete();
    let phase = progress.begin("trainer_create");
    let mut trainer = FoundationTrainer::from_session(
        alife_runtime::GpuAuthoritativeSession::new(
            staging,
            alife_runtime::GpuSessionConsumerKind::Training,
        ),
        phenotype.clone(),
        asset.clone(),
        actor.stage_mask.clone(),
        actor.config,
    )?;
    phase.complete();
    let phase = progress.begin("actor_checkpoint_restore");
    trainer.restore_checkpoint(&actor)?;
    phase.complete();

    let phase = progress.begin("replay_writers_create");
    let checkpoint_bytes = initial_actor_checkpoint_bytes(Some(&actor), || {
        Ok(serde_json::to_vec(&trainer.checkpoint()?)?)
    })?;
    let source = foundation_replay_source(&phenotype, &asset, policy_version, &checkpoint_bytes)?;
    let mut lives = population
        .into_iter()
        .map(|(id, body, energy)| {
            let directory = output.join("replay").join(format!("organism-{}", id.raw()));
            Ok((
                id.raw(),
                Life {
                    id,
                    phenotype: body,
                    writer: FoundationReplayWriter::new(
                        &directory,
                        source.clone(),
                        phenotype.clone(),
                        &asset,
                        budget,
                    )?,
                    directory,
                    references: Vec::new(),
                    features: Vec::new(),
                    death: None,
                    initial_energy: energy,
                    minimum_energy: energy,
                    last_captured_energy: energy,
                    initial_sensory_sha256: None,
                    successes: Vec::new(),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    phase.complete();
    let collection_phase = progress.begin("collection");
    let mut captured = first;
    let mut neural_work = alife_core::BrainWorkCounters::default();
    let mut max_batch_rows = 0;
    let mut no_progress_poll_count = 0u64;
    let mut batch_histogram = BTreeMap::<usize, u64>::new();
    let mut verified_replay_boundaries = 0u64;
    let mut stalled = Instant::now();
    let mut collector_tick_phase = None;
    let mut collected_rows = 0usize;
    loop {
        let rows = captured.len();
        max_batch_rows = max_batch_rows.max(rows);
        if rows > 0 {
            *batch_histogram.entry(rows).or_default() += 1;
        }
        for step in captured.drain(..) {
            let life = lives
                .get_mut(&step.frame.organism_id().raw())
                .ok_or("unexpected organism admitted into pinned group")?;
            if truncated(life.references.len(), request.decisions_per_organism) {
                continue;
            }
            if step.before.phenotype.as_ref() != &phenotype {
                return Err("group organism does not share the pinned founder graph".into());
            }
            let new_segment = life
                .references
                .last()
                .is_some_and(|r| r.tick + 1 != step.frame.tick().raw());
            if life.references.is_empty() || new_segment {
                let phase = progress.begin(format!(
                    "replay_boundary_verify organism={} row={}",
                    life.id.raw(),
                    life.references.len()
                ));
                verify_foundation_replay_step(&mut trainer, &step)?;
                phase.complete();
                verified_replay_boundaries += 1;
            }
            if new_segment {
                life.writer.start_segment()?;
            }
            let measured = step
                .patch
                .outcome()
                .measured_physiology
                .ok_or("group patch lacks organism-owned biology")?;
            life.minimum_energy = life.minimum_energy.min(measured.after.body.energy);
            life.last_captured_energy = measured.after.body.energy;
            if life.initial_sensory_sha256.is_none() {
                life.initial_sensory_sha256 =
                    Some(admission::hash(&serde_json::to_vec(step.frame.sensory())?));
            }
            if consumed(&step) {
                life.successes.push(serde_json::json!({"row":life.references.len(),"decision_tick":step.frame.tick().raw(),
                "outcome_tick":step.patch.outcome().outcome_tick.raw(),"elapsed_seconds":batch_available_seconds,"patch":step.patch.header()}));
            }
            neural_work.microsteps = neural_work
                .microsteps
                .saturating_add(step.work.counters.microsteps);
            neural_work.neuron_updates += step.work.counters.neuron_updates;
            neural_work.synapse_ops += step.work.counters.synapse_ops;
            neural_work.tile_visits += step.work.counters.tile_visits;
            neural_work.decoder_candidate_ops += step.work.counters.decoder_candidate_ops;
            neural_work.memory_context_ops += step.work.counters.memory_context_ops;
            life.features
                .push(crate::foundation_training::snapshot_activation(
                    &step.after_inference,
                )?);
            life.references.push(life.writer.append(&step)?);
            collected_rows += 1;
            progress.counts(
                "collection_row",
                serde_json::json!({
                "organism_id":life.id.raw(),"organism_rows":life.references.len(),
                "organism_row_target":request.decisions_per_organism+1,
                "captured_rows_total":collected_rows,
                "maximum_captured_rows":50*(request.decisions_per_organism+1),
                "decision_tick":step.frame.tick().raw()}),
            );
        }
        for life in lives.values_mut() {
            // Once the real next decision is captured, this trajectory is a
            // truncation. Later unrecorded actions/death cannot change its mask.
            if life.death.is_some()
                || truncated(life.references.len(), request.decisions_per_organism)
            {
                continue;
            }
            if let Some(record) = runtime.world().organism_registry().get(life.id) {
                life.minimum_energy = life.minimum_energy.min(record.biochemistry().body.energy);
                if !record.lifecycle().is_alive() {
                    life.death = Some((
                        record
                            .lifecycle()
                            .death_tick()
                            .ok_or("dead organism lacks death tick")?
                            .raw(),
                        *record.biochemistry(),
                    ));
                }
            } else {
                let death = runtime
                    .archive_retirement_receipt(life.id)
                    .ok_or("group organism disappeared without archived retirement")?
                    .death_tick
                    .raw();
                let biology = runtime
                    .take_foundation_terminal_biology(life.id)
                    .ok_or("archived group death lacks canonical terminal biology")?;
                if biology.tick.raw() != death {
                    return Err("terminal group biology tick mismatch".into());
                }
                life.death = Some((death, biology));
            }
        }
        if !runtime.last_memory_preparation_errors().is_empty()
            || !runtime.last_post_seal_learning_failures().is_empty()
        {
            return Err(
                "group encountered rejected canonical preparation/learning; refusing PPO".into(),
            );
        }
        if lives.values().all(|life| {
            life.death.is_some() || truncated(life.references.len(), request.decisions_per_organism)
        }) {
            break;
        }
        let tick = runtime.world().tick();
        if tick.raw() > request.decisions_per_organism as u64 * 8 + 4096 {
            return Err("group reached bounded world-tick limit".into());
        }
        budget.check_additional(256 * 1024 * 1024)?;
        // Keep one phase open across no-progress polls until this tick advances.
        collector_tick_phase.get_or_insert_with(|| {
            progress.begin(format!(
                "collector_tick_until_progress world_tick={}",
                tick.raw()
            ))
        });
        runtime.tick_outcome()?;
        batch_available_seconds = collection_started.elapsed().as_secs_f64();
        if runtime.world().tick() == tick {
            no_progress_poll_count += 1;
            if stalled.elapsed().as_secs() > 300 {
                return Err("group checkpoint made no progress for five minutes".into());
            }
        } else {
            stalled = Instant::now();
            if let Some(phase) = collector_tick_phase.take() {
                phase.complete();
            }
        }
        captured = runtime.take_foundation_training_steps();
    }
    collection_phase.complete();
    let collection_seconds = collection_started.elapsed().as_secs_f64();
    let replay_phase = progress.begin("value_restore_and_replay_prepare");
    let runtime_metrics = runtime.performance_metrics();
    let mut value = PpoTrainingState::from_checkpoint(old_value.clone())?;
    let mut batches = Vec::new();
    let mut life_vec = Vec::new();
    let mut all_spans = Vec::new();
    let mut peak_replay_bytes = 0;
    let mut critic_checkpoint = None;
    for (life_index, life) in lives.into_values().enumerate() {
        let phase = progress.begin(format!("replay_prepare organism={}", life.id.raw()));
        let rows = life
            .references
            .len()
            .saturating_sub(usize::from(life.death.is_none()));
        if rows == 0 {
            return Err("group organism has no complete learning transition".into());
        }
        std::fs::write(
            life.directory.join("manifest.json"),
            serde_json::to_vec(&life.references)?,
        )?;
        let mut values = Vec::new();
        let mut actions = Vec::new();
        let mut previous_after = None;
        for span in spans(0, &life.references, life.references.len(), 128) {
            let span = Span {
                burn: span.start,
                ..span
            };
            let replay = load(&life, span, &source, &phenotype, budget, limits)?;
            peak_replay_bytes =
                peak_replay_bytes.max(replay_peak_bytes(&phenotype, &replay.sequence)?);
            values.extend(value.predict_values(&mut trainer, &replay.sequence)?);
            if critic_checkpoint.is_none() {
                let restored = value.checkpoint(trainer.session())?;
                if restored != old_value {
                    return Err("group value/Adam changed before any training".into());
                }
                critic_checkpoint = Some(restored);
            }
            for (offset, (behavior, patch)) in
                replay.behavior.iter().zip(&replay.patches).enumerate()
            {
                let i = span.start + offset;
                let physiology = patch
                    .outcome()
                    .measured_physiology
                    .ok_or("group replay lacks measured biology")?;
                if i > 0 && life.references[i].tick > life.references[i - 1].tick + 1 {
                    let gap = alife_core::MeasuredPhysiologyTransition::new(
                        previous_after.ok_or("missing prior group biology")?,
                        physiology.before,
                    )?;
                    let dt =
                        (life.references[i].tick - life.references[i - 1].tick - 1) as f32 / 20.0;
                    let discount = (-std::f32::consts::LN_2 * dt
                        / cycle_ppo_config(request.sampling_temperature)?
                            .discount_half_life_seconds)
                        .exp();
                    let prior: &mut (PpoJointAction, f32) =
                        actions.last_mut().ok_or("missing group gap predecessor")?;
                    prior.1 = (prior.1
                        + training_biological_value(&gap, &life.phenotype)? * discount)
                        .clamp(-1.0, 1.0);
                }
                previous_after = Some(physiology.after);
                if i == rows {
                    continue;
                }
                let receptors = physiology.before.neural_receptor_frame(&life.phenotype)?;
                let modulator = alife_core::OutcomeCreditPacket::from_sealed_patch(patch)?
                    .with_biochemical_receptors(&receptors)?
                    .modulator();
                actions.push((
                    joint_action(behavior)?,
                    training_receptor_profile(&life.phenotype).project(&modulator.frame())?,
                ));
            }
        }
        if let Some((_, terminal)) = life.death {
            let previous = previous_after.ok_or("terminal group life lacks sealed biology")?;
            let gap = alife_core::MeasuredPhysiologyTransition::new(previous, terminal)?;
            let dt = (terminal.tick.raw() - previous.tick.raw()) as f32 / 20.0;
            let discount = (-std::f32::consts::LN_2 * dt
                / cycle_ppo_config(request.sampling_temperature)?.discount_half_life_seconds)
                .exp();
            let prior = actions
                .last_mut()
                .ok_or("terminal group life lacks trainable action")?;
            prior.1 = (prior.1 + training_biological_value(&gap, &life.phenotype)? * discount)
                .clamp(-1.0, 1.0);
        }
        let config = cycle_ppo_config(request.sampling_temperature)?;
        let transitions = actions
            .into_iter()
            .enumerate()
            .map(|(i, (action, reward))| {
                let terminal = life.death.is_some() && i + 1 == rows;
                let next_tick = if terminal {
                    life.death.unwrap().0
                } else {
                    life.references[i + 1].tick
                };
                PpoTransition {
                    policy_version,
                    trajectory_id: life.id.raw(),
                    step: i as u64,
                    action,
                    reward,
                    old_value: values[i],
                    next_value: if terminal { 0.0 } else { values[i + 1] },
                    elapsed_seconds: (next_tick - life.references[i].tick) as f32 / 20.0,
                    boundary: if terminal {
                        PpoBoundary::Terminated
                    } else if i + 1 == rows {
                        PpoBoundary::Truncated
                    } else {
                        PpoBoundary::Continuing
                    },
                }
            })
            .collect();
        let batch = PpoBatch::from_rollout(policy_version, transitions, config)?;
        let index = life_vec.len();
        let mut planned = spans(index, &life.references, rows, 256);
        // Fit actual packed buffers to the declared device ceiling. Splitting
        // loss spans preserves each segment's full available 128-row burn-in.
        let mut cursor = 0;
        while cursor < planned.len() {
            let span = planned[cursor];
            let replay = load_foundation_replay_window(
                &life.directory,
                &life.references[span.burn..span.end],
                span.start - span.burn,
                &source,
                &phenotype,
                budget,
            )?;
            let bytes = replay_peak_bytes(&phenotype, &replay.sequence)?;
            if bytes > limits.peak
                || FoundationTrainer::replay_storage_binding_bytes(&phenotype, &replay.sequence)?
                    > limits.binding
            {
                if span.end - span.start <= 1 {
                    return Err("device replay ceiling cannot hold required burn-in".into());
                }
                let mid = span.start + (span.end - span.start) / 2;
                planned[cursor] = Span { end: mid, ..span };
                planned.insert(
                    cursor + 1,
                    Span {
                        burn: mid.saturating_sub(128).max(span.burn),
                        start: mid,
                        ..span
                    },
                );
            } else {
                peak_replay_bytes = peak_replay_bytes.max(bytes);
                cursor += 1;
            }
        }
        all_spans.extend(planned);
        batches.push(batch);
        life_vec.push(life);
        phase.complete();
        progress.counts(
            "replay_prepare",
            serde_json::json!({
            "lives_complete":life_index+1,"lives_total":50,
            "windows_planned":all_spans.len()}),
        );
    }
    replay_phase.complete();
    // Interleave individuals before effective batches; every window still keeps
    // its exact private recurrent, chemistry, action, lifetime and sequence masks.
    all_spans.sort_by_key(|s| (s.start, s.life));
    let config = cycle_ppo_config(request.sampling_temperature)?;
    let actor_age = trainer.optimizer_step();
    let update_started = Instant::now();
    let phase = progress.begin("actor_update");
    let mut actor_windows_loaded = 0usize;
    let mut actor_window_phase: Option<GroupPhase<'_>> = None;
    let update = train_recurrent_ppo_cohort(
        &mut trainer,
        &mut value,
        all_spans.len(),
        50,
        |index| {
            if let Some(previous) = actor_window_phase.take() {
                previous.complete();
                progress.counts(
                    "actor_windows",
                    serde_json::json!({
                    "windows_completed":actor_windows_loaded,
                    "maximum_window_calls":all_spans.len()*(config.epochs as usize+1)}),
                );
            }
            actor_windows_loaded += 1;
            progress.counts(
                "actor_windows",
                serde_json::json!({
                "windows_started":actor_windows_loaded,"window_index":index,
                "pass_index":(actor_windows_loaded-1)/all_spans.len(),
                "windows_per_pass":all_spans.len(),
                "maximum_window_calls":all_spans.len()*(config.epochs as usize+1)}),
            );
            // The next load (or cohort return) proves that this window's opaque
            // trainer work returned. Do not add polls or infer an Adam commit.
            actor_window_phase = Some(progress.begin(format!(
                "actor_window_until_next_load_or_return index={index} call={actor_windows_loaded}"
            )));
            let span = all_spans[index];
            let replay = load(
                &life_vec[span.life],
                span,
                &source,
                &phenotype,
                budget,
                limits,
            )
            .map_err(|_| {
                alife_training::TrainingError::from(ScaffoldContractError::InvalidDecisionEvidence)
            })?;
            Ok(PpoTrainingWindow {
                speech_targets: vec![None; replay.sequence.ticks.len()],
                sequence: replay.sequence,
                batch: batches[span.life].window(span.start..span.end)?,
                auxiliary: None,
            })
        },
        PpoConfig {
            value_coefficient: 0.0,
            ..config
        },
        policy_version,
    )?;
    if let Some(last) = actor_window_phase.take() {
        last.complete();
    }
    progress.counts(
        "actor_update",
        serde_json::json!({
        "windows_completed":actor_windows_loaded,"completed_epochs":update.completed_epochs,
        "actor_optimizer_step":update.actor_optimizer_step,
        "actor_optimizer_updates":update.actor_optimizer_step-actor_age,
        "stopped_for_kl":update.stopped_for_kl}),
    );
    phase.complete();
    let phase = progress.begin("critic_update");
    let mut critic_checkpoint = critic_checkpoint.ok_or("group lacks restored value head")?;
    // The accepted critic objective uses frozen actual production next features.
    // One central value checkpoint/Adam history traverses all lives, including
    // failures, in bounded windows; no bootstrap ever points to another life.
    let mut critic_updates = 0u64;
    let critic_epochs = update
        .completed_epochs
        .max(u32::from(update.actor_optimizer_step > actor_age));
    let critic_total_updates =
        batches.iter().map(|b| b.len().div_ceil(256)).sum::<usize>() * critic_epochs as usize;
    for epoch in 0..critic_epochs {
        for (life, batch) in life_vec.iter().zip(&batches) {
            for start in (0..batch.len()).step_by(256) {
                let end = (start + 256).min(batch.len());
                let transitions = &batch.transitions()[start..end];
                let feature_end = (end
                    + usize::from(transitions.last().unwrap().boundary != PpoBoundary::Terminated))
                .min(life.features.len());
                progress.counts("critic_update", serde_json::json!({
                    "completed_updates":critic_updates,"total_updates":critic_total_updates,
                    "active_organism_id":life.id.raw(),"epoch_index":epoch,
                    "loss_row_start":start,"value_optimizer_step":critic_checkpoint.optimizer_step}));
                let critic_phase = progress.begin(format!(
                    "critic_window_update organism={} epoch={epoch} start={start}",
                    life.id.raw()
                ));
                let (next, _) = critic_residual::train(
                    trainer.session(),
                    critic_checkpoint,
                    &life.features[start..feature_end],
                    transitions,
                    config,
                    1,
                    policy_version,
                )?;
                critic_checkpoint = next;
                critic_updates += 1;
                critic_phase.complete();
                progress.counts(
                    "critic_update",
                    serde_json::json!({
                    "completed_updates":critic_updates,"total_updates":critic_total_updates,
                    "value_optimizer_step":critic_checkpoint.optimizer_step}),
                );
            }
        }
    }
    if update.actor_optimizer_step == actor_age || critic_updates == 0 {
        return Err("group completed without accepted actor/value updates".into());
    }
    phase.complete();
    let update_seconds = update_started.elapsed().as_secs_f64();
    let phase = progress.begin("candidate_export");
    let trained = trainer.export_candidate(TrainingStageManifest::new(
        1,
        1,
        u16::try_from(policy_version.checked_add(1).ok_or("stage overflow")?)?,
    ))?;
    std::fs::write(
        output.join("trained.alife-foundation"),
        trained.encode_canonical()?,
    )?;
    phase.complete();
    let phase = progress.begin("next_world_and_staging_create");
    let mut next_config = alife_world::CanonicalNewGameConfig::phase3(request.seed, 1)?;
    next_config.brain_class = BrainScaleTier::Standard2048;
    next_config.founder_seed_base = founder_seed_base;
    next_config.sensor_profile = trained.manifest().sensor_profile();
    let mut next_game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&next_config, &trained)?;
    crate::foundation_training::ground_foundation_training_world(&mut next_game.world)?;
    let next_staging = runtime.new_staging_like_live()?;
    phase.complete();
    let phase = progress.begin("next_runtime_create");
    let mut next = GpuLiveBrainRuntime::new_profiled_foundation_training(
        next_staging,
        next_game.world,
        request.seed,
        BrainScaleTier::Standard2048,
        next_config.sensor_profile,
        alife_archive::LineageLibraryConfig::profile_default(output.join("next-lineage")),
        next_cohort_run_id(request.seed),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        cycle_sampling_config(
            request.seed,
            policy_version
                .checked_add(1)
                .ok_or("next policy overflow")?,
            request.sampling_temperature,
        )?,
    )?;
    phase.complete();
    let phase = progress.begin("next_runtime_tick_and_capture_check");
    next.tick()?;
    let next_steps = next.take_foundation_training_steps();
    if next_steps.len() != 1 {
        return Err("exported group policy did not produce native next-cohort decision".into());
    }
    phase.complete();
    let phase = progress.begin("next_cohort_optimizer_rebind");
    trainer.rebind_for_next_cohort(
        next_steps[0].before.phenotype.as_ref().clone(),
        trained.clone(),
    )?;
    phase.complete();
    let phase = progress.begin("updated_checkpoints_write");
    let actor_bytes = serde_json::to_vec(&trainer.checkpoint()?)?;
    let value_bytes = serde_json::to_vec(&critic_checkpoint)?;
    std::fs::write(output.join("actor-checkpoint.json"), &actor_bytes)?;
    std::fs::write(output.join("value-checkpoint.json"), &value_bytes)?;
    phase.complete();
    let phase = progress.begin("final_receipt_write");
    let decisions = batches.iter().map(PpoBatch::len).sum::<usize>();
    let (actor_admitted_rows, actor_row_exposures) = admitted_actor_rows(
        &all_spans,
        update.actor_optimizer_step - actor_age,
        life_vec.len(),
    );
    let admitted_decisions = actor_admitted_rows.iter().sum::<usize>();

    let successes = life_vec.iter().filter(|l| !l.successes.is_empty()).count();
    let episodes=life_vec.iter().zip(&batches).zip(&actor_admitted_rows).map(|((life,batch),admitted)|serde_json::json!({"organism_id":life.id.raw(),"world_seed":request.seed,"policy_version":policy_version,
        "captured_rows":life.references.len(),"available_on_policy_decisions":batch.len(),"valid_learning_decisions":admitted,"terminal_death_tick":life.death.map(|v|v.0),"initial_energy":life.initial_energy,
        "minimum_energy":life.minimum_energy,"last_captured_outcome_energy":life.last_captured_energy,
        "initial_sensory_sha256":life.initial_sensory_sha256,
        "success_definition":"sealed_world_consumption","successes":life.successes.iter().map(|event|{let mut e=event.clone();e["training_loss_row"]=serde_json::json!(event["row"].as_u64().is_some_and(|r|r<(batch.len() as u64)));e}).collect::<Vec<_>>(),"replay_manifest":life.directory.join("manifest.json"),
        "disclosed_retry":false,"seed_admission":provenance["seed_admission"]})).collect::<Vec<_>>();
    let elapsed = started.elapsed().as_secs_f64();
    let receipt = serde_json::json!({"schema":1,"complete":true,"mode":"group50_shared_habitat","population":50,"world_seed":request.seed,"seed":request.seed,
        "founder_seed_base":founder_seed_base,"policy_version":policy_version,"biological_objective_version":BIOLOGICAL_OBJECTIVE_VERSION,"critic_objective_version":critic_residual::OBJECTIVE_VERSION,
        "objective_state_reset":false,"sampling_temperature":request.sampling_temperature,"source":source,"seed_provenance":provenance,"episodes":episodes,
        "new_asset_digest":digest(&trained),"actor_checkpoint_sha256":admission::hash(&actor_bytes),"value_checkpoint_sha256":admission::hash(&value_bytes),
        "actor_optimizer_step":update.actor_optimizer_step,"value_optimizer_step":critic_checkpoint.optimizer_step,"actor_optimizer_updates":update.actor_optimizer_step-actor_age,
        "value_optimizer_updates":critic_updates,"next_cohort_optimizer_rebound":true,"completed_epochs":update.completed_epochs,"stopped_for_kl":update.stopped_for_kl,
        "valid_learning_decisions":admitted_decisions,"available_on_policy_decisions":decisions,"actor_loss_row_exposures":actor_row_exposures,
        "learning_counter_basis":"committed_actor_optimizer_steps_and_deterministic_window_schedule; unique_rows_separate_from_epoch_exposures","collection_seconds":collection_seconds,"update_seconds":update_seconds,"total_seconds":elapsed,
        "valid_learning_decisions_per_collection_second":admitted_decisions as f64/collection_seconds,"valid_learning_decisions_per_total_second":admitted_decisions as f64/elapsed,
        "successful_episodes":successes,
        "successful_training_row_episodes":life_vec.iter().zip(&batches).filter(|(life,batch)|life.successes.iter().any(|s|s["row"].as_u64().is_some_and(|r|r<(batch.len() as u64)))).count(),
        "distinct_initial_sensory_states":life_vec.iter().filter_map(|l|l.initial_sensory_sha256.as_ref()).collect::<std::collections::BTreeSet<_>>().len(),
        "distinct_successful_initial_sensory_states":life_vec.iter().filter(|l|!l.successes.is_empty()).filter_map(|l|l.initial_sensory_sha256.as_ref()).collect::<std::collections::BTreeSet<_>>().len(),"successful_organism_ids":life_vec.iter().filter(|l|!l.successes.is_empty()).map(|l|l.id.raw()).collect::<Vec<_>>(),
        "time_to_first_genuine_success_seconds":life_vec.iter().flat_map(|l|&l.successes).filter_map(|v|v["elapsed_seconds"].as_f64()).reduce(f64::min),
        "success_diversity_scope":"distinct_organisms_and_initial_local_scenes_in_one_shared_world; correlated_outcomes_possible",
        "retention_evidence":"unknown_requires_trainer_removed_followup","neural_work_captured":neural_work,"max_collector_batch_rows":max_batch_rows,"GPU_batch_path":"staged_tick/tick_memory_batch_training/class_arena_hardware_rows","collector_batch_row_histogram":batch_histogram,"no_progress_poll_count":no_progress_poll_count,
        "runtime_performance":runtime_metrics,"hardware":runtime.hardware_receipt(),"collector_admission":runtime.admission_receipt(),"replay_windows":all_spans.len(),"effective_gradient_batch_windows":50,
        "active_BPTT_graphs":1,"transient_packed_buffer_sets_bound":2,"peak_replay_device_bound_bytes":peak_replay_bytes,"replay_device_limit_bytes":request.replay_device_limit_bytes,"adapter_storage_binding_limit_bytes":limits.binding,"replay_host_limit_bytes":request.replay_host_limit_bytes,
        "verified_replay_boundaries":verified_replay_boundaries,"replay_numerical_tolerance":"abs<=1e-4+1e-4*abs(expected)",
        "GPU_qualification":"requires_real_adapter_run; no_CPU_receipt_substitution"});
    std::fs::write(
        output.join("runtime-performance.json"),
        serde_json::to_vec_pretty(&runtime_metrics)?,
    )?;
    std::fs::write(
        output.join("group50.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    phase.complete();
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn committed_learning_counts_stop_at_accepted_batches_and_separate_repeated_epochs() {
        let plan = (0..53)
            .map(|i| Span {
                life: i % 2,
                burn: 0,
                start: i * 256,
                end: i * 256 + i + 1,
            })
            .collect::<Vec<_>>();
        let (none, exposures) = admitted_actor_rows(&plan, 0, 2);
        assert_eq!((none, exposures), (vec![0, 0], 0));
        let (first, one) = admitted_actor_rows(&plan, 1, 2);
        assert_eq!(first.iter().sum::<usize>(), 1275);
        assert_eq!(one, 1275);
        let (all, two) = admitted_actor_rows(&plan, 2, 2);
        assert_eq!(all.iter().sum::<usize>(), 1431);
        assert_eq!(two, 1431);
        let (same, three) = admitted_actor_rows(&plan, 3, 2);
        assert_eq!(same, all);
        assert_eq!(three, 1431 + 1275);
    }
    #[test]
    fn completed_live_trajectory_stays_truncated_while_other_lives_continue() {
        let requested = 16;
        let completed_rows = 17;
        assert!(truncated(completed_rows, requested));
        // A later death cannot enter the death-observation branch or turn its
        // extra captured real-next row into a learning/terminal row.
        let later_death_seen = true;
        let terminated = later_death_seen && !truncated(completed_rows, requested);
        assert!(!terminated);
        assert_eq!(completed_rows - usize::from(!terminated), requested);
        assert!(!truncated(8, requested));
    }
    #[test]
    fn replay_auxiliary_bound_covers_n2048_objective_checkpoint_and_critic() {
        let (n, s) = (2048u64, 400_000u64);
        let ppo = 128 + 2048 * (64 + 45 + 8) * 4 + (n + 1) * 20 * 2;
        let checkpoint_and_accumulation = s * (16 + 4);
        let critic = 257 * n * 4 + (n + 1) * 16 + 256 * 32 + 64;
        assert!(replay_auxiliary_bound_bytes(n, s) >= ppo + checkpoint_and_accumulation + critic);
    }
    #[test]
    fn recurrent_spans_never_cross_organisms_or_sleep_boundaries() {
        let refs = (0..1000)
            .map(|i| FoundationReplayRecordRef {
                record_index: i,
                segment: u64::from(i >= 600),
                tick: i + 1,
                encoded_bytes: 1,
                digest: [0; 32],
            })
            .collect::<Vec<_>>();
        let plan = spans(17, &refs, 999, 256);
        assert_eq!(plan.iter().map(|s| s.end - s.start).sum::<usize>(), 999);
        assert!(plan.iter().all(|s| s.life == 17
            && s.start - s.burn <= 128
            && s.end - s.start <= 256
            && refs[s.burn].segment == refs[s.end - 1].segment));
        assert!(plan.iter().any(|s| s.start == 600 && s.burn == 600));
    }
    #[test]
    fn fifty_canonical_newborns_have_private_identity_and_one_policy() {
        let (asset, founder_seed) = checkpoint_tests::accepted_asset();
        let mut game =
            alife_world::create_canonical_foundation_training_group(943111, founder_seed, &asset)
                .unwrap();
        assert_eq!(game.creatures.len(), 50);
        game.world.validate_organism_bindings().unwrap();
        let records = game.world.organism_registry().iter().collect::<Vec<_>>();
        assert_eq!(records.len(), 50);
        let first = records[0].phenotype().brain_genome.clone();
        for record in &records {
            assert_eq!(record.phenotype().brain_genome, first);
            assert_eq!(record.biochemistry().tick, alife_core::Tick::ZERO);
        }
        let ids = records
            .iter()
            .map(|r| r.organism_id().raw())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), 50);
        assert!(alife_world::CanonicalNewGameConfig::phase3(943111, 50).is_err());
        crate::foundation_training::ground_foundation_training_world(&mut game.world).unwrap();
        let positions = game
            .receipt
            .founders
            .iter()
            .map(|f| game.world.entity(f.world_entity_id).unwrap().position)
            .collect::<Vec<_>>();
        assert_eq!(
            positions
                .iter()
                .map(|p| (p.x.to_bits(), p.z.to_bits()))
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            50
        );
        for (i, p) in positions.iter().enumerate() {
            for q in &positions[i + 1..] {
                assert!((p.x - q.x).hypot(p.z - q.z) >= 24.0);
            }
            let terrain = game.world.terrain().unwrap();
            assert!(terrain.walkable(p.x, p.z));
            assert!(terrain
                .resolve_move(*p, alife_core::Vec3f::new(p.x + 10.0, 0.0, p.z))
                .is_none());
            let food = game
                .world
                .entity(
                    game.world
                        .entity_id(&format!("group-food-{}", i + 1))
                        .unwrap(),
                )
                .unwrap();
            assert!((food.position.x - p.x).hypot(food.position.z - p.z) < 3.0);
        }
    }
}
