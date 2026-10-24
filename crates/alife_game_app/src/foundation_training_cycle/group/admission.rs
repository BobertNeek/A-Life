//! Read-only seed admission. A lack of historical evidence never becomes freshness.
use super::super::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// The receipt name is part of the exact source hash contract; ambiguity is an error.
pub(super) fn source_name(previous: &Path) -> Result<&'static str> {
    let names = ["cycle.json", "group50.json", "cpu-cycle.json"]
        .into_iter()
        .filter(|name| previous.join(name).is_file())
        .collect::<Vec<_>>();
    match names.as_slice() {
        [name] => Ok(*name),
        [] => Err("source has no native training receipt".into()),
        _ => Err("source has ambiguous training receipt domains".into()),
    }
}

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Return only ancestry links whose embedded receipt matches its pinned bytes.
/// Existing native writers use serde JSON pretty/compact, optionally newline.
/// Exact retained source bytes also admit other source formatting at the root.
fn verified_source_links(previous: &Path) -> Result<Value> {
    let receipt_name = source_name(previous)?;
    if receipt_name == "cycle.json" {
        return Ok(Value::Null);
    }
    let (_, native) = read(&previous.join(receipt_name))?;
    let lineage = alife_training::native_receipt::Lineage::load(&native, previous)?;
    if let Some(error) = lineage.legacy_proof_error {
        return Err(error.into());
    }
    Ok(lineage.terminal_proof.unwrap_or(Value::Null))
}

