fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|v| v == "--gradient-regression") {
        if args.len() != 1 {
            return Err("usage: qualify_critic_residual --gradient-regression".into());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&alife_game_app::run_critic_gradient_regression()?)?
        );
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|v| v == "--capture" || v == "--capture-summary")
    {
        let summary = args[0] == "--capture-summary";
        if args.len() != 6 {
            return Err("usage: qualify_critic_residual --capture CHECKPOINT OUTPUT SEED DECISIONS TEMPERATURE".into());
        }
        let previous = std::path::PathBuf::from(&args[1]);
        let output = std::path::PathBuf::from(&args[2]);
        let receipt_path = output.join("critic-capture.json");
        let seed = args[3].to_string_lossy().parse()?;
        let decisions = args[4].to_string_lossy().parse()?;
        let temperature = args[5].to_string_lossy().parse()?;
        let result = std::thread::Builder::new()
            .name("critic-capture-only".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                alife_game_app::capture_foundation_critic_trajectory(
                    &previous,
                    &output,
                    seed,
                    decisions,
                    temperature,
                )
                .map_err(|e| e.to_string())
            })?
            .join()
            .map_err(|_| "critic capture worker panicked")??;
        if summary {
            println!(
                "{}",
                serde_json::json!({"receipt":receipt_path,"seed":result["seed"],
                "transition_count":result["transition_count"],"actor_policy":result["actor_policy"],
                "features_sha256":result["features_sha256"]})
            );
        } else {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        return Ok(());
    }
    if args.first().is_some_and(|v| v == "--evaluate-capture") {
        if args.len() != 5 {
            return Err("usage: qualify_critic_residual --evaluate-capture CAPTURE BASELINE_VALUE CANDIDATE_VALUE OUTPUT".into());
        }
        let paths = args[1..]
            .iter()
            .map(std::path::PathBuf::from)
            .collect::<Vec<_>>();
        let result = std::thread::Builder::new()
            .name("critic-heldout-evaluation".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                alife_game_app::evaluate_critic_capture(&paths[0], &paths[1], &paths[2], &paths[3])
                    .map_err(|e| e.to_string())
            })?
            .join()
            .map_err(|_| "critic heldout worker panicked")??;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    if args.len() != 2 {
        return Err("usage: qualify_critic_residual MANIFEST OUTPUT".into());
    }
    let manifest = std::path::PathBuf::from(&args[0]);
    let output = std::path::PathBuf::from(&args[1]);
    let result = std::thread::Builder::new()
        .name("critic-only-qualification".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            alife_game_app::run_critic_qualification(&manifest, &output).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "critic qualification worker panicked")??;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
