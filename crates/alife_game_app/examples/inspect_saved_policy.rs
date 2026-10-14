//! Read original GPU scores from one immutable replay row. No neural execution.
use alife_core::{FoundationWeightAsset, Validate};
use alife_game_app::{FoundationCycleReceipt, FoundationReplayRecord, FoundationReplayRecordRef};
use std::{error::Error, io::Read, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("inspection input exceeds its byte limit".into());
    }
    Ok(bytes)
}

fn inspect(directory: &Path, row: usize) -> Result<serde_json::Value> {
    let cycle: FoundationCycleReceipt =
        serde_json::from_slice(&read_bounded(&directory.join("cycle.json"), 1_048_576)?)?;
    let refs: Vec<FoundationReplayRecordRef> = serde_json::from_slice(&read_bounded(
        &directory.join("replay-manifest.json"),
        65_536,
    )?)?;
    if cycle.biological_objective_version != 2
        || cycle.objective_state_reset
        || !cycle.next_cohort_tick_captured
        || !cycle.next_cohort_optimizer_rebound
        || refs.is_empty()
        || refs.len() > 129
        || refs.len() != cycle.captured_replay_rows
        || cycle
            .trained_replay_rows
            .checked_add(cycle.bootstrap_replay_rows)
            != Some(refs.len())
    {
        return Err("inspection requires a sealed bounded objective2 cycle".into());
    }
    let reference = refs
        .get(row)
        .ok_or("requested row is outside the manifest")?;
    if reference.record_index != row as u64 || reference.encoded_bytes > 268_435_456 {
        return Err("replay reference identity or size is invalid".into());
    }
    let bytes = read_bounded(
        &directory.join("replay").join(reference.file_name()),
        reference.encoded_bytes,
    )?;
    if bytes.len() as u64 != reference.encoded_bytes
        || blake3::hash(&bytes).as_bytes() != &reference.digest
    {
        return Err("replay record digest mismatch".into());
    }
    let decoder = zstd::stream::read::Decoder::new(std::io::Cursor::new(bytes))?;
    let record: FoundationReplayRecord = bincode::serde::decode_from_std_read(
        &mut decoder.take(268_435_457),
        bincode::config::standard().with_limit::<268435456>(),
    )?;
    let asset = FoundationWeightAsset::decode_canonical(&read_bounded(
        &directory.join("initial.alife-foundation"),
        33_554_432,
    )?)?;
    let asset_digest: String = asset
        .digest()
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let frame = record.patch.pre_action().perception();
    let scores = &record.behavior.logits;
    if record.record_index != reference.record_index
        || record.segment != reference.segment
        || record.tick != reference.tick
        || record.source.policy_version != cycle.policy_version
        || record.source.foundation_asset_digest != asset.digest()
        || asset_digest != cycle.old_asset_digest
        || record.sequence.phenotype_hash != record.source.phenotype_hash
        || record.behavior.tick != record.tick
        || frame.tick().raw() != record.tick
        || frame.frame_digest() != record.behavior.frame_digest
        || scores.len() != frame.candidates().len()
        || scores.is_empty()
        || scores.len() > 32
        || scores.iter().any(|score| !score.is_finite())
        || usize::from(record.behavior.representative_index) >= scores.len()
    {
        return Err("saved policy identity, source or score layout is invalid".into());
    }
    record.patch.validate_contract()?;
    record.behavior.sampling.validate()?;
    Ok(serde_json::json!({
        "scope": "original GPU scores; no new evaluation or replay admission",
        "neural_execution": false,
        "weights_modified": false,
        "row": row,
        "trained_row": row < cycle.trained_replay_rows,
        "tick": record.tick,
        "source": record.source,
        "record_digest": reference.digest,
        "sampling": record.behavior.sampling,
        "behavior_kind": record.behavior.behavior,
        "representative_mask": record.behavior.representative_mask,
        "motor_masks": record.behavior.motor_masks,
        "forced_motor_slots": record.behavior.forced_motor_slots,
        "representative_index": record.behavior.representative_index,
        "motor_indices": record.behavior.motor_indices,
        "candidates": frame.candidates().iter().enumerate().map(|(index, candidate)| {
            serde_json::json!({
                "index": index,
                "action": candidate.action_id.raw(),
                "family": candidate.family,
                "target": candidate.target.entity,
                "logit": scores[index],
            })
        }).collect::<Vec<_>>(),
    }))
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let directory = std::path::PathBuf::from(args.next().ok_or("missing sealed cycle")?);
    let row = args
        .next()
        .ok_or("missing row index")?
        .to_string_lossy()
        .parse()?;
    if args.next().is_some() {
        return Err("expected SEALED_CYCLE ROW".into());
    }
    let receipt = std::thread::Builder::new()
        .name("saved-policy-inspection".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || inspect(&directory, row).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "saved-policy inspection worker panicked")??;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
