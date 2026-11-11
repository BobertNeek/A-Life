//! One CPU encoder oracle shared by the distinct hardware and offline fixtures.
use alife_core::{
    phenotype::SensorEncoderSourceGroup, BrainCapacityClass, OrganismId, PerceptionFrame,
    SensorProfile, SensorProfileProvenance, SensoryAbiVersion, Tick,
};
use alife_gpu_backend::GpuClosedLoopBackend;

use super::{perception_frame_for_profile_at_tick, phenotype_for_capacity_at_maturation};

/// Fresh synthetic founders only. The caller selects the execution device.
pub fn assert_visible_object_ports_match_cpu_encoding(backend: &mut GpuClosedLoopBackend) {
    let profile = SensorProfile::GroundedTerrainObjectPortsV2;
    let phenotype =
        phenotype_for_capacity_at_maturation(BrainCapacityClass::n2048(), 93, 1.0, profile);
    let physiology = super::test_physiology(93, &phenotype).unwrap();
    let bearing_neuron = phenotype
        .sensor_encoder()
        .assignments()
        .iter()
        .find(|a| {
            a.source_group() == SensorEncoderSourceGroup::VisibleObject && a.source_index() == 1
        })
        .expect("positive bearing lane must be assigned to a neuron")
        .target_neuron() as usize;
    let mut encoded = Vec::new();
    for (index, bearing) in [0.0f32, 0.5].into_iter().enumerate() {
        let id = OrganismId(index as u64 + 1);
        let at = Tick::new(220);
        let base = perception_frame_for_profile_at_tick(id.raw(), at.raw(), profile, false, 1);
        let mut slots = base.grounded_object_slots().to_vec();
        slots[0].bearing = [bearing, 0.5];
        let mut candidates = base.candidates().to_vec();
        candidates[0].features = slots[0].candidate_features().unwrap();
        let frame = PerceptionFrame::new(
            id,
            at,
            profile,
            base.sensory().clone(),
            base.body(),
            *base.homeostasis(),
            candidates,
            SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, at).unwrap(),
            slots,
        )
        .unwrap();
        let receptors = super::physiology_receptor_frame(&physiology, at).unwrap();
        let expression = alife_core::NeuralReceptorPhenotype::compile(&phenotype).unwrap();
        let effects =
            alife_core::NeuralReceptorEffects::from_frame(&receptors, &expression).unwrap();
        let sensory = frame.sensory().channels.as_flat_array();
        let body = frame.body().neural_projection(profile);
        let t = body.pose.translation;
        let r = body.pose.rotation;
        let v = body.velocity.linear;
        let av = body.velocity.angular;
        let body_lanes = [
            t.x, t.y, t.z, r.x, r.y, r.z, r.w, v.x, v.y, v.z, av.x, av.y, av.z,
        ];
        let home = frame
            .homeostasis()
            .drives
            .to_array()
            .into_iter()
            .chain(frame.homeostasis().hormones.to_array())
            .collect::<Vec<_>>();
        let language = frame.sensory().language_prior_neural_lanes();
        let objects = alife_core::visible_object_neural::visible_object_neural_lanes(
            profile,
            frame.grounded_object_slots(),
        )
        .unwrap();
        let mut expected = vec![0f32; phenotype.neuron_count() as usize];
        for a in phenotype.sensor_encoder().assignments() {
            let i = a.source_index() as usize;
            let source = match a.source_group() {
                SensorEncoderSourceGroup::SensoryChannel => sensory[i] * effects.attention_gain,
                SensorEncoderSourceGroup::Body => body_lanes[i],
                SensorEncoderSourceGroup::Homeostasis => home[i] * effects.interoceptive_gain,
                SensorEncoderSourceGroup::HeardLanguage => language[i],
                SensorEncoderSourceGroup::SemanticPrior => language[128 + i],
                SensorEncoderSourceGroup::VisibleObject => objects[i] * effects.attention_gain,
            };
            let (lo, hi) = a.clamp_range();
            expected[a.target_neuron() as usize] += (source * a.scale() + a.bias()).clamp(lo, hi);
        }
        let gate =
            (effects.structural_growth_gate + effects.sleep_gate + effects.consolidation_gate) / 3.;
        for v in &mut expected {
            *v = (*v * effects.regional_excitability + (gate - 0.5) * 0.05).clamp(-1., 1.);
        }
        let handle = backend.insert_brain(id, phenotype.clone()).unwrap();
        let (frame, recall) = super::try_empty_recall(&frame).unwrap();
        let upload = backend
            .prepare_memory_context_upload(handle, &frame, &recall)
            .unwrap();
        let upload =
            super::bind_chemistry_receptor_effects(upload, &phenotype, &physiology, at).unwrap();
        let input =
            alife_gpu_backend::GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload)
                .unwrap();
        let batch = alife_gpu_backend::GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
        let tick = backend.tick_memory_batch(&batch).unwrap().remove(0);
        let state = backend.capture_training_state(handle, at).unwrap();
        let range = state.brain_slot.word_ranges().encoded_input_words.clone();
        let actual = state.mutable_words[(range.start - state.mutable_word_base) as usize
            ..(range.end - state.mutable_word_base) as usize]
            .iter()
            .map(|v| f32::from_bits(*v))
            .collect::<Vec<_>>();
        assert_eq!(
            actual.len(),
            expected.len(),
            "read back every encoded neuron"
        );
        for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
            assert!((a - b).abs() < 1e-5, "neuron {i}: GPU {a}, reference {b}");
        }
        encoded.push(actual[bearing_neuron]);
        let receipt = backend
            .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
            .unwrap();
        assert_eq!(
            receipt.discarded_staging_generation,
            tick.pending_eligibility
                .identity()
                .staging_eligibility_generation()
        );
        backend.remove_brain(handle).unwrap();
    }
    assert!(
        (encoded[1] - encoded[0]).abs() > 0.05,
        "observed bearing must reach the brain"
    );
}
