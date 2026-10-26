//! Disposable serial-history measurements and complete recovery checks.
use super::snapshot_tree;
use std::{fs, path::Path};

pub(super) fn measure_history(episodes: &[&Path]) {
    use sha2::{Digest, Sha256};
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    #[cfg(unix)]
    let mut inodes = std::collections::BTreeSet::new();
    #[cfg(unix)]
    let mut inode_allocated_bytes = 0u64;
    let mut contents = std::collections::BTreeMap::<String, (usize, Vec<String>)>::new();
    let mut cumulative_files = 0;
    let mut cumulative_bytes = 0;
    for (episode, directory) in episodes.iter().enumerate() {
        verify_private_recovery(directory);
        let tree = snapshot_tree(directory);
        let mut files = 0;
        let mut bytes = 0;
        let mut categories = std::collections::BTreeMap::<String, (usize, usize)>::new();
        for (path, data) in &tree {
            let Some(data) = data else { continue };
            #[cfg(unix)]
            {
                let metadata = fs::metadata(directory.join(path)).unwrap();
                if inodes.insert((metadata.dev(), metadata.ino())) {
                    inode_allocated_bytes += metadata.blocks() * 512;
                }
            }
            files += 1;
            bytes += data.len();
            let category = if path.components().count() == 1 {
                path.to_string_lossy().into_owned()
            } else {
                path.components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy()
                    .into_owned()
            };
            let total = categories.entry(category).or_default();
            total.0 += 1;
            total.1 += data.len();
            contents
                .entry(format!("{:x}", Sha256::digest(data)))
                .or_insert_with(|| (data.len(), Vec::new()))
                .1
                .push(format!("{}/{}", episode + 1, path.display()));
        }
        cumulative_files += files;
        cumulative_bytes += bytes;
        let allocation: Option<u64> = {
            #[cfg(unix)]
            {
                Some(inode_allocated_bytes)
            }
            #[cfg(not(unix))]
            {
                None
            }
        };
        println!(
            "SERIAL_STORAGE {}",
            serde_json::json!({
                "episode":episode+1,"files":files,"logical_bytes":bytes,
                "cumulative_files":cumulative_files,"cumulative_logical_bytes":cumulative_bytes,
                "cumulative_inode_allocated_bytes":allocation,
                "categories":categories,
            })
        );
    }
    let duplicate_bytes: usize = contents
        .values()
        .map(|(bytes, paths)| bytes * (paths.len() - 1))
        .sum();
    println!(
        "SERIAL_DUPLICATES {}",
        serde_json::json!({
            "logical_duplicate_bytes":duplicate_bytes,
            "large_groups":contents.values().filter(|(bytes, paths)|*bytes>=65536 && paths.len()>1).collect::<Vec<_>>(),
            "scope":"byte-identical content; not removable bytes or unique physical extents",
        })
    );
}

fn verify_private_recovery(directory: &Path) {
    use alife_training::cpu::{CpuCanonicalCheckpoint, CpuCanonicalHost, CpuFoundationTrainer};
    let inputs: alife_core::PhenotypeCompilerInputs = serde_json::from_slice(
        &fs::read(directory.join("collection-compiler-inputs.json")).unwrap(),
    )
    .unwrap();
    let actor = serde_json::from_slice(
        &fs::read(directory.join("collection-actor-checkpoint.json")).unwrap(),
    )
    .unwrap();
    let phenotype = alife_core::PhenotypeCompiler::compile_validated(
        &inputs,
        &alife_core::BrainCapacityClass::n2048(),
    )
    .unwrap();
    let founder = CpuFoundationTrainer::from_checkpoint(
        phenotype,
        inputs.n2048_candidate_asset().unwrap().clone(),
        actor,
    )
    .unwrap();
    let saved: CpuCanonicalCheckpoint =
        serde_json::from_slice(&fs::read(directory.join("collector-checkpoint.json")).unwrap())
            .unwrap();
    let restored = CpuCanonicalHost::from_checkpoint(saved.clone(), &founder).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.checkpoint(&founder).unwrap()).unwrap(),
        serde_json::to_vec(&saved).unwrap(),
    );
}

pub(super) fn verify_recovery(previous: &Path, next: &Path) {
    let source = alife_training::cpu::read_cpu_founder_source(previous).unwrap();
    for (name, expected) in [
        (
            "collection-compiler-inputs.json",
            serde_json::to_vec(&source.handoff.compiler_inputs).unwrap(),
        ),
        (
            "collection-actor-checkpoint.json",
            serde_json::to_vec(&source.handoff.actor).unwrap(),
        ),
        (
            "initial-value-checkpoint.json",
            serde_json::to_vec(&source.handoff.value).unwrap(),
        ),
    ] {
        assert_eq!(fs::read(next.join(name)).unwrap(), expected, "{name}");
    }
    assert_eq!(
        fs::read(next.join("collection-policy.alife-foundation")).unwrap(),
        fs::read(previous.join("trained.alife-foundation")).unwrap()
    );
    alife_training::cpu::read_cpu_founder_source(next).unwrap();
}
