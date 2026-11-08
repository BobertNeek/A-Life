//! Inspect the future basis contract. No brain admission or runtime activation.
#[path = "support/broad_drive_basis.rs"]
mod basis;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(&basis::contract())?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::*;
    use alife_world::*;
    use std::collections::BTreeSet;
    fn frame() -> PerceptionFrameDraft {
        let mut world = HeadlessScenarioBuilder::new(61_011)
            .agent("observer", OrganismId(1), Vec3f::ZERO)
            .agent("peer", OrganismId(2), Vec3f::new(0.4, 0., 0.))
            .food("object", Vec3f::new(2., 0., 0.), 0.6)
            .build()
            .unwrap();
        world
            .perception_frame_draft(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::GroundedObjectSlotsV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap()
    }
    #[test]
    fn legacy_ports_including_memory_cognitive_and_grip_are_bit_preserved() {
        let f = frame();
        for (i, c) in f.candidates().iter().enumerate() {
            let mut old = [0.; 54];
            old[..24].copy_from_slice(&c.features.0);
            for (j, v) in old.iter_mut().enumerate().skip(24) {
                *v = (j as f32 - 40.) / 64.;
            }
            let new = basis::project(&f, i, &old).unwrap();
            for j in 0..54 {
                assert_eq!(old[j].to_bits(), new[j].to_bits());
            }
            assert_eq!(&new[62..], &[0., 0.]);
        }
    }
    #[test]
    fn peer_and_intrinsic_vocal_use_only_same_frame_observations() {
        let f = frame();
        let mut peers = 0;
        let mut ordinary = 0;
        let mut vocals = 0;
        for (i, c) in f.candidates().iter().enumerate() {
            let mut old = [0.; 54];
            old[..24].copy_from_slice(&c.features.0);
            let new = basis::project(&f, i, &old).unwrap();
            let observed_peer = f
                .sensory()
                .social_context
                .nearest_agents
                .iter()
                .flatten()
                .any(|p| p.body_entity.is_some() && p.body_entity == c.target.entity);
            if matches!(c.observation, CandidateObservationRef::ObjectSlot(_)) {
                assert_eq!(new[54], 1.);
                assert_eq!(new[59], f32::from(observed_peer));
                if observed_peer {
                    peers += 1;
                } else {
                    ordinary += 1;
                }
                assert_eq!(new[60], 0.);
            } else {
                assert_eq!(new[59], 0.);
            }
            if c.kind == ActionKind::Vocalize {
                assert_eq!(new[60], 1.);
                vocals += 1;
            }
        }
        assert!(peers > 0 && ordinary > 0 && vocals > 0);
    }
    #[test]
    fn temperature_is_symmetric_and_unknown_expectancies_are_unassigned() {
        for t in [0., 0.1, 0.25, 0.3, 0.8, 1.] {
            let hot = basis::thermal_pair(t).unwrap();
            let cold = basis::thermal_pair(-t).unwrap();
            assert_eq!(hot, [cold[1], cold[0]]);
            if t <= 0.25 {
                assert_eq!(hot, [0., 0.]);
            }
        }
        assert!(basis::thermal_pair(f32::NAN).is_err());
        assert!(basis::thermal_pair(1.1).is_err());
    }
    #[test]
    fn future_head_and_rows_do_not_alias_an_admitted_legacy_decoder() {
        assert!(DecoderHeadKind::try_from_raw(basis::NEW_HEAD_RAW).is_err());
        let c = basis::contract();
        let rows = c["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1280);
        let mut seen = BTreeSet::new();
        for row in rows {
            let tuple = (
                row["family"].as_u64().unwrap(),
                row["input_lane"].as_u64().unwrap(),
                row["motor_index"].as_u64().unwrap(),
            );
            assert!((54..64).contains(&tuple.1));
            assert!(seen.insert(tuple));
        }
        assert_eq!(c["admitted_by_current_runtime"], false);
    }
}
