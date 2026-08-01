use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;

use alife_tools::era1_promotion::run_era1_promotion_and_write;

fn main() -> Result<(), Box<dyn Error>> {
    let output_path = required_path("--out")?;
    run_era1_promotion_and_write(output_path)?;
    Ok(())
}

fn required_path(flag: &str) -> Result<PathBuf, Box<dyn Error>> {
    let mut arguments = env::args_os();
    let value = arguments
        .position(|argument| argument == flag)
        .and_then(|_| arguments.next())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("missing required {flag} path"),
            )
        })?;
    Ok(PathBuf::from(value))
}
