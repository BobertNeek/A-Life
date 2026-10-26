//! Lossless private-life recovery. Legacy JSON stays readable; new profitable
//! encodings use a checksummed standard Zstd frame after a skippable metadata
//! frame binding the exact original JSON length and SHA256. No dictionary or
//! other checkpoint is needed to recover a file.
use super::{collector_output, CpuCanonicalCheckpoint, CpuContinuationResult};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

pub const CPU_PRIVATE_CHECKPOINT_FILE: &str = "collector-checkpoint.json.zst";
pub const CPU_LEGACY_PRIVATE_CHECKPOINT_FILE: &str = "collector-checkpoint.json";
pub const CPU_PRIVATE_CHECKPOINT_PENDING: &str = "collector-checkpoint.pending";
const SKIPPABLE: u32 = 0x184D2A50;
const TAG: &[u8] = b"ALIFE-CPU-PRIVATE1\n";
const METADATA_BYTES: usize = TAG.len() + 8 + 32;
// Larger private lives retain legacy JSON rather than gaining a new size limit.
const MAX_COMPRESSED_JSON: u64 = 1024 * 1024 * 1024;
const MIN_SAVINGS: usize = 4096;
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Exactly one published representation is authoritative. Never prefer a new
/// file silently when a legacy file is also present; an unsealed pending file
/// is inspectable explicitly, but is never selected as a published checkpoint.
pub fn cpu_private_checkpoint_path(root: &Path) -> io::Result<PathBuf> {
    let mut selected = None;
    for name in [
        CPU_LEGACY_PRIVATE_CHECKPOINT_FILE,
        CPU_PRIVATE_CHECKPOINT_FILE,
    ] {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(m) => {
                if !m.is_file() || m.file_type().is_symlink() {
                    return Err(invalid(
                        "private checkpoint must be a regular nonsymlink file",
                    ));
                }
                if selected.replace(path).is_some() {
                    return Err(invalid("ambiguous private checkpoint representations"));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    selected.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "published private checkpoint missing; preserve any pending file for explicit recovery",
        )
    })
}
pub fn read_cpu_private_checkpoint(root: &Path) -> CpuContinuationResult<CpuCanonicalCheckpoint> {
    Ok(serde_json::from_slice(&read_cpu_private_checkpoint_bytes(
        root,
        u64::MAX,
    )?)?)
}
pub fn read_cpu_private_checkpoint_bytes(root: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    read_cpu_private_checkpoint_file(&cpu_private_checkpoint_path(root)?, max_bytes)
}
/// Decode a specifically selected file, including a fully written unpublished
/// pending file after interrupted publication. This does not admit a collector
/// commit, resume a life, or publish/replace any file.
pub fn read_cpu_private_checkpoint_file(path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err(invalid(
            "private checkpoint must be a regular nonsymlink file",
        ));
    }
    let mut file = File::open(path)?;
    let mut prefix = Vec::new();
    Read::by_ref(&mut file).take(8).read_to_end(&mut prefix)?;
    if prefix.len() != 8 || u32::from_le_bytes(prefix[..4].try_into().unwrap()) != SKIPPABLE {
        if path.extension().is_some_and(|e| e == "zst") {
            return Err(invalid("invalid compressed private checkpoint header"));
        }
        if m.len() > max_bytes {
            return Err(invalid("private checkpoint exceeds caller byte limit"));
        }
        Read::by_ref(&mut file)
            .take(
                max_bytes
                    .saturating_add(1)
                    .saturating_sub(prefix.len() as u64),
            )
            .read_to_end(&mut prefix)?;
        if prefix.len() as u64 > max_bytes {
            return Err(invalid("private checkpoint exceeds caller byte limit"));
        }
        return Ok(prefix);
    }
    if u32::from_le_bytes(prefix[4..].try_into().unwrap()) as usize != METADATA_BYTES {
        return Err(invalid("invalid private checkpoint metadata length"));
    }
    let mut metadata = vec![0; METADATA_BYTES];
    file.read_exact(&mut metadata)?;
    if &metadata[..TAG.len()] != TAG {
        return Err(invalid("unsupported private checkpoint encoding"));
    }
    let raw_bytes = u64::from_le_bytes(metadata[TAG.len()..TAG.len() + 8].try_into().unwrap());
    if raw_bytes > MAX_COMPRESSED_JSON
        || raw_bytes > max_bytes
        || m.len() > MAX_COMPRESSED_JSON + 1024 * 1024
    {
        return Err(invalid("compressed private checkpoint exceeds byte limit"));
    }
    let expected = &metadata[TAG.len() + 8..];
    let mut decoder =
        zstd::stream::read::Decoder::with_buffer(BufReader::new(file))?.single_frame();
    decoder.window_log_max(raw_bytes.max(1024).next_power_of_two().trailing_zeros())?;
    let mut raw = Vec::new();
    Read::by_ref(&mut decoder)
        .take(raw_bytes + 1)
        .read_to_end(&mut raw)?;
    if raw.len() as u64 != raw_bytes || &Sha256::digest(&raw)[..] != expected {
        return Err(invalid("private checkpoint JSON length or SHA256 mismatch"));
    }
    if !decoder.finish().fill_buf()?.is_empty() {
        return Err(invalid("trailing private checkpoint data"));
    }
    Ok(raw)
}
fn encode(raw: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend(SKIPPABLE.to_le_bytes());
    out.extend((METADATA_BYTES as u32).to_le_bytes());
    out.extend(TAG);
    out.extend((raw.len() as u64).to_le_bytes());
    out.extend(Sha256::digest(raw));
    let mut encoder = zstd::stream::write::Encoder::new(out, 1)?;
    encoder.include_checksum(true)?;
    encoder.set_pledged_src_size(Some(raw.len() as u64))?;
    encoder.write_all(raw)?;
    encoder.finish()
}
/// Write one private snapshot into an exclusively owned output. Existing final
/// or pending work is never overwritten. Small/incompressible or >1 GiB JSON
/// keeps its legacy representation; profitable snapshots use lossless Zstd.
pub fn write_cpu_private_checkpoint(
    root: &Path,
    checkpoint: &CpuCanonicalCheckpoint,
) -> io::Result<()> {
    let raw = serde_json::to_vec(checkpoint).map_err(io::Error::other)?;
    write_with_publish(root, &raw, collector_output::publish_new)
}
fn write_with_publish(
    root: &Path,
    raw: &[u8],
    publish: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    for name in [
        CPU_LEGACY_PRIVATE_CHECKPOINT_FILE,
        CPU_PRIVATE_CHECKPOINT_FILE,
    ] {
        match fs::symlink_metadata(root.join(name)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "private checkpoint already exists; original preserved",
                ))
            }
        }
    }
    let compressed = if raw.len() >= MIN_SAVINGS && raw.len() as u64 <= MAX_COMPRESSED_JSON {
        Some(encode(raw)?)
    } else {
        None
    };
    let (name, bytes) = match compressed.as_ref() {
        Some(bytes)
            if bytes
                .len()
                .checked_add(MIN_SAVINGS)
                .is_some_and(|n| n < raw.len()) =>
        {
            (CPU_PRIVATE_CHECKPOINT_FILE, bytes.as_slice())
        }
        _ => (CPU_LEGACY_PRIVATE_CHECKPOINT_FILE, raw),
    };
    let pending = root.join(CPU_PRIVATE_CHECKPOINT_PENDING);
    let target = root.join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    // Verify the complete encoded candidate without another full JSON decode.
    // The encoder and SHA256 bind the canonical bytes; the read path verifies
    // them on recovery. Atomic publication only moves a closed immutable file.
    let mut saved = File::open(&pending)?;
    let mut buffer = [0; 64 * 1024];
    for expected in bytes.chunks(buffer.len()) {
        saved.read_exact(&mut buffer[..expected.len()])?;
        if &buffer[..expected.len()] != expected {
            return Err(invalid("private checkpoint changed before publication"));
        }
    }
    if saved.read(&mut buffer[..1])? != 0 {
        return Err(invalid("trailing private checkpoint write"));
    }
    drop(saved);
    collector_output::sync_directory_ancestry(root)?;
    publish(&pending, &target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "alife-private-codec-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn raw() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"weights":[-0.0,1e-30,1.2345678],"ages":[0,19800,u32::MAX],"private_state":"memory/biology ".repeat(10000)})).unwrap()
    }
    #[test]
    fn exact_raw_bytes_standard_zstd_recovery_and_portable_copy() {
        let root = Directory::new();
        let data = raw();
        write_with_publish(&root.0, &data, collector_output::publish_new).unwrap();
        let path = cpu_private_checkpoint_path(&root.0).unwrap();
        assert_eq!(path.file_name().unwrap(), CPU_PRIVATE_CHECKPOINT_FILE);
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).unwrap(),
            data
        );
        assert_eq!(
            zstd::stream::decode_all(&fs::read(&path).unwrap()[..]).unwrap(),
            data
        );
        let copy = Directory::new();
        fs::copy(path, copy.0.join(CPU_PRIVATE_CHECKPOINT_FILE)).unwrap();
        drop(root);
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&copy.0, u64::MAX).unwrap(),
            data
        );
        assert!(read_cpu_private_checkpoint_bytes(&copy.0, data.len() as u64 - 1).is_err());
        assert!(write_with_publish(
            &copy.0,
            b"replace private state",
            collector_output::publish_new
        )
        .is_err());
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&copy.0, u64::MAX).unwrap(),
            data
        );
    }
    #[test]
    fn legacy_plain_json_and_small_fallback_remain_exact_and_ambiguity_rejects() {
        let root = Directory::new();
        let data = b" {\"old\":1,\"age\":4294967295} \n";
        write_with_publish(&root.0, data, collector_output::publish_new).unwrap();
        assert_eq!(
            cpu_private_checkpoint_path(&root.0)
                .unwrap()
                .file_name()
                .unwrap(),
            CPU_LEGACY_PRIVATE_CHECKPOINT_FILE
        );
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).unwrap(),
            data
        );
        fs::write(
            root.0.join(CPU_PRIVATE_CHECKPOINT_FILE),
            encode(&raw()).unwrap(),
        )
        .unwrap();
        assert!(cpu_private_checkpoint_path(&root.0).is_err());
    }
    #[test]
    fn corruption_truncation_trailing_frames_and_resource_claims_fail_closed() {
        let root = Directory::new();
        let data = raw();
        let encoded = encode(&data).unwrap();
        let path = root.0.join(CPU_PRIVATE_CHECKPOINT_FILE);
        let mut cuts = (0..8 + METADATA_BYTES).collect::<Vec<_>>();
        cuts.extend([encoded.len() / 2, encoded.len() - 4, encoded.len() - 1]);
        for cut in cuts {
            fs::write(&path, &encoded[..cut]).unwrap();
            assert!(
                read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).is_err(),
                "cut {cut}"
            );
        }
        for index in [
            0,
            4,
            8,
            8 + TAG.len(),
            8 + TAG.len() + 8,
            encoded.len() / 2,
            encoded.len() - 1,
        ] {
            let mut bad = encoded.clone();
            bad[index] ^= 1;
            fs::write(&path, bad).unwrap();
            assert!(
                read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).is_err(),
                "byte {index}"
            );
        }
        for tail in [b"trailing".as_slice(), encoded.as_slice()] {
            let mut bad = encoded.clone();
            bad.extend(tail);
            fs::write(&path, bad).unwrap();
            assert!(read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).is_err());
        }
        let mut oversized = encoded.clone();
        oversized[8 + TAG.len()..8 + TAG.len() + 8]
            .copy_from_slice(&(MAX_COMPRESSED_JSON + 1).to_le_bytes());
        fs::write(&path, oversized).unwrap();
        assert!(read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).is_err());
    }
    #[test]
    fn interruption_after_publication_keeps_the_complete_file_and_never_overwrites() {
        let root = Directory::new();
        let data = raw();
        assert!(write_with_publish(&root.0, &data, |pending, target| {
            fs::rename(pending, target)?;
            Err(io::Error::other("interrupted after atomic publication"))
        })
        .is_err());
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).unwrap(),
            data
        );
        assert!(
            write_with_publish(&root.0, b"other state", collector_output::publish_new).is_err()
        );
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&root.0, u64::MAX).unwrap(),
            data
        );
    }

    #[test]
    fn interrupted_publication_keeps_exact_pending_recovery_and_retries_never_overwrite() {
        let root = Directory::new();
        let data = raw();
        assert!(
            write_with_publish(&root.0, &data, |_, _| Err(io::Error::other(
                "interrupted before publish"
            )))
            .is_err()
        );
        assert!(cpu_private_checkpoint_path(&root.0).is_err());
        let pending = root.0.join(CPU_PRIVATE_CHECKPOINT_PENDING);
        let encoded = fs::read(&pending).unwrap();
        assert_eq!(
            read_cpu_private_checkpoint_file(&pending, u64::MAX).unwrap(),
            data
        );
        assert!(
            write_with_publish(&root.0, b"other state", collector_output::publish_new).is_err()
        );
        assert_eq!(fs::read(&pending).unwrap(), encoded);
        let copy = Directory::new();
        let saved = copy.0.join("preserved-private.pending");
        fs::copy(&pending, &saved).unwrap();
        drop(root);
        assert_eq!(
            read_cpu_private_checkpoint_file(&saved, u64::MAX).unwrap(),
            data
        );
        fs::write(&saved, &encoded[..encoded.len() / 2]).unwrap();
        assert!(read_cpu_private_checkpoint_file(&saved, u64::MAX).is_err());
    }
}
