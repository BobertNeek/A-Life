//! Builds against the real shared evaluator and authentic asset-free World.
//! CPU headless actuator test supplies its own Step; it is not a native R44 action result.
use std::{fs,path::PathBuf,time::{SystemTime,UNIX_EPOCH}};
fn unique(label:&str)->PathBuf{
    let t=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let p=std::env::temp_dir().join(format!("qualia-armed-{label}-{}-{t}",std::process::id()));
    fs::create_dir(&p).unwrap();
    p.canonicalize().unwrap()
}
#[cfg(not(feature="wsp-cue-latch-armed"))]
#[test]
fn armed_schema13_is_off_before_current_or_output(){
    let r=unique("off");
    let m=r.join("experiment.json");
    fs::write(&m,b"{\"schema\":13}\n").unwrap();
    let c=r.join("absent-current");
    let o=r.join("absent-output");
    let e=alife_game_app::evaluate_saved_life_retention_experiment(&r,&c,&o,&m).unwrap_err();
    assert_eq!(e.to_string(),"WSP armed cue latch feature is disabled");
    assert!(!c.exists()&&!o.exists());
    fs::remove_file(m).unwrap();fs::remove_dir(r).unwrap();
}
#[cfg(feature="wsp-cue-latch-armed")]
#[test]
fn arm_schema13_rejects_injected_target_before_current_or_output(){
    let r=unique("target");
    let m=r.join("experiment.json");
    let v=serde_json::json!({"schema":13,"scope":"qualia.saved_life.wsp_cue_latch_armed.v1",
        "language_sha256":"a".repeat(64),"admission_sha256":"b".repeat(64),"force_action_target":211});
    fs::write(&m,serde_json::to_vec(&v).unwrap()).unwrap();
    let c=r.join("absent-current");
    let o=r.join("absent-output");
    let e=alife_game_app::evaluate_saved_life_retention_experiment(&r,&c,&o,&m).unwrap_err().to_string();
    assert!(e.contains("unknown field") && e.contains("force_action_target"), "{e}");
    assert!(!c.exists()&&!o.exists());
    fs::remove_file(m).unwrap();fs::remove_dir(r).unwrap();
}
#[cfg(feature="wsp-cue-latch-armed")]
#[path="../src/gpu_live_runtime/wsp_armed_latch.rs"]
mod exact_detached_core;

#[cfg(feature="wsp-cue-latch-armed")]
#[test]
fn actual_world_paid_cpu_step_crosses_hearing_and_grab_boundaries(){
    use alife_core::{ActionCommand, ActionKind, ActionTarget, BiochemistryState, BrainCapacityClass, Confidence,
        CreatureGenome, DurationTicks, FoundationGeneticIdentity, HomeostaticSnapshot, Intensity, LanguageTokenId,
        OrganismId, PerceptionContextBlock, SensorProfile, Tick, Vec3f};
    use alife_world::{HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, WorldEditorSpawnSpec,
        WorldObjectKind, WorldOrganismRecord};
    let owner=OrganismId(3);
    let mut w=HeadlessScenarioBuilder::new(9811).agent("bound-cpu-resident",owner,Vec3f::ZERO)
        .food("a",Vec3f::new(1.155,0.0,0.500),0.6).build().unwrap();
    w.editor_spawn_object(WorldEditorSpawnSpec{
        label:"b".into(),kind:WorldObjectKind::Ball,organism_id:None,
        position:Vec3f::new(1.159,0.0,-0.500),nutrition:0.0,hazard_pain:0.0,radius:0.30,token_id:None
    }).unwrap();
    let body=w.entity_id("bound-cpu-resident").unwrap();
    let frame=|w:&mut HeadlessWorld| {
        let t=w.tick();
        let (d,b)=w.perception_frame_draft_with_bindings(owner,t,SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(t)).unwrap();
        (d.finalize(PerceptionContextBlock::empty()).unwrap(),b.unwrap())
    };
    let t=w.tick();
    let foundation=FoundationGeneticIdentity::new(10,1,7,BrainCapacityClass::N512_ID).unwrap();
    let genome=CreatureGenome::early_mammal_founder(0xE10_32A1,foundation).unwrap();
    let phenotype=genome.express().unwrap();
    let biology=BiochemistryState::new(&phenotype,t).unwrap();
    w.register_organism_record(WorldOrganismRecord::new(owner,body,genome,phenotype,biology,t).unwrap()).unwrap();
    let (a,b)=(w.entity_id("a").unwrap(),w.entity_id("b").unwrap());
    let words=[9_u16,14_u16].map(|x|LanguageTokenId::new(x).unwrap());
    let atom=w.emit_player_tokens(Some(owner),Vec3f::new(-5.980,0.0,0.0),words.to_vec()).unwrap();
    let(f,fb)=frame(&mut w);
    assert!(w.cue_latch_world_witness(&f,&fb).unwrap().hand_empty);
    assert_eq!(w.retained_language_pilot_grab_probe(owner,a),(false,false));
    assert_eq!(w.retained_language_pilot_grab_probe(owner,b),(false,false));
    let heard=f.sensory().language_context.heard_tokens.iter().flatten().collect::<Vec<_>>();
    assert_eq!(heard.iter().map(|x|x.token_id).collect::<Vec<_>>(),vec![9,14]);
    assert!(heard.iter().all(|v|v.utterance_id==atom.utterance_id && v.addressee==Some(owner)
          && v.confidence.raw()>0.0 && v.confidence.raw()<0.01));
    let rail=f.sensory().language_prior_neural_lanes();
    for (i,t) in [9_u16,14_u16].into_iter().enumerate(){
        for b in 0..8 {
            assert!(rail[i*8+b].abs()>0.0 && rail[i*8+b].abs()<0.01);
            assert_eq!(rail[i*8+b].is_sign_positive(),t&(1<<b)!=0);
        }
    }
    // Unit exercise of the real World actuator, not an inference policy action.
    let next=Tick::new(t.raw()+1);
    let cmd=ActionCommand::structured(owner,HeadlessActionIds::STEP_FORWARD,ActionKind::Move,
        ActionTarget::NONE,Intensity::new(0.55).unwrap(),DurationTicks::new(1),
        Confidence::new(0.9).unwrap(),0,None,None,None).unwrap();
    let out=w.apply_registered_command(&cmd,body,next).unwrap();
    assert!(out.action_result.execution.succeeded);
    assert!(out.action_result.execution.physical.energy_cost.raw()>0.);
    assert_eq!(w.try_advance_tick().unwrap(),next);
    let d=w.entity(body).unwrap().position;
    assert!(d.x>0.020 && d.x<0.10);
    assert_eq!(w.audible_utterances(),vec![atom]);
    let f2=frame(&mut w).0;
    assert!(f2.sensory().language_context.heard_tokens.iter().all(Option::is_none));
    assert_eq!(w.retained_language_pilot_grab_probe(owner,a),(false,true));
    assert_eq!(w.retained_language_pilot_grab_probe(owner,b),(false,true));
    w.try_advance_tick().unwrap();
    assert!(w.audible_utterances().is_empty()); // actual World expiry, not masking or synthetic retention
}
