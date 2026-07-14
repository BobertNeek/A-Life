use alife_core::CompiledSynapseKind;
use alife_gpu_backend::GpuClosedLoopError;

use super::support;

#[test]
fn n512_two_slot_gpu_batch_reports_exact_cadence_counts_and_banks() {
    pollster::block_on(async {
        let phenotype = support::n512_phenotype(4101);
        assert!(phenotype
            .synapses()
            .iter()
            .any(|row| matches!(row.kind(), CompiledSynapseKind::Recurrent)));
        let frame0 = support::perception_frame(7, true, 1);
        let frame1 = support::perception_frame(9, true, 2);
        let frame2 = support::perception_frame(13, true, 1);
        let mut fixture = support::GpuPipelineFixture::new(&phenotype).await;
        let readback = fixture.run([&frame0, &frame1, &frame2]).await;
        let expected =
            support::expected_cadence_counts(&phenotype, phenotype.microstep_count().into());
        println!("closed-loop adapter: {}", readback.adapter_name);
        assert!(!readback.adapter_name.trim().is_empty());
        assert!(readback.readback_bytes <= 256);
        assert!(!readback.sample_indices.is_empty() && readback.sample_indices.len() <= 8);
        assert!(readback
            .sample_indices
            .iter()
            .all(|index| *index < phenotype.neuron_count()));
        assert!(readback
            .sample_indices
            .iter()
            .enumerate()
            .all(|(i, value)| !readback.sample_indices[..i].contains(value)));
        assert!(!readback.recurrent_sample_positions.is_empty());
        assert!(readback
            .recurrent_sample_positions
            .iter()
            .all(|p| *p < readback.sample_indices.len()));
        assert!(!readback.loop_sample_positions.is_empty());
        assert!(readback
            .loop_sample_positions
            .iter()
            .all(|p| *p < readback.sample_indices.len()));
        assert_eq!(readback.host_final_sides, vec![1, 1, 1]);
        for (index, slot) in readback.slots.iter().enumerate() {
            assert_eq!(slot.gpu_active_side, readback.host_final_sides[index]);
            assert_eq!((slot.active_tiles, slot.active_synapses), expected);
            assert!(slot.active_synapses > 0);
            assert_eq!(slot.finite_rejections, 0);
            assert!(slot
                .activation_a
                .iter()
                .chain(&slot.activation_b)
                .all(|value| value.is_finite()));
            assert!(active_sample(slot).iter().any(|value| *value != 0.0));
        }
    });
}

#[test]
fn n512_gpu_activity_changes_with_current_frame_and_recurrent_weights() {
    pollster::block_on(async {
        let phenotype = support::n512_phenotype(4101);
        let nonzero0 = support::perception_frame(7, true, 1);
        let nonzero1 = support::perception_frame(9, true, 2);
        let nonzero2 = support::perception_frame(13, true, 1);
        let mut fixture = support::GpuPipelineFixture::new(&phenotype).await;
        fixture.restore_mutable_checkpoint();
        fixture.set_recurrent_genetic_weights_zeroed(false);
        let nonzero = fixture.run([&nonzero0, &nonzero1, &nonzero2]).await;
        fixture.restore_mutable_checkpoint();
        let zero_frame = fixture
            .run_with_complete_frame_payload_zeroed([&nonzero0, &nonzero1, &nonzero2])
            .await;
        assert_ne!(
            active_sample(&nonzero.slots[0]),
            active_sample(&zero_frame.slots[0])
        );
        fixture.restore_mutable_checkpoint();
        fixture.set_recurrent_genetic_weights_zeroed(false);
        let normal = fixture.run([&nonzero0, &nonzero1, &nonzero2]).await;
        fixture.restore_mutable_checkpoint();
        fixture.set_recurrent_genetic_weights_zeroed(true);
        let zero = fixture.run([&nonzero0, &nonzero1, &nonzero2]).await;
        fixture.set_recurrent_genetic_weights_zeroed(false);
        assert_ne!(
            recurrent_active_sample(&normal, 0),
            recurrent_active_sample(&zero, 0)
        );
        assert!(normal
            .slots
            .iter()
            .chain(&zero.slots)
            .all(|slot| slot.finite_rejections == 0));
    });
}

