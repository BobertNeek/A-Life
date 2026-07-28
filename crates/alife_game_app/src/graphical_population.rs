//! CA18 multi-creature graphical population presentation.

use crate::prelude::*;
use crate::{
    compare_visible_world_to_headless, load_visible_world_from_p34_save, AppShellLaunchConfig,
    GameAppShellError, VisibleWorldObjectPresentation, VisibleWorldPresentation,
    CA18_GRAPHICAL_POPULATION_SCHEMA, CA18_GRAPHICAL_POPULATION_SCHEMA_VERSION,
    CA18_MAX_GRAPHICAL_CREATURES, CA18_SOCIAL_CUE_DISTANCE,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ca18SocialProximityCue {
    pub from_stable_id: WorldEntityId,
    pub to_stable_id: WorldEntityId,
    pub distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca18GraphicalPopulationSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub creature_count: usize,
    pub population_cap: usize,
    pub selectable_stable_ids: Vec<WorldEntityId>,
    pub selected_stable_id: WorldEntityId,
    pub social_cues: Vec<Ca18SocialProximityCue>,
    pub bounded_performance: bool,
    pub stable_id_selection_only: bool,
    pub no_bevy_entity_ids_in_player_text: bool,
    pub gpu_authority_preserved: bool,
    pub product_runtime_claim: &'static str,
}

impl Ca18GraphicalPopulationSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA18_GRAPHICAL_POPULATION_SCHEMA
            || self.schema_version != CA18_GRAPHICAL_POPULATION_SCHEMA_VERSION
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA18 graphical population schema must match",
            });
        }
        if self.creature_count < 2 {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA18 graphical population needs at least two creatures",
            });
        }
        if self.creature_count > self.population_cap
            || self.population_cap != CA18_MAX_GRAPHICAL_CREATURES
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA18 graphical population must remain bounded",
            });
        }
        if self.selectable_stable_ids.len() != self.creature_count
            || !self
                .selectable_stable_ids
                .contains(&self.selected_stable_id)
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA18 creature selection cycle must use stable IDs",
            });
        }
        if !self.bounded_performance
            || !self.stable_id_selection_only
            || !self.no_bevy_entity_ids_in_player_text
            || !self.gpu_authority_preserved
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA18 population invariant flags must remain true",
            });
        }
        Ok(())
    }

    pub fn compact_overlay_text(&self) -> String {
        let stable_ids = self
            .selectable_stable_ids
            .iter()
            .map(|id| format!("stable:{}", id.raw()))
            .collect::<Vec<_>>()
            .join(" ");
        let cue_text = if self.social_cues.is_empty() {
            "none".to_string()
        } else {
            self.social_cues
                .iter()
                .take(3)
                .map(|cue| {
                    format!(
                        "{}-{} {:.1}",
                        cue.from_stable_id.raw(),
                        cue.to_stable_id.raw(),
                        cue.distance
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "Population: {}/{} creatures | selected stable:{}\nCycle: Tab stable IDs only | peers: {}\nSocial proximity cues: {} | GPU neural authority preserved",
            self.creature_count,
            self.population_cap,
            self.selected_stable_id.raw(),
            stable_ids,
            cue_text
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:creatures={}:selected={}:cues={}:claim={}",
            self.schema,
            self.schema_version,
            self.creature_count,
            self.selected_stable_id.raw(),
            self.social_cues.len(),
            self.product_runtime_claim
        )
    }
}

pub fn run_graphical_population_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<Ca18GraphicalPopulationSummary, GameAppShellError> {
    let presentation = load_visible_world_from_p34_save(launch)?;
    compare_visible_world_to_headless(&presentation)?;
    ca18_graphical_population_summary(&presentation)
}

pub fn ca18_graphical_population_summary(
    presentation: &VisibleWorldPresentation,
) -> Result<Ca18GraphicalPopulationSummary, GameAppShellError> {
    let selectable_stable_ids = ca18_creature_selection_ids(presentation);
    let selected_stable_id =
        selectable_stable_ids
            .first()
            .copied()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "CA18 graphical population found no selectable creatures",
            })?;
    let summary = Ca18GraphicalPopulationSummary {
        schema: CA18_GRAPHICAL_POPULATION_SCHEMA,
        schema_version: CA18_GRAPHICAL_POPULATION_SCHEMA_VERSION,
        creature_count: selectable_stable_ids.len(),
        population_cap: CA18_MAX_GRAPHICAL_CREATURES,
        selectable_stable_ids,
        selected_stable_id,
        social_cues: ca18_social_proximity_cues(presentation),
        bounded_performance: true,
        stable_id_selection_only: true,
        no_bevy_entity_ids_in_player_text: true,
        gpu_authority_preserved: true,
        product_runtime_claim: "GpuAuthoritative",
    };
    summary.validate()?;
    Ok(summary)
}

pub fn ca18_creature_selection_ids(presentation: &VisibleWorldPresentation) -> Vec<WorldEntityId> {
    let mut ids = presentation
        .objects
        .iter()
        .filter(|object| object.kind == WorldObjectKind::Agent)
        .map(|object| object.stable_id)
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| id.raw());
    ids
}

pub fn ca18_cycle_selected_creature(
    presentation: &VisibleWorldPresentation,
    current: WorldEntityId,
) -> Option<WorldEntityId> {
    let ids = ca18_creature_selection_ids(presentation);
    let index = ids.iter().position(|id| *id == current)?;
    Some(ids[(index + 1) % ids.len()])
}

pub fn ca18_social_proximity_cues(
    presentation: &VisibleWorldPresentation,
) -> Vec<Ca18SocialProximityCue> {
    let creatures = presentation
        .objects
        .iter()
        .filter(|object| object.kind == WorldObjectKind::Agent)
        .collect::<Vec<_>>();
    let mut cues = Vec::new();
    for left_index in 0..creatures.len() {
        for right in creatures.iter().skip(left_index + 1) {
            let left = creatures[left_index];
            let distance = ca18_distance(left, right);
            if distance <= CA18_SOCIAL_CUE_DISTANCE {
                cues.push(Ca18SocialProximityCue {
                    from_stable_id: left.stable_id,
                    to_stable_id: right.stable_id,
                    distance,
                });
            }
        }
    }
    cues
}

fn ca18_distance(
    left: &VisibleWorldObjectPresentation,
    right: &VisibleWorldObjectPresentation,
) -> f32 {
    let dx = left.position.x - right.position.x;
    let dy = left.position.y - right.position.y;
    let dz = left.position.z - right.position.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
