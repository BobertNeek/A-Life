//! Fixed disposable native care comparison; never enters normal care admission.
use std::{collections::BTreeMap, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut values = BTreeMap::new();
    let mut long_horizon = false;
    while let Some(key) = args.next() {
        if key == "--long-horizon" {
            if long_horizon {
                return Err("duplicate --long-horizon".into());
            }
            long_horizon = true;
            continue;
        }
        if key == "--help" {
            println!("run_predictive_care_smoke [--long-horizon] --state LOCAL_ONE_TICK_FULL_STATE --upstream-result LOCAL_MOTION_RESULT --output NEW_DIRECTORY\nFixed: separate pretrained/source actors and critics, matched teacher/prior-removed heldout evaluations before/after. Default retains the original four 16-decision lessons. The separately labeled long follow-up uses 64 decisions each for held-food, reachable-food, feeding and relocated-food lessons. Retains original sensor profile and all failures. Local recovery artifacts only; no accepted-source admission or promotion.");
            return Ok(());
        }
        if !matches!(key.as_str(), "--state" | "--upstream-result" | "--output") {
            return Err("unknown argument".into());
        }
        let value = args.next().ok_or("missing argument value")?;
        if values.insert(key, PathBuf::from(value)).is_some() {
            return Err("duplicate argument".into());
        }
    }
    let run = if long_horizon {
        alife_game_app::run_predictive_care_long_smoke
    } else {
        alife_game_app::run_predictive_care_smoke
    };
    let result = run(
        &values.remove("--state").ok_or("--state required")?,
        &values
            .remove("--upstream-result")
            .ok_or("--upstream-result required")?,
        &values.remove("--output").ok_or("--output required")?,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