#[test]
fn n512_gpu_ping_pong_handles_two_three_four_and_prior_frame_persistence() {
    pollster::block_on(async {
        let frame0 = support::perception_frame(7, true, 1);
        let frame1 = support::perception_frame(9, true, 2);
        let frame2 = support::perception_frame(13, true, 1);
        for (maturation, microsteps) in [(0.2, 2_u32), (0.5, 3), (0.8, 4)] {
            let phenotype = support::n512_phenotype_at_maturation(4101, maturation);
            assert_eq!(u32::from(phenotype.microstep_count()), microsteps);
            let mut fixture = support::GpuPipelineFixture::new(&phenotype).await;
            let readback = fixture.run([&frame0, &frame1, &frame2]).await;
            let sides = vec![microsteps & 1; 3];
            assert_eq!(readback.host_final_sides, sides);
            assert_eq!(
                readback
                    .slots
                    .iter()
                    .map(|slot| slot.gpu_active_side)
                    .collect::<Vec<_>>(),
                sides
            );
            let counters = support::expected_cadence_counts(&phenotype, microsteps);
            assert!(readback
                .slots
                .iter()
                .all(|slot| (slot.active_tiles, slot.active_synapses) == counters));
        }
        let phenotype = support::n512_phenotype_at_maturation(4101, 0.5);
        let mut fixture = support::GpuPipelineFixture::new(&phenotype).await;
        fixture.restore_mutable_checkpoint();
        let control_first = fixture.run([&frame0, &frame1, &frame2]).await;
        let control_next = fixture.run([&frame0, &frame1, &frame2]).await;
        assert_eq!(control_first.host_final_sides, vec![1, 1, 1]);
        assert_eq!(control_next.host_final_sides, vec![0, 0, 0]);
        fixture.restore_mutable_checkpoint();
        let inactive_first = fixture.run([&frame0, &frame1, &frame2]).await;
        assert_eq!(inactive_first.slots, control_first.slots);
        for (index, side) in inactive_first.host_final_sides.iter().enumerate() {
            fixture.poison_activation_bank(index, side ^ 1, 0.9375);
        }
        let inactive_next = fixture.run([&frame0, &frame1, &frame2]).await;
        for index in 0..2 {
            assert_eq!(
                loop_active_sample(&inactive_next, index),
                loop_active_sample(&control_next, index)
            );
        }
        fixture.restore_mutable_checkpoint();
        let active_first = fixture.run([&frame0, &frame1, &frame2]).await;
        assert_eq!(active_first.slots, control_first.slots);
        for (index, side) in active_first.host_final_sides.iter().enumerate() {
            fixture.poison_activation_bank(index, *side, -0.875);
        }
        let active_next = fixture.run([&frame0, &frame1, &frame2]).await;
        assert_ne!(
            loop_active_sample(&active_next, 0),
            loop_active_sample(&control_next, 0)
        );
        assert!(control_next
            .slots
            .iter()
            .chain(&inactive_next.slots)
            .chain(&active_next.slots)
            .flat_map(|slot| slot.activation_a.iter().chain(&slot.activation_b))
            .all(|value| value.is_finite()));
    });
}

#[test]
fn one_n512_batch_honors_heterogeneous_compiled_two_three_four_schedules() {
    pollster::block_on(async {
        let two = support::n512_phenotype_at_maturation(4101, 0.2);
        let three = support::n512_phenotype_at_maturation(4101, 0.5);
        let four = support::n512_phenotype_at_maturation(4101, 0.8);
        assert_eq!(
            [
                two.microstep_count(),
                three.microstep_count(),
                four.microstep_count()
            ],
            [2, 3, 4]
        );
        let frames = [
            support::perception_frame(7, true, 1),
            support::perception_frame(9, true, 2),
            support::perception_frame(13, true, 1),
        ];
        let mut fixture =
            support::GpuPipelineFixture::new_with_phenotypes([&two, &three, &four]).await;
        let readback = fixture.run([&frames[0], &frames[1], &frames[2]]).await;
        assert_eq!(readback.host_final_sides, vec![0, 1, 0]);
        assert_eq!(
            readback
                .slots
                .iter()
                .map(|slot| slot.gpu_active_side)
                .collect::<Vec<_>>(),
            vec![0, 1, 0]
        );
        for (slot, phenotype) in readback.slots.iter().zip([&two, &three, &four]) {
            assert_eq!(
                (slot.active_tiles, slot.active_synapses),
                support::expected_cadence_counts(phenotype, phenotype.microstep_count().into())
            );
            assert_eq!(slot.finite_rejections, 0);
        }
    });
}

#[test]
fn pipeline_rejects_a_foreign_bucket_with_the_same_class_slot_and_generation_tuple() {
    pollster::block_on(async {
        let phenotype = support::n512_phenotype(4101);
        let frame = support::perception_frame(7, true, 1);
        let mut fixture = support::GpuPipelineFixture::new(&phenotype).await;
        assert_eq!(
            fixture.foreign_same_tuple_batch_error(&phenotype, &frame),
            GpuClosedLoopError::StaleOrForeignHandle
        );
        assert_eq!(
            fixture.oversized_frame_base_error(&frame),
            GpuClosedLoopError::CapacityExceeded
        );
    });
}

fn active_sample(slot: &support::SlotReadback) -> &[f32] {
    if slot.gpu_active_side == 0 {
        &slot.activation_a
    } else {
        &slot.activation_b
    }
}

fn recurrent_active_sample(readback: &support::BatchReadback, slot: usize) -> Vec<f32> {
    let active = active_sample(&readback.slots[slot]);
    readback
        .recurrent_sample_positions
        .iter()
        .map(|position| active[*position])
        .collect()
}

fn loop_active_sample(readback: &support::BatchReadback, slot: usize) -> Vec<f32> {
    let active = active_sample(&readback.slots[slot]);
    readback
        .loop_sample_positions
        .iter()
        .map(|position| active[*position])
        .collect()
}
