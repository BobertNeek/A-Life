//! Device-free, zero-step source construction for disposable scenario discovery.
use std::path::Path;

/// Initialize only. This deliberately bypasses care episode preparation and its
/// teacher, biology, decision collection, replay, and optimizer paths.
pub fn initialize_cpu_care_founder(output: &Path, seed: u64) -> super::Result<serde_json::Value> {
    alife_training::cpu::initialize_cpu_untrained_founder(
        output,
        seed,
        crate::initial_n2048_care_asset(seed)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_training::{cpu::read_cpu_founder_source, native_receipt::sha256};
    use serde_json::{json, Value};
    use std::{collections::BTreeMap, fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!("alife-zero-step-founder-{}-{}-{}",
                std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); }
    }
    fn bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fs::read_dir(root).unwrap().map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name().to_str().unwrap().to_owned(), fs::read(entry.path()).unwrap())
        }).collect()
    }

    #[test]
    fn untrained_founder_constructs_zero_state_and_admits_without_running_a_life() {
        let directory = Directory::new();
        let source = directory.0.join("source");
        let result = initialize_cpu_care_founder(&source, 77_777).unwrap();
        assert_eq!(result["mode"], "construction_only_zero_updates_v1");
        assert_eq!(result["founder_seed"], 77_777);
        for key in ["world_steps", "training_rows", "actor_optimizer_updates", "value_optimizer_updates", "actor_optimizer_step", "value_optimizer_step"] {
            assert_eq!(result[key], 0, "{key}");
        }
        assert_eq!(result["teacher_present"], false);
        assert_eq!(result["semantic_prior_enabled"], false);
        let frozen = bytes(&source);
        assert_eq!(frozen.len(), 6); // No private checkpoint, trace, replay, or collector artifact.
        let admitted = read_cpu_founder_source(&source).unwrap();
        let actor = &admitted.handoff.actor;
        let value = &admitted.handoff.value;
        assert_eq!(actor.optimizer_step, 0);
        assert!(actor.first_moment.iter().chain(&actor.second_moment).all(|value| value.to_bits() == 0));
        assert!(actor.update_ages.iter().all(|age| *age == 0));
        assert_eq!(value.optimizer_step, 0);
        assert!(value.parameters.iter().all(|value| value.to_bits() == 0));
        assert_eq!(value.last_updated_policy_version, Some(1));
        assert_eq!(admitted.handoff.policy_version, 1);
        assert_eq!(admitted.receipt["mode"], "native_cpu_untrained_founder");
        assert_eq!(admitted.receipt["fixture"], true);
        assert_eq!(admitted.receipt["canonical_valid_decisions"], 0);
        assert_eq!(admitted.handoff.compiler_inputs.n2048_candidate_asset().unwrap().manifest().training_stage(), alife_core::TrainingStageManifest::bootstrap());
        assert_eq!(frozen["trained.alife-foundation"], crate::initial_n2048_care_asset(77_777).unwrap().encode_canonical().unwrap());
        for (name, contents) in &frozen { assert_eq!(result["source_sha256"][name], sha256(contents)); }

        // These are the frozen evaluator's actual source and seed admission
        // APIs. They compile/check state only; no evaluate/collect call follows.
        let validation = crate::validate_foundation_training_group_source(&source).unwrap();
        assert_eq!(validation["source_execution_domain"], "native-cpu");
        let history = directory.0.join("history.json");
        let pins = ["cpu-cycle.json", "trained.alife-foundation", "actor-checkpoint.json", "value-checkpoint.json"]
            .into_iter().map(|name| (name, sha256(&frozen[name]))).collect::<BTreeMap<_, _>>();
        fs::write(&history, serde_json::to_vec(&json!({"schema":1,"kind":"mixed-care-seed-history","complete":true,
            "source_sha256":pins,"consumed_seed_ranges":[[77_777,77_777]],"assessment_seeds":[99_001],"imports":[]})).unwrap()).unwrap();
        let output = directory.0.join("never-run-assessment");
        let provenance = crate::foundation_training_cycle::admit_native_cpu_care_assessment_seed(&source, &history, &output, 99_001, false).unwrap();
        assert_eq!(provenance["training_seed_reserved"], false);
        assert!(!output.exists());
        assert_eq!(bytes(&source), frozen);
        assert!(initialize_cpu_care_founder(&source, 77_777).is_err());
        assert_eq!(bytes(&source), frozen);
    }

    #[test]
    fn untrained_founder_rejects_changed_state_even_with_recomputed_sidecar_pins() {
        let directory = Directory::new();
        let source = directory.0.join("source");
        initialize_cpu_care_founder(&source, 53).unwrap();
        let original = bytes(&source);
        for mutation in ["actor-age", "actor-moment", "value-moment", "training-claim", "seed"] {
            let mut files = original.clone();
            let mut receipt: Value = serde_json::from_slice(&files["cpu-cycle.json"]).unwrap();
            let mut handoff: Value = serde_json::from_slice(&files["training-handoff.json"]).unwrap();
            match mutation {
                "actor-age" => handoff["actor"]["update_ages"][0] = json!(1),
                "actor-moment" => handoff["actor"]["first_moment"][0] = json!(0.25),
                "value-moment" => {
                    let width = handoff["value"]["feature_count"].as_u64().unwrap() as usize + 1;
                    handoff["value"]["parameters"][width] = json!(0.25);
                }
                "training-claim" => receipt["training_rows"] = json!(1),
                "seed" => { receipt["seed"] = json!(54); receipt["founder_seed_base"] = json!(54); },
                _ => unreachable!(),
            }
            for (name, value) in [("training-handoff.json", handoff.clone()), ("actor-checkpoint.json", handoff["actor"].clone()), ("value-checkpoint.json", handoff["value"].clone())] {
                let contents = serde_json::to_vec(&value).unwrap();
                receipt["native_output_sha256"][name] = json!(sha256(&contents));
                files.insert(name.into(), contents);
            }
            files.insert("cpu-cycle.json".into(), serde_json::to_vec(&receipt).unwrap());
            for (name, contents) in &files { fs::write(source.join(name), contents).unwrap(); }
            assert!(read_cpu_founder_source(&source).is_err(), "{mutation}");
        }
        for (name, contents) in &original { fs::write(source.join(name), contents).unwrap(); }
        fs::write(source.join("private-checkpoint.json"), b"unexpected lifetime").unwrap();
        assert!(read_cpu_founder_source(&source).is_err());
    }

    #[test]
    fn untrained_founder_invalid_seed_and_existing_output_are_nonmutating() {
        let directory = Directory::new();
        for seed in [0, u64::MAX] {
            let output = directory.0.join(seed.to_string());
            assert!(initialize_cpu_care_founder(&output, seed).is_err());
            assert!(!output.exists());
        }
        let output = directory.0.join("partial-source");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("keep"), b"existing uncertain evidence").unwrap();
        let before = bytes(&output);
        assert!(initialize_cpu_care_founder(&output, 53).is_err());
        assert_eq!(bytes(&output), before);
    }
}
