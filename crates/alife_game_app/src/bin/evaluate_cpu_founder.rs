//! Explicit bare-founder native CPU assessment. No training or promotion.
use std::{fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result=std::thread::Builder::new().stack_size(32*1024*1024).spawn(move || -> Result<serde_json::Value,String> {
        let result=match args.as_slice() {
            [flag,path] if flag=="--spec" => fs::read(path).map_err(|e|e.to_string()).and_then(|bytes|
                serde_json::from_slice(&bytes).map_err(|e|e.to_string())).and_then(|request|
                alife_game_app::evaluate_cpu_founder(request).map_err(|e|e.to_string())),
            [flag,path] if flag=="--verify" => alife_game_app::verify_cpu_founder_evaluation(Path::new(path)).map_err(|e|e.to_string()),
            [flag,left,right] if flag=="--compare" => alife_game_app::compare_cpu_founder_evaluations(Path::new(left),Path::new(right)).map_err(|e|e.to_string()),
            _=>Err("evaluate_cpu_founder --spec REQUEST.json | --verify OUTPUT | --compare LEFT RIGHT".into()),
        }; result
    })?.join().map_err(|_|"founder evaluator panicked; preserve partial output")??;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
