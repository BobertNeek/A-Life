//! Direct native teacher/prior-off assessment in a distinct example executable.
//! Accepts the existing request schema; it never changes admission or learning.

#[cfg(not(feature = "cpu-training"))]
fn main() {
    eprintln!("requires --features cpu-training");
    std::process::exit(2);
}

#[cfg(feature = "cpu-training")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = std::path::PathBuf::from(
        args.next()
            .ok_or("usage: assess_cpu_care ASSESSMENT.json")?,
    );
    if args.next().is_some() || std::fs::metadata(&path)?.len() > 1024 * 1024 {
        return Err("expected one bounded native assessment request".into());
    }
    let request: alife_game_app::CpuCareEvaluationRequest =
        serde_json::from_slice(&std::fs::read(path)?)?;
    let result = alife_game_app::evaluate_cpu_care(request)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
