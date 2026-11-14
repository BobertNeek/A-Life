//! LOCAL-only diagnostics. These records are never admitted to ordinary training.
use super::evidence::LocalEvidence;
use alife_core::WorldEntityId;
use alife_training::cpu::{CpuCanonicalCheckpoint, CpuCanonicalStep};
use serde_json::json;
use sha2::{Digest, Sha256};

pub fn save(
    evidence: &LocalEvidence,
    interval: usize,
    step: &CpuCanonicalStep,
    checkpoint: &CpuCanonicalCheckpoint,
    bytes: &[u8],
    toy: WorldEntityId,
    food: &[WorldEntityId],
) -> Result<(), Box<dyn std::error::Error>> {
    // Remove only the head recipe from an independent diagnostic COPY. This
    // hash compares complete resident/world authority across matched read arms;
    // the copy is never restored, executed, saved as a CP, or used for training.
    let mut without_recipe = checkpoint.clone();
    without_recipe.terminal_context2135 = None;
    let state_digest = Sha256::digest(serde_json::to_vec(&without_recipe)?);
    evidence.save(
        &format!("TRACE-{interval:04}.json"),
        &serde_json::to_vec(&json!({
            "schema":"terminal_physical_local_trace_v1", "local_only":true,
            "host_interval":interval, "tick_before":step.tick_before, "tick_after":step.tick_after,
            "sleeping_organisms":step.sleeping_organisms,
            "sleep_consolidations":step.sleep_consolidations,
            "checkpoint_sha256":format!("{:x}",Sha256::digest(bytes)),
            "state_without_recipe_sha256":format!("{state_digest:x}"),
            "toy_entity_id":toy, "food_entity_ids":food,
            "decisions":&step.decisions,
        }))?,
    )?;
    Ok(())
}
