//! Observed, unscored feature projection for the explicit grounded-drive ABI.
//! No world object kind, hidden dose, reward or action choice enters this module.
use crate::*;

pub const GROUNDED_DRIVE_LANE_START: usize = 36;
pub const GROUNDED_DRIVE_LANE_END: usize = 46;

pub fn prepare_grounded_drive_features(
    frame: &PerceptionFrame,
) -> Result<Vec<[f32; 10]>, ScaffoldContractError> {
    frame.validate()?;
    if !matches!(
        frame.sensor_profile(),
        SensorProfile::GroundedObjectSlotsV1 | SensorProfile::GroundedTerrainVisionV1
    ) {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    let invalid = || ScaffoldContractError::InvalidPerceptionFrame;
    let mut slot_targets = std::collections::BTreeMap::new();
    let mut entity_slots = std::collections::BTreeMap::new();
    for a in frame.candidates() {
        if let CandidateObservationRef::ObjectSlot(sa) = a.observation {
            let entity = a.target.entity.ok_or_else(invalid)?;
            if slot_targets
                .insert(sa, a.target)
                .is_some_and(|prior| prior != a.target)
                || entity_slots
                    .insert(entity.raw(), sa)
                    .is_some_and(|prior| prior != sa)
            {
                return Err(invalid());
            }
        }
    }
    let thermal_contact = frame
        .grounded_object_slots()
        .iter()
        .fold(0_f32, |max, slot| {
            let excess = ((slot.temperature.abs() - 0.25) / 0.75).clamp(0., 1.);
            max.max(slot.contact * excess)
        });
    frame
        .candidates()
        .iter()
        .enumerate()
        .map(|(index, c)| {
            if usize::from(c.candidate_index) != index {
                return Err(invalid());
            }
            let mut out = [0.; 10];
            if let CandidateObservationRef::ObjectSlot(si) = c.observation {
                let slot = frame
                    .grounded_object_slots()
                    .get(usize::from(si))
                    .ok_or_else(invalid)?;
                if slot.slot_index != si || slot.candidate_features()? != c.features {
                    return Err(invalid());
                }
                out[0] = 1.;
                out[1] = slot.chemical[0].max(0.);
                out[2] = (-slot.chemical[0]).max(0.);
                out[3] = ((slot.temperature - 0.25) / 0.75).clamp(0., 1.);
                out[4] = ((-slot.temperature - 0.25) / 0.75).clamp(0., 1.);
                out[5] = f32::from(
                    frame
                        .sensory()
                        .social_context
                        .nearest_agents
                        .iter()
                        .flatten()
                        .any(|s| s.body_entity.is_some() && s.body_entity == c.target.entity),
                );
            }
            out[6] = f32::from(
                c.observation == CandidateObservationRef::None
                    && c.target == ActionTarget::NONE
                    && c.kind == ActionKind::Vocalize,
            );
            out[7] = thermal_contact;
            // Personal novelty/play expectancy stay zero until supported by memory.
            Ok(out)
        })
        .collect()
}
