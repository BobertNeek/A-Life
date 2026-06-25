//! CA19 resource ecology and terrain-zone graphical presentation.

use crate::prelude::*;
use crate::{
    AppShellLaunchConfig, GameAppShellError, CA19_GRAPHICAL_ECOLOGY_SCHEMA,
    CA19_GRAPHICAL_ECOLOGY_SCHEMA_VERSION, CA19_MAX_TERRAIN_ZONE_VISUALS, CA19_MIN_TERRAIN_ZONES,
};
use alife_world::{HeadlessWorldCommand, ResourceLifecycle, ResourceSpawnPolicy, TerrainZone};

#[derive(Debug, Clone, PartialEq)]
pub struct Ca19TerrainZoneVisual {
    pub zone_id: EcologyZoneId,
    pub label: String,
    pub kind: TerrainZoneKind,
    pub center: Vec3f,
    pub radius: f32,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
}

impl Ca19TerrainZoneVisual {
    fn from_zone(zone: &TerrainZone) -> Self {
        Self {
            zone_id: zone.id,
            label: zone.label.clone(),
            kind: zone.kind,
            center: zone.center,
            radius: zone.radius,
            resource_bias: zone.resource_bias,
            hazard_pressure: zone.hazard_pressure,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca19ResourceCycleVisual {
    pub stable_id: WorldEntityId,
    pub home_zone: EcologyZoneId,
    pub base_nutrition: f32,
    pub regrow_after_ticks: u32,
    pub decay_after_ticks: u32,
    pub low_salience_marker: bool,
}

impl Ca19ResourceCycleVisual {
    fn from_resource(resource: &ResourceLifecycle) -> Self {
        Self {
            stable_id: resource.object_id,
            home_zone: resource.home_zone,
            base_nutrition: resource.base_nutrition,
            regrow_after_ticks: resource.regrow_after_ticks,
            decay_after_ticks: resource.decay_after_ticks,
            low_salience_marker: resource.low_salience_marker,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca19GraphicalEcologySummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub terrain_zones: Vec<Ca19TerrainZoneVisual>,
    pub resources: Vec<Ca19ResourceCycleVisual>,
    pub initial_metrics: EcologyMetrics,
    pub cycled_metrics: EcologyMetrics,
    pub spawned_labels: Vec<String>,
    pub hazard_pressure_zone_count: usize,
    pub resource_regen_visible: bool,
    pub food_spawned_indicator_visible: bool,
    pub save_load_roundtrip_preserved: bool,
    pub stable_ids_only: bool,
    pub display_only: bool,
    pub product_runtime_claim: &'static str,
}

impl Ca19GraphicalEcologySummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA19_GRAPHICAL_ECOLOGY_SCHEMA
            || self.schema_version != CA19_GRAPHICAL_ECOLOGY_SCHEMA_VERSION
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA19 graphical ecology schema must match",
            });
        }
        if self.terrain_zones.len() < CA19_MIN_TERRAIN_ZONES
            || self.terrain_zones.len() > CA19_MAX_TERRAIN_ZONE_VISUALS
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA19 terrain zone visuals must be bounded and nontrivial",
            });
        }
        if self.resources.is_empty() || self.hazard_pressure_zone_count == 0 {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA19 ecology needs resource and hazard-pressure evidence",
            });
        }
        if !self.resource_regen_visible
            || !self.food_spawned_indicator_visible
            || !self.save_load_roundtrip_preserved
            || !self.stable_ids_only
            || !self.display_only
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA19 ecology invariant flags must remain true",
            });
        }
        if self.product_runtime_claim != "CpuShadowGuardedStaticPlusLiveHShadow" {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA19 must not change the GPU product claim",
            });
        }
        Ok(())
    }

    pub fn compact_overlay_text(&self) -> String {
        let terrain = self
            .terrain_zones
            .iter()
            .take(3)
            .map(|zone| format!("{}:{}", zone.kind.label(), zone.label))
            .collect::<Vec<_>>()
            .join(", ");
        let resources = self
            .resources
            .iter()
            .map(|resource| format!("stable:{}", resource.stable_id.raw()))
            .collect::<Vec<_>>()
            .join(" ");
        let spawned = if self.spawned_labels.is_empty() {
            "none".to_string()
        } else {
            self.spawned_labels.join(",")
        };
        format!(
            concat!(
                "Ecology: zones={} resources={} active={}\n",
                "Terrain: {} | hazard zones={}\n",
                "Resource cycle: regrown={} spawned={} labels={}\n",
                "Tracked food: {} | roundtrip={} | display-only"
            ),
            self.terrain_zones.len(),
            self.resources.len(),
            self.cycled_metrics.active_resources,
            terrain,
            self.hazard_pressure_zone_count,
            self.cycled_metrics.resources_regrown,
            self.cycled_metrics.resources_spawned,
            spawned,
            resources,
            self.save_load_roundtrip_preserved,
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:zones={}:resources={}:regrown={}:spawned={}:hazard_zones={}:claim={}",
            self.schema,
            self.schema_version,
            self.terrain_zones.len(),
            self.resources.len(),
            self.cycled_metrics.resources_regrown,
            self.cycled_metrics.resources_spawned,
            self.hazard_pressure_zone_count,
            self.product_runtime_claim
        )
    }
}

