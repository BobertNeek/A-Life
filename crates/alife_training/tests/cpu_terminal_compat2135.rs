#![cfg(feature = "cpu-backend")]
use alife_core::*;
use alife_training::cpu::*;
use alife_world::PlayerInteractionV1;
#[path = "support/terminal_parent2135.rs"]
mod parent;

// Compile this unchanged against 2d196898 to compare the actual original host.
#[test]
fn original_host_full_parent_compatibility_probe() {
    let (mut host, trainer) = parent::fixture(None);
    let output = parent::local_output();
    let mut transcript = Vec::new();
    transcript.push(serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap());
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    for _ in 0..3 {
        let step = host.step(&trainer, 1.0).unwrap();
        assert_eq!(step.decisions.len(), 1);
        transcript.push(serde_json::to_vec(&step.decisions).unwrap());
        transcript.push(serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap());
    }
    // Also compare actual delivery, biology and acquired learner/eligibility state.
    for kind in [
        PlayerInteractionV1::Praise,
        PlayerInteractionV1::Scold,
        PlayerInteractionV1::Pet,
        PlayerInteractionV1::Slap,
    ] {
        let mut branch = host.snapshot();
        let (id, entity) = branch.world().organism_entity_ids()[0];
        let position = branch.world().entity(entity).unwrap().position;
        branch
            .world_mut()
            .queue_player_interaction_v1(id, position, kind)
            .unwrap();
        let step = branch.step(&trainer, 1.0).unwrap();
        assert_eq!(step.decisions.len(), 1);
        step.decisions[0].patch.validate_contract().unwrap();
        transcript.push(serde_json::to_vec(&step.decisions).unwrap());
        transcript.push(serde_json::to_vec(&branch.checkpoint(&trainer).unwrap()).unwrap());
    }
    assert!(serde_json::to_vec(&trainer.checkpoint()).unwrap() == founder);
    if let Some(output) = output {
        for (index, bytes) in transcript.iter().enumerate() {
            std::fs::write(output.join(format!("{index:02}.json")), bytes).unwrap();
        }
        std::fs::write(output.join("founder.json"), founder).unwrap();
    }
}

// Explicitly run in an actual pre-feature executable with a real schema3 CP
// and real enabled row emitted by cpu_terminal_context2135. Not a projection.
#[test]
#[ignore = "requires external research CP/row and the pre-feature host executable"]
fn original_schema2_executable_rejects_research_artifacts() {
    let root = std::path::PathBuf::from(std::env::var_os("ALIFE_NATIVE_QUALIFY_INPUT").unwrap());
    let (_, trainer) = parent::fixture(None);
    let cp: CpuCanonicalCheckpoint =
        serde_json::from_slice(&std::fs::read(root.join("enabled.cp.json")).unwrap()).unwrap();
    assert_eq!(cp.schema_version, 3);
    let result = CpuCanonicalHost::from_checkpoint(cp, &trainer);
    assert!(
        result.is_err(),
        "old executable silently admitted the enabled head"
    );
    eprintln!(
        "actual pre-feature executable rejected schema3: {}",
        result.err().unwrap()
    );
    let row = std::fs::read(root.join("enabled.row.json")).unwrap();
    assert!(serde_json::from_slice::<CpuCollectedDecision>(&row).is_err());
}
