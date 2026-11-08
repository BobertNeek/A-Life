//! Prospective basis prototype, NOT an admitted phenotype or runtime decoder.
//! Proposed replacement of cognitive lanes36..45 only. NOT runtime-admitted.
//! Raw0..23, memory24..35, cognitive46..52 and grip53 remain bit-preserved.
use alife_core::*;
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const SELECTOR: &str = "N2048GroundedDriveBasisV1";
pub const BASIS_DOMAIN: &str = "alife.n2048.grounded-drive-basis.v1";
pub const PROPOSED_CANONICAL_TAG: u16 = 10;

pub const START: usize = 36;
pub const STRIDE: usize = 54;
pub const LANES: [&str; 10] = [
    "observed_object",
    "positive_taste",
    "negative_taste",
    "hot_excess",
    "cold_excess",
    "perceived_peer",
    "intrinsic_vocal",
    "observed_current_thermal_contact",
    "personal_novelty_reserved",
    "play_expectancy_reserved",
];
pub fn thermal_pair(temperature: f32) -> Result<[f32; 2]> {
    if !temperature.is_finite() || !(-1.0..=1.0).contains(&temperature) {
        return Err("invalid signed sensed temperature".into());
    }
    Ok([
        ((temperature - 0.25) / 0.75).clamp(0., 1.),
        ((-temperature - 0.25) / 0.75).clamp(0., 1.),
    ])
}
/// The optional peer match uses only the same already observed social snapshot.
/// Neither world objects nor their kinds/health/reproduction eligibility enter.
pub fn project(
    frame: &PerceptionFrameDraft,
    index: usize,
    legacy: &[f32; 54],
) -> Result<[f32; 54]> {
    frame.validate()?;
    if !matches!(
        frame.sensor_profile(),
        SensorProfile::GroundedObjectSlotsV1 | SensorProfile::GroundedTerrainVisionV1
    ) {
        return Err("requires grounded observed objects".into());
    }
    let c = frame.candidates().get(index).ok_or("missing candidate")?;
    if c.candidate_index as usize != index
        || legacy[..24] != c.features.0
        || legacy.iter().any(|x| !x.is_finite())
    {
        return Err("legacy candidate/frame mismatch".into());
    }
    // A slot has one entity target and an entity cannot alias two slots.
    // Transport-to-world truth remains the canonical frame producer's duty.
    for a in frame.candidates() {
        if let CandidateObservationRef::ObjectSlot(sa) = a.observation {
            if a.target.entity.is_none() {
                return Err("missing observed target".into());
            }
            for b in frame.candidates() {
                if let CandidateObservationRef::ObjectSlot(sb) = b.observation {
                    if (sa == sb && a.target != b.target)
                        || (sa != sb && a.target.entity == b.target.entity)
                    {
                        return Err("cross-slot target alias".into());
                    }
                }
            }
        }
    }
    let mut out = *legacy;
    if let CandidateObservationRef::ObjectSlot(slot_index) = c.observation {
        let slot = frame
            .grounded_object_slots()
            .iter()
            .find(|s| s.slot_index == slot_index)
            .ok_or("candidate is not observed")?;
        if slot.candidate_features()? != c.features {
            return Err("candidate physical evidence mismatch".into());
        }
        out[36] = 1.;
        out[37] = slot.chemical[0].max(0.);
        out[38] = (-slot.chemical[0]).max(0.);
        [out[39], out[40]] = thermal_pair(slot.temperature)?;
        out[41] = f32::from(
            frame
                .sensory()
                .social_context
                .nearest_agents
                .iter()
                .flatten()
                .any(|s| s.body_entity.is_some() && s.body_entity == c.target.entity),
        );
    }
    out[42] = f32::from(
        c.observation == CandidateObservationRef::None
            && c.target == ActionTarget::NONE
            && c.kind == ActionKind::Vocalize,
    );
    for slot in frame.grounded_object_slots() {
        let [hot, cold] = thermal_pair(slot.temperature)?;
        out[43] = out[43].max(slot.contact * (hot + cold));
    }
    // No privileged novelty, object kind or guessed welfare expectation.
    out[44] = 0.;
    out[45] = 0.;
    Ok(out)
}
/// Prospective added rows, interleaved after each family's unchanged raw rows.
/// Packing changes old IDs after family zero; identity is the full coordinate.
pub fn contract() -> Value {
    let rows = (0_u8..8)
        .flat_map(|family| {
            (36_u16..46).flat_map(move |lane| {
                (0_u16..16).map(move |motor| json!({
                "head":1,"family":family,"input_lane":lane,
                "motor_index":16*u16::from(family)+motor,
                "packed_id":24576+544*u32::from(family)+384+16*u32::from(lane-36)+u32::from(motor),
                "initial_weight":0.0
            }))
            })
        })
        .collect::<Vec<_>>();
    json!({"schema":"prospective_grounded_drive_basis_contract_v1",
        "selector":SELECTOR,"domain":BASIS_DOMAIN,"proposed_canonical_tag":PROPOSED_CANONICAL_TAG,
        "legacy_stride":54,"stride":STRIDE,"input_lane_start":START,"lane_names":LANES,
        "head":1,"row_order":["family","input_lane","motor_index"],"rows":rows,
        "replacement_count":0,"additional_synapses":1280,"total_synapses":34048,
        "candidate_synapses":4352,"fresh_branch_count":64,"branch_gain":0.25,"branch_threshold":0.1,
        "required_capacity_total_ceiling":34112,"required_capacity_action_ceiling":5376,
        "must_bind_fresh_graph_foundation_and_branch_plan":true,
        "cognitive_extension_allowed":false,"cognitive_decoder_allowed":false,
        "admitted_by_current_runtime":false,"legacy_cpu_replay_input_supported":false,
        "unknown_novelty_and_play_expectancy":0.})
}
