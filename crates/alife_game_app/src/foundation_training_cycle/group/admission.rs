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
    let (_, mut native) = read(&previous.join(receipt_name))?;

    for depth in 0..64 {
        let group = native["complete"] == true
            && native["mode"] == "group50_shared_habitat"
            && native["population"] == 50;
        let cpu = native["complete"] == true
            && native["mode"] == "native_cpu_fresh_cohort"
            && native["backend"] == "native-cpu"
            && native["receipt_filename"] == "cpu-cycle.json"
            && native["objective_state_reset"] == false
            && native["biological_objective_version"] == 2
            && native["critic_objective_version"] == 1;
        if !group && !cpu {
            return Err("invalid training ancestry receipt".into());
        }
        let provenance = &native["seed_provenance"];
        let map = provenance["source_sha256"]
            .as_object()
            .ok_or("missing training ancestry hashes")?;
        let names = ["cycle.json", "group50.json", "cpu-cycle.json"]
            .into_iter()
            .filter(|name| map.contains_key(*name))
            .collect::<Vec<_>>();
        if names.len() != 1
            || map.len() != 4
            || [
                "trained.alife-foundation",
                "actor-checkpoint.json",
                "value-checkpoint.json",
            ]
            .iter()
            .any(|name| !map.contains_key(*name))
        {
            return Err("invalid native ancestry pin set".into());
        }
        let name = names[0];
        let child = &provenance["native_source_receipt"];
        let expected = map[name]
            .as_str()
            .ok_or("missing pinned ancestry receipt hash")?;
        // The terminal native receipt uses a typed field order. Its exact raw
        // hash and parsed content are checked against the surviving mixed file
        // once Python resolves that file; no byte reconstruction is needed.
        if name == "cycle.json" {
            return Ok(
                json!({"pin_set_sha256":hash(&serde_json::to_vec(map)?), "terminal_native_receipt":child}),
            );
        }
        let mut verified = false;
        if depth == 0 {
            let retained = previous
                .join(if cpu {
                    "source-native"
                } else {
                    "source-receipts"
                })
                .join(name);
            if retained.is_file() {
                let (bytes, document) = read(&retained)?;
                verified = hash(&bytes) == expected && document == *child;
                if !verified {
                    return Err("retained training ancestry receipt changed".into());
                }
            }
        }
        if !verified {
            for mut bytes in [
                serde_json::to_vec(child)?,
                serde_json::to_vec_pretty(child)?,
            ] {
                verified |= hash(&bytes) == expected;
                bytes.push(b'\n');
                verified |= hash(&bytes) == expected;
            }
        }
        if !verified {
            return Err("embedded training receipt differs from pinned source hash".into());
        }
        native = child.clone();
    }
    Err("training ancestry exceeds64 links".into())
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

// Reject duplicate keys at every depth; an overwritten exclusion is a safety
// failure, even when another JSON reader would silently accept the last value.
struct Unique(Value);
impl<'de> serde::Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON with unique object keys")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Unique, M::Error> {
                let mut out = serde_json::Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if out.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate seed evidence key"));
                    }
                }
                Ok(Unique(Value::Object(out)))
            }
            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                mut seq: S,
            ) -> std::result::Result<Unique, S::Error> {
                let mut out = Vec::new();
                while let Some(Unique(value)) = seq.next_element()? {
                    out.push(value);
                }
                Ok(Unique(Value::Array(out)))
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
pub(super) fn read(path: &Path) -> Result<(Vec<u8>, Value)> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("seed evidence exceeds64 MiB".into());
    }
    let Unique(value) = serde_json::from_slice(&bytes)?;
    Ok((bytes, value))
}

