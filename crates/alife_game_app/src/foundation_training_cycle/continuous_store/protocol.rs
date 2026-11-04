//! Strict unpublished-prefix recognition. A complete malformed record is never
//! downgraded to a torn stage, and a first stage owns no run directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeaderOwner {
    Continuous,
    Legacy,
    Shared,
    Unknown,
}

pub(super) fn header_owner(bytes: &[u8], continuous: &[u8; 16], legacy: &[u8; 16]) -> HeaderOwner {
    let new = bytes.starts_with(continuous) || continuous.starts_with(bytes);
    let old = bytes.starts_with(legacy) || legacy.starts_with(bytes);
    match (new, old) {
        (true, false) => HeaderOwner::Continuous,
        (false, true) => HeaderOwner::Legacy,
        (true, true) => HeaderOwner::Shared,
        (false, false) => HeaderOwner::Unknown,
    }
}

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

pub(super) fn effective_sampling_temperature(
    request: f32,
    saved: Option<f32>,
) -> Result<f32, &'static str> {
    let actual = saved.unwrap_or(request);
    if !actual.is_finite() || !(0.0001..=128.).contains(&actual) {
        return Err("retained sampling temperature exceeds continuous admission bounds");
    }
    Ok(actual)
}

/// The producer's existing snapshot admission predicate, with an explicit clock
/// for deterministic deadline tests. A completed capture has no deadline grace.
pub(super) fn snapshot_boundary(
    consumed: bool,
    request: &super::ResidentContinuousRequest,
    progress: &alife_training::SameIndividualProgressV1,
    actual_temperature: f32,
    now: u64,
) -> Result<(), String> {
    if !consumed {
        return Err("session was not durably dispatched".into());
    }
    if progress.session_nonce != request.seed_range[0] {
        return Err(format!(
            "session nonce {} differs from request {}",
            progress.session_nonce, request.seed_range[0]
        ));
    }
    if progress.trained_rows > request.total_trained_decisions as u64 {
        return Err(format!(
            "trained rows {} exceed request {}",
            progress.trained_rows, request.total_trained_decisions
        ));
    }
    if progress.sampling_temperature.to_bits() != actual_temperature.to_bits() {
        return Err(format!(
            "sampling temperature bits {} differ from retained {}",
            progress.sampling_temperature.to_bits(),
            actual_temperature.to_bits()
        ));
    }
    if now > request.finalization_deadline_unix_seconds {
        return Err(format!(
            "snapshot admission time {now} exceeds finalization deadline {} by {} seconds",
            request.finalization_deadline_unix_seconds,
            now - request.finalization_deadline_unix_seconds
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot_fixture() -> (
        super::super::ResidentContinuousRequest,
        alife_training::SameIndividualProgressV1,
    ) {
        (
            super::super::ResidentContinuousRequest {
                schema: 1,
                seed_range: [8002, 8010],
                total_trained_decisions: 4,
                decisions_per_update: 4,
                snapshot_every_updates: 1,
                care_assistance: false,
                sampling_temperature: 0.9,
                collection_cutoff_unix_seconds: 240,
                finalization_deadline_unix_seconds: 300,
                max_temporary_bytes: 2 * 1024 * 1024 * 1024,
            },
            alife_training::SameIndividualProgressV1 {
                session_nonce: 8002,
                organism_id: 2,
                session_start_world_tick: 0,
                world_tick: 5,
                trained_rows: 4,
                completed_updates: 1,
                sampling_temperature: 0.9,
            },
        )
    }
    #[test]
    fn snapshot_boundary_rejects_capture_crossing_deadline_without_grace() {
        let (request, progress) = snapshot_fixture();
        // Capturing starts before the deadline and returns after it. The
        // callback's time remains authoritative, not its pre-capture check.
        assert!(snapshot_boundary(true, &request, &progress, 0.9, 299).is_ok());
        assert!(snapshot_boundary(true, &request, &progress, 0.9, 300).is_ok());
        assert_eq!(
            snapshot_boundary(true, &request, &progress, 0.9, 301).unwrap_err(),
            "snapshot admission time 301 exceeds finalization deadline 300 by 1 seconds"
        );
    }
    #[test]
    fn snapshot_boundary_preserves_every_nonclock_rejection() {
        let (request, progress) = snapshot_fixture();
        assert!(snapshot_boundary(false, &request, &progress, 0.9, 299)
            .unwrap_err()
            .contains("not durably dispatched"));
        let mut changed = progress.clone();
        changed.session_nonce += 1;
        assert!(snapshot_boundary(true, &request, &changed, 0.9, 299)
            .unwrap_err()
            .contains("session nonce"));
        let mut changed = progress.clone();
        changed.trained_rows += 1;
        assert!(snapshot_boundary(true, &request, &changed, 0.9, 299)
            .unwrap_err()
            .contains("trained rows"));
        assert!(snapshot_boundary(true, &request, &progress, 1.0, 299)
            .unwrap_err()
            .contains("temperature bits"));
    }
    #[test]
    fn shared_torn_magic_never_identifies_one_store_kind() {
        let new = b"ALIFE-CON-RET-1\n";
        let old = b"ALIFE-RES-RET-1\n";
        for length in 0..=6 {
            assert_eq!(header_owner(&new[..length], new, old), HeaderOwner::Shared);
        }
        for length in 7..=16 {
            assert_eq!(
                header_owner(&new[..length], new, old),
                HeaderOwner::Continuous
            );
            assert_eq!(header_owner(&old[..length], new, old), HeaderOwner::Legacy);
        }
        assert_eq!(header_owner(b"foreign", new, old), HeaderOwner::Unknown);
    }
    #[test]
    fn saved_sampler_temperature_overrides_new_request_without_reset() {
        assert_eq!(
            effective_sampling_temperature(1., Some(0.75))
                .unwrap()
                .to_bits(),
            0.75f32.to_bits()
        );
        assert_eq!(
            effective_sampling_temperature(1., None).unwrap().to_bits(),
            1f32.to_bits()
        );
        assert!(effective_sampling_temperature(1., Some(f32::NAN)).is_err());
        assert!(effective_sampling_temperature(1., Some(0.)).is_err());
    }
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
