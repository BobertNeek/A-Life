//! Native, read-only archive validation. This executable never creates a GPU session.
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let source = PathBuf::from(
        args.next()
            .ok_or("usage: inspect_archived_success SEALED_CYCLE MANIFEST")?,
    );
    let manifest = PathBuf::from(args.next().ok_or("missing archived success manifest")?);
    if args.next().is_some() {
        return Err("unexpected archive inspection argument".into());
    }
    // Native replay records contain large fixed-size state frames. Keep their
    // read-only decoding off Windows' small default executable stack.
    let inspection = std::thread::Builder::new()
        .name("archive-inspection".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            alife_game_app::inspect_archived_successes(&source, &manifest)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "native archive inspection worker panicked")??;
    println!("{}", serde_json::to_string_pretty(&inspection)?);
    Ok(())
}
