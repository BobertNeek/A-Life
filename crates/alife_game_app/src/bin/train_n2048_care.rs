//! Training-only entry point; never linked into the ordinary game executable.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--pilot")) {
        return Err("usage: train_n2048_care --pilot OUTPUT_DIRECTORY [TICKS]; campaign requires the fidelity gate first".into());
    }
    let output = std::path::PathBuf::from(args.next().ok_or("missing output directory")?);
    let ticks = args
        .next()
        .map(|s| s.to_string_lossy().parse::<usize>())
        .transpose()?
        .unwrap_or(32);
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let receipt = alife_game_app::run_foundation_training_pilot(&output, 0x2026_0921, ticks)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
