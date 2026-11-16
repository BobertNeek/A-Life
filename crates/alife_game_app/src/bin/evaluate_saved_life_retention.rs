//! Frozen-head saved-life evaluation. Publishes only its advanced copy endpoint, never the source store.
use std::{ffi::OsString, path::PathBuf};

type Error = Box<dyn std::error::Error>;
const USAGE: &str = "usage: evaluate_saved_life_retention CURRENT OUTPUT REQUEST.json";

fn arguments(args: Vec<OsString>) -> Result<(PathBuf, PathBuf, PathBuf), Error> {
    if args.len() != 3 || args.iter().any(|s| s.is_empty()) {
        return Err(USAGE.into());
    }
    Ok(((&args[0]).into(), (&args[1]).into(), (&args[2]).into()))
}

fn main() -> Result<(), Error> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && args[0] == "--capabilities" {
        println!(
            "{}",
            serde_json::json!({
                "kind":"saved-life-retention-v2", "request_schema":2,
                "restores_saved_life":true, "requires_saved_sampling_stream":true,
                "offline_actor_updates":false, "offline_value_updates":false,
                "ordinary_private_learning_active":true, "requires_prior_off":true,
                "rejects_existing_teacher_cues":true, "allows_passive_label_only_teacher_objects":true,
                "preserves_final_private_current":true, "requires_admitted_or_fresh_session_nonce":true, "restore_only_bounds":[0,0],
                "max_decisions":256, "max_world_ticks":8192, "max_deadline_seconds":600
            })
        );
        return Ok(());
    }
    let (source, output, request_path) = arguments(args)?;
    // Bound request parsing too, before any checkpoint read or output creation.
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(request_path)?
        .take(4097)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err("retention request exceeds 4096 bytes".into());
    }
    let request: alife_game_app::SavedLifeRetentionRequest = serde_json::from_slice(&bytes)?;
    let result = std::thread::Builder::new()
        .name("saved-life-retention".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            alife_game_app::evaluate_saved_life_retention(&source, &output, request)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "saved-life retention worker panicked")??;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_exactly_three_nonempty_paths() {
        for args in [
            vec![],
            vec!["current"],
            vec!["current", "out"],
            vec!["current", "out", "request", "extra"],
            vec!["current", "", "request"],
        ] {
            assert!(arguments(args.into_iter().map(OsString::from).collect()).is_err());
        }
        assert_eq!(
            arguments(vec!["current".into(), "out".into(), "request".into()]).unwrap(),
            ("current".into(), "out".into(), "request".into())
        );
    }
}
