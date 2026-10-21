//! Bounded legacy receipt import and flat, hash-addressed native ancestry.
//!
//! Legacy JSON is accepted unchanged up to an explicit compatibility depth.
//! New receipts refer to immutable SHA256-named blobs, so serial continuation
//! does not increase JSON depth or pretty-print every predecessor again.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::Path,
};
pub const MAX_RECEIPT_BYTES: usize = 64 * 1024 * 1024;
// Compatibility only: flat writers never accumulate this depth over episodes.
pub const MAX_LEGACY_JSON_DEPTH: usize = 256;
pub const MAX_LINEAGE_RECEIPTS: usize = 100_000;
const DIRECTORY: &str = "source-lineage";
fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
pub fn sha256(bytes: &[u8]) -> String {
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

fn depth(bytes: &[u8]) -> io::Result<usize> {
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(invalid("seed evidence exceeds64 MiB"));
    }
    let (mut string, mut escape, mut maximum) = (false, false, 0);
    let mut containers = Vec::new();
    for &byte in bytes {
        if string {
            if escape {
                escape = false;
            } else if byte == b'\\' {
                escape = true;
            } else if byte == b'"' {
                string = false;
            }
        } else {
            match byte {
                b'"' => string = true,
                b'{' | b'[' => {
                    containers.push(byte);
                    maximum = maximum.max(containers.len());
                    if maximum > MAX_LEGACY_JSON_DEPTH {
                        return Err(invalid(
                            "legacy receipt JSON exceeds bounded compatibility depth",
                        ));
                    }
                }
                b'}' | b']' if containers.pop() != Some(if byte == b'}' { b'{' } else { b'[' }) => {
                    return Err(invalid("invalid JSON containers"));
                }
                _ => (),
            }
        }
    }
    Ok(maximum)
}
fn parse_checked(bytes: &[u8]) -> io::Result<Value> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    // Only reachable after the byte/depth preflight, never unbounded input.
    decoder.disable_recursion_limit();
    let Unique(value) =
        <Unique as serde::Deserialize>::deserialize(&mut decoder).map_err(io::Error::other)?;
    decoder.end().map_err(io::Error::other)?;
    Ok(value)
}
pub fn parse(bytes: &[u8]) -> io::Result<Value> {
    let depth = depth(bytes)?;
    if depth > 96 {
        let bytes = bytes.to_vec();
        std::thread::Builder::new()
            .name("legacy-receipt-reader".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || parse_checked(&bytes))?
            .join()
            .map_err(|_| invalid("legacy receipt reader panicked"))?
    } else {
        parse_checked(bytes)
    }
}
fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_RECEIPT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(invalid("seed evidence exceeds64 MiB"));
    }
    Ok(bytes)
}
pub fn read(path: &Path) -> io::Result<(Vec<u8>, Value)> {
    let bytes = read_bytes(path)?;
    let value = parse(&bytes)?;
    Ok((bytes, value))
}
/// A frame excludes only its recursively embedded predecessor, which has its
/// own frame. Ordinary exclusions and all native fields remain available.
#[derive(Debug)]
pub struct Lineage {
    pub frames: Vec<Value>,
    pub terminal_proof: Option<Value>,
    pub legacy_proof_error: Option<String>,
    blobs: BTreeMap<String, Vec<u8>>,
    imported: BTreeSet<String>,
}
fn header(value: &Value) -> Value {
    let mut result = serde_json::Map::new();
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if key == "seed_provenance" {
                let mut provenance = serde_json::Map::new();
                if let Some(map) = value.as_object() {
                    for (key, value) in map {
                        if key != "native_source_receipt" {
                            provenance.insert(key.clone(), value.clone());
                        }
                    }
                }
                result.insert(key.clone(), Value::Object(provenance));
            } else {
                result.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(result)
}
fn parent(value: &Value) -> io::Result<(&str, &str)> {
    let map = value["seed_provenance"]["source_sha256"]
        .as_object()
        .ok_or_else(|| invalid("missing training ancestry hashes"))?;
    let names: Vec<_> = ["cycle.json", "cpu-cycle.json", "group50.json"]
        .into_iter()
        .filter(|n| map.contains_key(*n))
        .collect();
    if names.len() != 1
        || map.len() != 4
        || [
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ]
        .iter()
        .any(|n| !map.contains_key(*n))
    {
        return Err(invalid("invalid native ancestry pin set"));
    }
    Ok((
        names[0],
        map[names[0]]
            .as_str()
            .ok_or_else(|| invalid("missing pinned ancestry receipt hash"))?,
    ))
}
fn digest_name(digest: &str) -> io::Result<String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(invalid("invalid lineage SHA256"));
    }
    Ok(format!("{digest}.json"))
}
fn load_blob(directory: &Path, digest: &str) -> io::Result<Vec<u8>> {
    let base = directory.join(DIRECTORY);
    if fs::symlink_metadata(&base)?.file_type().is_symlink() {
        return Err(invalid("lineage directory must not be a symlink"));
    }
    let path = base.join(digest_name(digest)?);
    if !fs::symlink_metadata(&path)?.is_file() {
        return Err(invalid("lineage blob must be a regular file"));
    }
    let bytes = read_bytes(&path)?;
    if sha256(&bytes) != digest {
        return Err(invalid("lineage blob differs from pinned SHA256"));
    }
    Ok(bytes)
}
impl Lineage {
    /// All references use the existing direct predecessor pin; a free-standing
    /// reference can never select a different source. Imported legacy inline
    /// proof retains the historical canonical-serialization checks.
    pub fn load(root: &Value, directory: &Path) -> io::Result<Self> {
        let mut result = Self {
            frames: Vec::new(),
            terminal_proof: None,
            legacy_proof_error: None,
            blobs: BTreeMap::new(),
            imported: BTreeSet::new(),
        };
        let mut current = root.clone();
        let mut seen = BTreeSet::new();
        let mut aggregate = 0usize;
        let mut expected_domain: Option<String> = None;
        loop {
            if result.frames.len() >= MAX_LINEAGE_RECEIPTS {
                return Err(invalid("native lineage work bound exceeded"));
            }
            result.frames.push(header(&current));
            let provenance = &current["seed_provenance"];
            let group = current["complete"] == true
                && current["mode"] == "group50_shared_habitat"
                && current["population"] == 50;
            let cpu = current["complete"] == true
                && current["mode"] == "native_cpu_fresh_cohort"
                && current["backend"] == "native-cpu"
                && current["receipt_filename"] == "cpu-cycle.json"
                && current["objective_state_reset"] == false
                && current["biological_objective_version"] == 2
                && current["critic_objective_version"] == 1;
            if expected_domain.as_deref().is_some_and(|name| match name {
                "cpu-cycle.json" => !cpu,
                "group50.json" => !group,
                _ => true,
            }) {
                return Err(invalid("invalid training ancestry receipt domain"));
            }
            if !provenance.is_object()
                || (!provenance["native_source_receipt"].is_object()
                    && provenance.get("native_source_receipt_ref").is_none())
            {
                if expected_domain.is_some()
                    && !(cpu
                        && current["fixture"] == true
                        && provenance["source_sha256"]
                            .as_object()
                            .is_some_and(|map| map.is_empty()))
                {
                    return Err(invalid("truncated native training ancestry"));
                }
                break;
            }
            if !group && !cpu {
                return Err(invalid("invalid training ancestry receipt"));
            }
            let (name, expected) = parent(&current)?;
            let (name, expected) = (name.to_owned(), expected.to_owned());
            let terminal_pins = provenance["source_sha256"].clone();
            let child;
            let reference = provenance.get("native_source_receipt_ref");
            if let Some(reference) = reference {
                if provenance.get("native_source_receipt").is_some()
                    || reference.as_str() != Some(expected.as_str())
                {
                    return Err(invalid(
                        "lineage reference differs from direct predecessor pin",
                    ));
                }
                if !seen.insert(expected.to_owned()) {
                    return Err(invalid("cyclic or repeated lineage reference"));
                }
                let bytes = load_blob(directory, &expected)?;
                let value = parse(&bytes)?;
                aggregate = aggregate
                    .checked_add(bytes.len())
                    .ok_or_else(|| invalid("lineage byte overflow"))?;
                if aggregate > MAX_RECEIPT_BYTES {
                    return Err(invalid("aggregate native lineage exceeds64 MiB"));
                }
                result.blobs.insert(expected.to_owned(), bytes);
                result.imported.insert(expected.to_owned());
                child = value;
            } else {
                child = current["seed_provenance"]["native_source_receipt"].take();
                if name != "cycle.json" {
                    let mut verified = false;
                    if result.frames.len() == 1 {
                        let retained = directory
                            .join(if current["mode"] == "native_cpu_fresh_cohort" {
                                "source-native"
                            } else {
                                "source-receipts"
                            })
                            .join(&name);
                        if retained.is_file() {
                            let (bytes, document) = read(&retained)?;
                            verified = sha256(&bytes) == expected && document == child;
                            if !verified {
                                result.legacy_proof_error.get_or_insert(
                                    "retained training ancestry receipt changed".into(),
                                );
                            }
                        }
                    }
                    if !verified {
                        for mut bytes in [
                            serde_json::to_vec(&child).map_err(io::Error::other)?,
                            serde_json::to_vec_pretty(&child).map_err(io::Error::other)?,
                        ] {
                            verified |= sha256(&bytes) == expected;
                            bytes.push(b'\n');
                            verified |= sha256(&bytes) == expected;
                        }
                    }
                    if !verified {
                        result.legacy_proof_error.get_or_insert(
                            "embedded training receipt differs from pinned source hash".into(),
                        );
                    }
                }
            }
            if name == "cycle.json" {
                result.terminal_proof = Some(
                    json!({"pin_set_sha256":sha256(&serde_json::to_vec(&terminal_pins).map_err(io::Error::other)?),"terminal_native_receipt":child}),
                );
                result.frames.push(header(&child));
                break;
            }
            expected_domain = Some(name);
            current = child;
        }
        Ok(result)
    }
    /// Copy only reachable, verified raw blobs. No source file is rewritten.
    pub fn write_to(&self, destination: &Path) -> io::Result<()> {
        if self.blobs.is_empty() {
            return Ok(());
        }
        let base = destination.join(DIRECTORY);
        fs::create_dir(&base)?;
        for (digest, bytes) in &self.blobs {
            fs::write(base.join(digest_name(digest)?), bytes)?;
        }
        Ok(())
    }
    /// Match the original immutable-source guard after collection and export.
    pub fn verify_at(&self, source: &Path) -> io::Result<()> {
        for digest in &self.imported {
            // Admission parsed these bytes already; exact equality preserves that validation.
            let bytes = load_blob(source, digest)?;
            if self.blobs.get(digest) != Some(&bytes) {
                return Err(invalid("source lineage changed during native run"));
            }
        }
        Ok(())
    }
    /// Include the exact direct parent as a new blob, retaining inherited blobs.
    pub fn check_parent(&self, bytes: &[u8]) -> io::Result<String> {
        let digest = sha256(bytes);
        let total: usize = self.blobs.values().map(Vec::len).sum();
        if total
            .checked_add(bytes.len())
            .is_none_or(|n| n > MAX_RECEIPT_BYTES)
        {
            return Err(invalid("aggregate native lineage exceeds64 MiB"));
        }
        Ok(digest)
    }
    pub fn retain_parent(&mut self, bytes: Vec<u8>) -> io::Result<String> {
        let digest = self.check_parent(&bytes)?;
        self.blobs.insert(digest.clone(), bytes);
        Ok(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Tree(std::path::PathBuf);
    impl Tree {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "alife-lineage-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn store(&self, bytes: &[u8]) -> String {
            let digest = sha256(bytes);
            let dir = self.0.join(DIRECTORY);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(format!("{digest}.json")), bytes).unwrap();
            digest
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn reference(name: &str, digest: &str, seed: u64) -> Value {
        json!({"complete":true,"mode":"native_cpu_fresh_cohort","backend":"native-cpu","receipt_filename":"cpu-cycle.json",
            "objective_state_reset":false,"biological_objective_version":2,"critic_objective_version":1,"seed":seed,
            "seed_provenance":{"source_sha256":{name:digest,"trained.alife-foundation":"asset","actor-checkpoint.json":"actor","value-checkpoint.json":"value"},"native_source_receipt_ref":digest}})
    }
    #[test]
    fn bounded_legacy_depth_and_operations_on_small_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let bytes = format!(
                    "{}0{}",
                    "{\"a\":".repeat(MAX_LEGACY_JSON_DEPTH),
                    "}".repeat(MAX_LEGACY_JSON_DEPTH)
                )
                .into_bytes();
                assert!(serde_json::from_slice::<Value>(&bytes).is_err());
                let value = parse(&bytes).unwrap();
                let copy = value.clone();
                assert_eq!(value, copy);
                assert_eq!(serde_json::to_vec(&value).unwrap(), bytes);
                drop(copy);
                drop(value);
                assert!(parse(
                    format!(
                        "{}0{}",
                        "[".repeat(MAX_LEGACY_JSON_DEPTH + 1),
                        "]".repeat(MAX_LEGACY_JSON_DEPTH + 1)
                    )
                    .as_bytes()
                )
                .is_err());
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn deep_duplicate_keys_and_trailing_data_are_rejected() {
        let bytes = format!(
            "{}{{\"seed\":1,\"seed\":2}}{}",
            "[".repeat(128),
            "]".repeat(128)
        );
        assert!(parse(bytes.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("duplicate"));
        assert!(parse(br#"{} {}"#).is_err());
        assert!(parse(&vec![b' '; MAX_RECEIPT_BYTES + 1]).is_err());
    }
    #[test]
    fn thousand_flat_receipts_retain_hashes_exclusions_and_cpu_group_roundtrip() {
        let tree = Tree::new();
        let terminal = json!({"seed":17,"assessment_seeds":[18],"reserved_seed_ranges":[[19,20]]});
        let mut bytes = serde_json::to_vec_pretty(&terminal).unwrap();
        let mut name = "cycle.json";
        let mut root = Value::Null;
        for seed in 21..1045 {
            let digest = tree.store(&bytes);
            root = reference(name, &digest, seed);
            if seed % 2 == 0 {
                root["mode"] = json!("group50_shared_habitat");
                root["population"] = json!(50);
            }
            bytes = serde_json::to_vec_pretty(&root).unwrap();
            name = if seed % 2 == 0 {
                "group50.json"
            } else {
                "cpu-cycle.json"
            };
            assert!(depth(&bytes).unwrap() < 8);
        }
        let mut lineage = Lineage::load(&root, &tree.0).unwrap();
        assert_eq!(lineage.frames.len(), 1025);
        assert_eq!(
            lineage.frames.last().unwrap()["assessment_seeds"],
            json!([18])
        );
        assert_eq!(
            lineage.frames.last().unwrap()["reserved_seed_ranges"],
            json!([[19, 20]])
        );
        assert!(lineage.legacy_proof_error.is_none());
        lineage.verify_at(&tree.0).unwrap();
        lineage.retain_parent(bytes.clone()).unwrap();
        let copy = Tree::new();
        lineage.write_to(&copy.0).unwrap();
        let child = reference(name, &sha256(&bytes), 1045);
        assert_eq!(Lineage::load(&child, &copy.0).unwrap().frames.len(), 1026);
        // Copying never normalized or rewrote any historical receipt byte.
        for (digest, original) in &lineage.blobs {
            assert_eq!(
                fs::read(copy.0.join(DIRECTORY).join(format!("{digest}.json"))).unwrap(),
                *original
            );
        }
    }
    #[test]
    fn references_cannot_select_another_parent_or_hide_corruption() {
        let tree = Tree::new();
        let bytes = br#"{"seed":7}"#;
        let digest = tree.store(bytes);
        let root = reference("cycle.json", &digest, 8);
        assert!(Lineage::load(&root, &tree.0).is_ok());
        let mut wrong = root.clone();
        wrong["seed_provenance"]["native_source_receipt_ref"] = json!("other");
        assert!(Lineage::load(&wrong, &tree.0).is_err());
        fs::write(tree.0.join(DIRECTORY).join(format!("{digest}.json")), b"{}").unwrap();
        assert!(Lineage::load(&root, &tree.0).is_err());
        fs::remove_file(tree.0.join(DIRECTORY).join(format!("{digest}.json"))).unwrap();
        assert!(Lineage::load(&root, &tree.0).is_err());
        let bad = reference("cycle.json", "../outside", 8);
        assert!(Lineage::load(&bad, &tree.0).is_err());
    }
    #[test]
    fn correctly_hashed_invalid_ancestor_and_post_collection_change_fail() {
        let tree = Tree::new();
        let terminal = tree.store(br#"{"seed":7}"#);
        let mut bad = reference("cycle.json", &terminal, 8);
        bad["objective_state_reset"] = json!(true);
        let digest = tree.store(&serde_json::to_vec(&bad).unwrap());
        let root = reference("cpu-cycle.json", &digest, 9);
        assert!(Lineage::load(&root, &tree.0).is_err());
        for domain in ["cpu-cycle.json", "group50.json"] {
            let truncated = reference(domain, &terminal, 9);
            assert!(Lineage::load(&truncated, &tree.0).is_err());
        }
        let root = reference("cycle.json", &terminal, 8);
        let lineage = Lineage::load(&root, &tree.0).unwrap();
        lineage.verify_at(&tree.0).unwrap();
        fs::write(
            tree.0.join(DIRECTORY).join(format!("{terminal}.json")),
            b"{}",
        )
        .unwrap();
        assert!(lineage.verify_at(&tree.0).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn lineage_symlink_escape_is_rejected() {
        use std::os::unix::fs::symlink;
        let tree = Tree::new();
        let external = Tree::new();
        let bytes = br#"{"seed":7}"#;
        let digest = external.store(bytes);
        symlink(external.0.join(DIRECTORY), tree.0.join(DIRECTORY)).unwrap();
        assert!(Lineage::load(&reference("cycle.json", &digest, 8), &tree.0).is_err());
    }
}
