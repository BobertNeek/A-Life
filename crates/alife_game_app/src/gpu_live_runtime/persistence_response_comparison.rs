//! Readiness checks distinguish inactive external hints from gameplay differences.

use alife_core::PerceptionFrame;

pub(super) fn gameplay_perception_matches(left: &PerceptionFrame, right: &PerceptionFrame) -> bool {
    let mut left_sensory = left.sensory().clone();
    let mut right_sensory = right.sensory().clone();
    for sensory in [&mut left_sensory, &mut right_sensory] {
        if sensory.semantic_context.as_ref().is_some_and(|context| {
            context.confidence.0 == 0.0
                && context.feature_flags == alife_core::ContextFeatureFlags::NONE
                && context.salience.is_empty()
        }) {
            sensory.semantic_context = None;
        }
    }
    left.organism_id() == right.organism_id()
        && left.tick() == right.tick()
        && left.profile_provenance() == right.profile_provenance()
        && left.body() == right.body()
        && left.homeostasis() == right.homeostasis()
        && left.grounded_object_slots() == right.grounded_object_slots()
        && left.candidates() == right.candidates()
        && left.context() == right.context()
        && left_sensory == right_sensory
}

#[cfg(feature = "gpu-tests")]
pub(super) fn gameplay_response_matches(
    left: &alife_core::ExperiencePatch,
    right: &alife_core::ExperiencePatch,
) -> bool {
    use alife_core::DecisionEvidence;
    let evidence_matches = match (&left.decision().evidence, &right.decision().evidence) {
        (
            DecisionEvidence::NeuralClosedLoopGpu(left),
            DecisionEvidence::NeuralClosedLoopGpu(right),
        ) => {
            let mut left = *left;
            left.base_digest = right.base_digest;
            left.frame_digest = right.frame_digest;
            left == *right
        }
        (left, right) => left == right,
    };
    // Each patch is validated against its own frame by the caller. Compare the
    // predictor/concept/attention payloads while allowing their source identities
    // to differ with the inactive external hint metadata.
    let mut cognitive = left.pre_action().cognitive_context.clone();
    if let (Some(left), Some(right)) = (&mut cognitive, &right.pre_action().cognitive_context) {
        left.prediction.source_digest = right.prediction.source_digest;
        if let (Some(left), Some(right)) =
            (&mut left.cognitive_projection, &right.cognitive_projection)
        {
            left.base_frame_digest = right.base_frame_digest;
            for (left, right) in left.candidates.iter_mut().zip(&right.candidates) {
                left.prediction.source_digest = right.prediction.source_digest;
            }
        }
    }
    let mut prediction_target = left.prediction_target().cloned();
    if let (Some(left), Some(right)) = (&mut prediction_target, right.prediction_target()) {
        left.source_digest = right.source_digest;
    }
    left.header() == right.header()
        && left.pre_action().genome_id == right.pre_action().genome_id
        && left.pre_action().genome_schema_version == right.pre_action().genome_schema_version
        && left.pre_action().development_state == right.pre_action().development_state
        && gameplay_perception_matches(
            left.pre_action().perception(),
            right.pre_action().perception(),
        )
        && left.decision().selected_action == right.decision().selected_action
        && left.decision().selected_bundle == right.decision().selected_bundle
        && left.decision().confidence == right.decision().confidence
        && evidence_matches
        && cognitive == right.pre_action().cognitive_context
        && prediction_target.as_ref() == right.prediction_target()
        && left.outcome() == right.outcome()
        && left.cognitive_work() == right.cognitive_work()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{
        CompressedSemanticCode, Confidence, ContextFeatureFlags, HomeostaticSnapshot,
        NormalizedScalar, OrganismId, PerceptionContextBlock, SemanticContextRef, SensorProfile,
        Tick, Vec3f,
    };
    use alife_world::HeadlessScenarioBuilder;

    fn frame(distance: f32, gain: Option<f32>) -> PerceptionFrame {
        frame_with_flags(distance, gain, ContextFeatureFlags::NONE)
    }

    fn frame_with_flags(
        distance: f32,
        gain: Option<f32>,
        flags: ContextFeatureFlags,
    ) -> PerceptionFrame {
        let mut world = HeadlessScenarioBuilder::new(31_117)
            .agent("learner", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(distance, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        world
            .perception_frame_draft(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap()
            .with_semantic_context(gain.map(|gain| {
                SemanticContextRef {
                    feature_flags: flags,
                    confidence: Confidence(gain),
                    compressed_codes: [1, 3, 14]
                        .into_iter()
                        .map(|code| CompressedSemanticCode {
                            codebook_id: 1,
                            code,
                            salience: NormalizedScalar(0.3),
                        })
                        .collect(),
                    salience: Vec::new(),
                }
            }))
            .unwrap()
            .finalize(PerceptionContextBlock::empty())
            .unwrap()
    }

    #[test]
    fn zero_gain_reply_changes_diagnostics_without_changing_ordinary_perception() {
        let no_reply = frame(1.0, None);
        let reply = frame(1.0, Some(0.0));
        assert_ne!(no_reply.base_digest(), reply.base_digest());
        assert_ne!(no_reply.frame_digest(), reply.frame_digest());
        assert!(gameplay_perception_matches(&no_reply, &reply));
        assert!(reply.sensory().semantic_context.is_some());
    }

    #[test]
    fn meaningful_prior_and_world_changes_remain_readiness_failures() {
        let original = frame(1.0, None);
        assert!(!gameplay_perception_matches(
            &original,
            &frame(1.0, Some(0.05))
        ));
        assert!(!gameplay_perception_matches(
            &original,
            &frame(4.0, Some(0.0))
        ));
        assert!(!gameplay_perception_matches(
            &original,
            &frame_with_flags(1.0, Some(0.0), ContextFeatureFlags::INTERNAL_SLM_MODULATION)
        ));
    }
}
