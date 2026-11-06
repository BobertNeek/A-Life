//! Reconstruct native motor transactions from independently sampled logits.
//! The inputs are local diagnostic artifacts, never a new gameplay score.
use alife_core::*;
use alife_training::cpu::{CpuCanonicalCheckpoint, CpuCollectedDecision};
use alife_world::SavedBodyRoundingCompatibilityV1;
use serde_json::{json, Value};
use std::{error::Error, fs, path::Path};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn same<T: serde::Serialize + serde::de::DeserializeOwned>(
    actual: &T,
    value: Value,
) -> Result<bool> {
    // Compare the typed f32 ABI, not incidental f64 JSON parser formatting.
    let expected: T = serde_json::from_value(value)?;
    Ok(serde_json::to_vec(actual)? == serde_json::to_vec(&expected)?)
}

fn main() -> Result<()> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    if a.len() != 2 {
        return Err("BATTERY_DIRECTORY DIAGNOSTIC_DIRECTORY".into());
    }
    let battery = Path::new(&a[0]);
    let root = Path::new(&a[1]);
    let load = |p: &Path| -> Result<Value> { Ok(serde_json::from_slice(&fs::read(p)?)?) };
    let plan = load(&root.join("native-plan.json"))?;
    if plan["phenotype"]["cognitive_architecture"]["parameters"]["motor_head_count"] != 1 {
        return Err("this bounded motor diagnostic supports the saved one-head layout only".into());
    }
    let qualification = load(&battery.join("qualification.json"))?;
    let proof: SavedBodyRoundingCompatibilityV1 =
        serde_json::from_value(qualification["body_compatibility"]["explicit_proof"].clone())?;
    let independent = load(&root.join("independent-packed-audit.json"))?;
    let mut results = Vec::new();
    for condition in ["held", "feeding_aged"] {
        let (initial, used) = alife_world::with_saved_body_rounding_compatibility(
            proof.clone(),
            || -> Result<CpuCanonicalCheckpoint> {
                Ok(serde_json::from_slice(&fs::read(battery.join(format!(
                    "{condition}-202611185878/initial-checkpoint.json"
                )))?)?)
            },
        )?;
        if !used {
            return Err("explicit saved body proof unused during checkpoint decode".into());
        }
        let initial = initial?;
        let records = load(&root.join(format!("{condition}-rows.json")))?;
        for (number, record) in records
            .as_array()
            .ok_or("rows not an array")?
            .iter()
            .enumerate()
        {
            let reference = independent["cases"]
                .as_array()
                .ok_or("cases not an array")?
                .iter()
                .find(|v| v["condition"] == condition && v["decision"] == number)
                .ok_or("independent case absent")?;
            let reference = &reference["results"][0];
            if reference["variant"] != "full" || reference["independent_sample_matches_cpu"] != true
            {
                return Err("independent action qualification failed".into());
            }
            let row: CpuCollectedDecision = serde_json::from_value(record["row"].clone())?;
            let frame = row.patch.pre_action().perception();
            let representative = reference["independent_sample"]["representative"]
                .as_u64()
                .ok_or("representative absent")? as usize;
            let candidate = frame.candidates()[representative];
            let confidence =
                Confidence::new((candidate.sensor_confidence.raw() * 65535.).round() / 65535.)?;
            let command = candidate.to_command(row.organism_id, confidence)?;
            let mut slots = [0; 6];
            for (slot, index) in reference["independent_sample"]["motor_candidates"]
                .as_array()
                .ok_or("slots absent")?
                .iter()
                .enumerate()
            {
                slots[slot] = index.as_u64().map_or(0, |i| i as u16 + 1);
            }
            let bundle = factorized_motor_bundle_for_candidates(
                row.organism_id,
                row.patch.pre_action().sequence_id,
                frame.tick(),
                frame,
                slots,
                &[MotorChannel::Locomotion],
                &command,
                representative as u16,
                None,
                false,
            )?;
            let bundle_exact = same(&bundle, record["original_trace"]["bundle"].clone())?;
            let mut save = initial.world.clone();
            let (restored, _used) = alife_world::with_saved_body_rounding_compatibility(
                proof.clone(),
                || -> Result<_> {
                    save.world = serde_json::from_value(record["before_world"].clone())?;
                    for creature in &mut save.creatures {
                        let record = save
                            .world
                            .organism_records
                            .as_ref()
                            .ok_or("native records absent")?
                            .iter()
                            .find(|r| r.organism_id() == creature.organism_id)
                            .ok_or("native summary owner absent")?;
                        creature.mind.tick = record.biochemistry().tick;
                        creature.mind.homeostasis = record.biochemistry().homeostasis;
                        creature.development_tick =
                            record.biochemistry().development.last_update_tick;
                    }
                    Ok(save.restore_headless_world()?)
                },
            )?;
            let mut world = restored?;
            let owner = world
                .organism_registry()
                .get(row.organism_id)
                .ok_or("native owner missing")?
                .world_entity_id();
            let logit = reference["logits"][representative]
                .as_f64()
                .ok_or("native logit absent")? as f32;
            let emissions = NeuralEmissionFrame::new(
                frame.tick(),
                record["after_neural"]["dispatch_generation"]
                    .as_u64()
                    .ok_or("dispatch absent")?,
                vec![
                    NeuralEmission::new(
                        NeuralEmissionClass::RegionalArousal,
                        logit.abs().tanh(),
                        confidence.raw(),
                    )?,
                    NeuralEmission::new(
                        NeuralEmissionClass::MotorCommitment,
                        confidence.raw(),
                        confidence.raw(),
                    )?,
                    NeuralEmission::new(
                        NeuralEmissionClass::ExecutiveSustain,
                        confidence.raw(),
                        1.,
                    )?,
                ],
            )?;
            // One native world motor transaction. Cognitive prediction/credit and
            // the following world tick are outside this physical-outcome control.
            let receipt = world
                .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
                    &bundle,
                    owner,
                    &emissions,
                    frame.sensory().channels.novelty_signal.raw(),
                    &[],
                )
                .map_err(|e| format!("native motor transaction rejected: {e:?}"))?;
            let joint_exact = same(
                &receipt.joint,
                record["original_trace"]["outcome"]["joint"].clone(),
            )?;
            results.push(json!({"condition":condition,"decision":number,"command_exact":same(&command, record["original_trace"]["action"].clone())?,"bundle_exact":bundle_exact,"native_joint_outcome_exact":joint_exact,"native_joint":receipt.joint,"native_biology_before":receipt.biology_before,"native_biology_after_motor":receipt.biology_after}));
        }
    }
    let passed = results.iter().all(|r| {
        r["command_exact"] == true
            && r["bundle_exact"] == true
            && r["native_joint_outcome_exact"] == true
    });
    let output = root.join("independent-native-motor-audit.json");
    if output.exists() {
        return Err("motor audit already exists; preserve and inspect".into());
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(
            &json!({"scope":"independent packed scalar decisions passed through the native world motor transaction; no GPU or full-life parity","passed":passed,"rows":results}),
        )?,
    )?;
    if !passed {
        return Err("independent native motor transaction differs".into());
    }
    println!("12 independent native motor transactions matched");
    Ok(())
}
