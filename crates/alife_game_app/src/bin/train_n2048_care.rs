//! Training-only entry point; never linked into the ordinary game executable.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let mode = args
        .next()
        .ok_or("usage: train_n2048_care --pilot|--cycle OUTPUT_DIRECTORY [TICKS]")?;
    if mode != "--pilot" && mode != "--cycle" && mode != "--resume-cycle" {
        return Err("usage: train_n2048_care --pilot|--cycle OUTPUT_DIRECTORY [TICKS] | --resume-cycle PREVIOUS_DIRECTORY OUTPUT_DIRECTORY [TICKS]".into());
    }
    let first_path = std::path::PathBuf::from(args.next().ok_or("missing output directory")?);
    let (previous, output) = if mode == "--resume-cycle" {
        (
            Some(first_path),
            std::path::PathBuf::from(args.next().ok_or("missing new output directory")?),
        )
    } else {
        (None, first_path)
    };
    let ticks = args
        .next()
        .map(|s| s.to_string_lossy().parse::<usize>())
        .transpose()?
        .unwrap_or(32);
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    if mode == "--pilot" {
        let receipt = alife_game_app::run_foundation_training_pilot(&output, 0x2026_0921, ticks)?;
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    } else {
        // The production runtime and replay buffers have large debug-build
        // stack frames on Windows. Keep the CLI's main thread small.
        let receipt = std::thread::Builder::new()
            .name("foundation-training-cycle".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                let result = if let Some(previous) = previous {
                    alife_game_app::resume_foundation_training_cycle(
                        &previous,
                        &output,
                        0x2026_0921,
                        ticks,
                    )
                } else {
                    alife_game_app::run_foundation_training_cycle(&output, 0x2026_0921, ticks)
                };
                result.map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "training cycle thread panicked")??;
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    }
    Ok(())
}
