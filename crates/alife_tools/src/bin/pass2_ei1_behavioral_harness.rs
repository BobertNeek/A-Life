use std::{env, error::Error, io, path::PathBuf};

use alife_tools::pass2_ei1_behavioral::run_planned_manifest;

fn main() -> Result<(), Box<dyn Error>> {
    run_planned_manifest(
        &required_path("--planned-manifest")?,
        &required_string("--manifest-identity")?,
    )?;
    Ok(())
}

fn required_path(flag: &str) -> Result<PathBuf, Box<dyn Error>> {
    Ok(PathBuf::from(required_string(flag)?))
}

fn required_string(flag: &str) -> Result<String, Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    arguments
        .by_ref()
        .position(|argument| argument == flag)
        .and_then(|_| arguments.next())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("missing {flag}")))
        .map_err(Into::into)
}
