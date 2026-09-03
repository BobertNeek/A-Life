//! Production Bevy shell for the canonical Vulkan voxel frontend.

#[cfg(feature = "gpu-runtime")]
use std::path::PathBuf;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

use alife_bevy_adapter::{AlifeBevyAdapterPlugin, BevyEntityMap};
use alife_core::{OrganismId, Tick, WorldEntityId};
#[cfg(feature = "gpu-runtime")]
use alife_world::persistence::PortableSaveFile;
use alife_world::presentation::{WorldOrganismPresentationRow, WorldPresentationSnapshot};
use alife_world::{HeadlessWorld, WorldObject, WorldObjectKind};
use bevy::{
    app::AppExit,
    asset::{AssetPlugin, Assets},
    ecs::schedule::IntoScheduleConfigs,
    prelude::{
        default, App, ButtonInput, ClearColor, Color, Commands, DefaultPlugins, Entity, KeyCode,
        Mesh, Message, MessageWriter, MinimalPlugins, MouseButton, NonSendMut, PluginGroup, Query,
        Res, ResMut, Resource, StandardMaterial, Time, Update,
    },
    render::{
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    window::{ExitCondition, PresentMode, Window, WindowPlugin, WindowTheme},
    winit::WinitSettings,
};

use crate::{
    AppShellLaunchConfig, GameAppShellError, LiveBrainTickSummary,
    LiveCognitivePresentationSnapshot, RuntimePlaybackState,
};
#[cfg(feature = "gpu-runtime")]
use crate::{ProductionVoxelLaunchConfig, ProductionVoxelLaunchSummary};

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct ProductionVoxelFrontendResource {
    pub summary: crate::ProductionVoxelLaunchSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveBrainPresentationFrame {
    pub tick_summaries: Vec<LiveBrainTickSummary>,
    pub authoritative_world_tick: Tick,
    world_objects_by_id: BTreeMap<u64, WorldObject>,
    organisms_by_world_id: BTreeMap<u64, WorldOrganismPresentationRow>,
    cognitive_by_organism_id: BTreeMap<u64, LiveCognitivePresentationSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBrainPresentationFrameError {
    DuplicateWorldEntityId(WorldEntityId),
}

impl LiveBrainPresentationFrame {
    pub fn try_new(
        tick_summaries: Vec<LiveBrainTickSummary>,
        authoritative_world_tick: Tick,
        world_objects: Vec<WorldObject>,
    ) -> Result<Self, LiveBrainPresentationFrameError> {
        let mut world_objects_by_id = BTreeMap::new();
        for object in world_objects {
            let stable_id = object.id;
            if world_objects_by_id
                .insert(stable_id.raw(), object)
                .is_some()
            {
                return Err(LiveBrainPresentationFrameError::DuplicateWorldEntityId(
                    stable_id,
                ));
            }
        }
        Ok(Self {
            tick_summaries,
            authoritative_world_tick,
            world_objects_by_id,
            organisms_by_world_id: BTreeMap::new(),
            cognitive_by_organism_id: BTreeMap::new(),
        })
    }

    pub fn from_authoritative_world(
        tick_summaries: Vec<LiveBrainTickSummary>,
        world: &HeadlessWorld,
    ) -> Result<Self, LiveBrainPresentationFrameError> {
        let snapshot = world.presentation_snapshot();
        let mut frame = Self::try_new(tick_summaries, world.tick(), world.object_snapshots())?;
        frame.install_organism_snapshot(snapshot);
        Ok(frame)
    }

    pub fn object(&self, stable_id: WorldEntityId) -> Option<&WorldObject> {
        self.world_objects_by_id.get(&stable_id.raw())
    }

    pub fn object_snapshots(&self) -> Vec<WorldObject> {
        self.world_objects_by_id.values().cloned().collect()
    }

    pub fn objects(&self) -> impl ExactSizeIterator<Item = &WorldObject> {
        self.world_objects_by_id.values()
    }

    pub fn object_count(&self) -> usize {
        self.world_objects_by_id.len()
    }

    pub fn organism(&self, stable_id: WorldEntityId) -> Option<&WorldOrganismPresentationRow> {
        self.organisms_by_world_id.get(&stable_id.raw())
    }

    pub fn organism_snapshots(&self) -> Vec<WorldOrganismPresentationRow> {
        self.organisms_by_world_id.values().cloned().collect()
    }

    pub fn organism_count(&self) -> usize {
        self.organisms_by_world_id.len()
    }

    pub fn cognitive_for_organism(
        &self,
        organism_id: OrganismId,
    ) -> Option<&LiveCognitivePresentationSnapshot> {
        self.cognitive_by_organism_id.get(&organism_id.raw())
    }

    fn install_organism_snapshot(&mut self, snapshot: WorldPresentationSnapshot) {
        let mut world_id_by_organism = BTreeMap::new();
        self.organisms_by_world_id = snapshot
            .organisms
            .into_iter()
            .map(|row| {
                world_id_by_organism.insert(row.organism_id.raw(), row.world_entity_id.raw());
                (row.world_entity_id.raw(), row)
            })
            .collect();
        for summary in &self.tick_summaries {
            let Some(world_id) = world_id_by_organism.get(&summary.organism_id.raw()) else {
                continue;
            };
            let Some(row) = self.organisms_by_world_id.get_mut(world_id) else {
                continue;
            };
            row.motor = Some(alife_world::PresentationMotorSnapshot {
                action_kind: summary.selected_action_kind.clone(),
                action_id: summary.selected_action_id.clone(),
                target_entity: summary.target_entity.clone(),
            });
            row.outcome = Some(alife_world::PresentationOutcomeSnapshot {
                patch_sealed: summary.patch_sealed,
                patch_sequence_id: summary.patch_sequence_id,
                patch_success: summary.patch_success,
                physical_contact: summary.physical_contact.clone(),
                action_failure: summary.action_failure.clone(),
            });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct LiveBrainPresentationFrameResource {
    pub previous: LiveBrainPresentationFrame,
    pub current: LiveBrainPresentationFrame,
}

impl LiveBrainPresentationFrameResource {
    pub fn from_authoritative_world(
        world: &HeadlessWorld,
    ) -> Result<Self, LiveBrainPresentationFrameError> {
        let baseline = LiveBrainPresentationFrame::from_authoritative_world(Vec::new(), world)?;
        Ok(Self {
            previous: baseline.clone(),
            current: baseline,
        })
    }

    pub fn from_current_frame(current: LiveBrainPresentationFrame) -> Self {
        Self {
            previous: current.clone(),
            current,
        }
    }

    pub fn reseed_from_loaded_world(
        &mut self,
        world: &HeadlessWorld,
    ) -> Result<(), LiveBrainPresentationFrameError> {
        let baseline = LiveBrainPresentationFrame::from_authoritative_world(Vec::new(), world)?;
        self.previous = baseline.clone();
        self.current = baseline;
        Ok(())
    }

    pub fn try_publish_successful_tick(
        &mut self,
        tick_summaries: Vec<LiveBrainTickSummary>,
        world: &HeadlessWorld,
    ) -> Result<(), LiveBrainPresentationFrameError> {
        self.try_publish_successful_tick_with_cognitive(tick_summaries, Vec::new(), world)
    }

    pub fn try_publish_successful_tick_with_cognitive(
        &mut self,
        tick_summaries: Vec<LiveBrainTickSummary>,
        cognitive_snapshots: Vec<LiveCognitivePresentationSnapshot>,
        world: &HeadlessWorld,
    ) -> Result<(), LiveBrainPresentationFrameError> {
        let authoritative_world_tick = world.tick();
        if tick_summaries.is_empty()
            || tick_summaries.iter().any(|summary| {
                summary.tick_after != authoritative_world_tick
                    || summary.world_tick_after != authoritative_world_tick
            })
        {
            // Render-only frames retain the last frame whose summaries were
            // produced by the same authoritative world tick. Never rotate a
            // moved snapshot with an empty or stale summary batch.
            return Ok(());
        }
        let snapshot = world.presentation_snapshot();
        let expected_organism_ids = snapshot
            .organisms
            .iter()
            .filter(|organism| organism.lifecycle.is_alive())
            .map(|organism| organism.organism_id.raw())
            .collect::<BTreeSet<_>>();
        let returned_organism_ids = tick_summaries
            .iter()
            .map(|summary| summary.organism_id.raw())
            .collect::<BTreeSet<_>>();
        if tick_summaries.len() != expected_organism_ids.len()
            || returned_organism_ids != expected_organism_ids
        {
            // A partial production tick advances the world but does not prove
            // that absent organisms ticked. Keep the last complete causal
            // frame until the next all-resident batch arrives.
            return Ok(());
        }
        let mut next = LiveBrainPresentationFrame::try_new(
            tick_summaries,
            authoritative_world_tick,
            world.object_snapshots(),
        )?;
        next.install_organism_snapshot(snapshot);
        next.cognitive_by_organism_id = cognitive_snapshots
            .into_iter()
            .map(|snapshot| (snapshot.organism_id.raw(), snapshot))
            .collect();
        self.previous = std::mem::replace(&mut self.current, next);
        Ok(())
    }

    pub fn try_publish(
        &mut self,
        tick_summaries: Vec<LiveBrainTickSummary>,
        authoritative_world_tick: Tick,
        world_objects: Vec<WorldObject>,
    ) -> Result<(), LiveBrainPresentationFrameError> {
        let next = LiveBrainPresentationFrame::try_new(
            tick_summaries,
            authoritative_world_tick,
            world_objects,
        )?;
        self.previous = std::mem::replace(&mut self.current, next);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub(crate) struct ProductionGpuBrainAuthorityResource {
    pub telemetry: crate::GpuBrainAuthorityTelemetry,
}

#[cfg(feature = "gpu-runtime")]
pub(crate) struct ProductionGpuBrainRuntimeResource {
    pub(crate) runtime: crate::GpuLiveBrainRuntime,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, PartialEq, Eq, Message)]
pub(crate) enum ProductionCuratedFounderResetCommand {
    Attempt(crate::gpu_live_runtime::LiveAgentResetIntent),
    Retry,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub(crate) struct ProductionCuratedFounderResetResultResource {
    pub(crate) outcome: crate::gpu_live_runtime::CuratedFounderResetDispatchResult,
}

#[cfg(feature = "gpu-runtime")]
impl Default for ProductionCuratedFounderResetResultResource {
    fn default() -> Self {
        Self {
            outcome: crate::gpu_live_runtime::CuratedFounderResetDispatchResult::Idle,
        }
    }
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub(crate) struct ProductionGpuBrainTickScheduleResource {
    startup_render_frames_remaining: u8,
    playback: RuntimePlaybackState,
    run_speed_ticks: u32,
    step_pending: bool,
    scheduler: crate::DoubleBufferedGraphicalScheduler,
    scheduler_attempts: u64,
    checkpoint_publication_waits: u64,
    checkpoint_failed_waits: u64,
    deferred_catch_up_ticks: u64,
    failed: bool,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProductionGpuTickPerformanceCounters {
    pub fixed_tick_hz: u32,
    pub frames_observed: u64,
    pub completed_ticks: u64,
    pub catch_up_ticks_dropped: u64,
    pub scheduler_attempts: u64,
    pub checkpoint_publication_waits: u64,
    pub checkpoint_failed_waits: u64,
    pub deferred_catch_up_ticks: u64,
    pub deferred_debt_micros: u64,
}

#[cfg(feature = "gpu-runtime")]
impl ProductionGpuBrainTickScheduleResource {
    fn new(startup_render_frames: u8) -> Self {
        Self {
            startup_render_frames_remaining: startup_render_frames,
            playback: RuntimePlaybackState::Running,
            run_speed_ticks: 1,
            step_pending: false,
            scheduler: crate::DoubleBufferedGraphicalScheduler::default(),
            scheduler_attempts: 0,
            checkpoint_publication_waits: 0,
            checkpoint_failed_waits: 0,
            deferred_catch_up_ticks: 0,
            failed: false,
        }
    }

    fn take_dispatch_permit(&mut self) -> bool {
        if self.startup_render_frames_remaining == 0 {
            true
        } else {
            self.startup_render_frames_remaining -= 1;
            false
        }
    }

    pub(crate) fn toggle_playback(&mut self) {
        self.playback = match self.playback {
            RuntimePlaybackState::Paused => RuntimePlaybackState::Running,
            RuntimePlaybackState::Running => RuntimePlaybackState::Paused,
            RuntimePlaybackState::ShutdownRequested => RuntimePlaybackState::ShutdownRequested,
        };
    }

    pub(crate) fn queue_step(&mut self) {
        self.playback = RuntimePlaybackState::Paused;
        self.step_pending = true;
    }

    pub(crate) fn pause(&mut self) {
        self.playback = RuntimePlaybackState::Paused;
    }

    pub(crate) fn set_running_speed(&mut self, ticks: u32) {
        self.run_speed_ticks = ticks.clamp(1, crate::S02_MAX_RUN_TICKS_PER_UPDATE);
        self.playback = RuntimePlaybackState::Running;
    }

    pub(crate) fn reset_after_load(&mut self, playback: RuntimePlaybackState, speed_ticks: u32) {
        *self = Self::new(PRODUCTION_GPU_STARTUP_RENDER_FRAMES);
        self.playback = playback;
        self.run_speed_ticks = speed_ticks.clamp(1, crate::S02_MAX_RUN_TICKS_PER_UPDATE);
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.playback == RuntimePlaybackState::Paused
    }

    pub(crate) fn speed_ticks(&self) -> u32 {
        self.run_speed_ticks
    }

    pub(crate) fn performance_counters(&self) -> ProductionGpuTickPerformanceCounters {
        ProductionGpuTickPerformanceCounters {
            fixed_tick_hz: self.scheduler.config.fixed_tick_hz,
            frames_observed: self.scheduler.frames_observed,
            completed_ticks: self.scheduler.ticks_executed,
            catch_up_ticks_dropped: self.scheduler.catch_up_ticks_dropped,
            scheduler_attempts: self.scheduler_attempts,
            checkpoint_publication_waits: self.checkpoint_publication_waits,
            checkpoint_failed_waits: self.checkpoint_failed_waits,
            deferred_catch_up_ticks: self.deferred_catch_up_ticks,
            deferred_debt_micros: self.scheduler.accumulator_micros,
        }
    }

    pub(crate) const fn performance_failed(&self) -> bool {
        self.failed
    }
}

#[cfg(feature = "gpu-runtime")]
const PRODUCTION_GPU_STARTUP_RENDER_FRAMES: u8 = 12;

#[cfg(feature = "gpu-runtime")]
fn production_tick_decision(
    playback: RuntimePlaybackState,
    step_pending: bool,
    scheduled_ticks: u32,
) -> (u32, bool) {
    if step_pending {
        (1, true)
    } else if playback == RuntimePlaybackState::Running {
        (scheduled_ticks, false)
    } else {
        (0, false)
    }
}

#[cfg(feature = "gpu-runtime")]
fn mark_production_gpu_authority_unavailable(
    authority: &mut ProductionGpuBrainAuthorityResource,
    reason: impl Into<String>,
) {
    authority.telemetry.authoritative = false;
    authority.telemetry.unavailable_reason = Some(reason.into());
}

#[cfg(feature = "gpu-runtime")]
fn prepare_production_gpu_runtime_launch(
    launch: &ProductionVoxelLaunchConfig,
    summary: &ProductionVoxelLaunchSummary,
) -> Result<AppShellLaunchConfig, GameAppShellError> {
    let runtime_save_path = PathBuf::from(&summary.ui_settings.runtime_save_path);
    if runtime_save_path.exists() && !launch.dry_run {
        let existing = PortableSaveFile::from_json_file(&runtime_save_path)?;
        existing.validate_with_asset_root(&summary.asset_root)?;
        let existing_population = existing
            .world
            .objects
            .iter()
            .filter(|object| object.kind == alife_world::WorldObjectKind::Agent)
            .count();
        if existing_population != usize::from(summary.effective_population) {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "runtime save population {existing_population} does not match requested profile population {}; select a matching save or create a new world",
                    summary.effective_population
                ),
            });
        }
    } else {
        let source = PortableSaveFile::from_json_file(&summary.save_path)?;
        let production = crate::production_voxel_save_with_population(
            &source,
            &summary.asset_root,
            summary.profile_id,
            summary.effective_population,
        )?
        .with_gpu_runtime_state(summary.gpu_runtime_state.clone())?;
        crate::GpuDurableSaveManifest::publish_snapshot(
            &runtime_save_path,
            &summary.asset_root,
            &production,
        )?;
    }
    let mut runtime_launch = launch.app_launch.clone();
    runtime_launch.save_path = runtime_save_path;
    Ok(runtime_launch)
}

fn apply_presentation_retirements(
    commands: &mut Commands,
    map: &mut BevyEntityMap,
    retired_ids: &[WorldEntityId],
    coat_keys: &Query<&crate::production_voxel_renderer::ProductionCreatureCoatKey>,
    coat_context: &mut crate::production_voxel_renderer::Fvr04CreatureSpawnContext,
) {
    for world_id in retired_ids.iter().copied() {
        if let Some(entity) = map.remove_by_world_id(world_id) {
            if let Ok(key) = coat_keys.get(entity) {
                coat_context.release_coat(key.0);
            }
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(feature = "gpu-runtime")]
fn tick_production_gpu_brain(
    time: Res<Time>,
    mut runtime: NonSendMut<ProductionGpuBrainRuntimeResource>,
    mut authority: ResMut<ProductionGpuBrainAuthorityResource>,
    mut schedule: ResMut<ProductionGpuBrainTickScheduleResource>,
    mut presentation: ResMut<LiveBrainPresentationFrameResource>,
    mut performance: Option<
        ResMut<crate::production_voxel_renderer::Phase31PerformanceMetricsResource>,
    >,
    mut commands: Commands,
    mut map: ResMut<BevyEntityMap>,
    coat_keys: Query<&crate::production_voxel_renderer::ProductionCreatureCoatKey>,
    mut coat_context: ResMut<crate::production_voxel_renderer::Fvr04CreatureSpawnContext>,
) {
    if performance
        .as_deref()
        .is_some_and(|metrics| metrics.draining())
    {
        if let Err(error) = runtime.runtime.poll_persistence_for_shutdown() {
            schedule.failed = true;
            mark_production_gpu_authority_unavailable(&mut authority, error.to_string());
        }
        return;
    }

    if schedule.failed {
        return;
    }

    let playback = schedule.playback;
    let speed = schedule.run_speed_ticks;
    let plan = match schedule
        .scheduler
        .observe_render_frame(time.delta_secs(), playback, speed)
    {
        Ok(plan) => plan,
        Err(error) => {
            schedule.failed = true;
            mark_production_gpu_authority_unavailable(
                &mut authority,
                format!("production scheduler failed: {error}"),
            );
            return;
        }
    };
    if !schedule.take_dispatch_permit() {
        return;
    }
    let (planned_ticks, consume_step) =
        production_tick_decision(playback, schedule.step_pending, plan.ticks_to_run);
    if consume_step {
        schedule.step_pending = false;
    }
    let (ticks_to_run, paced_deferred_ticks) = if consume_step {
        (planned_ticks, 0)
    } else {
        let max_ticks_this_frame = schedule.run_speed_ticks;
        match schedule
            .scheduler
            .pace_planned_ticks(planned_ticks, max_ticks_this_frame)
        {
            Ok(paced) => paced,
            Err(error) => {
                schedule.failed = true;
                mark_production_gpu_authority_unavailable(
                    &mut authority,
                    format!("production scheduler pacing failed: {error}"),
                );
                return;
            }
        }
    };
    schedule.deferred_catch_up_ticks = schedule
        .deferred_catch_up_ticks
        .saturating_add(u64::from(paced_deferred_ticks));

    for attempt_index in 0..ticks_to_run {
        schedule.scheduler_attempts = schedule.scheduler_attempts.saturating_add(1);
        let tick_summaries = match runtime.runtime.tick_outcome() {
            Ok(crate::GpuLiveTickOutcome::Progressed(tick_summaries)) => tick_summaries,
            Ok(crate::GpuLiveTickOutcome::NoProgress(
                crate::GpuLiveNoProgressReason::CheckpointPublicationPending,
            )) => {
                schedule.checkpoint_publication_waits =
                    schedule.checkpoint_publication_waits.saturating_add(1);
                if consume_step {
                    schedule.step_pending = true;
                } else {
                    let unspent = ticks_to_run.saturating_sub(attempt_index);
                    if let Err(error) = schedule.scheduler.preserve_unspent_planned_ticks(unspent) {
                        schedule.failed = true;
                        mark_production_gpu_authority_unavailable(
                            &mut authority,
                            format!("production scheduler debt preservation failed: {error}"),
                        );
                        return;
                    }
                    schedule.deferred_catch_up_ticks = schedule
                        .deferred_catch_up_ticks
                        .saturating_add(u64::from(unspent));
                }
                return;
            }
            Ok(crate::GpuLiveTickOutcome::NoProgress(
                crate::GpuLiveNoProgressReason::CheckpointFailed,
            )) => {
                schedule.checkpoint_failed_waits =
                    schedule.checkpoint_failed_waits.saturating_add(1);
                schedule.failed = true;
                mark_production_gpu_authority_unavailable(
                    &mut authority,
                    "exact checkpoint transaction entered the failed state".to_string(),
                );
                return;
            }
            Err(error) => {
                schedule.failed = true;
                mark_production_gpu_authority_unavailable(&mut authority, error.to_string());
                return;
            }
        };
        if performance
            .as_deref()
            .is_some_and(|metrics| metrics.measuring())
        {
            if let Some(sample) = runtime.runtime.take_completed_neural_timing_sample() {
                if let Some(metrics) = performance.as_deref_mut() {
                    metrics.record_gpu_sample(sample);
                }
            }
        }
        let cognitive_snapshots = runtime
            .runtime
            .live_cognitive_presentation_snapshots(&tick_summaries);
        if let Err(error) = schedule.scheduler.record_executed_ticks(1) {
            schedule.failed = true;
            mark_production_gpu_authority_unavailable(
                &mut authority,
                format!("production scheduler accounting failed: {error}"),
            );
            return;
        }
        let retired_ids = runtime.runtime.take_presentation_retirements();
        apply_presentation_retirements(
            &mut commands,
            &mut map,
            &retired_ids,
            &coat_keys,
            &mut coat_context,
        );
        let world = runtime.runtime.world();

        match presentation.try_publish_successful_tick_with_cognitive(
            tick_summaries,
            cognitive_snapshots,
            world,
        ) {
            Ok(()) => {
                authority.telemetry = runtime.runtime.authority_telemetry();
            }
            Err(error) => {
                schedule.failed = true;
                mark_production_gpu_authority_unavailable(
                    &mut authority,
                    format!("live presentation frame publication failed: {error:?}"),
                );
                return;
            }
        }
    }
}

pub fn build_production_voxel_frontend_app_shell(
    launch: &crate::ProductionVoxelLaunchConfig,
) -> Result<(App, crate::ProductionVoxelLaunchSummary), GameAppShellError> {
    #[cfg(feature = "gpu-runtime")]
    {
        if let crate::ProductionWorldSource::NewGame { seed } = launch.world_source {
            let save_path = launch.canonical_new_game_save_path()?;
            let save_directory =
                save_path
                    .parent()
                    .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                        message: "canonical New Game save path requires a parent directory"
                            .to_string(),
                    })?;
            std::fs::create_dir_all(save_directory)?;
            let assets =
                alife_world::AssetManifest::from_json_file(&launch.app_launch.asset_manifest_path)?;
            assets.validate_with_root(&launch.app_launch.asset_root)?;
            let mut config = alife_world::RuntimeConfig::deterministic_default(
                seed,
                alife_core::BrainScaleTier::Nano512,
            );
            config.features.gpu_backend_enabled = true;
            let created =
                crate::create_canonical_new_game_runtime(crate::CanonicalNewGameLaunchRequest {
                    world_seed: seed,
                    population: launch.effective_population(),
                    save_path,
                    asset_root: launch.app_launch.asset_root.clone(),
                    config,
                    assets,
                })?;
            let exact_save = created.exact_save.clone();
            let mut admitted_launch = launch.clone();
            admitted_launch.app_launch.save_path = created.save_path;
            let summary = crate::run_production_voxel_frontend_preflight(&admitted_launch)?;
            let persisted = PortableSaveFile::from_json_file(&summary.save_path)?;
            if persisted != exact_save {
                return Err(GameAppShellError::InvalidProductionFrontend {
                    message: "production New Game preflight changed the exact canonical save"
                        .to_string(),
                });
            }
            return build_production_voxel_frontend_app_shell_inner(
                &admitted_launch,
                summary,
                created.runtime,
            );
        }

        let summary = crate::run_production_voxel_frontend_preflight(launch)?;
        let runtime_launch = prepare_production_gpu_runtime_launch(launch, &summary)?;
        let backend = alife_gpu_backend::GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .map_err(|error| GameAppShellError::NeuralBackendUnavailable {
            message: error.to_string(),
        })?;
        let mut runtime = crate::GpuLiveBrainRuntime::from_p34_launch(backend, &runtime_launch)?;
        runtime.attach_lineage_archive(
            alife_archive::LineageLibraryConfig::profile_default(
                crate::production_conversation_lineage_ui::default_lineage_root(),
            ),
            alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        )?;
        return build_production_voxel_frontend_app_shell_inner(launch, summary, runtime);
    }
    #[cfg(not(feature = "gpu-runtime"))]
    {
        let summary = crate::run_production_voxel_frontend_preflight(launch)?;
        build_production_voxel_frontend_app_shell_inner(launch, summary)
    }
}

#[cfg(feature = "gpu-runtime")]
pub fn build_production_voxel_frontend_app_shell_with_runtime(
    launch: &crate::ProductionVoxelLaunchConfig,
    runtime: crate::GpuLiveBrainRuntime,
) -> Result<(App, crate::ProductionVoxelLaunchSummary), GameAppShellError> {
    let summary = crate::run_production_voxel_frontend_preflight(launch)?;
    build_production_voxel_frontend_app_shell_inner(launch, summary, runtime)
}

fn build_production_voxel_frontend_app_shell_inner(
    launch: &crate::ProductionVoxelLaunchConfig,
    summary: crate::ProductionVoxelLaunchSummary,
    #[cfg(feature = "gpu-runtime")] mut runtime: crate::GpuLiveBrainRuntime,
) -> Result<(App, crate::ProductionVoxelLaunchSummary), GameAppShellError> {
    let mut app = App::new();
    if launch.dry_run {
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        #[cfg(feature = "vfx-hanabi")]
        app.init_resource::<Assets<bevy_hanabi::prelude::EffectAsset>>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_message::<bevy::input::keyboard::KeyboardInput>();
    } else {
        let present_mode = if launch.record_performance {
            PresentMode::Immediate
        } else {
            PresentMode::AutoVsync
        };
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: production_voxel_asset_root(),
                    ..default()
                })
                .set(production_voxel_render_plugin(launch.record_performance))
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: summary.window_title.clone(),
                        name: Some("alife.production_voxel_frontend".to_string()),
                        resolution: summary.resolution.into(),
                        present_mode,
                        window_theme: Some(WindowTheme::Dark),
                        ..default()
                    }),
                    exit_condition: ExitCondition::OnPrimaryClosed,
                    ..default()
                }),
        );
        #[cfg(feature = "vfx-hanabi")]
        app.add_plugins(bevy_hanabi::prelude::HanabiPlugin);
    }
    app.add_plugins(AlifeBevyAdapterPlugin)
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.065, 0.105, 0.090)))
        .insert_resource(ProductionVoxelFrontendResource {
            summary: summary.clone(),
        });
    let initial_authority = crate::GpuBrainAuthorityTelemetry::pending(
        summary
            .gpu_runtime_state
            .class_bucket_allocations
            .first()
            .and_then(|allocation| allocation.brain_class.neuron_count())
            .map_or_else(|| "unknown".to_string(), |count| format!("N{count}")),
    );
    app.insert_resource(ProductionGpuBrainAuthorityResource {
        telemetry: initial_authority,
    });
    #[cfg(feature = "gpu-runtime")]
    app.add_message::<ProductionCuratedFounderResetCommand>()
        .insert_resource(ProductionCuratedFounderResetResultResource::default());
    #[cfg(feature = "gpu-runtime")]
    {
        runtime.set_performance_measurement_enabled(launch.record_performance);
        let telemetry = runtime.authority_telemetry();
        let initial_world = runtime.world_snapshot();
        let presentation = LiveBrainPresentationFrameResource::from_authoritative_world(
            &initial_world,
        )
        .map_err(|error| GameAppShellError::InvalidProductionFrontend {
            message: format!("failed to seed live presentation frames: {error:?}"),
        })?;
        app.insert_resource(ProductionGpuBrainAuthorityResource { telemetry })
            .insert_resource(presentation)
            .insert_resource(ProductionGpuBrainTickScheduleResource::new(
                PRODUCTION_GPU_STARTUP_RENDER_FRAMES,
            ))
            .insert_non_send_resource(ProductionGpuBrainRuntimeResource { runtime })
            .add_systems(
                Update,
                tick_production_gpu_brain.in_set(
                    crate::production_voxel_renderer::ProductionVoxelPresentationSet::LiveGpuTick,
                ),
            );
    }
    crate::spawn_fvr03_production_voxel_scene(&mut app, &summary)?;
    #[cfg(feature = "gpu-runtime")]
    app.add_systems(
        Update,
        reconcile_production_presentation.in_set(
            crate::production_voxel_renderer::ProductionVoxelPresentationSet::ProceduralAnimation,
        ),
    );
    if let Some(seconds) = launch.smoke_seconds {
        app.insert_resource(ProductionVoxelSmokeTimer {
            started: Instant::now(),
            duration: Duration::from_secs(seconds as u64),
        })
        .add_systems(Update, close_after_production_smoke_timeout);
    }
    Ok((app, summary))
}

