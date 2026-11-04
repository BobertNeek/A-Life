fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        return Err("usage: assess_actual_interpolation REQUEST.json".into());
    }
    let result =
        alife_game_app::run_actual_interpolation_assessment(std::path::Path::new(&args[0]))?;
    println!(
        "assessment phase complete: {} trials",
        result["trials"].as_array().unwrap().len()
    );
    Ok(())
}
