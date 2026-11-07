#[test]
#[ignore = "Manual capture on the pre-change release; fresh disposable life only"]
fn capture_disposable_pre_outcome_phase_checkpoint() {
    let root = PathBuf::from(std::env::var("ALIFE_OLD_NATIVE_FIXTURE").expect("fresh fixture destination"));
    fs::create_dir(&root).unwrap();
    let collection = collect_cpu_episode(
        fixture_config(root.join("collection"), 0x62_1801),
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    let inputs = collection.compiler_inputs().clone();
    let actor = collection.actor_checkpoint();
    let trainer = restore_trainer(&inputs, actor.clone());
    let checkpoint = collection.host_checkpoint().unwrap();
    assert!(checkpoint
        .residents
        .iter()
        .any(|r| !r.memory.records.is_empty()));
    fs::write(
        root.join("inputs.json"),
        serde_json::to_vec(&inputs).unwrap(),
    )
    .unwrap();
    fs::write(root.join("actor.json"), serde_json::to_vec(&actor).unwrap()).unwrap();
    fs::write(
        root.join("checkpoint.json"),
        serde_json::to_vec(&checkpoint).unwrap(),
    )
    .unwrap();
    let mut host = CpuCanonicalHost::from_checkpoint(checkpoint, &trainer).unwrap();
    let step = host.step(&trainer, 0.9).unwrap();
    fs::write(
        root.join("next-decisions.json"),
        serde_json::to_vec(&step.decisions).unwrap(),
    )
    .unwrap();
}
