//! Native read-only inspection of a sealed CPU collector, with no replay or learning.
//! Prints private diagnostic evidence locally; never use it as creature input.

#[cfg(not(feature = "cpu-backend"))]
fn main() {
    eprintln!("requires --features cpu-backend");
    std::process::exit(2);
}

#[cfg(feature = "cpu-backend")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use alife_core::{OrganismId, WorldEntityId};
    use alife_training::cpu::{
        read_cpu_founder_source, read_cpu_private_checkpoint, CpuCanonicalHost,
        CpuFoundationTrainer,
    };
    use serde_json::{json, Value};
    use std::path::PathBuf;

    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(
        args.next()
            .ok_or("usage: inspect_cpu_observation_terminal CPU_EPISODE")?,
    );
    if args.next().is_some() {
        return Err("exactly one sealed CPU episode is required".into());
    }
    let receipt: Value = serde_json::from_slice(&std::fs::read(output.join("cpu-cycle.json"))?)?;
    let organism = OrganismId::new(
        receipt["episode"]["food_outcome"]["organism"]
            .as_u64()
            .ok_or("missing exact learner identity")?,
    )
    .ok_or("invalid learner identity")?;
    let target = WorldEntityId::new(
        receipt["episode"]["food_outcome"]["target"]
            .as_u64()
            .ok_or("missing exact food target identity")?,
    )
    .ok_or("invalid food target identity")?;
    let inherited = read_cpu_founder_source(&output.join("source-native"))?;
    let collection_policy = inherited.receipt["policy_version"].clone();
    let admitted = inherited.handoff.admit()?;
    let founder = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        admitted.actor,
    )?;
    let checkpoint = read_cpu_private_checkpoint(&output)?;
    let before = serde_json::to_value(&checkpoint)?;
    let terminal_hysteresis = checkpoint
        .residents
        .iter()
        .map(|resident| resident.hysteresis)
        .collect::<Vec<_>>();
    let host = CpuCanonicalHost::from_checkpoint(checkpoint, &founder)?;
    let food = host
        .world()
        .entity(target)
        .ok_or("saved food target is absent from terminal world")?;
    let key = food.tracking_provenance.canonical_key();
    let tracking = host
        .world()
        .tracked_objects()
        .records_for(organism)
        .and_then(|mut records| records.find(|record| record.tracking_key == key))
        .map(|record| {
            json!({
                "tracked_object_id":record.tracked_object_id,
                "tracking_provenance":record.tracking_provenance,
                "tracking_key":record.tracking_key,
                "last_seen_tick":record.last_seen_tick,
            })
        });
    let terminal = host
        .world()
        .observation_target_diagnostics(organism, target)?;
    let after = serde_json::to_value(host.checkpoint(&founder)?)?;
    if before != after {
        return Err("read-only observation inspection changed private state".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":1,"kind":"read-only-terminal-private-observation-evidence",
            "source_policy":collection_policy,"updated_policy":receipt["policy_version"],
            "seed":receipt["seed"],"original_consumer_build":receipt["consumer_build"],
            "private_checkpoint_schema":before["schema_version"],
            "sampling_counter":before["sampling_counter"],
            "terminal_observation":terminal,"preserved_food_tracking_record":tracking,
            "terminal_hysteresis":terminal_hysteresis,
            "entire_checkpoint_roundtrip_exact":true,"neural_dispatches":0,"optimizer_updates":0,
            "historical_per_decision_pose_or_focus_reconstructed":false,
            "claim_scope":"terminal state and persisted tracking evidence only; last_seen_tick is an ordinary raw observation, not proof of a selectable candidate or traversable route",
        }))?
    );
    Ok(())
}