#[cfg(feature = "gpu-runtime")]
fn reconcile_production_presentation(
    frame: Res<LiveBrainPresentationFrameResource>,
    mut entity_map: ResMut<BevyEntityMap>,
    roots: Query<(
        Entity,
        &crate::production_voxel_renderer::ProductionCreatureAssemblyRoot,
    )>,
    mut selection: ResMut<crate::production_voxel_renderer::Fvr03ProductionVoxelSelectionResource>,
    mut follow: ResMut<crate::production_voxel_renderer::Fvr04ProductionCreatureFollowResource>,
    mut scene: ResMut<crate::production_voxel_renderer::Fvr04ProductionCreatureSceneResource>,
) {
    if !frame.is_changed() {
        return;
    }

    for (entity, root) in roots.iter() {
        let is_live = frame.current.organism(root.stable_id).is_some_and(|row| {
            row.organism_id == root.organism_id
                && row.lifecycle.is_alive()
                && row.object.kind == WorldObjectKind::Agent
        });
        if is_live {
            let _ = entity_map.bind(entity, root.stable_id);
        } else {
            entity_map.remove_by_world_id(root.stable_id);
        }
    }

    let is_live_creature_ref = |reference: &alife_world::StableVoxelObjectRef| {
        if reference.kind != alife_world::StableVoxelRefKind::Creature {
            return true;
        }
        reference.stable_id.is_some_and(|stable_id| {
            frame.current.organism(stable_id).is_some_and(|row| {
                row.lifecycle.is_alive() && entity_map.bevy_entity(stable_id).is_some()
            })
        })
    };
    let next_selected = selection
        .selected
        .filter(|reference| is_live_creature_ref(reference));
    if selection.selected != next_selected {
        selection.selected = next_selected;
    }
    let next_hovered = selection
        .hovered
        .filter(|reference| is_live_creature_ref(reference));
    if selection.hovered != next_hovered {
        selection.hovered = next_hovered;
    }

    if follow.enabled
        && !follow.target_stable_id.is_some_and(|stable_id| {
            frame.current.organism(stable_id).is_some_and(|row| {
                row.lifecycle.is_alive() && entity_map.bevy_entity(stable_id).is_some()
            })
        })
    {
        follow.enabled = false;
        follow.target_stable_id = None;
    }

    let prior_expression_count = scene.expression_buffer.len();
    scene.expression_buffer.retain_mut(|sample| {
        let Some(row) = frame.current.organism(sample.stable_id) else {
            return false;
        };
        if row.organism_id != sample.organism_id
            || !row.lifecycle.is_alive()
            || entity_map.bevy_entity(sample.stable_id).is_none()
        {
            return false;
        }
        let homeostasis = &row.biochemistry.homeostasis;
        let cognitive = frame.current.cognitive_for_organism(sample.organism_id);
        let memory_record_count = cognitive.and_then(|snapshot| {
            snapshot
                .fast_memory_count
                .zip(snapshot.lifetime_memory_count)
                .and_then(|(fast, lifetime)| fast.checked_add(lifetime))
        });
        sample.brain_class_id = cognitive.and_then(|snapshot| snapshot.brain_class_id);
        sample.brain_neuron_count = cognitive.and_then(|snapshot| snapshot.brain_neuron_count);
        sample.fast_memory_count = cognitive.and_then(|snapshot| snapshot.fast_memory_count);
        sample.lifetime_memory_count =
            cognitive.and_then(|snapshot| snapshot.lifetime_memory_count);
        sample.memory_record_count = memory_record_count;
        sample.concept_count = cognitive.and_then(|snapshot| snapshot.concept_count);
        sample.unresolved_gap_count = cognitive.and_then(|snapshot| snapshot.unresolved_gap_count);
        sample.lifetime_learning_enabled = cognitive.and_then(|snapshot| snapshot.learning_active);
        sample.sleep_phase_raw = cognitive.and_then(|snapshot| snapshot.sleep_phase_raw);
        sample.consolidation_state_raw =
            cognitive.and_then(|snapshot| snapshot.consolidation_state_raw);
        sample.last_consolidated_tick =
            cognitive.and_then(|snapshot| snapshot.last_consolidated_tick);
        sample.topology_update_count =
            cognitive.and_then(|snapshot| snapshot.topology_update_count);
        sample.hunger = homeostasis.drives.hunger;
        sample.fatigue = homeostasis.drives.fatigue;
        sample.fear = homeostasis.drives.fear;
        sample.cortisol = homeostasis.hormones.cortisol;
        sample.dopamine = homeostasis.hormones.dopamine;
        sample.reproductive_drive = homeostasis.drives.reproductive_drive;
        sample.sleep_pressure = homeostasis.hormones.sleep_pressure;
        sample.social = ((row.object.social_affinity + 1.0) * 0.5).clamp(0.0, 1.0);

        let selected_action_kind = row.motor.as_ref().and_then(|motor| motor.action_kind);
        if let Ok(visual) = crate::creature_visual_snapshot_from_parts(
            row.organism_id,
            row.world_entity_id,
            row.object.position,
            None,
            None,
            homeostasis,
            row.sleep_phase,
            selected_action_kind,
        ) {
            sample.expression = visual.expression;
            sample.animation = visual.animation;
        }
        sample.display_label = row.object.label.clone();
        true
    });
    if scene.expression_buffer.len() != prior_expression_count {
        let lookup_entries = scene
            .expression_buffer
            .iter()
            .enumerate()
            .map(|(index, sample)| (sample.stable_id.raw(), index))
            .collect::<Vec<_>>();
        scene.stable_lookup_by_raw_id.clear();
        for (raw_id, index) in lookup_entries {
            scene.stable_lookup_by_raw_id.insert(raw_id, index);
        }
    }
    let rendered_creature_count = scene.expression_buffer.len();
    if scene.rendered_creature_count != rendered_creature_count {
        scene.rendered_creature_count = rendered_creature_count;
    }
}

