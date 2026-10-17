use super::*;

fn sample_record(candidate_index: u32) -> GpuCandidateMemoryRecord {
    GpuCandidateMemoryRecord {
        candidate_index,
        target_latent: [1.0, -2.0, 3.5, -4.5, 5.0, 6.25, -7.0, 8.0],
        family_value: [9.0, -10.0, 11.5, -12.5],
        target_confidence: 0.25,
        family_confidence: 0.75,
        source_counts_packed: 17 | (u32::from(u16::MAX) << 16),
    }
}

fn legacy_context(records: &[GpuCandidateMemoryRecord]) -> Vec<f32> {
    let mut values = Vec::with_capacity(records.len() * 16);
    for row in records {
        values.extend_from_slice(&row.target_latent);
        values.extend_from_slice(&row.family_value);
        values.push(row.target_confidence);
        values.push(row.family_confidence);
        values.push((row.source_counts_packed & 0xffff) as f32);
        values.push((row.source_counts_packed >> 16) as f32);
    }
    values
}

#[test]
fn memory_context_comparison_matches_legacy_lanes_and_lengths() {
    assert!(memory_records_match_context(&[], &[]));
    assert!(!memory_records_match_context(&[], &[0.0]));
    for count in [1, 2, 32] {
        let records = (0..count).map(sample_record).collect::<Vec<_>>();
        let context = legacy_context(&records);
        assert!(memory_records_match_context(&records, &context));
        assert!(!memory_records_match_context(&records, &context[1..]));
        let mut longer = context.clone();
        longer.push(0.0);
        assert!(!memory_records_match_context(&records, &longer));
        for lane in 0..context.len() {
            let mut changed = context.clone();
            changed[lane] += 1.0;
            assert!(!memory_records_match_context(&records, &changed));
        }
    }
}

#[test]
fn memory_context_comparison_preserves_ordinary_float_equality() {
    let mut row = sample_record(0);
    row.target_latent[0] = -0.0;
    row.family_value[0] = 0.0;
    row.target_confidence = -0.0;
    row.family_confidence = 0.0;
    let records = [row];
    let mut context = legacy_context(&records);
    for value in &mut context {
        if *value == 0.0 {
            *value = -*value;
        }
    }
    assert!(memory_records_match_context(&records, &context));
    for nonfinite in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut records = records;
        records[0].target_latent[0] = nonfinite;
        let context = legacy_context(&records);
        assert_eq!(
            memory_records_match_context(&records, &context),
            context == legacy_context(&records),
        );
    }
}
