//! Unscored logical candidate identity for the GPU exploration counter.
//! This encodes identity only; it never evaluates neural activity or selects actions.

use alife_core::{ActionCandidate, CanonicalDigestBuilder, ScaffoldContractError};

pub(crate) const GAME_EXPLORATION_IDENTITY_WORDS: usize = 4;

pub(crate) fn game_candidate_exploration_identity(
    candidate: &ActionCandidate,
) -> Result<[u32; GAME_EXPLORATION_IDENTITY_WORDS], ScaffoldContractError> {
    candidate.validate()?;
    let mut digest = CanonicalDigestBuilder::new(b"alife.game.exploration-candidate.v1");
    digest.write_u32(candidate.action_id.raw());
    digest.write_u8(candidate.kind.raw());
    digest.write_u8(candidate.family.raw());
    match candidate.target.entity {
        Some(entity) => {
            digest.write_some();
            digest.write_u64(entity.raw());
        }
        None => digest.write_none(),
    }
    match candidate.target.position {
        Some(position) => {
            digest.write_some();
            digest.write_f32(position.x)?;
            digest.write_f32(position.y)?;
            digest.write_f32(position.z)?;
        }
        None => digest.write_none(),
    }
    digest.write_u32(candidate.min_duration.raw());
    digest.write_u32(candidate.max_duration.raw());
    let [lo, hi] = digest.finish128();
    Ok([lo as u32, (lo >> 32) as u32, hi as u32, (hi >> 32) as u32])
}
