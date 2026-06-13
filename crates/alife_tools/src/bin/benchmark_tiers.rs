use std::path::PathBuf;

use alife_tools::benchmark::{BenchmarkHarness, BenchmarkHarnessConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let full = args.iter().any(|arg| arg == "--all");
    let output_dir = args
        .windows(2)
        .find(|window| window[0] == "--out")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from("target").join("artifacts"));
    let config = if full {
        BenchmarkHarnessConfig::manual_full()
    } else {
        BenchmarkHarnessConfig::smoke()
    };
    let report = BenchmarkHarness::run(config)?;
    let path = report.write_markdown(&output_dir)?;
    println!("{}", path.display());
    Ok(())
}
