//! Portable immutable receipt packs. The manifest is a binary-carry forest:
//! each active pack contains a distinct power-of-two count of exact raw receipts.
use super::*;
use std::io::{BufWriter, Seek, SeekFrom, Write};

const MANIFEST: &str = "manifest.json";
const MAGIC: &[u8] = b"ALIFE-ANCESTRY1\n";
const END: &[u8] = b"ALIFE-END\n";
const RECORD_HEADER: u64 = 72;
const MAX_MANIFEST: usize = 64 * 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    sha256: String,
    receipts: usize,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u16,
    chunks: Vec<Object>,
}
#[derive(Debug)]
struct Slot {
    offset: u64,
    bytes: usize,
}
#[derive(Debug)]
pub(super) struct Chunk {
    object: Object,
    order: Vec<String>,
    slots: BTreeMap<String, Slot>,
    bytes: u64,
}
#[derive(Debug)]
pub(super) enum Store {
    Flat,
    Packed {
        manifest: String,
        chunks: Vec<Chunk>,
    },
}
fn object_name(digest: &str) -> io::Result<String> {
    digest_name(digest)?;
    Ok(format!("{digest}.pack"))
}
fn regular(path: &Path) -> io::Result<fs::Metadata> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err(invalid("ancestry object must be a regular nonsymlink file"));
    }
    Ok(m)
}
fn directory(directory: &Path) -> io::Result<PathBuf> {
    let base = directory.join(DIRECTORY);
    let m = fs::symlink_metadata(&base)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err(invalid("lineage directory must be a real directory"));
    }
    Ok(base)
}
fn manifest_bytes(base: &Path) -> io::Result<Vec<u8>> {
    let path = base.join(MANIFEST);
    if regular(&path)?.len() > MAX_MANIFEST as u64 {
        return Err(invalid("ancestry manifest exceeds byte bound"));
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(MAX_MANIFEST as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_MANIFEST {
        return Err(invalid("ancestry manifest exceeds byte bound"));
    }
    Ok(bytes)
}
fn chunk(base: &Path, object: &Object) -> io::Result<Chunk> {
    let path = base.join(object_name(&object.sha256)?);
    let length = regular(&path)?.len();
    let maximum = (MAGIC.len() + 8 + END.len()) as u64
        + object.receipts as u64 * (RECORD_HEADER + MAX_RECEIPT_BYTES as u64);
    if length > maximum {
        return Err(invalid("ancestry pack exceeds declared bound"));
    }
    let mut file = fs::File::open(&path)?;
    let mut hash = Sha256::new();
    let read = io::copy(&mut Read::by_ref(&mut file).take(length + 1), &mut hash)?;
    if read != length || format!("{:x}", hash.finalize()) != object.sha256 {
        return Err(invalid(
            "ancestry pack differs from its immutable object hash",
        ));
    }
    file.rewind()?;
    let mut magic = vec![0; MAGIC.len()];
    file.read_exact(&mut magic)?;
    let mut word = [0; 8];
    file.read_exact(&mut word)?;
    if magic != MAGIC || u64::from_le_bytes(word) != object.receipts as u64 {
        return Err(invalid("invalid ancestry pack header"));
    }
    let mut slots = BTreeMap::new();
    let mut order = Vec::new();
    for _ in 0..object.receipts {
        let mut digest = [0; 64];
        file.read_exact(&mut digest)?;
        let digest = std::str::from_utf8(&digest)
            .map_err(io::Error::other)?
            .to_owned();
        digest_name(&digest)?;
        file.read_exact(&mut word)?;
        let bytes = u64::from_le_bytes(word);
        let offset = file.stream_position()?;
        if bytes > MAX_RECEIPT_BYTES as u64
            || offset.checked_add(bytes).is_none_or(|end| end > length)
        {
            return Err(invalid("ancestry receipt exceeds pack or document bound"));
        }
        if slots
            .insert(
                digest.clone(),
                Slot {
                    offset,
                    bytes: bytes as usize,
                },
            )
            .is_some()
        {
            return Err(invalid("duplicate ancestry receipt in pack"));
        }
        order.push(digest);
        file.seek(SeekFrom::Start(offset + bytes))?;
    }
    let mut end = vec![0; END.len()];
    file.read_exact(&mut end)?;
    if end != END || file.stream_position()? != length {
        return Err(invalid("trailing or incomplete ancestry pack"));
    }
    Ok(Chunk {
        object: object.clone(),
        order,
        slots,
        bytes: length,
    })
}
impl Store {
    pub(super) fn open(root: &Path) -> io::Result<Self> {
        let base = root.join(DIRECTORY);
        match fs::symlink_metadata(&base) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::Flat),
            Err(e) => return Err(e),
            Ok(_) => (),
        }
        let base = directory(root)?;
        match fs::symlink_metadata(base.join(MANIFEST)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                for entry in fs::read_dir(&base)? {
                    let path = entry?.path();
                    if path
                        .extension()
                        .is_some_and(|e| e == "pack" || e == "pending")
                    {
                        return Err(invalid("incomplete packed ancestry has no manifest"));
                    }
                }
                return Ok(Self::Flat);
            }
            Err(e) => return Err(e),
            Ok(_) => (),
        }
        let raw = manifest_bytes(&base)?;
        let m: Manifest = serde_json::from_value(parse(&raw)?).map_err(io::Error::other)?;
        if m.schema_version != 1 || m.chunks.is_empty() || m.chunks.len() > 17 {
            return Err(invalid("invalid ancestry pack manifest"));
        }
        let mut previous = MAX_LINEAGE_RECEIPTS.next_power_of_two();
        let mut count = 0usize;
        let mut chunks = Vec::new();
        let mut seen = BTreeSet::new();
        let mut names = BTreeSet::from([MANIFEST.to_owned()]);
        for object in m.chunks {
            if !object.receipts.is_power_of_two() || object.receipts >= previous {
                return Err(invalid(
                    "ancestry pack counts must be decreasing powers of two",
                ));
            }
            count = count
                .checked_add(object.receipts)
                .filter(|n| *n <= MAX_LINEAGE_RECEIPTS)
                .ok_or_else(|| invalid("ancestry pack work bound exceeded"))?;
            previous = object.receipts;
            names.insert(object_name(&object.sha256)?);
            let c = chunk(&base, &object)?;
            for digest in &c.order {
                if !seen.insert(digest.clone()) {
                    return Err(invalid("duplicate ancestry receipt across packs"));
                }
            }
            chunks.push(c);
        }
        for entry in fs::read_dir(&base)? {
            if !names.contains(&entry?.file_name().to_string_lossy().into_owned()) {
                return Err(invalid("foreign file in packed ancestry"));
            }
        }
        Ok(Self::Packed {
            manifest: sha256(&raw),
            chunks,
        })
    }
    pub(super) fn same_manifest(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Flat, Self::Flat) => true,
            (Self::Packed { manifest: a, .. }, Self::Packed { manifest: b, .. }) => a == b,
            _ => false,
        }
    }
    pub(super) fn read(&self, root: &Path, digest: &str) -> io::Result<Vec<u8>> {
        digest_name(digest)?;
        let Self::Packed { manifest, chunks } = self else {
            return super::load_flat_blob(root, digest);
        };
        let base = directory(root)?;
        if sha256(&manifest_bytes(&base)?) != *manifest {
            return Err(invalid("source ancestry manifest changed"));
        }
        for c in chunks {
            if let Some(slot) = c.slots.get(digest) {
                let path = base.join(object_name(&c.object.sha256)?);
                if regular(&path)?.len() != c.bytes {
                    return Err(invalid("source ancestry pack length changed"));
                }
                let mut file = fs::File::open(path)?;
                file.seek(SeekFrom::Start(slot.offset - RECORD_HEADER))?;
                let mut header = [0; 72];
                file.read_exact(&mut header)?;
                if &header[..64] != digest.as_bytes()
                    || u64::from_le_bytes(header[64..].try_into().unwrap()) != slot.bytes as u64
                {
                    return Err(invalid("source ancestry pack index changed"));
                }
                let mut bytes = vec![0; slot.bytes];
                file.read_exact(&mut bytes)?;
                if sha256(&bytes) != digest {
                    return Err(invalid("lineage blob differs from pinned SHA256"));
                }
                return Ok(bytes);
            }
        }
        Err(invalid("missing ancestry receipt in packs"))
    }
}