fn production_voxel_render_plugin(record_performance: bool) -> RenderPlugin {
    let mut wgpu_settings = WgpuSettings::default();
    if record_performance {
        wgpu_settings.instance_flags = wgpu::InstanceFlags::empty();
    }
    RenderPlugin {
        render_creation: RenderCreation::Automatic(wgpu_settings),
        synchronous_pipeline_compilation: false,
        debug_flags: default(),
    }
}

fn production_voxel_asset_root() -> String {
    crate::ca12_workspace_root()
        .join("crates/alife_game_app/assets")
        .to_string_lossy()
        .to_string()
}

pub fn run_production_voxel_frontend_window(
    launch: &crate::ProductionVoxelLaunchConfig,
) -> Result<crate::ProductionVoxelLaunchSummary, GameAppShellError> {
    let (mut app, mut summary) = build_production_voxel_frontend_app_shell(launch)?;
    require_successful_production_app_exit(app.run())?;
    if summary.state_trace.last() != Some(&crate::ProductionAppState::Shutdown) {
        summary
            .state_trace
            .push(crate::ProductionAppState::Shutdown);
    }
    Ok(summary)
}

fn require_successful_production_app_exit(exit: AppExit) -> Result<(), GameAppShellError> {
    if let AppExit::Error(code) = exit {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!("production voxel window exited with error code {code}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod production_app_exit_tests {
    use std::num::NonZeroU8;

    use bevy::app::AppExit;

    use super::require_successful_production_app_exit;

    #[test]
    fn production_window_rejects_bevy_error_exit() {
        let error =
            require_successful_production_app_exit(AppExit::Error(NonZeroU8::new(7).unwrap()))
                .unwrap_err();
        assert!(error.to_string().contains("error code 7"));
    }

    #[test]
    fn production_window_accepts_bevy_success_exit() {
        require_successful_production_app_exit(AppExit::Success).unwrap();
    }
}

#[derive(Debug, Resource)]
struct ProductionVoxelSmokeTimer {
    started: Instant,
    duration: Duration,
}

fn close_after_production_smoke_timeout(
    timer: Res<ProductionVoxelSmokeTimer>,
    mut exit: MessageWriter<AppExit>,
) {
    if timer.started.elapsed() >= timer.duration {
        exit.write(AppExit::Success);
    }
}
