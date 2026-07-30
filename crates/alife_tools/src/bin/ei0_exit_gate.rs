use std::{env, path::PathBuf};

use alife_tools::ei0_exit_gate::{
    default_report_path, execute_ei0_exit_gate, write_ei0_exit_gate_report,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let mut output = None;
    let mut evidence_root = None;
    while let Some(argument) = args.next() {
        if argument == "--out" {
            output = Some(PathBuf::from(args.next().ok_or("--out requires a path")?));
        } else if argument == "--evidence-root" {
            evidence_root = Some(PathBuf::from(
                args.next().ok_or("--evidence-root requires a path")?,
            ));
        } else {
            return Err(format!("unknown argument: {}", argument.to_string_lossy()).into());
        }
    }

    let evidence_root = evidence_root.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ei0-exit-gate-evidence")
    });
    let output = output.unwrap_or_else(|| default_report_path(&evidence_root));
    let execution = execute_ei0_exit_gate(&evidence_root);
    write_ei0_exit_gate_report(&output, &execution.report)?;
    println!("report={}", output.display());
    println!(
        "era0_exit_gate_passed={}",
        execution.report.verdict.era0_exit_gate_passed
    );
    if let Some(error) = execution.operational_error {
        return Err(error.into());
    }
    if !execution.report.verdict.era0_exit_gate_passed {
        return Err("Era 0 exit gate failed; inspect the emitted report".into());
    }
    Ok(())
}
