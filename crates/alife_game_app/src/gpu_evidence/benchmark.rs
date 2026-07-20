//! Production-runtime benchmark trials that expose only compact causal and GPU timing evidence.

use std::collections::BTreeSet;

use alife_core::{
    BrainCapacityClass, BrainScaleTier, BrainTickStatus, OrganismId, PhenotypeEvidenceManifest,
    ScaffoldContractError, SensorProfile, Tick, Vec3f,
};
use alife_gpu_backend::{GpuAdmissionReceipt, GpuClosedLoopBackend};
use alife_world::{GpuBackendProvenanceSave, HeadlessScenarioBuilder};
use thiserror::Error;

use crate::{
    compile_gpu_birth_components, gpu_checkpoint_assets::current_backend_provenance,
    GameAppShellError, GpuLiveBrainRuntime,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuClosedLoopBenchmarkTrialOptions {
    pub capacity: BrainCapacityClass,
    pub sensor_profile: SensorProfile,
    pub population: u32,
    pub fixture_seed: u64,
    pub warmup_ticks: u32,
    pub measured_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuClosedLoopBenchmarkTrial {
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub backend_provenance: GpuBackendProvenanceSave,
    pub admission: GpuAdmissionReceipt,
    pub runtime_profile_digest: [u64; 4],
    pub activity_policy_digest: [u64; 4],
    pub timestamp_period_ns_q24: u64,
    pub raw_inference_timestamp_ticks: Vec<u64>,
    pub raw_plasticity_timestamp_ticks: Vec<u64>,
    pub gpu_selections: u64,
    pub executed_actions: u64,
    pub sealed_patches: u64,
    pub learning_commits: u64,
    pub distinct_selected_families: u16,
    pub active_synapses: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuClosedLoopBenchmarkPhenotypeFixture {
    pub manifest: PhenotypeEvidenceManifest,
    pub active_synapses: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuClosedLoopBenchmarkUnavailable {
    pub reason_code: u16,
    pub adapter: Option<GpuBackendProvenanceSave>,
    pub admission: Option<GpuAdmissionReceipt>,
}

#[derive(Debug, Error)]
pub enum GpuClosedLoopBenchmarkTrialError {
    #[error("GPU benchmark row is unavailable with reason {0:?}")]
    Unavailable(Box<GpuClosedLoopBenchmarkUnavailable>),
    #[error("GPU benchmark {phase} tick {tick} failed: {source}")]
    Tick {
        phase: &'static str,
        tick: u32,
        #[source]
        source: GameAppShellError,
    },
    #[error("GPU benchmark causal invariant failed: {0}")]
    CausalInvariant(String),
    #[error(transparent)]
    Fatal(#[from] GameAppShellError),
}

impl From<ScaffoldContractError> for GpuClosedLoopBenchmarkTrialError {
    fn from(error: ScaffoldContractError) -> Self {
        Self::Fatal(GameAppShellError::Core(error))
    }
}

impl GpuClosedLoopBenchmarkTrialOptions {
    fn validate(self) -> Result<Self, GameAppShellError> {
        self.capacity.validate_contract()?;
        if self.population == 0
            || self.population > 500
            || self.fixture_seed == 0
            || self.warmup_ticks == 0
            || self.measured_ticks == 0
            || self.measured_ticks > 1_024
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame.into());
        }
        Ok(self)
    }
}

pub fn run_gpu_closed_loop_benchmark_trial(
    options: GpuClosedLoopBenchmarkTrialOptions,
) -> Result<GpuClosedLoopBenchmarkTrial, GpuClosedLoopBenchmarkTrialError> {
    let options = options.validate()?;
    let tier = tier_for_capacity(options.capacity)?;
    let world = benchmark_world(options.fixture_seed, options.population)?;
    let backend = match GpuClosedLoopBackend::new_required(Default::default()) {
        Ok(backend) => backend,
        Err(ScaffoldContractError::GpuTimestampQueryUnavailable) => {
            return Err(GpuClosedLoopBenchmarkTrialError::Unavailable(Box::new(
                GpuClosedLoopBenchmarkUnavailable {
                    reason_code: 2,
                    adapter: None,
                    admission: None,
                },
            )));
        }
        Err(ScaffoldContractError::NeuralBackendUnavailable) => {
            return Err(GpuClosedLoopBenchmarkTrialError::Unavailable(Box::new(
                GpuClosedLoopBenchmarkUnavailable {
                    reason_code: 1,
                    adapter: None,
                    admission: None,
                },
            )));
        }
        Err(error) => return Err(GameAppShellError::Core(error).into()),
    };
    let backend_provenance = current_backend_provenance(&backend, &options.capacity)?;
    let mut runtime = match GpuLiveBrainRuntime::new_benchmark_profiled(
        backend,
        world,
        options.fixture_seed,
        tier,
        options.sensor_profile,
    ) {
        Ok(runtime) => runtime,
        Err(GameAppShellError::Core(ScaffoldContractError::NeuralBackendUnavailable)) => {
            return Err(GpuClosedLoopBenchmarkTrialError::Unavailable(Box::new(
                GpuClosedLoopBenchmarkUnavailable {
                    reason_code: 3,
                    adapter: Some(backend_provenance),
                    admission: None,
                },
            )));
        }
        Err(error) => return Err(error.into()),
    };
    let phenotype_manifest = compile_gpu_closed_loop_benchmark_phenotype(options)?.manifest;
    let runtime_profile_digest = runtime.runtime_profile_digest()?;
    let activity_policy_digest = runtime.activity_policy_digest();

    for tick in 0..options.warmup_ticks {
        let summaries =
            runtime
                .tick()
                .map_err(|source| GpuClosedLoopBenchmarkTrialError::Tick {
                    phase: "warmup",
                    tick,
                    source,
                })?;
        validate_complete_awake_tick(&runtime, &summaries, options.population, "warmup", tick)?;
        runtime
            .take_completed_neural_timing_sample()
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?;
    }
    runtime.prepare_measured_benchmark_phase()?;

    let start_patches = runtime.sealed_patch_count();
    let mut raw_inference_timestamp_ticks = Vec::with_capacity(options.measured_ticks as usize);
    let mut raw_plasticity_timestamp_ticks = Vec::with_capacity(options.measured_ticks as usize);
    let mut gpu_selections = 0_u64;
    let mut executed_actions = 0_u64;
    let mut learning_commits = 0_u64;
    let mut selected_families = BTreeSet::new();
    let mut active_synapses = 0_u32;
    let mut timestamp_period_ns_q24 = None;
    for tick in 0..options.measured_ticks {
        if tick == options.measured_ticks / 2 {
            runtime.enter_isolated_benchmark_phase()?;
        }
        let before_patches = runtime.sealed_patch_count();
        let summaries =
            runtime
                .tick()
                .map_err(|source| GpuClosedLoopBenchmarkTrialError::Tick {
                    phase: "measured",
                    tick,
                    source,
                })?;
        validate_complete_awake_tick(&runtime, &summaries, options.population, "measured", tick)?;
        let timing = runtime
            .take_completed_neural_timing_sample()
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?;
        if timing.class_id_raw != options.capacity.id().raw()
            || timing.population != options.population
            || timing.inference_timestamp_ticks == 0
            || timing.plasticity_timestamp_ticks == 0
            || timing.timestamp_period_ns_q24 == 0
            || timestamp_period_ns_q24
                .is_some_and(|period| period != timing.timestamp_period_ns_q24)
        {
            return Err(ScaffoldContractError::GpuTimestampQueryUnavailable.into());
        }
        timestamp_period_ns_q24 = Some(timing.timestamp_period_ns_q24);
        raw_inference_timestamp_ticks.push(timing.inference_timestamp_ticks);
        raw_plasticity_timestamp_ticks.push(timing.plasticity_timestamp_ticks);

        let after_patches = runtime.sealed_patch_count();
        let patch_delta = after_patches
            .checked_sub(before_patches)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        gpu_selections = gpu_selections
            .checked_add(summaries.len() as u64)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        executed_actions = executed_actions
            .checked_add(summaries.len() as u64)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        learning_commits = learning_commits
            .checked_add(runtime.last_learning_receipts().len() as u64)
            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
        if patch_delta != summaries.len() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let committed_patches = runtime.last_sealed_patches();
        if committed_patches.len() != patch_delta {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        for patch in committed_patches {
            selected_families.insert(patch.decision().neural_evidence()?.action_family.raw());
        }
        active_synapses = active_synapses.max(runtime.evidence_metrics().active_synapses);
    }
    let sealed_patches = runtime
        .sealed_patch_count()
        .checked_sub(start_patches)
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)? as u64;
    let admission = runtime.admission_receipt().clone();
    admission.validate_contract()?;
    let distinct_selected_families = u16::try_from(selected_families.len())
        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
    if selected_families.len() < 2 {
        return Err(GpuClosedLoopBenchmarkTrialError::CausalInvariant(format!(
            "selected action families {selected_families:?}; expected at least two"
        )));
    }
    Ok(GpuClosedLoopBenchmarkTrial {
        phenotype_manifest,
        backend_provenance,
        admission,
        runtime_profile_digest,
        activity_policy_digest,
        timestamp_period_ns_q24: timestamp_period_ns_q24
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?,
        raw_inference_timestamp_ticks,
        raw_plasticity_timestamp_ticks,
        gpu_selections,
        executed_actions,
        sealed_patches,
        learning_commits,
        distinct_selected_families,
        active_synapses,
    })
}

pub fn compile_gpu_closed_loop_benchmark_phenotype(
    options: GpuClosedLoopBenchmarkTrialOptions,
) -> Result<GpuClosedLoopBenchmarkPhenotypeFixture, GameAppShellError> {
    let options = options.validate()?;
    let tier = tier_for_capacity(options.capacity)?;
    let (phenotype, _, _) = compile_gpu_birth_components(
        options.fixture_seed,
        tier,
        OrganismId(1),
        Tick::ZERO,
        options.sensor_profile,
    )?;
    let active_synapses = u32::try_from(phenotype.synapses().len())
        .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
    Ok(GpuClosedLoopBenchmarkPhenotypeFixture {
        manifest: PhenotypeEvidenceManifest::from_learning_phenotype(
            &phenotype,
            &options.capacity,
        )?,
        active_synapses,
    })
}

fn validate_complete_awake_tick(
    runtime: &GpuLiveBrainRuntime,
    summaries: &[crate::LiveBrainTickSummary],
    population: u32,
    phase: &'static str,
    tick: u32,
) -> Result<(), GpuClosedLoopBenchmarkTrialError> {
    if summaries.len() != population as usize {
        return Err(GpuClosedLoopBenchmarkTrialError::CausalInvariant(format!(
            "{phase} tick {tick} returned {} summaries for population {population}",
            summaries.len()
        )));
    }
    if let Some(summary) = summaries.iter().find(|summary| {
        summary.status != BrainTickStatus::Normal
            || !summary.patch_sealed
            || summary.selected_action_id.is_none()
    }) {
        return Err(GpuClosedLoopBenchmarkTrialError::CausalInvariant(format!(
            "{phase} tick {tick} organism {} status {:?}, patch_sealed={}, selected={:?}, stages={:?}",
            summary.organism_id.raw(),
            summary.status,
            summary.patch_sealed,
            summary.selected_action_id,
            summary.causal_stages
        )));
    }
    if runtime.last_learning_receipts().len() != population as usize
        || !runtime.last_post_seal_learning_failures().is_empty()
        || !runtime.last_pre_seal_discard_failures().is_empty()
    {
        return Err(GpuClosedLoopBenchmarkTrialError::CausalInvariant(format!(
            "{phase} tick {tick} learning receipts={}, post-seal failures={}, pre-seal failures={}, population={population}",
            runtime.last_learning_receipts().len(),
            runtime.last_post_seal_learning_failures().len(),
            runtime.last_pre_seal_discard_failures().len()
        )));
    }
    Ok(())
}

fn tier_for_capacity(capacity: BrainCapacityClass) -> Result<BrainScaleTier, GameAppShellError> {
    match capacity.id() {
        BrainCapacityClass::N512_ID => Ok(BrainScaleTier::Nano512),
        BrainCapacityClass::N1024_ID => Ok(BrainScaleTier::Small1024),
        BrainCapacityClass::N2048_ID => Ok(BrainScaleTier::Standard2048),
        _ => Err(ScaffoldContractError::UnsupportedProductionBrainClass.into()),
    }
}

fn benchmark_world(
    seed: u64,
    population: u32,
) -> Result<alife_world::HeadlessWorld, GameAppShellError> {
    let mut builder = HeadlessScenarioBuilder::new(seed)
        .food("benchmark-food", Vec3f::new(1.5, 0.0, 0.0), 0.2)
        .obstacle("benchmark-obstacle", Vec3f::new(-1.5, 0.0, 0.0), 0.4);
    for index in 0..population {
        // One organism receives the fixed populated stimulus set while every
        // other brain is spatially isolated. This preserves real world
        // enumeration, execution, sealing, and learning for every organism,
        // while avoiding an accidental all-to-all social candidate fixture
        // whose O(population^2) host work is outside the measured GPU span.
        let position = if index == 0 {
            Vec3f::ZERO
        } else {
            Vec3f::new(index as f32 * 1_024.0, 0.0, 0.0)
        };
        builder = builder.agent(
            &format!("benchmark-agent-{index}"),
            OrganismId(u64::from(index) + 1),
            position,
        );
    }
    Ok(builder.build()?)
}
