use alife_core::{DendriticBranchSet, FoundationWeightAsset};
use std::{env,fs,path::Path};
fn main() -> Result<(),Box<dyn std::error::Error>> {
 let args:Vec<String>=env::args().collect();
 let data:serde_json::Value=serde_json::from_slice(&fs::read(&args[1])?)?;
 let f:FoundationWeightAsset=serde_json::from_value(data["foundation"].clone())?;
 let b:DendriticBranchSet=serde_json::from_value(data["branches"].clone())?;
 let dir=Path::new(&args[2]);let bytes=f.encode_canonical()?;
 fs::write(dir.join("foundation.alife-foundation"),bytes)?;
 fs::write(dir.join("dendritic_branches.json"),serde_json::to_vec_pretty(&b)?)?;
 let digest=f.digest().bytes().iter().map(|x|format!("{x:02x}")).collect::<String>();
 println!("{}",serde_json::json!({"foundation_digest":digest,"weight_count":f.weights().len(),"dendritic_branch_count":b.branches().len()}));
 Ok(())
}
