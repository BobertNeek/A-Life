use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 || !matches!(args[0].as_str(), "inspect" | "battery") {
        return Err(
            "usage: assess_retained_cpu inspect|battery CURRENT NEW_OUTPUT EXPECTED_SHA256".into(),
        );
    }
    let result = alife_game_app::run_retained_cpu_battery(
        Path::new(&args[1]),
        Path::new(&args[2]),
        &args[3],
        args[0] == "inspect",
    )?;
    if args[0] == "inspect" {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        println!(
            "battery complete: {} trials",
            result["trials"].as_array().unwrap().len()
        );
    }
    Ok(())
}
