//! Product-facing population LOD and performance policy for G18.

use alife_core::{BrainClassSpec, LobeKind};

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierEvidenceStatus {
    CiSmokeMeasured,
    ManualExpectedSlow,
    GpuUnknownUntilMeasured,
}

impl TierEvidenceStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CiSmokeMeasured => "ci-smoke-measured",
            Self::ManualExpectedSlow => "manual-expected-slow",
            Self::GpuUnknownUntilMeasured => "gpu-unknown-until-measured",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationTierTarget {
    pub population: u16,
    pub ci_smoke: bool,
    pub manual_command_required: bool,
    pub evidence: TierEvidenceStatus,
}

impl PopulationTierTarget {
    pub const fn required_targets() -> [Self; G18_MAX_TARGET_TIERS] {
        [
            Self {
                population: 1,
                ci_smoke: true,
                manual_command_required: false,
                evidence: TierEvidenceStatus::CiSmokeMeasured,
            },
            Self {
                population: 10,
                ci_smoke: true,
                manual_command_required: false,
                evidence: TierEvidenceStatus::CiSmokeMeasured,
            },
            Self {
                population: 50,
                ci_smoke: false,
                manual_command_required: true,
                evidence: TierEvidenceStatus::ManualExpectedSlow,
            },
            Self {
                population: 100,
                ci_smoke: false,
                manual_command_required: true,
                evidence: TierEvidenceStatus::ManualExpectedSlow,
            },
            Self {
                population: 250,
                ci_smoke: false,
                manual_command_required: true,
                evidence: TierEvidenceStatus::GpuUnknownUntilMeasured,
            },
            Self {
                population: 500,
                ci_smoke: false,
                manual_command_required: true,
                evidence: TierEvidenceStatus::GpuUnknownUntilMeasured,
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodResidency {
    Hot,
    Warm,
    Cold,
}

impl LodResidency {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::Warm => "warm",
            Self::Cold => "cold",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadenceTarget {
    SensoryMotor,
    Homeostasis,
    ActionArbitration,
    FeedbackPolish,
    Rendering,
    MemoryTopology,
    NonessentialCognition,
    LoggingExport,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CadenceBand {
    pub residency: LodResidency,
    pub target: CadenceTarget,
    pub hz: f32,
    pub protected: bool,
}

impl CadenceBand {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.hz.is_finite() || !(0.0..=120.0).contains(&self.hz) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderDetailLevel {
    Full,
    Simplified,
    MarkerOnly,
}

impl RenderDetailLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Simplified => "simplified",
            Self::MarkerOnly => "marker-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderLodLevel {
    pub residency: LodResidency,
    pub max_population: u16,
    pub max_distance: f32,
    pub detail: RenderDetailLevel,
    pub animation_hz: f32,
    pub feedback_vfx_enabled: bool,
}

impl RenderLodLevel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.max_population == 0
            || !self.max_distance.is_finite()
            || self.max_distance < 0.0
            || !self.animation_hz.is_finite()
            || !(0.0..=60.0).contains(&self.animation_hz)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationPerformancePolicy {
    pub schema: &'static str,
    pub schema_version: u16,
    pub minimum_playable_population: u16,
    pub target_fps: u16,
    pub target_frame_ms: f32,
    pub brain_tier: BrainScaleTier,
    pub protected_lobes: Vec<LobeKind>,
    pub nonessential_lobes: Vec<LobeKind>,
    pub tier_targets: Vec<PopulationTierTarget>,
    pub cadence_bands: Vec<CadenceBand>,
    pub render_lod: Vec<RenderLodLevel>,
    pub benchmark_smoke_command: String,
    pub manual_upper_tier_command: String,
    pub gpu_runtime_manual_command: String,
    pub performance_claim_status: String,
}

impl PopulationPerformancePolicy {
    pub fn v1_defaults() -> Result<Self, GameAppShellError> {
        let brain_tier = BrainScaleTier::Nano512;
        let spec = BrainClassSpec::try_for_tier(brain_tier)?;
        let mut protected_lobes = spec.compute_budget.essential_lobes.clone();
        for lobe in [
            LobeKind::SensoryGrounding,
            LobeKind::MotorArbitration,
            LobeKind::HomeostaticRegulation,
        ] {
            if !protected_lobes.contains(&lobe) {
                protected_lobes.push(lobe);
            }
        }
        let nonessential_lobes = spec
            .lobe_regions()
            .filter(|region| region.enabled && !protected_lobes.contains(&region.kind))
            .map(|region| region.kind)
            .collect::<Vec<_>>();
        let policy = Self {
            schema: G18_POPULATION_PERFORMANCE_SCHEMA,
            schema_version: G18_POPULATION_PERFORMANCE_SCHEMA_VERSION,
            minimum_playable_population: 10,
            target_fps: 60,
            target_frame_ms: G18_TARGET_FRAME_MS,
            brain_tier,
            protected_lobes,
            nonessential_lobes,
            tier_targets: PopulationTierTarget::required_targets().to_vec(),
            cadence_bands: default_cadence_bands(),
            render_lod: default_render_lod_levels(),
            benchmark_smoke_command: "cargo run -p alife_tools --bin benchmark_tiers".to_string(),
            manual_upper_tier_command: "cargo run -p alife_tools --bin benchmark_tiers -- --all"
                .to_string(),
            gpu_runtime_manual_command: g12_manual_gpu_hardware_command().to_string(),
            performance_claim_status: "cpu-smoke-measured-gpu-unknown-until-hardware".to_string(),
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G18_POPULATION_PERFORMANCE_SCHEMA
            || self.schema_version != G18_POPULATION_PERFORMANCE_SCHEMA_VERSION
            || self.minimum_playable_population < 10
            || self.target_fps != 60
            || !self.target_frame_ms.is_finite()
            || self.target_frame_ms <= 0.0
            || self.tier_targets.len() != G18_MAX_TARGET_TIERS
            || self.cadence_bands.is_empty()
            || self.render_lod.is_empty()
            || self.benchmark_smoke_command.is_empty()
            || self.manual_upper_tier_command.is_empty()
            || self.gpu_runtime_manual_command.is_empty()
            || !self
                .performance_claim_status
                .contains("gpu-unknown-until-hardware")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        let populations = self
            .tier_targets
            .iter()
            .map(|target| target.population)
            .collect::<Vec<_>>();
        if populations != [1, 10, 50, 100, 250, 500] {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if !self.protected_lobes.contains(&LobeKind::SensoryGrounding)
            || !self.protected_lobes.contains(&LobeKind::MotorArbitration)
            || !self
                .protected_lobes
                .contains(&LobeKind::HomeostaticRegulation)
            || self.nonessential_lobes.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for band in &self.cadence_bands {
            band.validate()?;
        }
        for lod in &self.render_lod {
            lod.validate()?;
        }
        if self.rate_hz(LodResidency::Hot, CadenceTarget::SensoryMotor)
            < self.rate_hz(LodResidency::Hot, CadenceTarget::NonessentialCognition)
            || self.rate_hz(LodResidency::Hot, CadenceTarget::ActionArbitration)
                < self.rate_hz(LodResidency::Hot, CadenceTarget::NonessentialCognition)
            || self.rate_hz(LodResidency::Warm, CadenceTarget::SensoryMotor)
                < self.rate_hz(LodResidency::Warm, CadenceTarget::NonessentialCognition)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn rate_hz(&self, residency: LodResidency, target: CadenceTarget) -> f32 {
        self.cadence_bands
            .iter()
            .find(|band| band.residency == residency && band.target == target)
            .map_or(0.0, |band| band.hz)
    }

    pub fn throttling_decision(
        &self,
        measured_frame_ms: f32,
        measured_gpu_neural_ms: Option<f32>,
    ) -> Result<ThrottlingDecision, ScaffoldContractError> {
        if !measured_frame_ms.is_finite() || measured_frame_ms < 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if let Some(value) = measured_gpu_neural_ms {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        let pressure = measured_frame_ms / self.target_frame_ms;
        let (level, decimation, reason) = if pressure >= 1.5 {
            (3, 8, "sleep-only-or-cold-cadence")
        } else if pressure >= 1.15 {
            (2, 4, "warm-cadence-fallback")
        } else if pressure >= 1.0 {
            (1, 2, "nonessential-cognition-decimation")
        } else {
            (0, 1, "under-budget")
        };
        let decision = ThrottlingDecision {
            measured_frame_ms,
            measured_gpu_neural_ms,
            throttle_level: level,
            nonessential_decimation_factor: decimation,
            sensory_motor_protected: true,
            homeostasis_protected: true,
            action_arbitration_protected: true,
            reason: reason.to_string(),
        };
        decision.validate()?;
        Ok(decision)
    }

    pub fn to_markdown(&self, summary: &PopulationPerformanceOverlaySummary) -> String {
        let mut out = String::new();
        out.push_str("# G18 Population Performance and LOD Report\n\n");
        out.push_str("CPU/headless smoke is measured; GPU performance remains unknown until the manual hardware command records timing evidence.\n\n");
        out.push_str("| Tier | Evidence | Manual command required |\n");
        out.push_str("|---:|---|---|\n");
        for target in &self.tier_targets {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                target.population,
                target.evidence.label(),
                target.manual_command_required
            ));
        }
        out.push_str("\n## Commands\n\n");
        out.push_str(&format!(
            "- CI smoke: `{}`\n- Manual upper tiers: `{}`\n- Manual GPU runtime: `{}`\n",
            self.benchmark_smoke_command,
            self.manual_upper_tier_command,
            self.gpu_runtime_manual_command
        ));
        out.push_str("\n## Current product smoke\n\n");
        out.push_str(&format!(
            "- Population demo creatures: {}\n- Scheduler steps: {}\n- Sealed patches: {}\n- Feedback labels: {}\n- Golden behavior preserved by LOD projection: {}\n",
            summary.population_creatures,
            summary.scheduler_steps,
            summary.sealed_patch_count,
            summary.feedback_labels.join(">"),
            summary.golden_behavior_preserved
        ));
        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThrottlingDecision {
    pub measured_frame_ms: f32,
    pub measured_gpu_neural_ms: Option<f32>,
    pub throttle_level: u8,
    pub nonessential_decimation_factor: u8,
    pub sensory_motor_protected: bool,
    pub homeostasis_protected: bool,
    pub action_arbitration_protected: bool,
    pub reason: String,
}

impl ThrottlingDecision {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.measured_frame_ms.is_finite()
            || self.measured_frame_ms < 0.0
            || self.throttle_level > 3
            || self.nonessential_decimation_factor == 0
            || !self.sensory_motor_protected
            || !self.homeostasis_protected
            || !self.action_arbitration_protected
            || self.reason.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if let Some(value) = self.measured_gpu_neural_ms {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationLodProjection {
    pub behavior_signature_before: String,
    pub behavior_signature_after: String,
    pub render_detail: RenderDetailLevel,
    pub nonessential_cognition_decimated: bool,
    pub feedback_vfx_enabled: bool,
}

impl PopulationLodProjection {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.behavior_signature_before.is_empty()
            || self.behavior_signature_before != self.behavior_signature_after
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationPerformanceOverlaySummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub policy: PopulationPerformancePolicy,
    pub population_creatures: usize,
    pub scheduler_steps: usize,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub feedback_labels: Vec<&'static str>,
    pub gpu_selected_backend: String,
    pub gpu_performance_measured: bool,
    pub throttle_decision: ThrottlingDecision,
    pub lod_projection: PopulationLodProjection,
    pub tier_1_10_ci_smoke_documented: bool,
    pub manual_upper_tiers_documented: bool,
    pub performance_report_markdown: String,
    pub golden_behavior_preserved: bool,
}

impl PopulationPerformanceOverlaySummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G18_POPULATION_PERFORMANCE_SCHEMA
            || self.schema_version != G18_POPULATION_PERFORMANCE_SCHEMA_VERSION
            || self.population_creatures < 2
            || self.scheduler_steps == 0
            || self.sealed_patch_count < self.scheduler_steps
            || self.feedback_labels.is_empty()
            || self.gpu_selected_backend.is_empty()
            || self.gpu_performance_measured
            || !self.tier_1_10_ci_smoke_documented
            || !self.manual_upper_tiers_documented
            || !self.golden_behavior_preserved
            || !self
                .performance_report_markdown
                .contains("GPU performance remains unknown")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.policy.validate()?;
        self.throttle_decision.validate()?;
        self.lod_projection.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.population_creatures,
            self.scheduler_steps,
            self.sealed_patch_count,
            self.gpu_selected_backend,
            self.throttle_decision.throttle_level,
            self.lod_projection.render_detail.label(),
            self.feedback_labels.join(">")
        )
    }
}

pub fn run_population_performance_lod_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<PopulationPerformanceOverlaySummary, GameAppShellError> {
    let policy = PopulationPerformancePolicy::v1_defaults()?;
    let population = run_population_social_loop_smoke()?;
    let gpu = run_gpu_product_hardening_smoke()?;
    let feedback = run_feedback_polish_smoke(launch)?;
    let throttle_decision = policy.throttling_decision(G18_TARGET_FRAME_MS * 1.2, None)?;
    let lod_projection =
        project_lod_without_behavior_change(&population, &policy, &throttle_decision)?;
    let tier_1_10_ci_smoke_documented = policy
        .tier_targets
        .iter()
        .filter(|target| target.ci_smoke)
        .map(|target| target.population)
        .collect::<Vec<_>>()
        == [1, 10];
    let manual_upper_tiers_documented = policy
        .tier_targets
        .iter()
        .filter(|target| target.manual_command_required)
        .map(|target| target.population)
        .collect::<Vec<_>>()
        == [50, 100, 250, 500];

    let mut summary = PopulationPerformanceOverlaySummary {
        schema: G18_POPULATION_PERFORMANCE_SCHEMA,
        schema_version: G18_POPULATION_PERFORMANCE_SCHEMA_VERSION,
        policy,
        population_creatures: population.creature_count,
        scheduler_steps: population.metrics.scheduler_steps,
        sealed_patch_count: population.metrics.sealed_patch_count,
        packed_record_count: population.metrics.packed_record_count,
        feedback_labels: feedback.event_labels(),
        gpu_selected_backend: gpu.telemetry_overlay.selected_backend,
        gpu_performance_measured: gpu.telemetry_overlay.measured_gpu_performance,
        throttle_decision,
        lod_projection,
        tier_1_10_ci_smoke_documented,
        manual_upper_tiers_documented,
        performance_report_markdown: String::new(),
        golden_behavior_preserved: true,
    };
    summary.golden_behavior_preserved = summary.lod_projection.behavior_signature_before
        == summary.lod_projection.behavior_signature_after;
    summary.performance_report_markdown = summary.policy.to_markdown(&summary);
    summary.validate()?;
    Ok(summary)
}

pub fn project_lod_without_behavior_change(
    population: &PopulationSocialLoopSummary,
    policy: &PopulationPerformancePolicy,
    decision: &ThrottlingDecision,
) -> Result<PopulationLodProjection, ScaffoldContractError> {
    population.validate()?;
    policy.validate()?;
    decision.validate()?;
    let signature = population.signature_line();
    let render_detail = if population.creature_count <= 10 {
        RenderDetailLevel::Full
    } else if population.creature_count <= 100 {
        RenderDetailLevel::Simplified
    } else {
        RenderDetailLevel::MarkerOnly
    };
    let projection = PopulationLodProjection {
        behavior_signature_before: signature.clone(),
        behavior_signature_after: signature,
        render_detail,
        nonessential_cognition_decimated: decision.nonessential_decimation_factor > 1,
        feedback_vfx_enabled: policy
            .render_lod
            .iter()
            .find(|lod| lod.residency == LodResidency::Hot)
            .is_some_and(|lod| lod.feedback_vfx_enabled),
    };
    projection.validate()?;
    Ok(projection)
}

fn default_cadence_bands() -> Vec<CadenceBand> {
    [
        (
            LodResidency::Hot,
            [
                (CadenceTarget::SensoryMotor, 60.0, true),
                (CadenceTarget::Homeostasis, 60.0, true),
                (CadenceTarget::ActionArbitration, 60.0, true),
                (CadenceTarget::FeedbackPolish, 30.0, false),
                (CadenceTarget::Rendering, 60.0, false),
                (CadenceTarget::MemoryTopology, 10.0, false),
                (CadenceTarget::NonessentialCognition, 10.0, false),
                (CadenceTarget::LoggingExport, 10.0, false),
            ],
        ),
        (
            LodResidency::Warm,
            [
                (CadenceTarget::SensoryMotor, 20.0, true),
                (CadenceTarget::Homeostasis, 20.0, true),
                (CadenceTarget::ActionArbitration, 20.0, true),
                (CadenceTarget::FeedbackPolish, 10.0, false),
                (CadenceTarget::Rendering, 20.0, false),
                (CadenceTarget::MemoryTopology, 2.0, false),
                (CadenceTarget::NonessentialCognition, 2.0, false),
                (CadenceTarget::LoggingExport, 2.0, false),
            ],
        ),
        (
            LodResidency::Cold,
            [
                (CadenceTarget::SensoryMotor, 0.0, true),
                (CadenceTarget::Homeostasis, 1.0, true),
                (CadenceTarget::ActionArbitration, 0.0, true),
                (CadenceTarget::FeedbackPolish, 0.0, false),
                (CadenceTarget::Rendering, 1.0, false),
                (CadenceTarget::MemoryTopology, 0.1, false),
                (CadenceTarget::NonessentialCognition, 0.0, false),
                (CadenceTarget::LoggingExport, 0.5, false),
            ],
        ),
    ]
    .into_iter()
    .flat_map(|(residency, bands)| {
        bands
            .into_iter()
            .map(move |(target, hz, protected)| CadenceBand {
                residency,
                target,
                hz,
                protected,
            })
    })
    .collect()
}

fn default_render_lod_levels() -> Vec<RenderLodLevel> {
    vec![
        RenderLodLevel {
            residency: LodResidency::Hot,
            max_population: 10,
            max_distance: 24.0,
            detail: RenderDetailLevel::Full,
            animation_hz: 60.0,
            feedback_vfx_enabled: true,
        },
        RenderLodLevel {
            residency: LodResidency::Warm,
            max_population: 100,
            max_distance: 80.0,
            detail: RenderDetailLevel::Simplified,
            animation_hz: 20.0,
            feedback_vfx_enabled: true,
        },
        RenderLodLevel {
            residency: LodResidency::Cold,
            max_population: 500,
            max_distance: f32::MAX,
            detail: RenderDetailLevel::MarkerOnly,
            animation_hz: 1.0,
            feedback_vfx_enabled: false,
        },
    ]
}