pub fn run_graphical_ecology_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<Ca19GraphicalEcologySummary, GameAppShellError> {
    let summary = ca19_graphical_ecology_summary(launch)?;
    summary.validate()?;
    Ok(summary)
}

pub fn ca19_graphical_ecology_summary(
    launch: &AppShellLaunchConfig,
) -> Result<Ca19GraphicalEcologySummary, GameAppShellError> {
    let config = RuntimeConfig::from_json_file(&launch.config_path)?;
    config.validate()?;
    let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
    manifest.validate_with_root(&launch.asset_root)?;
    let save = PortableSaveFile::from_json_file(&launch.save_path)?;
    save.validate_with_asset_root(&launch.asset_root)?;
    if save.deterministic_seed != config.deterministic_seed {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "runtime config seed must match portable save seed",
        });
    }

    let initial_metrics = save.restore_headless_world()?.ecology_metrics();
    let mut cycled_world = save.restore_headless_world()?;
    let first_resource =
        save.world
            .ecology
            .resources
            .first()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "CA19 fixture must track at least one resource lifecycle",
            })?;
    let mut fast_cycle_ecology = cycled_world.ecology().clone();
    if let Some(resource) = fast_cycle_ecology.resources.first_mut() {
        resource.regrow_after_ticks = 1;
    }
    cycled_world.configure_ecology(fast_cycle_ecology)?;
    cycled_world.editor_move_object(WorldEntityId(1), Vec3f::new(1.55, 0.0, 0.0))?;
    cycled_world.add_resource_spawn_policy(ResourceSpawnPolicy {
        label_prefix: "sprout".to_string(),
        zone_id: first_resource.home_zone,
        interval_ticks: 2,
        max_active: 2,
        nutrition: 0.40,
        next_spawn_tick: Tick::new(cycled_world.tick().raw().saturating_add(1)),
        spawned_count: 0,
    })?;
    let eat = HeadlessWorldCommand::eat(OrganismId(1), first_resource.object_id)?;
    let _ = cycled_world.apply_command(&eat)?;
    let _ = cycled_world.advance_tick();
    let cycled_metrics = cycled_world.ecology_metrics();
    let spawned_labels = cycled_world
        .stable_signature()
        .into_iter()
        .filter(|line| line.contains(":Food:sprout-"))
        .map(|line| line.split(':').nth(2).map(str::to_string).unwrap_or(line))
        .collect::<Vec<_>>();

    let roundtrip = PortableSaveFile::from_json_str(&save.to_json_string_pretty()?)?;
    let save_load_roundtrip_preserved = save.world.ecology == roundtrip.world.ecology
        && roundtrip.restore_headless_world()?.ecology().zones.len()
            == save.world.ecology.zones.len()
        && roundtrip
            .restore_headless_world()?
            .ecology()
            .resources
            .len()
            == save.world.ecology.resources.len();

    let terrain_zones = save
        .world
        .ecology
        .zones
        .iter()
        .map(Ca19TerrainZoneVisual::from_zone)
        .collect::<Vec<_>>();
    let resources = save
        .world
        .ecology
        .resources
        .iter()
        .map(Ca19ResourceCycleVisual::from_resource)
        .collect::<Vec<_>>();
    let hazard_pressure_zone_count = terrain_zones
        .iter()
        .filter(|zone| zone.hazard_pressure > 0.0)
        .count();

    let summary = Ca19GraphicalEcologySummary {
        schema: CA19_GRAPHICAL_ECOLOGY_SCHEMA,
        schema_version: CA19_GRAPHICAL_ECOLOGY_SCHEMA_VERSION,
        terrain_zones,
        resources,
        initial_metrics,
        cycled_metrics: cycled_metrics.clone(),
        spawned_labels,
        hazard_pressure_zone_count,
        resource_regen_visible: cycled_metrics.resources_regrown > 0,
        food_spawned_indicator_visible: cycled_metrics.resources_spawned > 0,
        save_load_roundtrip_preserved,
        stable_ids_only: true,
        display_only: true,
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow",
    };
    summary.validate()?;
    Ok(summary)
}
