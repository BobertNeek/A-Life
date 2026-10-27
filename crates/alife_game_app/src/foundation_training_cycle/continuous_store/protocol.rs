//! Strict unpublished-prefix recognition. A complete malformed record is never
//! downgraded to a torn stage, and a first stage owns no run directory.
pub(super) fn incomplete_stage(bytes: &[u8], magic: &[u8; 16], limit: u64) -> bool {
    if bytes.len() < 16 {
        return magic.starts_with(bytes);
    }
    if &bytes[..16] != magic {
        return false;
    }
    if bytes.len() < 24 {
        return true;
    }
    let length = u64::from_le_bytes(bytes[16..24].try_into().expect("eight bytes"));
    length > 0
        && length <= limit
        && length
            .checked_add(56)
            .is_some_and(|total| (bytes.len() as u64) < total)
}

#[cfg(test)]
mod tests {
    use super::*;
    const MAGIC: &[u8; 16] = b"ALIFE-CON-INI-1\n";
    #[test]
    fn every_actual_torn_prefix_is_recognized_but_complete_record_is_not() {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&4u64.to_le_bytes());
        bytes.extend([0u8; 36]);
        for n in 0..bytes.len() {
            assert!(incomplete_stage(&bytes[..n], MAGIC, 100));
        }
        assert!(!incomplete_stage(&bytes, MAGIC, 100));
    }
    #[test]
    fn malformed_and_cross_kind_stages_never_authorize_cleanup() {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&u64::MAX.to_le_bytes());
        assert!(!incomplete_stage(&bytes, MAGIC, 100));
        bytes[16..24].copy_from_slice(&0u64.to_le_bytes());
        assert!(!incomplete_stage(&bytes, MAGIC, 100));
        assert!(!incomplete_stage(b"ALIFE-RES-INI-1\n", MAGIC, 100));
    }
}
