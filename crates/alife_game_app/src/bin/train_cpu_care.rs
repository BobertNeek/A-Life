//! Explicit CPU continuation using the existing held/reachable/feeding lessons.
use alife_game_app::{run_cpu_care, CpuCareRequest, FoundationTeacherLesson};
use std::{collections::BTreeMap, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    while let Some(key) = arguments.next() {
        match key.as_str() {
            "--help" | "-h" => {
                println!("train_cpu_care --previous FINAL_PORTABLE_OR_CPU_SOURCE --output NEW_CPU_LANE/NEW_EPISODE --seed-history EXACT_HISTORY --seed EXPLICIT_NEW_SEED --lesson eat_held_food|eat_reachable_food|feeding --decisions 1..128 [--temperature 1] [--known-exclusions]\n\nDevelopment only: --fixture replaces --previous and --seed-history. No GPU/device is created. New prepared episode body/private cognition; full inherited founder and actor/value Adam state. A separate original individual archive remains required. No automatic curriculum progression or terminal GPU individual restore is claimed.");
                return Ok(());
            }
            "--fixture" => {
                if fixture {
                    return Err("duplicate --fixture".into());
                }
                fixture = true;
            }
            "--known-exclusions" => {
                if known_exclusions {
                    return Err("duplicate admission flag".into());
                }
                known_exclusions = true;
            }
            "--previous" | "--output" | "--seed-history" | "--seed" | "--lesson"
            | "--decisions" | "--temperature" => {
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
        fixture,
        known_exclusions,
    };
    println!("{}", serde_json::to_string(&run_cpu_care(request)?)?);
    Ok(())
}
