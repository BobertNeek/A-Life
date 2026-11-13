//! World-issued observation availability, separate from measured feature values.
//! No public constructor/deserialize path: issued only with an actual World frame.
use super::*;
use alife_core::{PerceptionFrameDigest, TrackedObjectId};

#[derive(Clone, Debug, PartialEq)]
pub struct GroundedSourceAvailabilityV1 {
    source: TrackedObjectId,
    known_groups: u8,
    gain_bits: [u32; 3],
}
impl GroundedSourceAvailabilityV1 {
    pub fn source(&self) -> TrackedObjectId {
        self.source
    }
    /// RGB/material/shape/chemical/temperature bits. Unknown is not neutral.
    pub fn known_groups(&self) -> u8 {
        self.known_groups
    }
    pub fn sensor_gain_bits(&self) -> [u32; 3] {
        self.gain_bits
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroundedAvailabilityFrameV1 {
    owner: OrganismId,
    tick: Tick,
    frame_digest: PerceptionFrameDigest,
    sources: Vec<GroundedSourceAvailabilityV1>,
}
impl GroundedAvailabilityFrameV1 {
    pub fn owner(&self) -> OrganismId {
        self.owner
    }
    pub fn tick(&self) -> Tick {
        self.tick
    }
    pub fn sources(&self) -> &[GroundedSourceAvailabilityV1] {
        &self.sources
    }
    pub fn matches_frame(&self, frame: &PerceptionFrame) -> bool {
        frame.validate().is_ok()
            && frame.frame_digest() == self.frame_digest
            && frame.organism_id() == self.owner
            && frame.tick() == self.tick
    }
}

impl HeadlessWorld {
    /// Read the ordinary current World view, with calibrated availability attached.
    /// This may update ordinary identity tracking, but does not advance physics or
    /// biology. The old frame API and all its serialized values remain unchanged.
    pub fn perception_frame_with_availability_v1(
        &mut self,
        organism_id: OrganismId,
        profile: SensorProfile,
        homeostasis: HomeostaticSnapshot,
    ) -> Result<(PerceptionFrame, GroundedAvailabilityFrameV1), ScaffoldContractError> {
        if !profile.has_terrain_vision() || homeostasis.tick != self.tick {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let tick = self.tick;
        let mut sources = Vec::new();
        let (draft, _) = self.perception_frame_draft_with_availability_sink(
            organism_id,
            tick,
            profile,
            homeostasis,
            None,
            Some(&mut sources),
        )?;
        let frame = draft.finalize(PerceptionContextBlock::empty())?;
        let receipt = GroundedAvailabilityFrameV1 {
            owner: organism_id,
            tick,
            frame_digest: frame.frame_digest(),
            sources,
        };
        Ok((frame, receipt))
    }
}

// Only called from World's actual assembly, with the very snapshot used to
// extract this calibrated frame. Public extractor callers cannot obtain a receipt.
pub(super) fn capture_calibrated_sources(
    sink: &mut Vec<GroundedSourceAvailabilityV1>,
    snapshot: &PhysicalObservationSnapshot,
    grounded: &crate::GroundedSensingFrame,
    embodiment: Option<&EmbodimentState>,
) -> Result<(), ScaffoldContractError> {
    let gain = |cap| embodiment.map_or(1.0, |body| body.sensor_gain(cap));
    let gains = [
        gain(SensorCapability::Vision),
        gain(SensorCapability::Chemical),
        gain(SensorCapability::Touch),
    ];
    for (slot, transport) in grounded.slots().iter().zip(grounded.transports()) {
        let raw = snapshot
            .visible
            .iter()
            .find(|o| o.transport_entity == transport.transport_entity)
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        let expected = if raw.contact { gains[2].min(1.0) } else { 0.0 };
        if slot.contact.to_bits() != expected.to_bits() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let known_groups = (if gains[0] > 0.0 { 7 } else { 0 })
            | (if raw.contact && gains[1] > 0.0 { 8 } else { 0 })
            | (if raw.contact { 16 } else { 0 });
        sink.push(GroundedSourceAvailabilityV1 {
            source: slot.tracked_object_id,
            known_groups,
            gain_bits: gains.map(f32::to_bits),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_canonical_new_game, CanonicalNewGameConfig, PlayerInteractionV1};
    use alife_core::FoundationWeightAsset;
    fn world(distance: f32) -> HeadlessWorld {
        let mut world = HeadlessScenarioBuilder::new(2114)
            .agent("observer", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "target".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(distance, 0.0, 0.0),
                nutrition: 0.3,
                hazard_pain: 0.0,
                radius: 0.25,
                token_id: None,
            })
            .unwrap();
        world
    }
    const PROFILE: SensorProfile = SensorProfile::GroundedTerrainObjectPortsV2;
    #[test]
    fn same_world_frame_exact_off_path() {
        let mut old = world(0.2);
        let mut new = old.clone();
        let before = new.tick;
        let a = old
            .perception_frame(
                OrganismId(1),
                before,
                PROFILE,
                HomeostaticSnapshot::baseline(before),
            )
            .unwrap();
        let (b, receipt) = new
            .perception_frame_with_availability_v1(
                OrganismId(1),
                PROFILE,
                HomeostaticSnapshot::baseline(before),
            )
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&b).unwrap()
        );
        assert_eq!(a.base_digest(), b.base_digest());
        assert_eq!(a.frame_digest(), b.frame_digest());
        assert!(receipt.matches_frame(&b));
        assert_eq!(new.tick, before);
        assert_eq!(receipt.sources().len(), b.grounded_object_slots().len());
        let old_next = old
            .perception_frame(
                OrganismId(1),
                before,
                PROFILE,
                HomeostaticSnapshot::baseline(before),
            )
            .unwrap();
        let new_next = new
            .perception_frame(
                OrganismId(1),
                before,
                PROFILE,
                HomeostaticSnapshot::baseline(before),
            )
            .unwrap();
        assert_eq!(old_next, new_next); // no extra tracking pass from receipt issuance
    }
    #[test]
    fn contact_and_unknown_are_distinct() {
        for (distance, known) in [(0.2, 31), (0.6, 7)] {
            let mut w = world(distance);
            let (_, r) = w
                .perception_frame_with_availability_v1(
                    OrganismId(1),
                    PROFILE,
                    HomeostaticSnapshot::baseline(Tick::ZERO),
                )
                .unwrap();
            assert_eq!(r.sources().len(), 1);
            assert_eq!(r.sources()[0].known_groups(), known);
        }
    }
    #[test]
    fn different_frame_and_wrong_time_reject() {
        let mut a = world(0.2);
        let mut b = world(0.6);
        let (_, r) = a
            .perception_frame_with_availability_v1(
                OrganismId(1),
                PROFILE,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        let other = b
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                PROFILE,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        assert!(!r.matches_frame(&other));
        assert!(a
            .perception_frame_with_availability_v1(
                OrganismId(1),
                PROFILE,
                HomeostaticSnapshot::baseline(Tick(1))
            )
            .is_err());
        assert!(a
            .perception_frame_with_availability_v1(
                OrganismId(1),
                SensorProfile::GroundedObjectSlotsV1,
                HomeostaticSnapshot::baseline(Tick::ZERO)
            )
            .is_err());
    }
    #[test]
    fn measured_neutral_does_not_equal_unknown() {
        for (distance, expected) in [(0.2, 31), (0.6, 7)] {
            let mut w = world(distance);
            let id = w.entity_id("target").unwrap();
            let mut props = GroundedPhysicalProperties::deterministic_default(1);
            props.chemical = [0.0; 3];
            w.set_grounded_physical_properties(id, props).unwrap();
            let (frame, r) = w
                .perception_frame_with_availability_v1(
                    OrganismId(1),
                    PROFILE,
                    HomeostaticSnapshot::baseline(Tick::ZERO),
                )
                .unwrap();
            assert_eq!(frame.grounded_object_slots()[0].chemical, [0.0; 3]);
            assert_eq!(r.sources()[0].known_groups(), expected);
        }
    }
    #[test]
    fn hidden_source_has_no_availability_receipt() {
        let mut w = world(2.5);
        w.editor_spawn_object(WorldEditorSpawnSpec {
            label: "wall".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(1.0, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.5,
            token_id: None,
        })
        .unwrap();
        let (frame, r) = w
            .perception_frame_with_availability_v1(
                OrganismId(1),
                PROFILE,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        let food = w.entity_id("target").unwrap();
        assert!(frame
            .candidates()
            .iter()
            .all(|c| c.target.entity != Some(food)));
        assert_eq!(r.sources().len(), frame.grounded_object_slots().len());
        for (source, slot) in r.sources().iter().zip(frame.grounded_object_slots()) {
            assert_eq!(source.source(), slot.tracked_object_id);
        }
    }
    #[test]
    fn early_rejection_preserves_next_observation() {
        let mut w = world(0.2);
        let mut control = w.clone();
        assert!(w
            .perception_frame_with_availability_v1(
                OrganismId(1),
                PROFILE,
                HomeostaticSnapshot::baseline(Tick(1))
            )
            .is_err());
        assert!(w
            .perception_frame_with_availability_v1(
                OrganismId(1),
                SensorProfile::GroundedObjectSlotsV1,
                HomeostaticSnapshot::baseline(Tick::ZERO)
            )
            .is_err());
        let a = w
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                PROFILE,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        let b = control
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                PROFILE,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(w.tick, Tick::ZERO);
    }
    #[test]
    fn pending_four_feedback_binds_final_frame_without_consuming_doses() {
        let foundation =
            FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)
                .unwrap();
        let game = create_canonical_new_game(
            &CanonicalNewGameConfig::phase3(2128, 1).unwrap(),
            &foundation,
        )
        .unwrap();
        for profile in [
            SensorProfile::GroundedTerrainVisionV1,
            SensorProfile::GroundedTerrainObjectPortsV2,
        ] {
            let mut w = game.world.clone();
            let (id, entity) = w.organism_entity_ids()[0];
            let position = w.entity(entity).unwrap().position;
            let homeostasis = w
                .organism_registry()
                .get(id)
                .unwrap()
                .biochemistry()
                .homeostasis;
            let mut no_feedback = w.clone();
            let unmodified = no_feedback
                .perception_frame(id, Tick::ZERO, profile, homeostasis)
                .unwrap();
            for kind in [
                PlayerInteractionV1::Praise,
                PlayerInteractionV1::Scold,
                PlayerInteractionV1::Pet,
                PlayerInteractionV1::Slap,
            ] {
                w.queue_player_interaction_v1(id, position, kind).unwrap();
            }
            let pending = w.pending_player_care.clone();
            let organism = serde_json::to_vec(w.organism_registry().get(id).unwrap()).unwrap();
            let mut legacy = w.clone();
            let expected = legacy
                .perception_frame(id, Tick::ZERO, profile, homeostasis)
                .unwrap();
            let (actual, receipt) = w
                .perception_frame_with_availability_v1(id, profile, homeostasis)
                .unwrap();
            assert_eq!(
                serde_json::to_vec(&expected).unwrap(),
                serde_json::to_vec(&actual).unwrap()
            );
            assert!(receipt.matches_frame(&actual));
            assert!(!receipt.matches_frame(&unmodified));
            assert_eq!(actual.sensory().channels.auditory_acoustic[1], 1.0);
            assert!(actual.sensory().channels.auditory_acoustic[2] > 0.0);
            assert!(actual.sensory().channels.tactile_contact[0] > 0.0);
            assert_eq!(w.pending_player_care, pending);
            assert_eq!(
                serde_json::to_vec(w.organism_registry().get(id).unwrap()).unwrap(),
                organism
            );
            assert_eq!(w.tick, Tick::ZERO);
        }
    }

    #[test]
    fn availability_uses_the_actual_calibration_not_zero_feature_values() {
        let foundation =
            FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)
                .unwrap();
        let game = create_canonical_new_game(
            &CanonicalNewGameConfig::phase3(2128, 1).unwrap(),
            &foundation,
        )
        .unwrap();
        let (id, _) = game.world.organism_entity_ids()[0];
        let embodiment = game.world.organism_registry().get(id).unwrap().embodiment();
        for removed in 0_u8..8 {
            // Fresh validated test embodiment, not an edit to an acquired life.
            let mut value = serde_json::to_value(embodiment).unwrap();
            let ports = value["sensors"].as_array().unwrap().clone();
            let old = value["sensor_calibration"].as_array().unwrap().clone();
            let mut sensors = Vec::new();
            let mut calibration = Vec::new();
            let mut offset = 0;
            for port in ports {
                let width = port["value_lanes"].as_u64().unwrap() as usize;
                let bit = match port["capability"].as_str().unwrap() {
                    "Vision" => 1,
                    "Chemical" => 2,
                    "Touch" => 4,
                    _ => 0,
                };
                if bit == 0 || removed & bit == 0 {
                    sensors.push(port);
                    calibration.extend_from_slice(&old[offset..offset + width]);
                }
                offset += width;
            }
            value["sensors"] = serde_json::json!(sensors);
            value["sensor_calibration"] = serde_json::json!(calibration);
            let body: EmbodimentState = serde_json::from_value(value).unwrap();
            body.validate_contract().unwrap();
            for (distance, contact) in [(0.2, true), (0.6, false)] {
                let mut w = world(distance);
                let snapshot = w
                    .physical_observation_snapshot(OrganismId(1), Tick::ZERO)
                    .unwrap();
                let mut grounded =
                    GroundedSensorExtractor::extract(&snapshot, &mut w.tracked_objects).unwrap();
                grounded.calibrate(Some(&body)).unwrap();
                let mut sources = Vec::new();
                capture_calibrated_sources(&mut sources, &snapshot, &grounded, Some(&body))
                    .unwrap();
                let vision = body.sensor_gain(SensorCapability::Vision);
                let chemical = body.sensor_gain(SensorCapability::Chemical);
                let touch = body.sensor_gain(SensorCapability::Touch);
                assert_eq!(
                    sources[0].sensor_gain_bits(),
                    [vision, chemical, touch].map(f32::to_bits)
                );
                assert_eq!(
                    sources[0].known_groups(),
                    (if vision > 0.0 { 7 } else { 0 })
                        | (if contact && chemical > 0.0 { 8 } else { 0 })
                        | (if contact { 16 } else { 0 })
                );
                if contact && touch == 0.0 {
                    assert_eq!(grounded.slots()[0].contact, 0.0);
                    // Chemistry/temperature precede the separate tactile gain.
                    assert_ne!(sources[0].known_groups() & 16, 0);
                }
            }
        }
    }
}
