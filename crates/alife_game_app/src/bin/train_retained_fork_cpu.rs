use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    let result=match a.as_slice() {
        [mode,spec] if mode=="run"=>alife_game_app::run_pc_donor_retained_training_fork(Path::new(spec))?,
        [mode,root,pin,experiment] if mode=="verify"=>alife_game_app::verify_pc_donor_retained_training_fork(Path::new(root),pin,experiment)?,
        _=>return Err("usage: train_retained_fork_cpu run REQUEST | verify FORK EXPECTED_MANIFEST_SHA256 EXPECTED_EXPERIMENT_SHA256".into()),
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
