use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, Cursor, Read, Write},
    path::Path,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RAW_CAP1941: u64 = 64 * 1024 * 1024;
const FRAME_CAP1941: u64 = 65 * 1024 * 1024;
const MAGIC: u32 = 0x184D2A50;
const TAG: &[u8] = b"ALIFE-1941-EXACT-EVIDENCE-ZSTD1\n";
fn sha(raw: &[u8]) -> [u8; 32] {
    Sha256::digest(raw).into()
}
struct BoundedFrame(Vec<u8>);
impl Write for BoundedFrame {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(b.len())
            .is_none_or(|n| n as u64 > FRAME_CAP1941)
        {
            return Err(std::io::Error::other("Zstd frame cap"));
        }
        self.0.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn encode_exact_frame1941(raw: &[u8]) -> Result<Vec<u8>> {
    if raw.len() as u64 > RAW_CAP1941 {
        return Err("raw evidence cap before compression".into());
    }
    let mut out = BoundedFrame(Vec::new());
    out.write_all(&MAGIC.to_le_bytes())?;
    out.write_all(&((TAG.len() + 8 + 32) as u32).to_le_bytes())?;
    out.write_all(TAG)?;
    out.write_all(&(raw.len() as u64).to_le_bytes())?;
    out.write_all(&sha(raw))?;
    let mut encoder = zstd::stream::write::Encoder::new(out, 1)?;
    encoder.include_checksum(true)?;
    encoder.set_pledged_src_size(Some(raw.len() as u64))?;
    encoder.write_all(raw)?;
    Ok(encoder.finish()?.0)
}
/// Exactly one bounded frame. Reject concatenation, truncation, over-expansion,
/// invalid checksum, raw-length/SHA mismatch, and invalid metadata before JSON parse.
pub fn decode_exact_frame1941(frame: &[u8]) -> Result<Vec<u8>> {
    if frame.len() as u64 > FRAME_CAP1941 {
        return Err("compressed evidence cap".into());
    }
    let header = 8 + TAG.len() + 8 + 32;
    if frame.len() < header
        || u32::from_le_bytes(frame[..4].try_into()?) != MAGIC
        || u32::from_le_bytes(frame[4..8].try_into()?) as usize != TAG.len() + 8 + 32
        || &frame[8..8 + TAG.len()] != TAG
    {
        return Err("invalid exact evidence header".into());
    }
    let offset = 8 + TAG.len();
    let raw_len = u64::from_le_bytes(frame[offset..offset + 8].try_into()?);
    if raw_len > RAW_CAP1941 {
        return Err("declared raw evidence exceeds cap".into());
    }
    let expected = &frame[offset + 8..header];
    let mut decoder =
        zstd::stream::read::Decoder::with_buffer(Cursor::new(&frame[header..]))?.single_frame();
    decoder.window_log_max(raw_len.max(1024).next_power_of_two().trailing_zeros())?;
    // Never grow a full raw buffer to raw_len+1 (which could double allocation).
    let mut raw = Vec::with_capacity(raw_len as usize);
    Read::by_ref(&mut decoder)
        .take(raw_len)
        .read_to_end(&mut raw)?;
    let mut extra = [0u8; 1];
    if decoder.read(&mut extra)? != 0 {
        return Err("exact raw evidence over-expansion".into());
    }
    if raw.len() as u64 != raw_len || &sha(&raw)[..] != expected {
        return Err("exact raw evidence length/SHA mismatch".into());
    }
    if !decoder.finish().fill_buf()?.is_empty() {
        return Err("trailing or concatenated evidence frame".into());
    }
    Ok(raw)
}
pub fn save(p: &Path, raw: &[u8]) -> Result<()> {
    let frame = encode_exact_frame1941(raw)?;
    if decode_exact_frame1941(&frame)? != raw {
        return Err("codec roundtrip".into());
    }
    let path = p.with_extension("json.zst");
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    f.write_all(&frame)?;
    f.sync_all()?;
    if decode_exact_frame1941(&fs::read(&path)?)? != raw {
        return Err("disk roundtrip".into());
    }
    let marker = p.with_extension("commit.json");
    let mut m = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker)?;
    m.write_all(&serde_json::to_vec(&serde_json::json!({"raw_bytes":raw.len(),"raw_sha256":sha(raw),"frame_bytes":frame.len(),"frame_sha256":sha(&frame)}))?)?;
    m.sync_all()?;
    fs::File::open(p.parent().unwrap())?.sync_all()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_raw_floats_and_integers() {
        let r = br#"{\"v\":-0.0,\"age\":18446744073709551615,\"x\":0.123456789012345}"#;
        let f = encode_exact_frame1941(r).unwrap();
        assert_eq!(decode_exact_frame1941(&f).unwrap(), r);
    }
    #[test]
    fn rejects_truncation_and_extra() {
        let f = encode_exact_frame1941(b"raw evidence").unwrap();
        assert!(decode_exact_frame1941(&f[..f.len() - 1]).is_err());
        let mut bad = f.clone();
        bad.push(0);
        assert!(decode_exact_frame1941(&bad).is_err());
        let mut bad = f.clone();
        bad[50] ^= 1;
        assert!(decode_exact_frame1941(&bad).is_err());
    }
}
