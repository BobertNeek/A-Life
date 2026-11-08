//! Prospective basis prototype, NOT an admitted phenotype or runtime decoder.
//! Existing ports0..53 are preserved. This cannot be passed to legacy54 replay.
use alife_core::*;
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const SELECTOR: &str = "N2048BroadDriveBasisV1";
pub const BASIS_DOMAIN: &str = "alife.n2048.broad-drive-physical-basis.v1";
pub const NEW_HEAD_RAW: u32 = 5;
pub const START: usize = 54;
pub const STRIDE: usize = 64;
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
) -> Result<[f32; 64]> {
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
    let mut out = [0.; 64];
    out[..54].copy_from_slice(legacy);
    if let CandidateObservationRef::ObjectSlot(slot_index) = c.observation {
        let slot = frame
            .grounded_object_slots()
            .iter()
            .find(|s| s.slot_index == slot_index)
            .ok_or("candidate is not observed")?;
        if slot.candidate_features()? != c.features {
            return Err("candidate physical evidence mismatch".into());
        }
        out[54] = 1.;
        out[55] = slot.chemical[0].max(0.);
        out[56] = (-slot.chemical[0]).max(0.);
        [out[57], out[58]] = thermal_pair(slot.temperature)?;
        out[59] = f32::from(
            frame
                .sensory()
                .social_context
                .nearest_agents
                .iter()
                .flatten()
                .any(|s| s.body_entity.is_some() && s.body_entity == c.target.entity),
        );
    }
    out[60] = f32::from(
        c.observation == CandidateObservationRef::None
            && c.target == ActionTarget::NONE
            && c.kind == ActionKind::Vocalize,
    );
    for slot in frame.grounded_object_slots() {
        let [hot, cold] = thermal_pair(slot.temperature)?;
        out[61] = out[61].max(slot.contact * (hot + cold));
    }
    // No privileged novelty, object kind or guessed welfare expectation.
    out[62] = 0.;
    out[63] = 0.;
    Ok(out)
}
/// Exact prospective canonical row order. Appended rows have their own head;
/// no old raw, memory, cognitive or grip coordinate is repurposed.
pub fn contract() -> Value {
    let rows=(0_u8..8).flat_map(|family|(54_u16..64).flat_map(move|lane|(0_u16..16).map(move|motor|json!({"head":NEW_HEAD_RAW,"family":family,"input_lane":lane,"motor_index":16*u16::from(family)+motor})))).collect::<Vec<_>>();
    json!({"schema":"prospective_broad_drive_basis_contract_v1","selector":SELECTOR,"domain":BASIS_DOMAIN,
        "legacy_stride":54,"stride":STRIDE,"input_lane_start":START,"lane_names":LANES,
        "new_decoder_head":NEW_HEAD_RAW,"canonical_order":["family","input_lane","motor_index"],"rows":rows,
        "additional_synapses":1280,"required_total_without_other_changes":34048,
        "must_bind_fresh_graph_foundation_and_branch_plan":true,"admitted_by_current_runtime":false,
        "legacy_cpu_replay_input_supported":false,"readiness_distress_coefficient":0.,"unbound_distress_coefficient":0.,
        "unknown_novelty_and_play_expectancy":0.})
}
