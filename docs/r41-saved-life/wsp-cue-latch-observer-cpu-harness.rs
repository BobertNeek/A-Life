//! Asset-free CPU qualification; no checkpoint or GPU.
use std::{fs, time::{SystemTime, UNIX_EPOCH}};

fn local_scope(label:&str) -> std::path::PathBuf {
    let t=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let p=std::env::temp_dir().join(format!("qualia-latch-{label}-{}-{t}",std::process::id()));
    fs::create_dir(&p).unwrap();p.canonicalize().unwrap()
}

#[cfg(not(feature="wsp-cue-latch-observer"))]
#[test]
fn actual_shared_admission_refuses_latch_off_before_any_saved_life_access() {
    let root=local_scope("off");
    let manifest=root.join("experiment.json");
    let world=root.join("must-remain-absent-current");
    let output=root.join("must-remain-absent-output");
    fs::write(&manifest,b"{\"schema\":9}\n").unwrap();
    let error=alife_game_app::evaluate_saved_life_retention_experiment(&root,&world,&output,&manifest).unwrap_err();
    assert_eq!(error.to_string(),"WSP CUE LATCH observer feature is disabled");
    assert!(!world.exists() && !output.exists());
    fs::remove_file(manifest).unwrap();fs::remove_dir(root).unwrap();
}

#[cfg(feature="wsp-cue-latch-observer")]
#[test]
fn actual_shared_admission_does_not_accept_a_selector_address_in_the_new_observer_mode() {
    let root=local_scope("bad-selector");
    let manifest=root.join("experiment.json");
    let world=root.join("must-remain-absent-current");
    let output=root.join("must-remain-absent-output");
    let bytes=serde_json::to_vec(&serde_json::json!({"schema":9,
        "scope":"qualia.saved_life.wsp_cue_latch_observer.v1","language_manifest_sha256":"a".repeat(64),
        "admission_sha256":"b".repeat(64),"selector_override":211})).unwrap();
    fs::write(&manifest,bytes).unwrap();
    let res=alife_game_app::evaluate_saved_life_retention_experiment(&root,&world,&output,&manifest);
    let e=res.unwrap_err().to_string();
    assert!(e.contains("unknown field") && e.contains("selector_override"));
    assert!(!world.exists() && !output.exists());
    fs::remove_file(manifest).unwrap();fs::remove_dir(root).unwrap();
}

#[cfg(feature="wsp-cue-latch-observer")]
#[test]
fn actual_world_track_binding_and_acoustic_expiry_are_not_fixture_results() {
    use alife_core::{HomeostaticSnapshot,LanguageTokenId,OrganismId,PerceptionContextBlock,SensorProfile,Vec3f};
    use alife_core::{BiochemistryState,BrainCapacityClass,CreatureGenome,
        FoundationGeneticIdentity,ScaffoldContractError};
    use alife_world::{HeadlessScenarioBuilder,WorldOrganismRecord};
    let owner=OrganismId(3);
    let mut world=HeadlessScenarioBuilder::new(9811)
        .agent("test-resident",owner,Vec3f::ZERO)
        .food("object-a",Vec3f::new(2.1,0.0,0.4),0.6)
        .toy("object-b",Vec3f::new(2.0,0.0,-0.5),true).build().unwrap();
    let age0=world.tick();
    // The builder inserts a physical Agent, not authoritative living biology.
    // Prove that this real body alone is rejected, then register it via the
    // ordinary World API using an in-code founder (no checkpoint or assets).
    assert!(world.organism_registry().get(owner).is_none());
    let (draft,binding)=world.perception_frame_draft_with_bindings(owner,age0,
        SensorProfile::GroundedTerrainVisionV1,HomeostaticSnapshot::baseline(age0)).unwrap();
    let frame=draft.finalize(PerceptionContextBlock::empty()).unwrap();
    assert_eq!(world.cue_latch_world_witness(&frame,&binding.unwrap()).unwrap_err(),
        ScaffoldContractError::InvalidDecisionEvidence);
    let body=world.entity_id("test-resident").unwrap();
    let founder=FoundationGeneticIdentity::new(10,1,7,BrainCapacityClass::N512_ID).unwrap();
    let genome=CreatureGenome::early_mammal_founder(0xE10_32A1,founder).unwrap();
    let phenotype=genome.express().unwrap();
    let biochemistry=BiochemistryState::new(&phenotype,age0).unwrap();
    world.register_organism_record(
        WorldOrganismRecord::new(owner,body,genome,phenotype,biochemistry,age0).unwrap()
    ).unwrap();
    let resident=world.organism_registry().get(owner).unwrap();
    assert!(resident.lifecycle().is_alive());
    assert_eq!(resident.world_entity_id(),body);
    let slot=Vec3f::new(0.25,0.0,0.0);
    let words=[9_u16,1].map(|x|LanguageTokenId::new(x).unwrap());
    let atom=world.emit_player_tokens(Some(owner),slot,words.to_vec()).unwrap();
    let mut tracks=None;let mut previous=None;
    for j in 0..3 {
        let tick=world.tick();
        let (draft,binding)=world.perception_frame_draft_with_bindings(owner,tick,
            SensorProfile::GroundedTerrainVisionV1,HomeostaticSnapshot::baseline(tick)).unwrap();
        let b=binding.unwrap();
        let frame=draft.finalize(PerceptionContextBlock::empty()).unwrap();
        let proof=world.cue_latch_world_witness(&frame,&b)
            .unwrap_or_else(|e|panic!("registered World witness failed at cut {j}, tick {}: {e:?}",tick.raw()));
        assert!(proof.hand_empty && proof.tracks.len()>=2);
        assert_eq!((proof.organism,proof.tick,proof.frame_digest),(owner,tick,frame.frame_digest()));
        if let Some((old_frame,old_bound))=&previous {
            assert!(world.cue_latch_world_witness(old_frame,old_bound).is_err());
        }
        let ordered=proof.tracks.iter().map(|r|(r.observed_track,r.physical_spawn_key)).collect::<Vec<_>>();
        if let Some(old)=&tracks { assert_eq!(old,&ordered); }
        tracks=Some(ordered);
        let heard=frame.sensory().language_context.heard_tokens.iter().flatten().collect::<Vec<_>>();
        if j<2 {
            assert_eq!(heard.iter().map(|h|h.token_id).collect::<Vec<_>>(),[9,1]);
            assert!(heard.iter().all(|h|h.utterance_id==atom.utterance_id && h.addressee==Some(owner)));
            assert_eq!(world.audible_utterances(),vec![atom.clone()]);
        } else {
            assert!(heard.is_empty() && world.audible_utterances().is_empty());
        }
        previous=Some((frame,b));
        if j<2 { world.try_advance_tick().unwrap(); }
    }
    assert_eq!(world.tick().raw(),age0.raw()+2);
}
