//! Grounded neural views of currently observed terrain-vision object slots.
//!
//! This does not supply a target, hidden position, learned label or world ID.
//! Slot availability is supplied by the world's line-of-sight observation.
use crate::{GroundedObjectSlotV1, ScaffoldContractError, SensorProfile, Validate};

pub const NEURAL_VISIBLE_OBJECT_COUNT: usize = 4;
pub const NEURAL_VISIBLE_OBJECT_WIDTH: usize = 12;
pub const NEURAL_VISIBLE_OBJECT_LANES: usize =
    NEURAL_VISIBLE_OBJECT_COUNT * NEURAL_VISIBLE_OBJECT_WIDTH;

/// The four nearest observed slots, in deterministic distance/index order.
/// Each block is presence, positive/negative bearing sine, bearing cosine,
/// proximity, RGB, shape XYZ and confidence. No tracked or transport ID is
/// encoded. Empty blocks are exactly zero.
pub fn visible_object_neural_lanes(
    profile: SensorProfile,
    slots: &[GroundedObjectSlotV1],
) -> Result<[f32; NEURAL_VISIBLE_OBJECT_LANES], ScaffoldContractError> {
    if profile != SensorProfile::GroundedTerrainVisionV1
        && profile != SensorProfile::GroundedTerrainObjectPortsV2
    {
        return Err(ScaffoldContractError::SensorProfileMismatch);
    }
    let mut ordered = slots.iter().collect::<Vec<_>>();
    for slot in &ordered {
        slot.validate_contract()?;
    }
    ordered.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then(a.slot_index.cmp(&b.slot_index))
    });
    let mut lanes = [0.0; NEURAL_VISIBLE_OBJECT_LANES];
    for (index, slot) in ordered
        .into_iter()
        .take(NEURAL_VISIBLE_OBJECT_COUNT)
        .enumerate()
    {
        let start = index * NEURAL_VISIBLE_OBJECT_WIDTH;
        let confidence = slot.confidence.raw();
        lanes[start..start + NEURAL_VISIBLE_OBJECT_WIDTH].copy_from_slice(&[
            confidence,
            slot.bearing[0].max(0.0) * confidence,
            (-slot.bearing[0]).max(0.0) * confidence,
            slot.bearing[1] * confidence,
            (1.0 - slot.distance) * confidence,
            slot.color[0] * confidence,
            slot.color[1] * confidence,
            slot.color[2] * confidence,
            slot.shape[0] * confidence,
            slot.shape[1] * confidence,
            slot.shape[2] * confidence,
            confidence,
        ]);
    }
    Ok(lanes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Confidence, TrackedObjectId};
    fn slot(index: u16, distance: f32, bearing: f32, id: u64) -> GroundedObjectSlotV1 {
        GroundedObjectSlotV1 {
            slot_index: index,
            tracked_object_id: TrackedObjectId(id),
            bearing: [bearing, 0.5],
            distance,
            relative_velocity: [0.; 3],
            color: [0.1, 0.2, 0.3],
            material: [0.; 3],
            shape: [0.4, 0.5, 0.6],
            chemical: [0.; 3],
            contact: 0.,
            proprioception: [0.; 2],
            temperature: 0.,
            terrain: [0.; 2],
            confidence: Confidence::new(0.8).unwrap(),
        }
    }
    #[test]
    fn empty_slots_encode_zero_and_non_terrain_profile_is_rejected() {
        assert_eq!(
            visible_object_neural_lanes(SensorProfile::GroundedTerrainVisionV1, &[]).unwrap(),
            [0.; NEURAL_VISIBLE_OBJECT_LANES]
        );
        assert!(visible_object_neural_lanes(SensorProfile::GroundedObjectSlotsV1, &[]).is_err());
    }
    #[test]
    fn no_vision_gene_means_no_object_ports() {
        use crate::phenotype::SensorEncoderSourceGroup;
        use crate::{
            BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
            SensorChannelKind, Tick,
        };
        let capacity = BrainCapacityClass::n2048();
        let mut genome = BrainGenome::scaffold(61, capacity.id());
        genome
            .sensor_layout
            .channels
            .retain(|gene| gene.kind != SensorChannelKind::Vision);
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedTerrainObjectPortsV2,
        )
        .unwrap();
        assert!(!phenotype
            .sensor_encoder()
            .assignments()
            .iter()
            .any(|a| a.source_group() == SensorEncoderSourceGroup::VisibleObject));
    }
    #[test]
    fn zero_visual_confidence_never_leaks_object_features() {
        let mut unseen = slot(0, 0.2, 0.6, 5);
        unseen.confidence = Confidence::new(0.0).unwrap();
        assert_eq!(
            visible_object_neural_lanes(SensorProfile::GroundedTerrainObjectPortsV2, &[unseen])
                .unwrap(),
            [0.0; NEURAL_VISIBLE_OBJECT_LANES]
        );
    }
    #[test]
    fn nearest_slots_are_deterministic_bounded_and_never_encode_identity() {
        let first = slot(2, 0.2, -0.6, 5);
        let second = slot(1, 0.8, 0.4, 6);
        let a =
            visible_object_neural_lanes(SensorProfile::GroundedTerrainVisionV1, &[second, first])
                .unwrap();
        let mut first_other_id = first;
        first_other_id.tracked_object_id = TrackedObjectId(999);
        let b = visible_object_neural_lanes(
            SensorProfile::GroundedTerrainVisionV1,
            &[first_other_id, second],
        )
        .unwrap();
        assert_eq!(a, b);
        assert_eq!(a[0], 0.8);
        assert_eq!(a[1], 0.);
        assert!((a[2] - 0.48).abs() < 1e-6);
        assert!((a[4] - 0.64).abs() < 1e-6);
        assert_eq!(a[NEURAL_VISIBLE_OBJECT_WIDTH], 0.8);
        assert!(a[2 * NEURAL_VISIBLE_OBJECT_WIDTH..]
            .iter()
            .all(|x| *x == 0.));
    }
}
