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

/// Ordinary copies must work after every original episode has disappeared.
pub(super) fn relocate_recovery(root: &Path, episodes: &[std::path::PathBuf]) {
    use super::{run, text};
    fn copy_tree(source: &Path, target: &Path) {
        fs::create_dir(target).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let path = entry.unwrap().path();
            let to = target.join(path.file_name().unwrap());
            if path.is_dir() {
                copy_tree(&path, &to);
            } else {
                fs::copy(path, to).unwrap();
            }
        }
    }
    let latest = episodes.last().unwrap();
    let expected = alife_training::cpu::read_cpu_founder_source(latest)
        .unwrap()
        .handoff;
    let expected_parent =
        alife_training::cpu::read_cpu_founder_source(&latest.join("source-native"))
            .unwrap()
            .handoff;
    let moved = root.join("ordinary-copy");
    let parent = root.join("standalone-founder-copy");
    copy_tree(latest, &moved);
    copy_tree(&latest.join("source-native"), &parent);
    let preserved = snapshot_tree(&moved);
    for episode in episodes {
        fs::remove_dir_all(episode).unwrap();
    }
    // Other synthetic derivatives must not accidentally supply an ancestor.
    for name in ["synthetic-group-source", "group-back-to-cpu"] {
        fs::remove_dir_all(root.join(name)).unwrap();
    }
    assert_eq!(
        alife_training::cpu::read_cpu_founder_source(&moved)
            .unwrap()
            .handoff,
        expected
    );
    assert_eq!(
        alife_training::cpu::read_cpu_founder_source(&parent)
            .unwrap()
            .handoff,
        expected_parent
    );
    verify_private_recovery(&moved);
    let pack = fs::read_dir(moved.join("source-lineage"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "pack"))
        .unwrap();
    let bytes = fs::read(&pack).unwrap();
    for corrupt in [false, true] {
        if corrupt {
            let mut bad = bytes.clone();
            let n = bad.len() / 2;
            bad[n] ^= 1;
            fs::write(&pack, bad).unwrap();
        } else {
            fs::remove_file(&pack).unwrap();
        }
        let output = root.join("invalid-ancestor-output");
        let result = run(&[
            "--previous",
            text(&moved),
            "--output",
            text(&output),
            "--seed",
            "899001",
            "--ticks",
            "2",
        ]);
        assert!(!result.status.success());
        assert!(!output.exists());
        fs::write(&pack, &bytes).unwrap();
    }
    for (source, name, seed) in [
        (&moved, "recovered-copy", "899002"),
        (&parent, "recovered-founder", "899003"),
    ] {
        let output = root.join(name);
        let result = run(&[
            "--previous",
            text(source),
            "--output",
            text(&output),
            "--seed",
            seed,
            "--ticks",
            "2",
        ]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        verify_recovery(source, &output);
        verify_private_recovery(&output);
    }
    assert_eq!(snapshot_tree(&moved), preserved);
}
