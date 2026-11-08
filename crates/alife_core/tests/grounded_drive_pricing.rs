use alife_core::*;
#[test]
fn old_cost_wire_and_rates_are_unchanged() {
    let old = BrainAtpCostModel::production_v1();
    assert_eq!(
        serde_json::to_string(&old).unwrap(),
        r#"{"schema_version":1,"q_fraction_bits":24,"rounding_mode_raw":1,"neuron_update_q24":32,"tile_visit_q24":256,"synapse_op_q24":4,"decoder_candidate_op_q24":128,"memory_context_op_q24":64}"#
    );
    for class in BrainCapacityClass::production_classes() {
        BrainActivityPolicyV1::production_v1()
            .validate_class(class.id())
            .unwrap();
        assert!(BrainActivityPolicyV1::grounded_drive_research_v1()
            .validate_class(class.id())
            .is_err());
    }
}
#[test]
fn incremental_model_keeps_candidate_count_semantics_and_rejects_cross_policy() {
    let old = BrainActivityPolicyV1::production_v1();
    let new = BrainActivityPolicyV1::grounded_drive_research_v1();
    old.validate_contract().unwrap();
    new.validate_contract().unwrap();
    assert_ne!(old.policy_digest, new.policy_digest);
    assert!(old
        .validate_class(BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID)
        .is_err());
    new.validate_class(BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID)
        .unwrap();
    for candidates in [0, 1, 17, 32] {
        let work = BrainWorkCounters {
            decoder_candidate_ops: candidates,
            ..Default::default()
        };
        assert_eq!(
            new.cost.neural_cost_q24(&work).unwrap() - old.cost.neural_cost_q24(&work).unwrap(),
            candidates * 160 * 4
        );
        assert_eq!(work.decoder_candidate_ops, candidates);
    }
    let mut bad = old;
    bad.cost = new.cost;
    assert!(bad.validate_contract().is_err());
    let mut bad = new;
    bad.cost.decoder_candidate_op_q24 += 1;
    assert!(bad.validate_contract().is_err());
}
