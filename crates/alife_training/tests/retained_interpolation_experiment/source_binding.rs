use super::*;
use alife_world::PortableSaveFile;

#[test]
fn projection_is_derived_from_exact_selected_assets_and_old_detached_input_is_rejected() {
    let inputs = fixture(1);
    let original = serde_json::to_vec(&inputs).unwrap();
    let (_, binding) = derive_retained_projection_from_current(
        &inputs.pc_current,
        &inputs.expected_pc_current_sha256,
    )
    .unwrap();
    let current = CurrentTrainingCheckpointV1::decode(&inputs.pc_current).unwrap();
    assert_eq!(
        binding.source_current_sha256,
        inputs.expected_pc_current_sha256
    );
    let life = current.life.as_ref().unwrap();
    let save = PortableSaveFile::from_json_str(&life.world_json).unwrap();
    for (role, id) in [
        ("phenotype", "phenotype"),
        ("compiler_inputs", "compiler_inputs"),
        ("topology", "live_topology"),
        ("exact_cognition", "exact"),
        ("lifetime", "lifetime"),
        ("fast", "fast"),
    ] {
        let entry = save
            .assets
            .entries
            .iter()
            .find(|e| e.asset_id == id)
            .unwrap();
        assert_eq!(
            binding.selected_asset_sha256[role],
            native_receipt::sha256(&life.asset_bytes(&entry.relative_path).unwrap())
        );
    }
    assert_eq!(
        binding.complete_declared_asset_count,
        save.assets.entries.len()
    );
    let candidate = prepare_retained_interpolation_experiment(inputs.clone()).unwrap();
    assert_eq!(candidate.receipt().pc_projection_binding, binding);
    assert!(candidate
        .receipt()
        .qualification
        .contains("full-native-recovery/runtime/skills-unqualified"));
    // A matching graph and a self-consistent detached payload no longer grant
    // permission to substitute neural state for the selected source's assets.
    let mut old = serde_json::to_value(&inputs).unwrap();
    old["pc_retained_projection_json"] = serde_json::json!([123, 125]);
    assert!(serde_json::from_value::<InterpolationExperimentInputs>(old).is_err());
    assert_eq!(serde_json::to_vec(&inputs).unwrap(), original);
}

#[test]
fn source_hash_pins_reject_different_same_graph_archives_and_missing_private_life() {
    let inputs = fixture(1);
    let other = fixture(0);
    let mut substituted = inputs.clone();
    substituted.pc_current = other.pc_current;
    let error = prepare_retained_interpolation_experiment(substituted.clone()).unwrap_err();
    assert!(error.to_string().contains("explicit SHA256 pin"));
    assert!(derive_retained_projection_from_current(
        &substituted.pc_current,
        &inputs.expected_pc_current_sha256
    )
    .is_err());
    let mut wrong_cloud = inputs.clone();
    wrong_cloud.expected_cloud_current_sha256 = "0".repeat(64);
    assert!(prepare_retained_interpolation_experiment(wrong_cloud).is_err());
    let mut current = CurrentTrainingCheckpointV1::decode(&inputs.pc_current).unwrap();
    current.life = None;
    substituted.pc_current = current.encode().unwrap();
    repin(&mut substituted);
    assert!(prepare_retained_interpolation_experiment(substituted)
        .unwrap_err()
        .to_string()
        .contains("private life missing"));
}

#[test]
fn native_binding_rejects_corrupt_closure_refs_identity_and_resealed_cognitive_substitution() {
    let inputs = fixture(1);
    let original = serde_json::to_vec(&inputs).unwrap();
    let cloud = CurrentTrainingCheckpointV1::decode(&inputs.cloud_current).unwrap();
    let current = CurrentTrainingCheckpointV1::decode(&inputs.pc_current).unwrap();
    let p = current.handoff.admit().unwrap().phenotype;
    for variant in 0..15 {
        let invalid = source_fixture::rewrite(&inputs, |life, save| match variant {
            0 => life.insert_asset("fast.json".into(), b"{}"),
            1 => {
                *save
                    .assets
                    .entries
                    .iter_mut()
                    .find(|e| e.asset_id == "fast")
                    .unwrap()
                    .size_bytes
                    .as_mut()
                    .unwrap() += 1;
            }
            2 => life.insert_asset("orphan.json".into(), b"{}"),
            3 => {
                life.assets_base64.remove("fast.json");
            }
            4 => save.assets.entries.push(
                save.assets
                    .entries
                    .iter()
                    .find(|e| e.asset_id == "fast")
                    .unwrap()
                    .clone(),
            ),
            5 => {
                let value = life.assets_base64.remove("fast.json").unwrap();
                life.assets_base64.insert("../fast.json".into(), value);
                save.assets
                    .entries
                    .iter_mut()
                    .find(|e| e.asset_id == "fast")
                    .unwrap()
                    .relative_path = "../fast.json".into();
            }
            6 => {
                save.creatures[0]
                    .gpu_brain
                    .as_mut()
                    .unwrap()
                    .active_weight_generation += 1
            }
            7 => {
                save.creatures[0]
                    .gpu_brain
                    .as_mut()
                    .unwrap()
                    .active_weight_bank ^= 1
            }
            8 | 9 | 13 => {
                let mut exact =
                    ExactCognitiveCheckpointState::decode(&life.asset_bytes("exact.json").unwrap())
                        .unwrap();
                if variant == 8 {
                    exact.organism_id = OrganismId(99);
                    exact.cognitive_context = CognitiveContextFrame::empty(
                        OrganismId(99),
                        ExperienceSequenceId(1),
                        Tick::ZERO,
                    )
                    .unwrap();
                } else if variant == 9 {
                    exact.structural_plasticity =
                        alife_gpu_backend::GpuV11Checkpoint::canonical_for_phenotype(&p)
                            .unwrap()
                            .structural;
                } else {
                    exact.checkpoint_tick = Tick(1);
                    exact.cognitive_context = CognitiveContextFrame::empty(
                        exact.organism_id,
                        ExperienceSequenceId(1),
                        Tick(1),
                    )
                    .unwrap();
                    exact.sleep_state = SleepState::awake_at(Tick(1));
                }
                source_fixture::replace_selected_asset(
                    life,
                    save,
                    "exact",
                    exact.encode().unwrap(),
                );
            }
            10 => source_fixture::replace_selected_asset(
                life,
                save,
                "compiler_inputs",
                serde_json::to_vec(&cloud.handoff.compiler_inputs).unwrap(),
            ),
            11 => {
                save.assets
                    .entries
                    .iter_mut()
                    .find(|e| e.asset_id == "fast")
                    .unwrap()
                    .presence = alife_world::AssetPresence::Optional
            }
            12 => {
                save.creatures[0]
                    .gpu_brain
                    .as_mut()
                    .unwrap()
                    .fast_weights
                    .digest = PortableAssetDigest::for_bytes(b"different")
            }
            14 => {
                let entry = save
                    .assets
                    .entries
                    .iter_mut()
                    .find(|e| e.asset_id == "fast")
                    .unwrap();
                entry.asset_id = "compiler_inputs".into();
            }
            _ => unreachable!(),
        });
        assert!(
            derive_retained_projection_from_current(
                &invalid.pc_current,
                &invalid.expected_pc_current_sha256
            )
            .is_err(),
            "source variant {variant} admitted"
        );
        assert!(
            prepare_retained_interpolation_experiment(invalid).is_err(),
            "candidate variant {variant} admitted"
        );
    }
    assert_eq!(serde_json::to_vec(&inputs).unwrap(), original);
}
