use super::*;

fn encode_words(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_ne_bytes()).collect()
}

fn packed_bytes(weights: &[u32], optimizer: &[u32]) -> Vec<u8> {
    let mut bytes = encode_words(weights);
    bytes.extend(encode_words(optimizer));
    bytes
}

fn decoded_words(state: CheckpointReadback) -> Vec<u32> {
    state
        .weights
        .into_iter()
        .chain(state.first_moment)
        .chain(state.second_moment)
        .map(f32::to_bits)
        .chain(state.update_ages)
        .collect()
}

#[test]
fn layout_has_two_disjoint_copies_and_one_aligned_map_without_padding() {
    for count in [1, 2, 3, 17, 512, 1024, 2048, 32768] {
        let layout = CheckpointReadbackLayout::new(count, u64::MAX).unwrap();
        assert_eq!(layout.lane_bytes, count as u64 * 4);
        assert_eq!(layout.optimizer_bytes, count as u64 * 12);
        assert_eq!(layout.total_bytes, count as u64 * 16);
        let copies = [
            (0, layout.lane_bytes),
            (layout.lane_bytes, layout.optimizer_bytes),
        ];
        assert_eq!(copies[0].0 + copies[0].1, copies[1].0);
        assert_eq!(copies[1].0 + copies[1].1, layout.total_bytes);
        // wgpu 29: copies require 4-byte alignment; the map begins at 0
        // (8-byte aligned) and ends on a whole word, including odd counts.
        for (offset, length) in copies {
            assert_eq!(offset % 4, 0);
            assert_eq!(length % 4, 0);
        }
        assert_eq!(layout.total_bytes % 8, 0);
    }
}

#[test]
fn layout_rejects_empty_overflow_host_span_and_device_limit() {
    for count in [0, usize::MAX, usize::MAX / 4, usize::MAX / 16 + 1] {
        assert!(matches!(
            CheckpointReadbackLayout::new(count, u64::MAX),
            Err(TrainingError::MalformedReadback)
        ));
    }
    let count = (isize::MAX as usize) / 16;
    let largest_host_span = CheckpointReadbackLayout::new(count, u64::MAX).unwrap();
    assert_eq!(largest_host_span.total_bytes, count as u64 * 16);
    assert!(CheckpointReadbackLayout::new(count + 1, u64::MAX).is_err());
    for count in [1, 3, 2048] {
        let exact_limit = count as u64 * 16;
        assert!(CheckpointReadbackLayout::new(count, exact_limit).is_ok());
        assert!(CheckpointReadbackLayout::new(count, exact_limit - 1).is_err());
    }
}

#[test]
fn source_sizes_reject_each_short_lane_before_copy_encoding() {
    for count in [1, 3, 32768] {
        let layout = CheckpointReadbackLayout::new(count, u64::MAX).unwrap();
        assert!(layout
            .validate_source_sizes(layout.lane_bytes, layout.optimizer_bytes)
            .is_ok());
        assert!(layout
            .validate_source_sizes(layout.lane_bytes + 4, layout.optimizer_bytes + 4)
            .is_ok());
        for (weights, optimizer) in [
            (0, layout.optimizer_bytes),
            (layout.lane_bytes - 1, layout.optimizer_bytes),
            (layout.lane_bytes, 0),
            (layout.lane_bytes, layout.optimizer_bytes - 1),
            (layout.lane_bytes - 4, layout.optimizer_bytes - 4),
        ] {
            assert!(matches!(
                layout.validate_source_sizes(weights, optimizer),
                Err(TrainingError::MalformedReadback)
            ));
        }
    }
}

