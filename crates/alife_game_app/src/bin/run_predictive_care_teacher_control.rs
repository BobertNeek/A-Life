use std::{collections::BTreeMap, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let all = std::env::args().skip(1).collect::<Vec<_>>();
    if all.len() == 2 && all[0] == "--validate-recovery" {
        let (actor, critic) =
            alife_game_app::restore_disposable_teacher_actor(std::path::Path::new(&all[1]))?;
        println!(
            "{}",
            serde_json::json!({"valid_disposable_actor_only_recovery":true,"actor_optimizer_step":actor.optimizer_step(),"critic_optimizer_step":critic.checkpoint().optimizer_step,"critic_source_policy_binding":critic.checkpoint().last_updated_policy_version})
        );
        return Ok(());
    }
    let mut args = all.into_iter();
    let mut values = BTreeMap::new();
    let mut check_only = false;
    while let Some(key) = args.next() {
        if key == "--check-only" {
            check_only = true;
            continue;
        }
        if key == "--help" {
            println!("run_predictive_care_teacher_control --state LOCAL_STATE --upstream-result LOCAL_RESULT --reference-care-run LOCAL_FROZEN_LONG_RUN --output NEW_DIRECTORY [--check-only]\nFixed disposable CPU teacher-imitation control: held/reachable cases, native recurrent + action-candidate coordinates, preserved Adam, actual-perception care teacher labels, teacher-free evaluation. No prior assistance for unsupported object-slot sensors. No accepted source or GPU.");
            return Ok(());
        }
        if !matches!(
            key.as_str(),
            "--state" | "--upstream-result" | "--reference-care-run" | "--output"
        ) {
            return Err("unknown argument".into());
        }
        let value = args.next().ok_or("missing value")?;
        if values.insert(key, PathBuf::from(value)).is_some() {
            return Err("duplicate argument".into());
        }
    }
    let result = alife_game_app::run_predictive_care_teacher_control(
        &values.remove("--state").ok_or("--state required")?,
        &values
            .remove("--upstream-result")
            .ok_or("--upstream-result required")?,
        &values
            .remove("--reference-care-run")
            .ok_or("--reference-care-run required")?,
        &values.remove("--output").ok_or("--output required")?,
        check_only,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
