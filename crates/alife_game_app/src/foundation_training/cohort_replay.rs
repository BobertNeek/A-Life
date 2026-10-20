//! One durable replay cohort. Pending collection is never a usable handoff.
use super::*;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

const PENDING: &str = "cohort-replay.pending";
const SEALED: &str = "cohort-replay.bundle";
const MAGIC: &[u8; 8] = b"ALCOH001";
const FOOTER_BYTES: u64 = 48;
const MAX_INDEX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORDS: usize = 50 * 2049;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FoundationCohortReplayLocation {
    pub offset: u64,
    pub organism_id: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Index {
    schema: u16,
    source: FoundationReplaySource,
    records: Vec<FoundationReplayRecordRef>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FoundationCohortReplayCommit {
    pub records: usize,
    pub encoded_record_bytes: u64,
    pub bundle_bytes: u64,
    pub index_blake3: String,
    pub data_sync_calls: u64,
    pub durable_cohort_commits: u64,
}

pub(crate) struct CohortReplayWriter {
    root: PathBuf,
    file: std::io::BufWriter<File>,
    index: Index,
    bytes: u64,
    poisoned: bool,
}

impl CohortReplayWriter {
    pub(crate) fn new(root: &Path, source: FoundationReplaySource) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        sync_directory_ancestry(root)?;
        if root.join(SEALED).exists() {
            return Err("replay cohort is already sealed".into());
        }
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(root.join(PENDING))?;
        Ok(Self {
            root: root.to_owned(),
            file: std::io::BufWriter::with_capacity(256 * 1024, file),
            index: Index {
                schema: 1,
                source,
                records: Vec::new(),
            },
            bytes: 0,
            poisoned: false,
        })
    }

    pub(crate) fn append(
        &mut self,
        record: &FoundationReplayRecord,
    ) -> Result<FoundationReplayRecordRef> {
        if record.source != self.index.source {
            return Err("cohort replay source changed".into());
        }
        let template = FoundationReplayRecordRef {
            record_index: record.record_index,
            segment: record.segment,
            tick: record.tick,
            encoded_bytes: 0,
            digest: [0; 32],
            cohort: Some(FoundationCohortReplayLocation {
                offset: self.bytes,
                organism_id: record.organism_id,
            }),
        };
        self.append_encoding(template, |out| encode_replay_record(out, record))
    }

    fn append_encoding(
        &mut self,
        mut reference: FoundationReplayRecordRef,
        encode: impl FnOnce(&mut std::io::BufWriter<File>) -> Result<(u64, [u8; 32])>,
    ) -> Result<FoundationReplayRecordRef> {
        if self.poisoned || self.index.records.len() >= MAX_RECORDS {
            return Err("invalid or exhausted pending replay cohort".into());
        }
        // Any failed/partial encoding poisons the entire uncommitted cohort.
        self.poisoned = true;
        let (bytes, digest) = encode(&mut self.file)?;
        if bytes == 0 || bytes > FOUNDATION_REPLAY_RECORD_LIMIT_BYTES {
            return Err("invalid encoded cohort row size".into());
        }
        reference.encoded_bytes = bytes;
        reference.digest = digest;
        self.bytes = self.bytes.checked_add(bytes).ok_or_else(invalid)?;
        self.index.records.push(reference.clone());
        self.poisoned = false;
        Ok(reference)
    }

    pub(crate) fn poison(&mut self) {
        self.poisoned = true;
    }

    pub(crate) fn seal(self) -> Result<FoundationCohortReplayCommit> {
        self.seal_with_publish(publish_new)
    }

    fn seal_with_publish(
        mut self,
        publish: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    ) -> Result<FoundationCohortReplayCommit> {
        if self.poisoned {
            return Err("cannot seal a torn replay cohort".into());
        }
        validate_index(&self.index, self.bytes)?;
        let index = serde_json::to_vec(&self.index)?;
        if index.len() as u64 > MAX_INDEX_BYTES {
            return Err("replay cohort index exceeds limit".into());
        }
        let digest = blake3::hash(&index);
        self.file.write_all(&index)?;
        self.file.write_all(MAGIC)?;
        self.file.write_all(&(index.len() as u64).to_le_bytes())?;
        self.file.write_all(digest.as_bytes())?;
        self.file.flush()?;
        // The sole data synchronization covers every record and its index.
        self.file.get_ref().sync_all()?;
        let bundle_bytes = self.file.get_ref().metadata()?.len();
        drop(self.file);
        publish(&self.root.join(PENDING), &self.root.join(SEALED))?;
        Ok(FoundationCohortReplayCommit {
            records: self.index.records.len(),
            encoded_record_bytes: self.bytes,
            bundle_bytes,
            index_blake3: digest.to_hex().to_string(),
            data_sync_calls: 1,
            durable_cohort_commits: 1,
        })
    }
}

fn validate_index(index: &Index, data_bytes: u64) -> Result<()> {
    if index.schema != 1 || index.records.is_empty() || index.records.len() > MAX_RECORDS {
        return Err("invalid replay cohort index inventory".into());
    }
    let mut offset = 0u64;
    let mut lives = BTreeMap::<u64, (u64, u64, u64)>::new();
    for record in &index.records {
        let location = record.cohort.as_ref().ok_or("cohort row lacks location")?;
        if location.offset != offset
            || location.organism_id == 0
            || record.encoded_bytes == 0
            || record.encoded_bytes > FOUNDATION_REPLAY_RECORD_LIMIT_BYTES
        {
            return Err("cohort row overlaps, escapes or leaves an unauthenticated gap".into());
        }
        if let Some((last_index, last_tick, last_segment)) = lives.get(&location.organism_id) {
            if record.record_index != last_index.checked_add(1).ok_or_else(invalid)?
                || record.tick <= *last_tick
                || record.segment < *last_segment
                || (record.segment == *last_segment && record.tick != last_tick + 1)
            {
                return Err("cohort crosses private replay ordering".into());
            }
        } else if record.record_index != 0 || record.segment != 0 {
            return Err("cohort life lacks its initial replay boundary".into());
        }
        lives.insert(
            location.organism_id,
            (record.record_index, record.tick, record.segment),
        );
        offset = offset
            .checked_add(record.encoded_bytes)
            .ok_or_else(invalid)?;
    }
    if offset != data_bytes {
        return Err("cohort data and index sizes differ".into());
    }
    Ok(())
}

pub(crate) struct CohortReplayReader {
    file: File,
    records: BTreeMap<u64, FoundationReplayRecordRef>,
    bundle_bytes: u64,
}

impl CohortReplayReader {
    pub(crate) fn open(root: &Path, expected: &FoundationReplaySource) -> Result<Self> {
        // A pending file is deliberately never considered, even with a footer.
        let mut file = File::open(root.join(SEALED))?;
        let size = file.metadata()?.len();
        if size < FOOTER_BYTES {
            return Err("torn replay cohort footer".into());
        }
        file.seek(SeekFrom::End(-(FOOTER_BYTES as i64)))?;
        let mut footer = [0u8; FOOTER_BYTES as usize];
        file.read_exact(&mut footer)?;
        if &footer[..8] != MAGIC {
            return Err("replay cohort footer magic changed".into());
        }
        let index_len = u64::from_le_bytes(footer[8..16].try_into()?);
        if index_len == 0 || index_len > MAX_INDEX_BYTES || index_len > size - FOOTER_BYTES {
            return Err("invalid replay cohort index size".into());
        }
        let data_bytes = size - FOOTER_BYTES - index_len;
        file.seek(SeekFrom::Start(data_bytes))?;
        let mut bytes = vec![0; index_len as usize];
        file.read_exact(&mut bytes)?;
        if blake3::hash(&bytes).as_bytes() != &footer[16..48] {
            return Err("replay cohort index digest changed".into());
        }
        let index: Index = serde_json::from_slice(&bytes)?;
        if &index.source != expected {
            return Err("replay cohort belongs to another frozen source".into());
        }
        validate_index(&index, data_bytes)?;
        let records = index
            .records
            .into_iter()
            .map(|r| (r.cohort.as_ref().unwrap().offset, r))
            .collect();
        Ok(Self {
            file,
            records,
            bundle_bytes: size,
        })
    }

    pub(crate) fn read(&mut self, reference: &FoundationReplayRecordRef) -> Result<Vec<u8>> {
        let location = reference
            .cohort
            .as_ref()
            .ok_or("not a cohort replay reference")?;
        if self.records.get(&location.offset) != Some(reference)
            || self.file.metadata()?.len() != self.bundle_bytes
        {
            return Err("replay reference is not in the sealed cohort".into());
        }
        self.file.seek(SeekFrom::Start(location.offset))?;
        read_verified_replay_bytes(
            (&mut self.file).take(reference.encoded_bytes),
            reference.encoded_bytes,
            &reference.digest,
        )
    }
}

#[derive(Debug, serde::Serialize)]
pub enum FoundationCohortReplayRecovery {
    Uncommitted { pending_bytes: Option<u64> },
    Committed { records: usize, bundle_bytes: u64 },
}

/// Read-only evidence recovery, never training resumption or seed readmission.
/// A torn committed bundle is an error; an unsealed pending cohort is unusable.
pub fn inspect_foundation_cohort_replay(
    root: &Path,
    expected: &FoundationReplaySource,
) -> Result<FoundationCohortReplayRecovery> {
    if !root.join(SEALED).exists() {
        let pending_bytes = match std::fs::metadata(root.join(PENDING)) {
            Ok(meta) => Some(meta.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        return Ok(FoundationCohortReplayRecovery::Uncommitted { pending_bytes });
    }
    let mut reader = CohortReplayReader::open(root, expected)?;
    let records = reader.records.values().cloned().collect::<Vec<_>>();
    for reference in &records {
        reader.read(reference)?;
    }
    Ok(FoundationCohortReplayRecovery::Committed {
        records: records.len(),
        bundle_bytes: reader.bundle_bytes,
    })
}

#[cfg(unix)]
fn sync_directory_ancestry(root: &Path) -> std::io::Result<()> {
    // Output and replay may both be newly created. Persist their directory
    // entries through the existing ancestor chain before acknowledging a seal.
    for directory in std::fs::canonicalize(root)?.ancestors() {
        File::open(directory)?.sync_all()?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory_ancestry(_: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn publish_new(pending: &Path, sealed: &Path) -> std::io::Result<()> {
    // Exclusive publication cannot overwrite any accepted or foreign bundle.
    std::fs::hard_link(pending, sealed)?;
    std::fs::remove_file(pending)?;
    File::open(
        sealed
            .parent()
            .ok_or_else(|| std::io::Error::other("cohort parent missing"))?,
    )?
    .sync_all()
}

#[cfg(windows)]
fn publish_new(pending: &Path, sealed: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
    let pending = pending
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let sealed = sealed
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: same-directory paths own live NUL-terminated UTF-16 buffers. Omitting
    // REPLACE_EXISTING preserves exclusive ownership; WRITE_THROUGH seals rename.
    if unsafe { MoveFileExW(pending.as_ptr(), sealed.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn publish_new(_: &Path, _: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "durable cohort publication unsupported",
    ))
}

#[cfg(test)]
mod tests;
