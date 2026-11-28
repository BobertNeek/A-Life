//! Source qualification only: no private Current, evaluator command, device, or action.
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
fn scratch(label:&str)->PathBuf {
    let t=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let p=std::env::temp_dir().join(format!("qualia-crossed-{label}-{}-{t}",std::process::id()));
    fs::create_dir(&p).unwrap();
    p.canonicalize().unwrap()
}
fn refuse(label:&str, payload:serde_json::Value, needles:&[&str]){
    let root=scratch(label);
    let manifest=root.join("experiment.json");
    let current=root.join("absent-current");
    let output=root.join("absent-output");
    fs::write(&manifest,serde_json::to_vec(&payload).unwrap()).unwrap();
    let e=alife_game_app::evaluate_saved_life_retention_experiment(
        &root,&current,&output,&manifest
    ).unwrap_err().to_string();
    assert!(needles.iter().all(|x|e.contains(x)),"{e}");
    assert!(!current.exists()&&!output.exists());
    fs::remove_file(manifest).unwrap();fs::remove_dir(root).unwrap();
}
#[cfg(not(feature="wsp-out-of-reach-fixture"))]
#[test]
fn shared_crossed_schema16_off_refuses_before_current(){
    refuse("off",serde_json::json!({"schema":16}),&["WSP crossed out-of-reach fixture is disabled"]);
}
#[cfg(feature="wsp-out-of-reach-fixture")]
#[test]
fn shared_crossed_schema16_rejects_target_or_forced_action(){
    for extra in ["chosen_target","force_action","pose_m","synthetic_hearing"] {
        let mut x=serde_json::json!({"schema":16,"scope":"qualia.saved_life.wsp_crossed_out_of_reach_observer.v1",
              "admission_sha256":"a".repeat(64)});
        x[extra]=serde_json::json!(1);
        refuse(extra,x,&["unknown field",extra]);
    }
}
#[cfg(feature="wsp-out-of-reach-fixture")]
#[test]
#[ignore = "requires isolated invocation with ALIFE_EXPLICIT_RETENTION_ARCHIVE_TEST=1"]
fn shared_crossed_schema16_archive_refuses_before_device_or_restore(){
    assert_eq!(std::env::var("ALIFE_EXPLICIT_RETENTION_ARCHIVE_TEST").unwrap(),"1");
    refuse("archive-off",serde_json::json!({"schema":16,"scope":"qualia.saved_life.wsp_crossed_out_of_reach_observer.v1",
            "admission_sha256":"a".repeat(64)}),&["crossed fixture does not admit isolated archives"]);
}
#[cfg(all(feature="wsp-out-of-reach-fixture",feature="wsp-cue-latch-armed"))]
#[test]
fn shared_crossed_schema16_on_preserves_schema13_closed_wire(){
    refuse("legacy",serde_json::json!({"schema":13,"scope":"qualia.saved_life.wsp_cue_latch_armed.v1",
            "language_sha256":"a".repeat(64),"admission_sha256":"b".repeat(64),"choose_crossed_fixture":true}),
            &["unknown field","choose_crossed_fixture"]);
}
#[cfg(feature="wsp-out-of-reach-fixture")]
type Result<T> = std::result::Result<T,Box<dyn std::error::Error>>;
#[cfg(feature="wsp-out-of-reach-fixture")]
#[path="../src/foundation_training_cycle/saved_life_retention/wsp_out_of_reach_fixture.rs"]
mod exact_shared_world_fixture;
