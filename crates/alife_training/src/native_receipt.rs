//! Bounded legacy receipt import and immutable, hash-addressed native ancestry.
//!
//! Legacy JSON is accepted unchanged up to an explicit compatibility depth.
//! Native/cooperative writers retain flat blobs; serial CPU writers pack raw bytes.
//! Receipts keep their existing direct predecessor SHA256 pins. Validate one
//! bounded document at a time; lifetime archive size is not resident state.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};
pub const MAX_RECEIPT_BYTES: usize = 64 * 1024 * 1024;
// Compatibility only: flat writers never accumulate this depth over episodes.
pub const MAX_LEGACY_JSON_DEPTH: usize = 256;
pub const MAX_LINEAGE_RECEIPTS: usize = 100_000;
// Bounds resident reference/legacy evidence, separately from each raw document.
// Full historical payloads remain hash-addressed on disk, never normalized.
const MAX_LINEAGE_INDEX_BYTES: usize = MAX_RECEIPT_BYTES;
const DIRECTORY: &str = "source-lineage";
mod packs;
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
    frames: Vec<Frame>,
    pub terminal_proof: Option<Value>,
    pub legacy_proof_error: Option<String>,
    blobs: BTreeMap<String, usize>,
    retained: BTreeMap<String, Vec<u8>>,
    index_bytes: usize,
    source_directory: PathBuf,
    store: packs::Store,
}
#[derive(Debug)]
enum Frame {
    Inline(Value),
    Archived(String),
    TerminalProof,
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
fn load_flat_blob(directory: &Path, digest: &str) -> io::Result<Vec<u8>> {
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
/// Read a pinned receipt from either a legacy flat archive or CPU receipt packs.
pub fn read_ancestry_blob(directory: &Path, digest: &str) -> io::Result<Vec<u8>> {
    packs::Store::open(directory)?.read(directory, digest)
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
            retained: BTreeMap::new(),
            index_bytes: 0,
            source_directory: directory.to_path_buf(),
            store: packs::Store::open(directory)?,
        };
        let mut current = root.clone();
        let mut seen = BTreeSet::new();
        let mut current_digest = None;
        let mut expected_domain: Option<String> = None;
        loop {
            result.push_frame(&current, current_digest.take())?;
            let provenance = &current["seed_provenance"];
            let group = current["complete"] == true
                && current["mode"] == "group50_shared_habitat"
                && current["population"] == 50;
            let cpu = current["complete"] == true
                && matches!(
                    current["mode"].as_str(),
                    Some("native_cpu_fresh_cohort" | "native_cpu_cooperative_founder")
                )
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
                let bytes = result.store.read(directory, &expected)?;
                let value = parse(&bytes)?;
                // The descriptor is resident; the exact historical payload is
                // already an immutable archive and is released after parsing.
                result.reserve_index(256)?;
                result.blobs.insert(expected.to_owned(), bytes.len());
                current_digest = Some(expected.clone());
                child = value;
            } else {
                child = current["seed_provenance"]["native_source_receipt"].take();
                if name != "cycle.json" {
                    let mut verified = false;
                    if result.frames.len() == 1 {
                        let retained = directory
                            .join(if cpu {
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
                let proof = json!({"pin_set_sha256":sha256(&serde_json::to_vec(&terminal_pins).map_err(io::Error::other)?),"terminal_native_receipt":child});
                result
                    .reserve_index(serde_json::to_vec(&proof).map_err(io::Error::other)?.len())?;
                result.terminal_proof = Some(proof);
                if current_digest.is_some() {
                    result.push_frame(&child, current_digest.take())?;
                } else {
                    // The inline terminal already lives in terminal_proof.
                    // Keep a reference instead of a second full metadata copy.
                    result.push_index_frame(Frame::TerminalProof, 128)?;
                }
                break;
            }
            expected_domain = Some(name);
            current = child;
        }
        Ok(result)
    }
    fn reserve_index(&mut self, bytes: usize) -> io::Result<()> {
        self.index_bytes = self
            .index_bytes
            .checked_add(bytes)
            .filter(|n| *n <= MAX_LINEAGE_INDEX_BYTES)
            .ok_or_else(|| invalid("native lineage reference index exceeds64 MiB"))?;
        Ok(())
    }
    fn push_frame(&mut self, value: &Value, digest: Option<String>) -> io::Result<()> {
        let (frame, bytes) = if let Some(digest) = digest {
            (Frame::Archived(digest), 128)
        } else {
            let value = header(value);
            let bytes = serde_json::to_vec(&value).map_err(io::Error::other)?.len();
            (Frame::Inline(value), bytes)
        };
        self.push_index_frame(frame, bytes)
    }
    fn push_index_frame(&mut self, frame: Frame, bytes: usize) -> io::Result<()> {
        if self.frames.len() >= MAX_LINEAGE_RECEIPTS {
            return Err(invalid("native lineage work bound exceeded"));
        }
        self.reserve_index(bytes)?;
        self.frames.push(frame);
        Ok(())
    }
    /// Visit complete original evidence one frame at a time, including arbitrary
    /// nested seed metadata. Every archived frame is reverified before use.
    pub fn frames(&self) -> impl Iterator<Item = io::Result<Value>> + '_ {
        self.frames.iter().map(|frame| match frame {
            Frame::Inline(value) => Ok(value.clone()),
            Frame::Archived(digest) => {
                let bytes = self.imported_bytes(digest)?;
                Ok(header(&parse(&bytes)?))
            }
            Frame::TerminalProof => self
                .terminal_proof
                .as_ref()
                .and_then(|proof| proof.get("terminal_native_receipt"))
                .map(header)
                .ok_or_else(|| invalid("missing retained inline terminal proof")),
        })
    }
    pub fn contains_fixture(&self) -> io::Result<bool> {
        let mut found = false;
        for frame in self.frames() {
            found |= frame?["fixture"] == true;
        }
        Ok(found)
    }
    fn imported_bytes(&self, digest: &str) -> io::Result<Vec<u8>> {
        let bytes = self.store.read(&self.source_directory, digest)?;
        if self.blobs.get(digest) != Some(&bytes.len()) {
            return Err(invalid("source lineage length changed"));
        }
        Ok(bytes)
    }
    fn verify_flat_blob(&self, destination: &Path, digest: &str) -> io::Result<()> {
        // write_with_link owns a newly created flat directory. Detecting its
        // format per blob would repeatedly enumerate the growing archive.
        let bytes = load_flat_blob(destination, digest)?;
        if self.blobs.get(digest) != Some(&bytes.len()) {
            return Err(invalid("source lineage length changed"));
        }
        Ok(())
    }
    /// Retain reachable, verified raw blobs without rewriting immutable ancestry.
    /// Same-filesystem hard links keep each output independently addressable even
    /// after an older output is removed. Unsupported links fall back to copying.
    /// These content-addressed files must never be edited in place.
    pub fn write_to(&self, destination: &Path) -> io::Result<()> {
        self.write_with_link(destination, |source, target| fs::hard_link(source, target))
    }
    /// CPU-only packed output retains the full closure in logarithmically many files.
    pub fn write_packed_to(&self, destination: &Path) -> io::Result<()> {
        packs::write_to(self, destination)
    }
    /// Verified immutable bytes for a publisher that must resume exact files
    /// without replacing an existing partial output. Includes a retained parent.
    pub fn raw_files(&self) -> io::Result<BTreeMap<String, Vec<u8>>> {
        let total = self
            .blobs
            .values()
            .try_fold(0usize, |total, bytes| total.checked_add(*bytes))
            .ok_or_else(|| invalid("materialized ancestry byte count overflow"))?;
        if total > MAX_RECEIPT_BYTES {
            return Err(invalid("cooperative source materialization supports at most64 MiB of ancestry blobs; originals remain intact"));
        }
        self.blobs
            .keys()
            .map(|digest| {
                let bytes = if let Some(bytes) = self.retained.get(digest) {
                    if sha256(bytes) != *digest {
                        return Err(invalid("retained lineage bytes changed"));
                    }
                    bytes.clone()
                } else {
                    self.imported_bytes(digest)?
                };
                Ok((format!("{DIRECTORY}/{}", digest_name(digest)?), bytes))
            })
            .collect()
    }
    fn write_with_link(
        &self,
        destination: &Path,
        link: impl Fn(&Path, &Path) -> io::Result<()>,
    ) -> io::Result<()> {
        if self.blobs.is_empty() {
            return Ok(());
        }
        let base = destination.join(DIRECTORY);
        fs::create_dir(&base)?;
        for digest in self.blobs.keys() {
            let name = digest_name(digest)?;
            let target = base.join(&name);
            let bytes = if let Some(bytes) = self.retained.get(digest) {
                bytes.clone()
            } else {
                let bytes = self.imported_bytes(digest)?;
                let source = self.source_directory.join(DIRECTORY).join(name);
                if link(&source, &target).is_ok() {
                    // Check the linked file too, rather than trusting a path that
                    // could have changed between validation and link creation.
                    self.verify_flat_blob(destination, digest)?;
                    continue;
                }
                bytes
            };
            fs::write(target, &bytes)?;
            self.verify_flat_blob(destination, digest)?;
        }
        Ok(())
    }
    /// Match the original immutable-source guard after collection and export.
    pub fn verify_at(&self, source: &Path) -> io::Result<()> {
        let store = packs::Store::open(source)?;
        if !self.store.same_manifest(&store) {
            return Err(invalid("source ancestry manifest changed"));
        }
        for digest in self
            .blobs
            .keys()
            .filter(|digest| !self.retained.contains_key(*digest))
        {
            let bytes = store.read(source, digest)?;
            if self.blobs.get(digest) != Some(&bytes.len()) {
                return Err(invalid("source lineage length changed"));
            }
        }
        Ok(())
    }
    /// Verify a published copy, including the newly retained direct parent.
    /// Unlike the immutable-source guard, every archive must already exist.
    pub fn verify_written_at(&self, destination: &Path) -> io::Result<()> {
        let store = packs::Store::open(destination)?;
        for digest in self.blobs.keys() {
            let bytes = store.read(destination, digest)?;
            if self.blobs.get(digest) != Some(&bytes.len()) {
                return Err(invalid("source lineage length changed"));
            }
        }
        Ok(())
    }
    /// Include the exact direct parent as a new blob, retaining inherited blobs.
    pub fn check_parent(&self, bytes: &[u8]) -> io::Result<String> {
        if bytes.len() > MAX_RECEIPT_BYTES {
            return Err(invalid("seed evidence exceeds64 MiB"));
        }
        let digest = sha256(bytes);
        if self.retained.contains_key(&digest) {
            return Ok(digest);
        }
        if !self.retained.is_empty() {
            return Err(invalid("native lineage already retained its direct parent"));
        }
        if self.frames.len() >= MAX_LINEAGE_RECEIPTS {
            return Err(invalid("native lineage work bound exceeded"));
        }
        // The single raw parent has its own per-document budget above. Its
        // header is already in the index: charging raw payload here as well
        // would reject previously valid large (32..64 MiB) direct parents.
        if self
            .index_bytes
            .checked_add(384)
            .is_none_or(|n| n > MAX_LINEAGE_INDEX_BYTES)
        {
            return Err(invalid("native lineage reference index exceeds64 MiB"));
        }
        Ok(digest)
    }
    pub fn retain_parent(&mut self, bytes: Vec<u8>) -> io::Result<String> {
        let digest = self.check_parent(&bytes)?;
        if self.retained.contains_key(&digest) {
            return Ok(digest);
        }
        self.reserve_index(384)?;
        self.blobs.insert(digest.clone(), bytes.len());
        self.retained.insert(digest.clone(), bytes);
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
            lineage.frames().last().unwrap().unwrap()["assessment_seeds"],
            json!([18])
        );
        assert_eq!(
            lineage.frames().last().unwrap().unwrap()["reserved_seed_ranges"],
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
        for digest in lineage.blobs.keys() {
            assert_eq!(
                fs::read(copy.0.join(DIRECTORY).join(format!("{digest}.json"))).unwrap(),
                lineage
                    .retained
                    .get(digest)
                    .cloned()
                    .unwrap_or_else(|| load_flat_blob(&tree.0, digest).unwrap())
            );
        }
    }
    #[test]
    fn historical_archive_exceeding_resident_budget_keeps_every_original_frame() {
        let source = Tree::new();
        // Noncanonical whitespace is part of the immutable source hash.
        let terminal = b"{ \"seed\" : 17, \"assessment_seeds\" : [18] }\n";
        let mut digest = source.store(terminal);
        let mut name = "cycle.json";
        let mut root = Value::Null;
        let mut archive_bytes = terminal.len();
        for seed in 21..89 {
            root = reference(name, &digest, seed);
            root["episode"] = json!({"diagnostic": "x".repeat(1024 * 1024),
                "opaque_metadata":{"assessment_seeds":[seed + 1000]}});
            if seed == 22 {
                root["fixture"] = json!(true);
            }
            let bytes = serde_json::to_vec(&root).unwrap();
            archive_bytes += bytes.len();
            digest = source.store(&bytes);
            name = "cpu-cycle.json";
        }
        assert!(archive_bytes > MAX_RECEIPT_BYTES);
        let mut lineage = Lineage::load(&root, &source.0).unwrap();
        assert!(lineage.index_bytes < 2 * 1024 * 1024);
        assert!(lineage.contains_fixture().unwrap());
        let mut visited = 0;
        for frame in lineage.frames() {
            let frame = frame.unwrap();
            if let Some(seed) = frame["seed"].as_u64().filter(|seed| *seed >= 21) {
                assert_eq!(
                    frame["episode"]["opaque_metadata"]["assessment_seeds"],
                    json!([seed + 1000])
                );
                assert_eq!(
                    frame["episode"]["diagnostic"].as_str().unwrap().len(),
                    1024 * 1024
                );
            }
            visited += 1;
        }
        assert_eq!(visited, 69);
        let parent = serde_json::to_vec(&root).unwrap();
        lineage.check_parent(&parent).unwrap();
        let parent_digest = lineage.retain_parent(parent.clone()).unwrap();
        let output = Tree::new();
        lineage.write_to(&output.0).unwrap();
        lineage.verify_at(&source.0).unwrap();
        for digest in lineage.blobs.keys() {
            assert_eq!(
                load_flat_blob(&output.0, digest).unwrap(),
                load_flat_blob(&source.0, digest).unwrap()
            );
        }
        let terminal_digest = sha256(terminal);
        assert_eq!(
            load_flat_blob(&output.0, &terminal_digest).unwrap(),
            terminal
        );
        drop(source);
        let child = reference("cpu-cycle.json", &parent_digest, 89);
        let restored = Lineage::load(&child, &output.0).unwrap();
        assert_eq!(
            restored.frames().last().unwrap().unwrap()["assessment_seeds"],
            json!([18])
        );
        // Even a correctly sized mutation in the oldest archive remains fatal.
        fs::write(
            output
                .0
                .join(DIRECTORY)
                .join(digest_name(&terminal_digest).unwrap()),
            terminal
                .iter()
                .map(|b| if *b == b'7' { b'9' } else { *b })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert!(restored.frames().any(|frame| frame.is_err()));
        assert!(restored.contains_fixture().is_err());
        assert!(restored.verify_at(&output.0).is_err());
        assert!(Lineage::load(&child, &output.0).is_err());
    }
    #[test]
    fn reference_and_document_budgets_still_reject_before_parent_retention() {
        let source = Tree::new();
        let digest = source.store(br#"{"seed":7}"#);
        let root = reference("cycle.json", &digest, 8);
        let mut lineage = Lineage::load(&root, &source.0).unwrap();
        assert!(lineage
            .check_parent(&vec![b' '; MAX_RECEIPT_BYTES + 1])
            .is_err());
        lineage.index_bytes = MAX_LINEAGE_INDEX_BYTES;
        assert!(lineage.check_parent(b"{}").is_err());
        assert!(lineage.reserve_index(1).is_err());
        lineage.index_bytes = 0;
        lineage.frames = (0..MAX_LINEAGE_RECEIPTS)
            .map(|_| Frame::Inline(Value::Null))
            .collect();
        assert!(lineage.push_frame(&Value::Null, None).is_err());
        assert!(lineage.check_parent(b"{}").is_err());
    }
    #[test]
    fn retaining_one_direct_parent_is_bounded_and_idempotent() {
        let source = Tree::new();
        let digest = source.store(br#"{"seed":7}"#);
        let root = reference("cycle.json", &digest, 8);
        let mut lineage = Lineage::load(&root, &source.0).unwrap();
        let bytes = serde_json::to_vec(&root).unwrap();
        let before = lineage.index_bytes;
        let retained = lineage.retain_parent(bytes.clone()).unwrap();
        assert_eq!(lineage.index_bytes, before + 384);
        assert_eq!(lineage.retain_parent(bytes).unwrap(), retained);
        assert_eq!(lineage.index_bytes, before + 384);
        assert!(lineage.retain_parent(b"{}".to_vec()).is_err());
        assert_eq!(lineage.retained.len(), 1);
    }
    #[test]
    fn large_valid_direct_parent_does_not_consume_the_index_budget_twice() {
        let source = Tree::new();
        let root = json!({"complete":true,"mode":"native_cpu_fresh_cohort","backend":"native-cpu",
            "receipt_filename":"cpu-cycle.json","objective_state_reset":false,
            "biological_objective_version":2,"critic_objective_version":1,"seed":7,"fixture":true,
            "seed_provenance":{"source_sha256":{}},
            "large_metadata":"x".repeat(40 * 1024 * 1024),"nested":{"assessment_seeds":[17]}});
        let bytes = serde_json::to_vec(&root).unwrap();
        assert!(bytes.len() > MAX_RECEIPT_BYTES / 2 && bytes.len() < MAX_RECEIPT_BYTES);
        let mut lineage = Lineage::load(&root, &source.0).unwrap();
        let before = lineage.index_bytes;
        lineage.check_parent(&bytes).unwrap();
        let digest = lineage.retain_parent(bytes.clone()).unwrap();
        assert_eq!(lineage.index_bytes, before + 384);
        let output = Tree::new();
        lineage.write_to(&output.0).unwrap();
        assert_eq!(load_flat_blob(&output.0, &digest).unwrap(), bytes);
        let child = reference("cpu-cycle.json", &digest, 8);
        let restored = Lineage::load(&child, &output.0).unwrap();
        let restored_parent = restored.frames().last().unwrap().unwrap();
        assert_eq!(restored_parent, root);
        assert!(restored.contains_fixture().unwrap());
    }
    #[test]
    fn large_inline_terminal_keeps_one_metadata_copy_and_complete_evidence() {
        let source = Tree::new();
        let terminal = json!({"seed":17,"large_metadata":"x".repeat(40 * 1024 * 1024),
            "nested":{"assessment_seeds":[18],"reserved_seed_ranges":[[19,20]]}});
        let mut root = reference(
            "cycle.json",
            &sha256(&serde_json::to_vec(&terminal).unwrap()),
            21,
        );
        root["seed_provenance"]
            .as_object_mut()
            .unwrap()
            .remove("native_source_receipt_ref");
        root["seed_provenance"]["native_source_receipt"] = terminal.clone();
        let bytes = serde_json::to_vec(&root).unwrap();
        assert!(bytes.len() > MAX_RECEIPT_BYTES / 2 && bytes.len() < MAX_RECEIPT_BYTES);
        let mut lineage = Lineage::load(&root, &source.0).unwrap();
        assert!(lineage.index_bytes < 41 * 1024 * 1024);
        assert_eq!(lineage.frames().last().unwrap().unwrap(), terminal);
        let digest = lineage.retain_parent(bytes.clone()).unwrap();
        let output = Tree::new();
        lineage.write_to(&output.0).unwrap();
        assert_eq!(load_flat_blob(&output.0, &digest).unwrap(), bytes);
        let child = reference("cpu-cycle.json", &digest, 22);
        let restored = Lineage::load(&child, &output.0).unwrap();
        assert_eq!(restored.frames().last().unwrap().unwrap(), terminal);
        assert_eq!(
            restored.terminal_proof.unwrap()["terminal_native_receipt"],
            terminal
        );
    }
    #[test]
    fn exported_ancestry_survives_source_removal_without_rewriting_blobs() {
        let source = Tree::new();
        let bytes = br#"{"seed":7}"#;
        let digest = source.store(bytes);
        let root = reference("cycle.json", &digest, 8);
        let lineage = Lineage::load(&root, &source.0).unwrap();
        let output = Tree::new();
        lineage.write_to(&output.0).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let name = digest_name(&digest).unwrap();
            let before = fs::metadata(source.0.join(DIRECTORY).join(&name)).unwrap();
            let after = fs::metadata(output.0.join(DIRECTORY).join(name)).unwrap();
            assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        }
        drop(source);
        assert_eq!(load_flat_blob(&output.0, &digest).unwrap(), bytes);
        assert_eq!(Lineage::load(&root, &output.0).unwrap().frames.len(), 2);
    }

    #[test]
    fn unsupported_links_fall_back_to_complete_verified_copies() {
        let source = Tree::new();
        let bytes = br#"{"seed":7}"#;
        let digest = source.store(bytes);
        let root = reference("cycle.json", &digest, 8);
        let lineage = Lineage::load(&root, &source.0).unwrap();
        let output = Tree::new();
        lineage
            .write_with_link(&output.0, |_, _| {
                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "fixture filesystem",
                ))
            })
            .unwrap();
        drop(source);
        assert_eq!(load_flat_blob(&output.0, &digest).unwrap(), bytes);
        assert_eq!(Lineage::load(&root, &output.0).unwrap().frames.len(), 2);
    }

    #[test]
    fn export_rejects_changed_imported_ancestry() {
        let source = Tree::new();
        let digest = source.store(br#"{"seed":7}"#);
        let lineage = Lineage::load(&reference("cycle.json", &digest, 8), &source.0).unwrap();
        fs::write(
            source.0.join(DIRECTORY).join(digest_name(&digest).unwrap()),
            b"{}",
        )
        .unwrap();
        let output = Tree::new();
        assert!(lineage.write_to(&output.0).is_err());
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