// Reuse the accepted read-only recovery reconciliation, including its Python
// canonical ledger hashing and surviving accepted/abandoned attempt checks.
// This calls no plan/reservation/runner function and suppresses bytecode writes.
fn mixed_lane_admission(
    history: &Path,
    lane: &Path,
    previous: &Path,
    history_sha256: &str,
    known: bool,
) -> Result<()> {
    let root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !root.status.success() {
        return Err("mixed-lane admission requires the qualified source checkout".into());
    }
    let root = PathBuf::from(std::str::from_utf8(&root.stdout)?.trim());
    if !root.join("scripts/run_mixed_care_serial.py").is_file() {
        return Err("mixed-lane admission requires the accepted recovery validator".into());
    }
    let verified_links = match verified_source_links(previous) {
        Ok(link) => json!({"link":link,"error":null}),
        Err(error) => json!({"link":null,"error":error.to_string()}),
    };
    let output = std::process::Command::new("python")
        .args([
            "-B",
            "-c",
            r#"
import pathlib, sys, json, hashlib
sys.path.insert(0, str(pathlib.Path.cwd() / "scripts"))
import run_mixed_care_serial as mixed
document, digest = mixed.load(pathlib.Path(sys.argv[1]))
mixed.require(digest == sys.argv[3], "group history changed during recovery validation")
ledger = pathlib.Path(sys.argv[2]) / mixed.LEDGER
if sys.argv[5] == "known-exclusions":
    mixed.recovery_attestation(document, ledger)
runs = mixed.read_ledger(ledger)["runs"] if ledger.exists() else []
if sys.argv[5] == "known-exclusions":
    mixed.reconcile_prior_runs(document, runs)
if runs:
    prior = runs[-1]
    mixed.require(not any(a["status"] == "pending" for a in prior["attempts"]),
                  "prior mixed attempt is uncertain; reconcile it before group allocation")
    accepted = [a for a in prior["attempts"] if a["status"] == "accepted"]
    latest = pathlib.Path(accepted[-1]["directory"] if accepted else prior["plan"]["source"]).resolve()
    expected = accepted[-1]["sha256"] if accepted else prior["plan"]["source_sha256"]
    expected = {name: expected[name] for name in mixed.FILES}
    previous = pathlib.Path(sys.argv[4])
    # Rust canonicalization uses the Windows extended-length namespace. Compare
    # filesystem identity so ordinary and verbatim spellings share one lineage.
    matches = document["source_sha256"] == expected and previous.samefile(latest)
    if not matches:
        expected_link = hashlib.sha256(json.dumps(expected, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        proof = json.loads(sys.argv[6])
        mixed.require(proof["error"] is None, proof["error"] or "invalid training ancestry")
        link = proof["link"] or {}
        latest_receipt, latest_receipt_hash = mixed.inspector_io.load_json(latest / "cycle.json", mixed.seeds.MAX_DOCUMENT_BYTES)
        matches = (link.get("pin_set_sha256") == expected_link
                   and latest_receipt_hash == expected["cycle.json"]
                   and link.get("terminal_native_receipt") == latest_receipt)
    mixed.require(matches, "group source does not descend from the latest surviving mixed checkpoint")
"#,
        ])
        .arg(history.canonicalize()?)
        .arg(std::path::absolute(lane)?)
        .arg(history_sha256)
        .arg(previous.canonicalize()?)
        .arg(if known { "known-exclusions" } else { "strict" })
        .arg(serde_json::to_string(&verified_links)?)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "mixed-lane recovery/lineage admission failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

#[cfg(feature = "cpu-training")]
pub(super) fn verify_recovery(previous: &Path, history: &Path, lane: &Path) -> Result<()> {
    let (bytes, _) = read(history)?;
    mixed_lane_admission(history, lane, previous, &hash(&bytes), true)
}

pub(super) fn read(path: &Path) -> Result<(Vec<u8>, Value)> {
    Ok(alife_training::native_receipt::read(path)?)
}

fn exclusions(value: &Value, seed: u64, assessment: bool) -> Result<()> {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                // Assessment may replay explicitly held-out seeds across arms;
                // all training/consumed/world-seed evidence remains exclusionary.
                if assessment && key == "assessment_seed" {
                    value
                        .as_u64()
                        .filter(|seed| *seed > 0)
                        .ok_or("assessment seed declaration must be a positive scalar")?;
                    continue;
                }
                if key.ends_with("seed_ranges")
                    || key.ends_with("seed_range")
                    || key == "seed_range"
                {
                    let values = value.as_array().ok_or("invalid seed range evidence")?;
                    if values.len() > 100_000 {
                        return Err("seed ranges exceed bounded entry count".into());
                    }
                    let pairs: Vec<&Value> = if values.first().is_some_and(Value::is_u64) {
                        vec![value]
                    } else {
                        values.iter().collect()
                    };
                    for pair in pairs {
                        let pair = pair.as_array().ok_or("invalid excluded seed interval")?;
                        let [first, last] = pair.as_slice() else {
                            return Err("invalid excluded seed interval".into());
                        };
                        let first = first
                            .as_u64()
                            .filter(|v| *v > 0)
                            .ok_or("invalid excluded seed")?;
                        let last = last
                            .as_u64()
                            .filter(|v| *v >= first)
                            .ok_or("invalid excluded seed")?;
                        if (first..=last).contains(&seed) {
                            return Err("group seed overlaps a known exclusion".into());
                        }
                    }
                } else if key == "assessment_seeds" {
                    for value in value.as_array().ok_or("invalid assessment seed evidence")? {
                        let declared = value.as_u64().ok_or("invalid assessment seed")?;
                        if !assessment && declared == seed {
                            return Err("group seed overlaps a known assessment".into());
                        }
                    }
                } else if matches!(
                    key.as_str(),
                    "seed" | "source_seed" | "world_seed" | "assessment_seed"
                ) && value.as_u64() == Some(seed)
                {
                    return Err("group seed is a disclosed historical retry; new group admission rejects reuse".into());
                } else {
                    exclusions(value, seed, assessment)?;
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                exclusions(value, seed, assessment)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // bounded import state plus assessment role
fn import(
    path: &Path,
    expected_sha256: &str,
    seed: u64,
    seen: &mut BTreeSet<PathBuf>,
    records: &mut Vec<Value>,
    depth: usize,
    bytes_total: &mut u64,
    assessment: bool,
) -> Result<()> {
    if depth > 8 || records.len() >= 128 {
        return Err("seed evidence import budget exceeded".into());
    }
    let path = path.canonicalize()?;
    if !seen.insert(path.clone()) {
        return Err("duplicate/cyclic seed evidence import".into());
    }
    let (bytes, document) = read(&path)?;
    let sha256 = hash(&bytes);
    if sha256 != expected_sha256 {
        return Err("seed evidence import changed".into());
    }
    *bytes_total = bytes_total
        .checked_add(bytes.len() as u64)
        .ok_or("seed evidence byte count overflow")?;
    if *bytes_total > 64 * 1024 * 1024 {
        return Err("aggregate seed evidence exceeds64 MiB".into());
    }
    if document["schema"] != 1 {
        return Err("seed evidence requires schema 1".into());
    }
    exclusions(&document, seed, assessment)?;
    if let Some(imports) = document.get("imports") {
        for item in imports.as_array().ok_or("invalid seed evidence imports")? {
            let child = Path::new(item["path"].as_str().ok_or("missing seed import path")?);
            let child = if child.is_absolute() {
                child.to_owned()
            } else {
                path.parent().unwrap().join(child)
            };
            import(
                &child,
                item["sha256"]
                    .as_str()
                    .ok_or("missing seed import digest")?,
                seed,
                seen,
                records,
                depth + 1,
                bytes_total,
                assessment,
            )?;
        }
    }
    records.push(json!({"path":path,"sha256":sha256,"document":document}));
    Ok(())
}

pub(super) fn admit(
    previous: &Path,
    history: &Path,
    output: &Path,
    seed: u64,
    known_exclusions: bool,
) -> Result<Value> {
    admit_with_lane_snapshots(
        previous,
        history,
        output,
        seed,
        known_exclusions,
        false,
        false,
    )
}

#[cfg(any(test, feature = "cpu-training"))]
pub(super) fn admit_collection(
    previous: &Path,
    history: &Path,
    output: &Path,
    seed: u64,
    known_exclusions: bool,
) -> Result<Value> {
    admit_with_lane_snapshots(
        previous,
        history,
        output,
        seed,
        known_exclusions,
        true,
        false,
    )
}

#[cfg(feature = "cpu-training")]
pub(super) fn admit_assessment(
    previous: &Path,
    history: &Path,
    output: &Path,
    seed: u64,
    known_exclusions: bool,
) -> Result<Value> {
    admit_with_lane_snapshots(
        previous,
        history,
        output,
        seed,
        known_exclusions,
        false,
        true,
    )
}

fn admit_with_lane_snapshots(
    previous: &Path,
    history: &Path,
    output: &Path,
    seed: u64,
    known_exclusions: bool,
    allow_immutable_lane_snapshots: bool,
    assessment: bool,
) -> Result<Value> {
    let (bytes, document) = read(history)?;
    let selected = json!({"policy":if known_exclusions {"known-exclusions"} else {"strict"},
        "historical_freshness":if known_exclusions {"unknown"} else {"operator-attested-complete"}});
    let recorded = document.get("seed_admission").cloned().unwrap_or_else(
        || json!({"policy":"strict","historical_freshness":"operator-attested-complete"}),
    );
    if document["kind"] != "mixed-care-seed-history"
        || document["schema"] != 1
        || (!known_exclusions && document["complete"] != true)
        || (known_exclusions && document["complete"] != false)
        || recorded != selected
    {
        return Err("strict admission requires complete attested seed history; incomplete history requires explicit known-exclusions".into());
    }
    if assessment
        && !document["assessment_seeds"]
            .as_array()
            .is_some_and(|seeds| seeds.iter().any(|value| value.as_u64() == Some(seed)))
    {
        return Err("care assessment requires an explicitly declared held-out seed".into());
    }
    let lane = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if known_exclusions || lane.join(".mixed-care-seed-reservations.json").exists() {
        mixed_lane_admission(history, lane, previous, &hash(&bytes), known_exclusions)?;
    }
    let source_name = source_name(previous)?;
    let (native_bytes, native) = read(&previous.join(source_name))?;
    let source_seed = native["seed"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or("native source lacks seed provenance")?;
    if source_seed == seed {
        return Err("group seed reuses the native source seed".into());
    }
    let lineage = alife_training::native_receipt::Lineage::load(&native, previous)?;
    lineage.check_parent(&native_bytes)?;
    for frame in lineage.frames() {
        exclusions(&frame?, seed, assessment)?;
    }
    let consumed = document["consumed_seed_ranges"]
        .as_array()
        .ok_or("history lacks consumed seed ranges")?;
    if !consumed.iter().any(|pair| {
        pair.as_array().is_some_and(|v| {
            v.len() == 2
                && v[0].as_u64().is_some_and(|first| first <= source_seed)
                && v[1].as_u64().is_some_and(|last| last >= source_seed)
        })
    }) {
        return Err("consumed history must include the native source seed".into());
    }
    let mut source_hashes = serde_json::Map::new();
    for name in [
        source_name,
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ] {
        source_hashes.insert(
            name.into(),
            json!(hash(&std::fs::read(previous.join(name))?)),
        );
    }
    if document["source_sha256"] != Value::Object(source_hashes.clone()) {
        return Err("seed history is not bound to the exact native source receipts".into());
    }
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    import(
        history,
        &hash(&bytes),
        seed,
        &mut seen,
        &mut records,
        0,
        &mut 0,
        assessment,
    )?;
    // Ordinary admission requires each actual ledger path as an exact-byte
    // import. Collect-only admission may retain an imported immutable snapshot,
    // but only when its verified raw digest matches the current ledger bytes.
    let mut lane_ledger_snapshots = Vec::new();
    {
        let lane = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        if lane.exists() {
            for entry in std::fs::read_dir(lane)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if entry.file_type()?.is_symlink() && name.starts_with('.') && name.contains("seed")
                {
                    return Err("seed reservation ledger must not be a symlink".into());
                }
                if name == ".grab-food-seed-reservations.json"
                    || name == ".mixed-care-seed-reservations.json"
                    || name.starts_with(".group50-seed-") && name.ends_with(".json")
                {
                    let ledger_path = entry.path().canonicalize()?;
                    if seen.contains(&ledger_path) {
                        continue;
                    }
                    if allow_immutable_lane_snapshots {
                        let (ledger_bytes, _) = read(&ledger_path)?;
                        let ledger_sha256 = hash(&ledger_bytes);
                        if let Some(record) = records.iter().find(|record| {
                            record["sha256"].as_str() == Some(ledger_sha256.as_str())
                        }) {
                            lane_ledger_snapshots.push(json!({
                                "ledger_path":ledger_path,
                                "snapshot_path":record["path"],
                                "sha256":ledger_sha256,
                            }));
                            continue;
                        }
                    }
                    return Err(format!(
                        "existing lane ledger requires an explicit exact-byte history import: {}",
                        entry.path().display()
                    )
                    .into());
                }
            }
        }
    }
    let mut provenance = json!({"schema":1,"world_seed":seed,"source_sha256":source_hashes,
        "seed_admission":selected,
        "historical_retry":false,"native_source_receipt":native,"evidence":records,"history_sha256":hash(&bytes)});
    if assessment {
        provenance["allocation"] = json!("declared-held-out-assessment");
        provenance["training_seed_reserved"] = json!(false);
    }
    if allow_immutable_lane_snapshots {
        // Reservation compares admission before and after taking the lane lock.
        // Filesystem enumeration order must not change the provenance value.
        lane_ledger_snapshots.sort_by(|left, right| {
            left["ledger_path"]
                .as_str()
                .cmp(&right["ledger_path"].as_str())
        });
        provenance["lane_ledger_snapshots"] = json!(lane_ledger_snapshots);
    }
    Ok(provenance)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assessment_exceptions_never_hide_training_or_consumed_seed_evidence() {
        let heldout = json!({"assessment_seeds":[11],"allocations":[{"assessment_seed":11}]});
        assert!(exclusions(&heldout, 11, true).is_ok());
        assert!(exclusions(&heldout, 11, false).is_err());
        for evidence in [
            json!({"training_seed_ranges":[[11,11]]}),
            json!({"consumed_seed_ranges":[[10,12]]}),
            json!({"nested":{"world_seed":11}}),
        ] {
            assert!(exclusions(&evidence, 11, true).is_err());
        }
        assert!(exclusions(&json!({"assessment_seeds":"malformed"}), 11, true).is_err());
        for malformed in [
            json!({"assessment_seed":{"training_seed_ranges":[[11,11]]}}),
            json!({"assessment_seeds":[11,{"consumed_seed_ranges":[[11,11]]}]}),
        ] {
            assert!(exclusions(&malformed, 11, true).is_err());
        }
    }
    #[test]
    fn nested_exclusions_and_assessments_cannot_be_bypassed() {
        for evidence in [
            json!({"reserved_seed_ranges":[[10,15]]}),
            json!({"runs":[{"plan":{"seed_range":[11,11]}}]}),
            json!({"allocations":[{"assessment_seed":11}]}),
            json!({"assessment_seeds":[11]}),
        ] {
            assert!(exclusions(&evidence, 11, false).is_err());
            assert!(exclusions(&evidence, 30, false).is_ok());
        }
        assert!(exclusions(&json!({"consumed_seed_ranges":[[15,10]]}), 30, false).is_err());
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        root: PathBuf,
        source: PathBuf,
        history: PathBuf,
        output: PathBuf,
        document: Value,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "alife-group-admission-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let source = root.join("source");
            let lane = root.join("lane");
            std::fs::create_dir_all(&source).unwrap();
            std::fs::create_dir(&lane).unwrap();
            let recovery = root.join("immutable-recovery-evidence.txt");
            let recovery_bytes = b"Synthetic CPU admission fixture; no real checkpoint or historical freshness is asserted.";
            std::fs::write(&recovery, recovery_bytes).unwrap();
            let mut hashes = serde_json::Map::new();
            for (name, bytes) in [
                (
                    "cycle.json",
                    br#"{"seed":100,"retained":{"reserved_seed_ranges":[[250,255]]}}"#.as_slice(),
                ),
                ("trained.alife-foundation", b"native asset".as_slice()),
                ("actor-checkpoint.json", b"native actor".as_slice()),
                ("value-checkpoint.json", b"native value".as_slice()),
            ] {
                std::fs::write(source.join(name), bytes).unwrap();
                hashes.insert(name.into(), json!(hash(bytes)));
            }
            let document = json!({"schema":1,"kind":"mixed-care-seed-history","complete":false,
                "seed_admission":{"policy":"known-exclusions","historical_freshness":"unknown"},
                "missing_history":"Synthetic older fixture history is absent and its completeness remains explicitly unknown.",
                "recovery_reconciliation":{"accepted_checkpoint_authoritative":true,
                    "known_inflight_attempts_reconciled_or_excluded":true,"no_known_detached_later_accepted_chain":true,
                    "prior_ledger_runs_sha256":hash(b"[]"),
                    "explanation":"This synthetic fixture cites preserved evidence and has no prior mixed ledger runs.",
                    "evidence":[{"path":recovery,"sha256":hash(recovery_bytes)}]},
                "evidence":"preserved recovery evidence; historical completeness remains unknown","source_sha256":hashes,
                "consumed_seed_ranges":[[100,100]],"reserved_seed_ranges":[[120,130]],"assessment_seeds":[140],"imports":[]});
            let history = root.join("history.json");
            std::fs::write(&history, serde_json::to_vec(&document).unwrap()).unwrap();
            Self {
                output: lane.join("run"),
                root,
                source,
                history,
                document,
            }
        }
        fn write(&self) {
            std::fs::write(&self.history, serde_json::to_vec(&self.document).unwrap()).unwrap();
        }
        fn admit(&self, seed: u64, known: bool) -> Result<Value> {
            admit(&self.source, &self.history, &self.output, seed, known)
        }
    }
    #[cfg(feature = "cpu-training")]
    #[test]
    fn declared_assessment_admission_checks_training_and_ancestral_exclusions() {
        let mut f = Fixture::new();
        f.document["complete"] = json!(true);
        f.document.as_object_mut().unwrap().remove("seed_admission");
        f.write();
        assert!(admit_assessment(&f.source, &f.history, &f.output, 140, false).is_ok());
        assert!(f.admit(140, false).is_err());
        assert!(admit_assessment(&f.source, &f.history, &f.output, 200, false).is_err());
        for seed in [100, 125, 252] {
            f.document["assessment_seeds"] = json!([seed]);
            f.write();
            assert!(admit_assessment(&f.source, &f.history, &f.output, seed, false).is_err());
        }
    }
    #[test]
    fn strict_default_and_explicit_unknown_freshness_preserve_recovery_evidence() {
        let mut f = Fixture::new();
        assert!(f.admit(200, false).is_err());
        let accepted = f.admit(200, true).unwrap();
        assert_eq!(
            accepted["seed_admission"],
            json!({"policy":"known-exclusions","historical_freshness":"unknown"})
        );
        assert_eq!(accepted["evidence"][0]["document"], f.document);
        f.document["complete"] = json!(true);
        f.write();
        assert!(f.admit(200, true).is_err());
        assert!(f.admit(200, false).is_err());
        f.document.as_object_mut().unwrap().remove("seed_admission");
        f.write();
        assert!(f.admit(200, false).is_ok());
        f.document["complete"] = json!(false);
        f.document["seed_admission"] =
            json!({"policy":"known-exclusions","historical_freshness":"unknown"});
        f.write();
        for seed in [100, 125, 140, 252] {
            assert!(f.admit(seed, true).is_err());
        }
    }
    #[test]
    fn known_exclusions_require_immutable_metadata_and_real_recovery_bindings() {
        let mut f = Fixture::new();
        let original = f.document.clone();
        for field in [
            "seed_admission",
            "missing_history",
            "recovery_reconciliation",
        ] {
            f.document = original.clone();
            f.document.as_object_mut().unwrap().remove(field);
            f.write();
            assert!(f.admit(200, true).is_err(), "missing {field}");
        }
        f.document = original.clone();
        f.document["recovery_reconciliation"]["prior_ledger_runs_sha256"] = json!("0".repeat(64));
        f.write();
        assert!(f.admit(200, true).is_err());
        f.document = original;
        f.write();
        let evidence = Path::new(
            f.document["recovery_reconciliation"]["evidence"][0]["path"]
                .as_str()
                .unwrap(),
        );
        std::fs::write(evidence, b"changed recovery snapshot").unwrap();
        assert!(f.admit(200, true).is_err());
    }
    #[test]
    fn surviving_mixed_tail_rejects_old_source_and_accepts_bound_group_descendants() {
        let mut f = Fixture::new();
        let status = std::process::Command::new("python").args(["-B", "-c", r#"
import pathlib, sys, json, shutil
sys.path.insert(0, str(pathlib.Path.cwd() / "scripts"))
import run_mixed_care_serial as mixed
root, history = map(pathlib.Path, sys.argv[1:])
document, _ = mixed.load(history)
source, lane = root / "source", root / "lane"
run_output = lane / "mixed-run"
later = run_output / "attempt-00000"
later.mkdir(parents=True)
for name in mixed.FILES:
    shutil.copyfile(source / name, later / name)
receipt = {"seed": 101, "struct_declared_first": True, "policy_version": 7}
(later / "cycle.json").write_text(json.dumps(receipt, indent=2))
(later / "initial.alife-foundation").write_bytes(b"synthetic original")
plan = {"source": str(source), "source_sha256": document["source_sha256"], "seed_range": [101, 101],
        "seed_admission": document["seed_admission"]}
attempt = {"seed": 101, "slot": 0, "status": "accepted", "previous": str(source), "directory": str(later),
           "receipt": receipt, "sha256": {**mixed.file_hashes(later), "initial.alife-foundation": mixed.host.sha256(later / "initial.alife-foundation")}}
run = {"output": str(run_output), "plan": plan, "plan_sha256": mixed.object_hash(plan),
       "history_import": {"complete": False, "seed_admission": document["seed_admission"]}, "attempts": [attempt]}
ledger = lane / mixed.LEDGER
ledger.write_text(json.dumps({"schema": 1, "kind": "mixed-care-seed-ledger", "excluded_seed_ranges": [[101, 101]],
                              "assessment_seeds": [], "runs": [run]}))
document["recovery_reconciliation"]["prior_ledger_runs_sha256"] = mixed.object_hash([run])
document["imports"] = [{"path": str(ledger), "sha256": mixed.host.sha256(ledger)}]
history.write_text(json.dumps(document))
"#]).arg(&f.root).arg(&f.history).current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).status().unwrap();
        assert!(status.success());
        f.document = read(&f.history).unwrap().1;
        let denial = f.admit(200, true).unwrap_err().to_string();
        assert!(denial.contains("latest surviving"), "{denial}");
        let known = f.document.clone();
        f.document["complete"] = json!(true);
        f.document["seed_admission"] =
            json!({"policy":"strict","historical_freshness":"operator-attested-complete"});
        f.write();
        assert!(f
            .admit(200, false)
            .unwrap_err()
            .to_string()
            .contains("latest surviving"));
        f.document = known.clone();
        let hashes = |path: &Path| -> Value {
            let receipt = source_name(path).unwrap();
            Value::Object(
                [
                    receipt,
                    "trained.alife-foundation",
                    "actor-checkpoint.json",
                    "value-checkpoint.json",
                ]
                .into_iter()
                .map(|name| {
                    (
                        name.into(),
                        json!(hash(&std::fs::read(path.join(name)).unwrap())),
                    )
                })
                .collect(),
            )
        };
        f.source = f.output.parent().unwrap().join("mixed-run/attempt-00000");
        f.document["source_sha256"] = hashes(&f.source);
        f.document["consumed_seed_ranges"] = json!([[100, 101]]);
        f.write();
        assert!(f.admit(200, true).is_ok());
        // Cross-domain receipt links retain the exact latest accepted native root.
        for (seed, cpu) in [(102, true), (103, false), (104, true)] {
            let next = f.root.join(format!("group-{seed}"));
            std::fs::create_dir(&next).unwrap();
            for name in [
                "trained.alife-foundation",
                "actor-checkpoint.json",
                "value-checkpoint.json",
            ] {
                std::fs::copy(f.source.join(name), next.join(name)).unwrap();
            }
            let native = source_name(&f.source).unwrap();
            let mut receipt = json!({"schema":1,"complete":true,"mode":"group50_shared_habitat","population":50,"seed":seed,
                "seed_provenance":{"source_sha256":hashes(&f.source),"native_source_receipt":read(&f.source.join(native)).unwrap().1}});
            let receipt_name = if cpu {
                "cpu-cycle.json"
            } else {
                "group50.json"
            };
            if cpu {
                receipt["mode"] = json!("native_cpu_fresh_cohort");
                receipt["backend"] = json!("native-cpu");
                receipt["receipt_filename"] = json!("cpu-cycle.json");
                receipt["objective_state_reset"] = json!(false);
                receipt["biological_objective_version"] = json!(2);
                receipt["critic_objective_version"] = json!(1);
            }
            std::fs::write(
                next.join(receipt_name),
                serde_json::to_vec(&receipt).unwrap(),
            )
            .unwrap();
            f.source = next;
            f.document["source_sha256"] = hashes(&f.source);
            f.document["consumed_seed_ranges"] = json!([[100, seed]]);
            f.write();
            let admitted = f.admit(200, true).unwrap();
            assert_eq!(
                admitted["seed_admission"]["historical_freshness"],
                "unknown"
            );
        }
        // Changing an embedded intermediate cannot manufacture a new lineage,
        // even after the caller pins the changed root receipt in its history.
        let receipt_name = source_name(&f.source).unwrap();
        let good_receipt = read(&f.source.join(receipt_name)).unwrap().1;
        let mut changed = good_receipt.clone();
        changed["seed_provenance"]["native_source_receipt"]["seed"] = json!(999);
        std::fs::write(
            f.source.join(receipt_name),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        f.document["source_sha256"] = hashes(&f.source);
        f.write();
        assert!(f
            .admit(200, true)
            .unwrap_err()
            .to_string()
            .contains("pinned source hash"));
        std::fs::write(
            f.source.join(receipt_name),
            serde_json::to_vec(&good_receipt).unwrap(),
        )
        .unwrap();
        // An exact-byte history for an older root must not become a descendant.
        let receipt_name = source_name(&f.source).unwrap();
        let mut receipt = read(&f.source.join(receipt_name)).unwrap().1;
        receipt["seed_provenance"]["source_sha256"] = known["source_sha256"].clone();
        receipt["seed_provenance"]["native_source_receipt"] = json!({"seed":100});
        std::fs::write(
            f.source.join(receipt_name),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        f.document["source_sha256"] = hashes(&f.source);
        f.write();
        assert!(f.admit(200, true).is_err());
    }
    #[test]
    fn source_seed_and_inherited_exclusions_are_required_even_in_a_new_lane() {
        let mut f = Fixture::new();
        f.document["consumed_seed_ranges"] = json!([]);
        f.write();
        assert!(f
            .admit(200, true)
            .unwrap_err()
            .to_string()
            .contains("source seed"));
        f.document["consumed_seed_ranges"] = json!([[100, 100]]);
        f.write();
        assert!(f.admit(252, true).is_err());
        assert!(f.admit(100, true).is_err());
    }
    #[test]
    fn all_lane_ledgers_require_exact_import_and_known_allocations_stay_excluded() {
        let mut f = Fixture::new();
        let ledger = f.output.parent().unwrap().join(".group50-seed-310.json");
        let bytes=serde_json::to_vec(&json!({"schema":1,"kind":"grab-food-seed-ledger","training_seed_ranges":[[310,310]],"assessment_seeds":[311],"allocations":[]})).unwrap();
        std::fs::write(&ledger, &bytes).unwrap();
        assert!(f.admit(200, true).is_err());
        f.document["imports"] = json!([{"path":ledger,"sha256":hash(&bytes)}]);
        f.write();
        assert!(f.admit(200, true).is_ok());
        assert!(f.admit(310, true).is_err());
        assert!(f.admit(311, true).is_err());
        std::fs::write(&ledger, b"{}").unwrap();
        assert!(f.admit(200, true).is_err());
    }
    #[test]
    fn collect_only_snapshot_imports_require_current_raw_ledger_bytes_and_keep_exclusions() {
        let mut f = Fixture::new();
        let source_before = std::fs::read_dir(&f.source)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect::<Vec<_>>();
        let ledger = f
            .output
            .parent()
            .unwrap()
            .join(".grab-food-seed-reservations.json");
        let snapshot = f.root.join("immutable-lane-ledger.json");
        // Noncanonical spacing is intentional: equality is over raw bytes,
        // rather than a parsed/re-encoded JSON representation.
        let bytes = br#"{ "schema": 1, "kind": "grab-food-seed-ledger", "training_seed_ranges": [[310,310]], "assessment_seeds": [311], "allocations": [] }"#;
        std::fs::write(&ledger, bytes).unwrap();
        std::fs::write(&snapshot, bytes).unwrap();
        f.document["imports"] = json!([{"path":snapshot,"sha256":hash(bytes)}]);
        f.write();
        assert!(f.admit(200, true).is_err());
        let accepted = admit_collection(&f.source, &f.history, &f.output, 200, true).unwrap();
        assert_eq!(
            accepted["lane_ledger_snapshots"],
            json!([{"ledger_path":ledger.canonicalize().unwrap(),
                "snapshot_path":snapshot.canonicalize().unwrap(),"sha256":hash(bytes)}])
        );
        assert_eq!(
            accepted["seed_admission"],
            json!({"policy":"known-exclusions","historical_freshness":"unknown"})
        );
        for seed in [310, 311] {
            assert!(admit_collection(&f.source, &f.history, &f.output, seed, true).is_err());
        }
        std::fs::write(
            &ledger,
            serde_json::to_vec(&read(&snapshot).unwrap().1).unwrap(),
        )
        .unwrap();
        assert!(admit_collection(&f.source, &f.history, &f.output, 200, true).is_err());
        assert_eq!(std::fs::read(&snapshot).unwrap(), bytes);
        // The ordinary path still accepts the actual ledger import and keeps
        // its original provenance shape.
        let ledger_bytes = std::fs::read(&ledger).unwrap();
        f.document["imports"] = json!([{"path":ledger,"sha256":hash(&ledger_bytes)}]);
        f.write();
        let ordinary = f.admit(200, true).unwrap();
        assert!(ordinary.get("lane_ledger_snapshots").is_none());
        for (path, bytes) in source_before {
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
    }
    #[test]
    fn duplicate_exclusion_keys_fail_at_any_depth() {
        for bytes in [
            br#"{"schema":1,"schema":1}"#.as_slice(),
            br#"{"imports":[{"reserved_seed_ranges":[[1,2]],"reserved_seed_ranges":[]}]}"#
                .as_slice(),
        ] {
            assert!(alife_training::native_receipt::parse(bytes).is_err());
        }
    }
}