#[test]
fn packed_decode_is_bit_exact_to_three_read_reference_for_all_lanes() {
    let finite = [
        0,
        (-0.0_f32).to_bits(),
        1,
        0x8000_0001,
        1.25_f32.to_bits(),
        (-2.5_f32).to_bits(),
        f32::MAX.to_bits(),
        f32::MIN_POSITIVE.to_bits(),
    ];
    let ages = [0, 1, 7, 0x7fc0_0001, 0xffff_ffff, 0x8000_0000];
    for count in [1, 3, 7, 2048, 32768] {
        let weights: Vec<_> = (0..count).map(|i| finite[i % finite.len()]).collect();
        let mut optimizer: Vec<_> = (0..count * 2)
            .map(|i| finite[(i + 3) % finite.len()])
            .collect();
        optimizer.extend((0..count).map(|i| ages[i % ages.len()]));
        // This is the old three-read interpretation: finite moments, finite
        // weights, then exact integer ages. It performs no neural computation.
        let reference: Vec<_> = weights
            .iter()
            .chain(&optimizer[..count * 2])
            .chain(&optimizer[count * 2..])
            .copied()
            .collect();
        let bytes = packed_bytes(&weights, &optimizer);
        let layout = CheckpointReadbackLayout::new(count, bytes.len() as u64).unwrap();
        let state = layout.decode(&bytes).unwrap();
        assert_eq!(state.weights.len(), count);
        assert_eq!(state.first_moment.len(), count);
        assert_eq!(state.second_moment.len(), count);
        assert_eq!(state.update_ages.len(), count);
        let actual = decoded_words(state);
        assert_eq!(actual, reference);
        assert_eq!(encode_words(&actual), bytes);
    }
}

#[test]
fn decoding_does_not_require_the_input_slice_to_be_word_aligned() {
    let layout = CheckpointReadbackLayout::new(3, 48).unwrap();
    let expected = [
        0,
        0x8000_0000,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        0x7fc0_0001,
        9,
        u32::MAX,
    ];
    let bytes = encode_words(&expected);
    let mut storage = vec![0; bytes.len() + 4];
    let start = (0..4)
        .find(|offset| !(storage.as_ptr() as usize + offset).is_multiple_of(4))
        .unwrap();
    storage[start..start + bytes.len()].copy_from_slice(&bytes);
    assert_eq!(
        decoded_words(layout.decode(&storage[start..start + bytes.len()]).unwrap()),
        expected
    );
}

#[test]
fn wrong_length_rejects_short_long_and_partial_words() {
    let layout = CheckpointReadbackLayout::new(3, 48).unwrap();
    for size in [0, 1, 44, 47, 49, 52, 96] {
        assert!(matches!(
            layout.decode(&vec![0; size]),
            Err(TrainingError::MalformedReadback)
        ));
    }
}

#[test]
fn every_float_lane_rejects_infinities_and_nan_payloads() {
    let layout = CheckpointReadbackLayout::new(3, 48).unwrap();
    for invalid in [0x7f80_0000, 0xff80_0000, 0x7fc0_0001, 0x7f80_0001] {
        for position in 0..9 {
            let mut values = vec![0; 12];
            values[position] = invalid;
            assert!(matches!(
                layout.decode(&encode_words(&values)),
                Err(TrainingError::MalformedReadback)
            ));
        }
        let values = [0, 0, 0, 0, 0, 0, 0, 0, 0, invalid, invalid, invalid];
        assert_eq!(
            layout.decode(&encode_words(&values)).unwrap().update_ages,
            vec![invalid; 3]
        );
    }
}

#[test]
fn a_later_capture_uses_new_bytes_and_cannot_change_the_earlier_result() {
    let layout = CheckpointReadbackLayout::new(1, 16).unwrap();
    let mut bytes = encode_words(&[0, 1, 2, 3]);
    let before = layout.decode(&bytes).unwrap();
    bytes.copy_from_slice(&encode_words(&[4, 5, 6, 7]));
    let after = layout.decode(&bytes).unwrap();
    assert_eq!(decoded_words(before), [0, 1, 2, 3]);
    assert_eq!(decoded_words(after), [4, 5, 6, 7]);
}
