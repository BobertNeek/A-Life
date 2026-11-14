//! LOCAL-only evidence persistence. Never overwrites or retries a run directory.
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
pub struct LocalEvidence {
    directory: PathBuf,
    last_valid: Option<Vec<u8>>,
}
impl LocalEvidence {
    pub fn begin(root: &Path, id: &str) -> io::Result<Self> {
        if id.is_empty()
            || id.len() > 96
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(io::Error::other("simple unique evidence_id required"));
        }
        fs::create_dir_all(root)?;
        // Raw evidence must not be uploaded. Never alter an existing ignore file.
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(".gitignore"))
        {
            Ok(mut file) => {
                file.write_all(b"*\n")?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        let directory = root.join(id);
        fs::create_dir(&directory)?;
        let evidence = Self {
            directory,
            last_valid: None,
        };
        // Always protect the newly claimed run, even when the caller's outer
        // ignore file exists but does not cover raw checkpoints. Failure leaves
        // a claimed, non-retryable directory and no raw evidence is written.
        evidence.save(".gitignore", b"*\n")?;
        Ok(evidence)
    }
    pub fn save(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name == "."
            || name == ".."
        {
            return Err(io::Error::other("invalid evidence filename"));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.directory.join(name))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        #[cfg(unix)]
        fs::File::open(&self.directory)?.sync_all()?;
        Ok(())
    }
    pub fn remember_valid(&mut self, bytes: Vec<u8>) {
        self.last_valid = Some(bytes);
    }
    pub fn retain_error_checkpoint(&self) -> io::Result<bool> {
        if let Some(bytes) = &self.last_valid {
            self.save("ERROR-LAST-VALID.cp.json", bytes)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "terminal-evidence-{}-{}-{tag}-LOCAL",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn create_only_and_error_checkpoint_preserve_bytes() {
        let root = root("retention");
        let mut e = LocalEvidence::begin(&root, "seed1-active").unwrap();
        e.save("BIRTH.cp.json", b"native-birth").unwrap();
        assert!(e.save("BIRTH.cp.json", b"overwrite").is_err());
        assert!(LocalEvidence::begin(&root, "seed1-active").is_err());
        e.remember_valid(b"latest-native-state".to_vec());
        assert!(e.retain_error_checkpoint().unwrap());
        assert_eq!(
            fs::read(e.directory().join("ERROR-LAST-VALID.cp.json")).unwrap(),
            b"latest-native-state"
        );
        assert_eq!(
            fs::read(e.directory().join("BIRTH.cp.json")).unwrap(),
            b"native-birth"
        );
        assert!(e.retain_error_checkpoint().is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn new_roots_are_created_and_unsafe_ids_reject() {
        let root = root("guard");
        let nested = root.join("ordinary-name");
        assert!(LocalEvidence::begin(&nested, "../reuse").is_err());
        let e = LocalEvidence::begin(&nested, "id").unwrap();
        assert_eq!(fs::read(nested.join(".gitignore")).unwrap(), b"*\n");
        assert!(e.directory().is_dir());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn existing_noncovering_outer_ignore_and_other_files_are_untouched() {
        let root = root("outer-ignore");
        fs::write(root.join(".gitignore"), b"target/\n").unwrap();
        fs::write(root.join("source.rs"), b"preserved source").unwrap();
        fs::create_dir(root.join("prior-run")).unwrap();
        fs::write(root.join("prior-run/CP.json"), b"preserved life").unwrap();
        let e = LocalEvidence::begin(&root, "new-run").unwrap();
        assert_eq!(fs::read(e.directory().join(".gitignore")).unwrap(), b"*\n");
        e.save("CP.json", b"new life").unwrap();
        assert!(LocalEvidence::begin(&root, "new-run").is_err());
        assert_eq!(fs::read(root.join(".gitignore")).unwrap(), b"target/\n");
        assert_eq!(
            fs::read(root.join("source.rs")).unwrap(),
            b"preserved source"
        );
        assert_eq!(
            fs::read(root.join("prior-run/CP.json")).unwrap(),
            b"preserved life"
        );
        assert_eq!(fs::read(root.join("new-run/CP.json")).unwrap(), b"new life");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_native_state_is_explicit() {
        let root = root("early-error");
        let e = LocalEvidence::begin(&root, "id").unwrap();
        assert!(!e.retain_error_checkpoint().unwrap());
        assert!(e.save("../escape", b"bad").is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

/// Opt-in lossless CPU evidence format. This is not a model checkpoint schema.
#[cfg(feature = "cpu-backend")]
#[allow(dead_code)] // Explicit APIs need not be used by every including example.
pub mod compressed_checkpoint {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Read;

    const MAGIC: &[u8; 8] = b"ALCPZ001";
    const FOOTER: &[u8; 8] = b"ENDCPZ01";
    const HEADER: usize = 88;
    /// Hard limit per uncompressed checkpoint; callers can choose less.
    pub const MAX_JSON_BYTES: usize = 256 * 1024 * 1024;
    const MAX_PAYLOAD_BYTES: usize = MAX_JSON_BYTES + 1024 * 1024;

    fn invalid() -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid compressed checkpoint evidence",
        )
    }
    fn digest(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }
    fn encode(bytes: &[u8], limit: usize) -> io::Result<Vec<u8>> {
        if bytes.len() > limit.min(MAX_JSON_BYTES) {
            return Err(invalid());
        }
        let payload = zstd::bulk::compress(bytes, 3)?;
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(invalid());
        }
        let mut encoded = Vec::with_capacity(HEADER + payload.len() + FOOTER.len());
        encoded.extend_from_slice(MAGIC);
        encoded.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        encoded.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        encoded.extend_from_slice(&digest(bytes));
        encoded.extend_from_slice(&digest(&payload));
        encoded.extend_from_slice(&payload);
        encoded.extend_from_slice(FOOTER);
        Ok(encoded)
    }
    /// Checks both hashes, exact lengths and one standard zstd frame before decoding.
    /// Returns the original bytes, never parsed/reformatted JSON. The byte limit is
    /// not a model validator; the caller must still validate the native checkpoint.
    pub fn read(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
        let mut file = fs::File::open(path)?;
        let mut header = [0_u8; HEADER];
        file.read_exact(&mut header)?;
        if &header[..8] != MAGIC {
            return Err(invalid());
        }
        let original_len = u64::from_le_bytes(header[8..16].try_into().unwrap());
        let payload_len = u64::from_le_bytes(header[16..24].try_into().unwrap());
        if original_len > limit.min(MAX_JSON_BYTES) as u64
            || payload_len > MAX_PAYLOAD_BYTES as u64
            || file.metadata()?.len() != HEADER as u64 + payload_len + FOOTER.len() as u64
        {
            return Err(invalid());
        }
        // This format is emitted by bulk::compress; do not allocate a large
        // payload for a tiny declared original, even within the global cap.
        if payload_len as usize > zstd::zstd_safe::compress_bound(original_len as usize) {
            return Err(invalid());
        }
        let mut payload = vec![0; payload_len as usize];
        file.read_exact(&mut payload)?;
        let mut footer = [0; 8];
        file.read_exact(&mut footer)?;
        let mut extra = [0];
        if footer != *FOOTER
            || file.read(&mut extra)? != 0
            || digest(&payload) != header[56..88]
            || !payload.starts_with(&[0x28, 0xb5, 0x2f, 0xfd])
            || zstd::zstd_safe::find_frame_compressed_size(&payload).map_err(|_| invalid())?
                != payload.len()
        {
            return Err(invalid());
        }
        let mut decoder = zstd::bulk::Decompressor::new()?;
        // Bounds decoder history independently of the declared output length.
        decoder.set_parameter(zstd::zstd_safe::DParameter::WindowLogMax(28))?;
        let bytes = decoder.decompress(&payload, original_len as usize)?;
        if bytes.len() != original_len as usize || digest(&bytes) != header[24..56] {
            return Err(invalid());
        }
        Ok(bytes)
    }
    impl LocalEvidence {
        /// Explicit opt-in. Final name must end in `.cp.zst`; default `save` is unchanged.
        /// Writes/syncs a create-only `.pending` file, verifies it, then atomically
        /// hard-links the final name without replacement. Pending evidence is retained
        /// on failure AND success (same inode, not a second payload allocation).
        /// Unsupported hard links fail closed. A post-link directory-sync error has
        /// an uncertain durability outcome: inspect, never retry/overwrite the name.
        pub fn save_compressed_checkpoint(
            &self,
            name: &str,
            bytes: &[u8],
            limit: usize,
        ) -> io::Result<()> {
            if !name.ends_with(".cp.zst") || name.len() > 160 {
                return Err(invalid());
            }
            let pending_name = format!("{name}.pending");
            let encoded = encode(bytes, limit)?;
            self.save(&pending_name, &encoded)?;
            let pending = self.directory.join(pending_name);
            if read(&pending, limit)? != bytes {
                return Err(invalid());
            }
            fs::hard_link(pending, self.directory.join(name))?;
            #[cfg(unix)]
            fs::File::open(&self.directory)?.sync_all()?;
            Ok(())
        }
        /// Separate API; the legacy JSON error-retention path remains byte-identical.
        pub fn retain_error_checkpoint_compressed(&self, limit: usize) -> io::Result<bool> {
            if let Some(bytes) = &self.last_valid {
                self.save_compressed_checkpoint("ERROR-LAST-VALID.cp.zst", bytes, limit)?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        fn root() -> PathBuf {
            let path = std::env::temp_dir().join(format!(
                "cpz-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            path
        }
        #[test]
        fn exact_empty_binary_and_json_roundtrips_and_create_only() {
            let root = root();
            let e = LocalEvidence::begin(&root, "run").unwrap();
            for (i, bytes) in [b"".as_slice(), b"{\n \"x\": -0.0 }\n", &[0, 255, 0, 1]]
                .into_iter()
                .enumerate()
            {
                let name = format!("{i}.cp.zst");
                e.save_compressed_checkpoint(&name, bytes, 1024).unwrap();
                assert_eq!(read(&e.directory().join(&name), 1024).unwrap(), bytes);
                assert!(e
                    .save_compressed_checkpoint(&name, b"replacement", 1024)
                    .is_err());
                assert_eq!(read(&e.directory().join(name), 1024).unwrap(), bytes);
            }
            fs::remove_dir_all(root).unwrap();
        }
        #[test]
        fn truncation_trailing_hash_lengths_and_extra_frames_reject() {
            let root = root();
            let path = root.join("test");
            let good = encode(b"example original bytes", 1024).unwrap();
            for end in 0..good.len() {
                fs::write(&path, &good[..end]).unwrap();
                assert!(read(&path, 1024).is_err());
            }
            for offset in [0, 8, 16, 24, 56, 88, good.len() - 1] {
                let mut bad = good.clone();
                bad[offset] ^= 1;
                fs::write(&path, bad).unwrap();
                assert!(read(&path, 1024).is_err());
            }
            let mut extra = good.clone();
            extra.push(0);
            fs::write(&path, extra).unwrap();
            assert!(read(&path, 1024).is_err());
            let mut second = good[..good.len() - 8].to_vec();
            second.extend_from_slice(&zstd::bulk::compress(b"second", 3).unwrap());
            let len = second.len() - HEADER;
            second[16..24].copy_from_slice(&(len as u64).to_le_bytes());
            let hash = digest(&second[HEADER..]);
            second[56..88].copy_from_slice(&hash);
            second.extend_from_slice(FOOTER);
            fs::write(&path, second).unwrap();
            assert!(read(&path, 1024).is_err());
            fs::write(&path, good).unwrap();
            assert!(read(&path, 4).is_err());
            fs::remove_dir_all(root).unwrap();
        }
        #[test]
        fn failed_claim_retains_pending_and_does_not_replace_final() {
            let root = root();
            let e = LocalEvidence::begin(&root, "run").unwrap();
            e.save("a.cp.zst", b"existing").unwrap();
            assert!(e
                .save_compressed_checkpoint("a.cp.zst", b"new", 1024)
                .is_err());
            assert_eq!(
                fs::read(e.directory().join("a.cp.zst")).unwrap(),
                b"existing"
            );
            assert_eq!(
                read(&e.directory().join("a.cp.zst.pending"), 1024).unwrap(),
                b"new"
            );
            e.save("b.cp.zst.pending", b"partial").unwrap();
            assert!(e
                .save_compressed_checkpoint("b.cp.zst", b"new", 1024)
                .is_err());
            assert!(!e.directory().join("b.cp.zst").exists());
            assert!(e
                .save_compressed_checkpoint("../escape.cp.zst", b"x", 1024)
                .is_err());
            assert!(e
                .save_compressed_checkpoint("limit.cp.zst", b"large", 1)
                .is_err());
            assert!(!e.directory().join("limit.cp.zst").exists());
            fs::remove_dir_all(root).unwrap();
        }
        #[test]
        fn compressed_error_retention_is_explicit() {
            let root = root();
            let mut e = LocalEvidence::begin(&root, "run").unwrap();
            assert!(!e.retain_error_checkpoint_compressed(1024).unwrap());
            e.remember_valid(b"exact old bytes".to_vec());
            e.retain_error_checkpoint_compressed(1024).unwrap();
            assert_eq!(
                read(&e.directory().join("ERROR-LAST-VALID.cp.zst"), 1024).unwrap(),
                b"exact old bytes"
            );
            assert!(!e.directory().join("ERROR-LAST-VALID.cp.json").exists());
            assert!(e.retain_error_checkpoint_compressed(1024).is_err());
            fs::remove_dir_all(root).unwrap();
        }
        #[test]
        fn forged_lengths_and_rehashed_payload_are_still_bounded() {
            let root = root();
            let path = root.join("test");
            let good = encode(&[42; 4096], 4096).unwrap();
            for length in [1_u64, MAX_JSON_BYTES as u64 + 1, u64::MAX] {
                let mut bad = good.clone();
                bad[8..16].copy_from_slice(&length.to_le_bytes());
                fs::write(&path, bad).unwrap();
                assert!(read(&path, usize::MAX).is_err());
            }
            let mut bad = good.clone();
            bad[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
            fs::write(&path, bad).unwrap();
            assert!(read(&path, usize::MAX).is_err());
            // A valid replacement frame with a matching payload hash is not enough:
            // the original uncompressed digest must match too.
            let payload = zstd::bulk::compress(&[43; 4096], 3).unwrap();
            let mut bad = good[..HEADER].to_vec();
            bad[16..24].copy_from_slice(&(payload.len() as u64).to_le_bytes());
            bad[56..88].copy_from_slice(&digest(&payload));
            bad.extend(payload);
            bad.extend_from_slice(FOOTER);
            fs::write(&path, bad).unwrap();
            assert!(read(&path, 4096).is_err());
            fs::remove_dir_all(root).unwrap();
        }
    }
}
