//! Explicit CPU continuation using the existing held/reachable/feeding/blocked-route lessons.
use alife_game_app::{run_cpu_care, CpuCareRequest, FoundationTeacherLesson};
use std::{collections::BTreeMap, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Cooperative predecessors validate full round proofs. Use the same bounded
    // offline stack as the round CLI on Windows' smaller executable main stack.
    #[cfg(windows)]
    {
        std::thread::Builder::new()
            .name("cpu-care".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| run().map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "CPU care worker panicked; retained output requires inspection")??;
        Ok(())
    }
    #[cfg(not(windows))]
    run()
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let special: Vec<_> = std::env::args().skip(1).collect();
    if special.len() == 2 {
        let result = match special[0].as_str() {
            "--validate-output" => Some(alife_game_app::validate_cpu_care_output(
                std::path::Path::new(&special[1]),
            )?),
            "--reconcile-pending" => Some(alife_game_app::reconcile_cpu_care_attempt(
                std::path::Path::new(&special[1]),
            )?),
            _ => None,
        };
        if let Some(result) = result {
            println!("{}", serde_json::to_string(&result)?);
            return Ok(());
        }
    }
    let mut arguments = std::env::args().skip(1);
    let mut values = BTreeMap::new();
    let mut fixture = false;
    let mut known_exclusions = false;
    let mut record_diagnostic_replay = false;
    while let Some(key) = arguments.next() {
        match key.as_str() {
            "--help" | "-h" => {
                println!("train_cpu_care --previous FINAL_PORTABLE_OR_CPU_SOURCE --output NEW_CPU_LANE/NEW_EPISODE --seed-history EXACT_HISTORY --seed EXPLICIT_NEW_SEED --lesson eat_held_food|eat_reachable_food|feeding|obstacle_navigation|relocated_food --decisions 1..128 [--temperature 1] [--food-need-condition newborn_v1|ordinary_healthy_aging_v1] [--known-exclusions] [--record-diagnostic-replay]\n\nThe opt-in ordinary healthy aging condition applies only to Feeding/RelocatedFood and preserves their geometry and cues.\n\nBulky raw diagnostic replay is opt-in; in-memory training and checkpoint state are unchanged.\n\nDevelopment only: --fixture replaces --previous and --seed-history. No GPU/device is created. New prepared episode body/private cognition; full inherited founder and actor/value Adam state. A separate original individual archive remains required. No automatic curriculum progression or terminal GPU individual restore is claimed.");
                return Ok(());
            }
            "--fixture" => {
                if fixture {
                    return Err("duplicate --fixture".into());
                }
                fixture = true;
            }
            "--record-diagnostic-replay" => {
                if record_diagnostic_replay {
                    return Err("duplicate diagnostic recording flag".into());
                }
                record_diagnostic_replay = true;
            }
            "--known-exclusions" => {
                if known_exclusions {
                    return Err("duplicate admission flag".into());
                }
                known_exclusions = true;
            }
            "--previous"
            | "--output"
            | "--seed-history"
            | "--seed"
            | "--lesson"
            | "--decisions"
            | "--temperature"
            | "--food-need-condition" => {
                let value = arguments.next().ok_or("missing argument value")?;
                if values.insert(key, value).is_some() {
                    return Err("duplicate argument".into());
                }
            }
            _ => return Err(format!("unknown CPU care argument {key}").into()),
        }
    }
    let lesson = match values.remove("--lesson").as_deref() {
        Some("eat_held_food") => FoundationTeacherLesson::EatHeldFood,
        Some("eat_reachable_food") => FoundationTeacherLesson::EatReachableFood,
        Some("feeding") => FoundationTeacherLesson::Feeding,
        Some("obstacle_navigation") => FoundationTeacherLesson::ObstacleNavigation,
        Some("relocated_food") => FoundationTeacherLesson::RelocatedFood,
        _ => return Err("an explicit supported care lesson is required".into()),
    };
    let request = CpuCareRequest {
        previous: values.remove("--previous").map(PathBuf::from),
        output: values.remove("--output").ok_or("--output required")?.into(),
        seed: values.remove("--seed").ok_or("--seed required")?.parse()?,
        decisions: values
            .remove("--decisions")
            .ok_or("--decisions required")?
            .parse()?,
        temperature: values
            .remove("--temperature")
            .unwrap_or_else(|| "1".into())
            .parse()?,
        seed_history: values.remove("--seed-history").map(PathBuf::from),
        lesson,
        food_need_condition: values
            .remove("--food-need-condition")
            .as_deref()
            .unwrap_or("newborn_v1")
            .parse()?,
        fixture,
        known_exclusions,
        record_diagnostic_replay,
    };
    println!("{}", serde_json::to_string(&run_cpu_care(request)?)?);
    Ok(())
}