fn receipt(lineage: &Lineage, digest: &str) -> io::Result<Vec<u8>> {
    let bytes = if let Some(bytes) = lineage.retained.get(digest) {
        bytes.clone()
    } else {
        lineage.imported_bytes(digest)?
    };
    if lineage.blobs.get(digest) != Some(&bytes.len()) || sha256(&bytes) != digest {
        return Err(invalid("retained ancestry bytes changed"));
    }
    Ok(bytes)
}
fn plan(lineage: &Lineage) -> Vec<Vec<String>> {
    let inherited: BTreeSet<_> = lineage
        .blobs
        .keys()
        .filter(|d| !lineage.retained.contains_key(*d))
        .cloned()
        .collect();
    let mut forest = if let Store::Packed { chunks, .. } = &lineage.store {
        let members: BTreeSet<_> = chunks
            .iter()
            .flat_map(|c| c.order.iter().cloned())
            .collect();
        if members == inherited {
            chunks.iter().map(|c| c.order.clone()).collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    if forest.is_empty() {
        let mut keys = inherited.into_iter().collect::<Vec<_>>().into_iter();
        while keys.len() > 0 {
            let size = 1usize << (usize::BITS - 1 - keys.len().leading_zeros());
            forest.push(keys.by_ref().take(size).collect());
        }
    }
    for digest in lineage.retained.keys() {
        let mut carry = vec![digest.clone()];
        while forest.last().is_some_and(|last| last.len() == carry.len()) {
            let mut left = forest.pop().unwrap();
            left.extend(carry);
            carry = left;
        }
        forest.push(carry);
    }
    forest
}
fn encode(lineage: &Lineage, members: &[String], out: &mut impl Write) -> io::Result<String> {
    let mut digest = Sha256::new();
    let mut put = |bytes: &[u8]| -> io::Result<()> {
        out.write_all(bytes)?;
        digest.update(bytes);
        Ok(())
    };
    put(MAGIC)?;
    put(&(members.len() as u64).to_le_bytes())?;
    for name in members {
        let bytes = receipt(lineage, name)?;
        put(name.as_bytes())?;
        put(&(bytes.len() as u64).to_le_bytes())?;
        put(&bytes)?;
    }
    put(END)?;
    Ok(format!("{:x}", digest.finalize()))
}
fn manifest(objects: Vec<Object>) -> io::Result<Vec<u8>> {
    serde_json::to_vec(&Manifest {
        schema_version: 1,
        chunks: objects,
    })
    .map_err(io::Error::other)
}
fn create(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
pub(super) fn write_to(lineage: &Lineage, root: &Path) -> io::Result<()> {
    write_with(lineage, root, |s, t| fs::hard_link(s, t), create)
}
fn write_with(
    lineage: &Lineage,
    root: &Path,
    link: impl Fn(&Path, &Path) -> io::Result<()>,
    publish: impl Fn(&Path, &[u8]) -> io::Result<()>,
) -> io::Result<()> {
    if lineage.blobs.is_empty() {
        return Ok(());
    }
    let base = root.join(DIRECTORY);
    fs::create_dir(&base)?;
    let mut objects = Vec::new();
    for members in plan(lineage) {
        let reuse = match &lineage.store {
            Store::Packed { chunks, .. } => chunks.iter().find(|c| c.order == members),
            _ => None,
        };
        let object = if let Some(c) = reuse {
            // Only an already verified, content-addressed immutable pack may be
            // linked. Manifests and mutable checkpoint/source names never are.
            let source_base = directory(&lineage.source_directory)?;
            chunk(&source_base, &c.object)?;
            let name = object_name(&c.object.sha256)?;
            let target = base.join(&name);
            if link(&source_base.join(&name), &target).is_err() {
                fs::copy(source_base.join(name), &target)?;
            }
            chunk(&base, &c.object)?;
            c.object.clone()
        } else {
            let pending = base.join("object.pending");
            let file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pending)?;
            let mut writer = BufWriter::new(file);
            let hash = encode(lineage, &members, &mut writer)?;
            writer.flush()?;
            writer.get_ref().sync_all()?;
            drop(writer);
            let object = Object {
                sha256: hash,
                receipts: members.len(),
            };
            fs::rename(pending, base.join(object_name(&object.sha256)?))?;
            chunk(&base, &object)?;
            object
        };
        objects.push(object);
    }
    // Last write commits the archive. A torn manifest or absent object is fatal;
    // retries do not overwrite the preserved, exclusively owned partial tree.
    publish(&base.join(MANIFEST), &manifest(objects)?)?;
    lineage.verify_written_at(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "alife-packs-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn child(&self, name: impl AsRef<Path>) -> PathBuf {
            let root = self.0.join(name);
            fs::create_dir(&root).unwrap();
            root
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn parent(digest: &str, seed: usize) -> Value {
        json!({"complete":true,"mode":"native_cpu_fresh_cohort","backend":"native-cpu","receipt_filename":"cpu-cycle.json",
            "objective_state_reset":false,"biological_objective_version":2,"critic_objective_version":1,"seed":seed,
            "seed_provenance":{"source_sha256":{"cpu-cycle.json":digest,"trained.alife-foundation":"asset","actor-checkpoint.json":"actor","value-checkpoint.json":"value"},"native_source_receipt_ref":digest}})
    }
    fn terminal() -> Value {
        let mut root = parent("unused", 1);
        root["fixture"] = json!(true);
        root["seed_provenance"] = json!({"source_sha256":{}});
        root
    }
    fn append(root: &Value, source: &Path, target: &Path, packed: bool) -> Value {
        let bytes = serde_json::to_vec(root).unwrap();
        let mut l = Lineage::load(root, source).unwrap();
        let digest = l.retain_parent(bytes).unwrap();
        if packed {
            l.write_packed_to(target).unwrap()
        } else {
            l.write_to(target).unwrap()
        }
        parent(&digest, l.blobs.len())
    }
    fn counts(roots: &[PathBuf]) -> (usize, u64, Option<u64>) {
        let mut count = 0;
        let mut bytes = 0;
        #[cfg(unix)]
        let mut inodes = BTreeSet::new();
        #[cfg(unix)]
        let mut allocated = 0;
        for root in roots {
            for e in fs::read_dir(root.join(DIRECTORY)).unwrap() {
                let m = e.unwrap().metadata().unwrap();
                count += 1;
                bytes += m.len();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if inodes.insert((m.dev(), m.ino())) {
                        allocated += m.blocks() * 512;
                    }
                }
            }
        }
        let allocation = {
            #[cfg(unix)]
            {
                Some(allocated)
            }
            #[cfg(not(unix))]
            {
                None
            }
        };
        (count, bytes, allocation)
    }
    #[test]
    fn forest_preserves_all_raw_bytes_with_subquadratic_paths_and_independent_copy() {
        let tree = Tree::new();
        let empty = tree.child("empty");
        let mut root = terminal();
        root["historical_seeds"] = json!([998877]);
        root["raw"] = json!(" preserved ");
        let (mut p, mut f) = (empty.clone(), empty);
        let (mut packed, mut flat) = (Vec::new(), Vec::new());
        for n in 1usize..=128 {
            let next = tree.child(format!("p{n}"));
            let old = tree.child(format!("f{n}"));
            let updated = append(&root, &p, &next, true);
            assert_eq!(updated, append(&root, &f, &old, false));
            let a = Lineage::load(&updated, &next).unwrap();
            let b = Lineage::load(&updated, &old).unwrap();
            assert_eq!(
                a.frames().collect::<io::Result<Vec<_>>>().unwrap(),
                b.frames().collect::<io::Result<Vec<_>>>().unwrap()
            );
            let files = fs::read_dir(next.join(DIRECTORY)).unwrap().count();
            assert_eq!(files, n.count_ones() as usize + 1);
            root = updated;
            p = next.clone();
            f = old.clone();
            packed.push(next);
            flat.push(old);
        }
        println!("ANCESTRY_STORAGE 128 receipts packed={:?} flat={:?}; (files,logical bytes,inode allocated bytes)",counts(&packed),counts(&flat));
        let l = Lineage::load(&root, &p).unwrap();
        let expected = l.frames().collect::<io::Result<Vec<_>>>().unwrap();
        let copied = tree.child("copied");
        // Force the portable copy fallback, then remove every original bundle.
        write_with(
            &l,
            &copied,
            |_, _| Err(io::Error::other("no hardlinks")),
            create,
        )
        .unwrap();
        for original in packed.iter().chain(&flat) {
            fs::remove_dir_all(original).unwrap();
        }
        assert_eq!(
            Lineage::load(&root, &copied)
                .unwrap()
                .frames()
                .collect::<io::Result<Vec<_>>>()
                .unwrap(),
            expected
        );
        let store = Store::open(&copied).unwrap();
        let Store::Packed { chunks, .. } = store else {
            panic!()
        };
        let path = copied
            .join(DIRECTORY)
            .join(object_name(&chunks[0].object.sha256).unwrap());
        let bytes = fs::read(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert!(Lineage::load(&root, &copied).is_err());
        fs::write(&path, &bytes).unwrap();
        let mut corrupt = bytes;
        corrupt[MAGIC.len() + 8 + 72] ^= 1;
        fs::write(&path, corrupt).unwrap();
        assert!(Lineage::load(&root, &copied).is_err());
    }
    #[test]
    fn interrupted_manifest_and_source_tampering_fail_closed_without_overwriting() {
        let tree = Tree::new();
        let empty = tree.child("empty");
        let source = tree.child("source");
        let root = append(&terminal(), &empty, &source, true);
        let mut l = Lineage::load(&root, &source).unwrap();
        l.retain_parent(serde_json::to_vec(&root).unwrap()).unwrap();
        let partial = tree.child("partial");
        assert!(write_with(
            &l,
            &partial,
            |_, _| Err(io::Error::other("copy")),
            |_, _| Err(io::Error::other("power loss"))
        )
        .is_err());
        let entries = fs::read_dir(partial.join(DIRECTORY))
            .unwrap()
            .map(|e| {
                let p = e.unwrap().path();
                (p.clone(), fs::read(p).unwrap())
            })
            .collect::<Vec<_>>();
        assert!(Store::open(&partial).is_err());
        assert!(l.write_packed_to(&partial).is_err());
        for (p, b) in entries {
            assert_eq!(fs::read(p).unwrap(), b);
        }
        l.verify_at(&source).unwrap();
        let Store::Packed { chunks, .. } = Store::open(&source).unwrap() else {
            panic!()
        };
        let path = source
            .join(DIRECTORY)
            .join(object_name(&chunks[0].object.sha256).unwrap());
        let mut raw = fs::read(&path).unwrap();
        raw[MAGIC.len() + 8 + 72] ^= 1;
        fs::write(&path, raw).unwrap();
        assert!(l.frames().collect::<io::Result<Vec<_>>>().is_err());
        assert!(l.verify_at(&source).is_err());
        assert!(l.write_packed_to(&tree.child("rejected")).is_err());
    }
    #[test]
    fn correctly_hashed_malformed_packs_and_manifest_changes_are_rejected() {
        let tree = Tree::new();
        let empty = tree.child("empty");
        let source = tree.child("source");
        let root = append(&terminal(), &empty, &source, true);
        let lineage = Lineage::load(&root, &source).unwrap();
        let base = source.join(DIRECTORY);
        let original_manifest = fs::read(base.join(MANIFEST)).unwrap();
        let original: Manifest = serde_json::from_slice(&original_manifest).unwrap();
        let bytes = fs::read(base.join(object_name(&original.chunks[0].sha256).unwrap())).unwrap();
        for mutation in 0..5 {
            let target = tree.child(format!("malformed{mutation}"));
            let dir = target.join(DIRECTORY);
            fs::create_dir(&dir).unwrap();
            let mut corrupt = bytes.clone();
            match mutation {
                0 => corrupt.push(0),
                1 => {
                    corrupt.pop();
                }
                2 => corrupt[MAGIC.len() + 8..MAGIC.len() + 8 + 64].fill(b'0'),
                3 => corrupt[MAGIC.len() + 8 + 64..MAGIC.len() + 8 + 72]
                    .copy_from_slice(&(MAX_RECEIPT_BYTES as u64 + 1).to_le_bytes()),
                _ => corrupt[MAGIC.len() + 8 + 72] ^= 1,
            }
            let digest = sha256(&corrupt);
            fs::write(dir.join(object_name(&digest).unwrap()), corrupt).unwrap();
            fs::write(
                dir.join(MANIFEST),
                manifest(vec![Object {
                    sha256: digest,
                    receipts: 1,
                }])
                .unwrap(),
            )
            .unwrap();
            assert!(Lineage::load(&root, &target).is_err());
        }
        // A semantically equivalent manifest rewrite still trips the cached
        // predecessor identity guard after admission.
        let mut changed = original_manifest.clone();
        changed.push(b'\n');
        fs::write(base.join(MANIFEST), changed).unwrap();
        assert!(lineage.verify_at(&source).is_err());
        assert!(lineage.frames().collect::<io::Result<Vec<_>>>().is_err());
        fs::write(base.join(MANIFEST), original_manifest).unwrap();
        lineage.verify_at(&source).unwrap();
    }

    #[test]
    fn packed_serial_history_can_exceed_one_document_bound() {
        let tree = Tree::new();
        let empty = tree.child("empty");
        let first = tree.child("one");
        let second = tree.child("two");
        let mut large = terminal();
        large["padding"] = json!("x".repeat(34 * 1024 * 1024));
        let mut root = append(&large, &empty, &first, true);
        root["padding"] = json!("y".repeat(34 * 1024 * 1024));
        let root = append(&root, &first, &second, true);
        let l = Lineage::load(&root, &second).unwrap();
        assert_eq!(l.frames().count(), 3);
        l.verify_at(&second).unwrap();
    }
}
