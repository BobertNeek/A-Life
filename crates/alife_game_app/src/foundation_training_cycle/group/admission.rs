//! Read-only seed admission. A lack of historical evidence never becomes freshness.
use super::super::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
fn read(path: &Path) -> Result<(Vec<u8>, Value)> {
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
    if document["kind"] != "mixed-care-seed-history"
        || document["schema"] != 1
        || document["evidence"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
        || (!known_exclusions && document["complete"] != true)
        || (known_exclusions && !document["complete"].is_boolean())
    {
        return Err("strict admission requires complete attested seed history; incomplete history requires explicit known-exclusions".into());
    }
    let source_name = if previous.join("group50.json").is_file() {
        "group50.json"
    } else {
        "cycle.json"
    };
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
        "seed_admission":{"policy":if known_exclusions {"known-exclusions"} else {"strict"},
            "historical_freshness":if known_exclusions {"unknown"} else {"operator_attested_complete"}},
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
        assert!(f.admit(200, false).is_ok());
        for seed in [100, 125, 140, 252] {
            assert!(f.admit(seed, true).is_err());
        }
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
