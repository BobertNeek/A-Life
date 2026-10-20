//! Export an existing validated native serial source for offline CPU transfer.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage =
        "export_native_training_handoff SOURCE NEW_OUTPUT (read-only source; no GPU execution)";
    let source = args.next().ok_or(usage)?;
    if source == "--help" {
        println!("{usage}");
        return Ok(());
    }
    let output = args.next().ok_or(usage)?;
    if args.next().is_some() {
        return Err(usage.into());
    }
    let binding = alife_game_app::export_native_training_handoff(
        std::path::Path::new(&source),
        std::path::Path::new(&output),
    )?;
    println!("{}", serde_json::to_string_pretty(&binding)?);
    Ok(())
}