fn exclusions(value: &Value, seed: u64) -> Result<()> {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
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
                        if value.as_u64().ok_or("invalid assessment seed")? == seed {
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
                    exclusions(value, seed)?;
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                exclusions(value, seed)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn import(
    path: &Path,
    seed: u64,
    seen: &mut BTreeSet<PathBuf>,
    records: &mut Vec<Value>,
    depth: usize,
    bytes_total: &mut u64,
) -> Result<()> {
    if depth > 8 || records.len() >= 128 {
        return Err("seed evidence import budget exceeded".into());
    }
    let path = path.canonicalize()?;
    if !seen.insert(path.clone()) {
        return Err("duplicate/cyclic seed evidence import".into());
    }
    let (bytes, document) = read(&path)?;
    *bytes_total = bytes_total
        .checked_add(bytes.len() as u64)
        .ok_or("seed evidence byte count overflow")?;
    if *bytes_total > 64 * 1024 * 1024 {
        return Err("aggregate seed evidence exceeds64 MiB".into());
    }
    if document["schema"] != 1 {
        return Err("seed evidence requires schema 1".into());
    }
    exclusions(&document, seed)?;
    if let Some(imports) = document.get("imports") {
        for item in imports.as_array().ok_or("invalid seed evidence imports")? {
            let child = Path::new(item["path"].as_str().ok_or("missing seed import path")?);
            let child = if child.is_absolute() {
                child.to_owned()
            } else {
                path.parent().unwrap().join(child)
            };
            if hash(&read(&child)?.0)
                != item["sha256"]
                    .as_str()
                    .ok_or("missing seed import digest")?
            {
                return Err("seed evidence import changed".into());
            }
            import(&child, seed, seen, records, depth + 1, bytes_total)?;
        }
    }
    records.push(json!({"path":path,"sha256":hash(&bytes),"document":document}));
    Ok(())
}

pub(super) fn admit(
    previous: &Path,
    history: &Path,
    output: &Path,
    seed: u64,
    known_exclusions: bool,
) -> Result<Value> {
    let (bytes, document) = read(history)?;
    let selected = json!({"policy":if known_exclusions {"known-exclusions"} else {"strict"},
        "historical_freshness":if known_exclusions {"unknown"} else {"operator-attested-complete"}});
    let recorded = document.get("seed_admission").cloned().unwrap_or_else(
        || json!({"policy":"strict","historical_freshness":"operator-attested-complete"}),
    );
    if document["kind"] != "mixed-care-seed-history"
        || document["schema"] != 1
        || document["evidence"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
        || (!known_exclusions && document["complete"] != true)
        || (known_exclusions && document["complete"] != false)
        || recorded != selected
    {
        return Err("strict admission requires complete attested seed history; incomplete history requires explicit known-exclusions".into());
    }
    let lane = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if known_exclusions || lane.join(".mixed-care-seed-reservations.json").exists() {
        mixed_lane_admission(history, lane, previous, &hash(&bytes), known_exclusions)?;
    }
    let source_name = source_name(previous)?;
    let (_, native) = read(&previous.join(source_name))?;
    let source_seed = native["seed"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or("native source lacks seed provenance")?;
    if source_seed == seed {
        return Err("group seed reuses the native source seed".into());
    }
    exclusions(&native, seed)?;
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
    import(history, seed, &mut seen, &mut records, 0, &mut 0)?;
    // Existing lane exclusions cannot be silently omitted. Each file must be an
    // exact-byte history import, including prior group reservations.
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
                if (name == ".grab-food-seed-reservations.json"
                    || name == ".mixed-care-seed-reservations.json"
                    || name.starts_with(".group50-seed-") && name.ends_with(".json"))
                    && !seen.contains(&entry.path().canonicalize()?)
                {
                    return Err(format!(
                        "existing lane ledger requires an explicit exact-byte history import: {}",
                        entry.path().display()
                    )
                    .into());
                }
            }
        }
    }
    Ok(
        json!({"schema":1,"world_seed":seed,"source_sha256":source_hashes,
        "seed_admission":selected,
        "historical_retry":false,"native_source_receipt":native,"evidence":records,"history_sha256":hash(&bytes)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_exclusions_and_assessments_cannot_be_bypassed() {
        for evidence in [
            json!({"reserved_seed_ranges":[[10,15]]}),
            json!({"runs":[{"plan":{"seed_range":[11,11]}}]}),
            json!({"allocations":[{"assessment_seed":11}]}),
            json!({"assessment_seeds":[11]}),
        ] {
            assert!(exclusions(&evidence, 11).is_err());
            assert!(exclusions(&evidence, 30).is_ok());
        }
        assert!(exclusions(&json!({"consumed_seed_ranges":[[15,10]]}), 30).is_err());
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
    fn duplicate_exclusion_keys_fail_at_any_depth() {
        for bytes in [
            br#"{"schema":1,"schema":1}"#.as_slice(),
            br#"{"imports":[{"reserved_seed_ranges":[[1,2]],"reserved_seed_ranges":[]}]}"#
                .as_slice(),
        ] {
            assert!(serde_json::from_slice::<Unique>(bytes).is_err());
        }
    }
}
