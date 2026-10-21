//! CPU reference arithmetic and shader checks for bounded game exploration.
//! GPU behavior is covered separately.

#[path = "support/exploration_reference.rs"]
mod reference;

use alife_gpu_backend::CLOSED_LOOP_DECODE_WGSL;
use reference::{
    alternate_count, identity_hash, noise, pair, scale, score, select, urgency, Candidate, State,
};

fn module() -> naga::Module {
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_DECODE_WGSL)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(CLOSED_LOOP_DECODE_WGSL)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    module
}

fn warm() -> State {
    State {
        organism_id: 0x12_3456_789a,
        novelty: 1.0,
        curiosity: 1.0,
        ..State::default()
    }
}

#[test]
fn production_decode_validates_entry_points_and_bindings() {
    let module = module();
    let mut entries: Vec<_> = module
        .entry_points
        .iter()
        .map(|entry| (entry.name.as_str(), entry.workgroup_size))
        .collect();
    entries.sort_by_key(|(name, _)| *name);
    assert_eq!(
        entries,
        [
            ("decode_candidates", [32, 1, 1]),
            ("decode_speech_payload", [1, 1, 1]),
            ("select_candidate", [1, 1, 1]),
        ]
    );
    let mut bindings: Vec<_> = module
        .global_variables
        .iter()
        .filter_map(|(_, global)| {
            global
                .binding
                .as_ref()
                .map(|binding| (binding.group, binding.binding))
        })
        .collect();
    bindings.sort_unstable();
    assert_eq!(
        bindings,
        (0..7).map(|binding| (0, binding)).collect::<Vec<_>>()
    );
}

#[test]
fn familiar_gap_free_inputs_preserve_greedy_ties_and_raw_zero_exactly() {
    let mut state = warm();
    state.novelty = 0.0;
    let mut candidates = pair();
    candidates[1].logit = candidates[0].logit;
    for tick in 0..1024 {
        state.tick = tick;
        assert_eq!(select(state, &candidates, None), Some(0));
        assert_eq!(scale(state, candidates[0]), 0.0);
    }
    candidates[0].logit = -0.0;
    assert_eq!(score(state, candidates[0]).to_bits(), (-0.0_f32).to_bits());
}

#[test]
fn novel_and_targeted_gap_evidence_warm_while_curiosity_alone_cannot() {
    let candidates = pair();
    let high = alternate_count(warm(), &candidates, 16_384);
    let low_curiosity = alternate_count(
        State {
            curiosity: 0.0,
            ..warm()
        },
        &candidates,
        16_384,
    );
    assert!(high > 5_000 && high < 8_000, "novel alternate count {high}");
    assert!(
        high > low_curiosity + 200,
        "curiosity must modulate actual evidence: {high}/{low_curiosity}"
    );
    assert_eq!(
        alternate_count(
            State {
                novelty: 0.0,
                ..warm()
            },
            &candidates,
            16_384
        ),
        0
    );
    let mut targeted = candidates;
    targeted[1].gap_match = 1.0;
    let familiar = State {
        novelty: 0.0,
        ..warm()
    };
    assert_eq!(scale(familiar, targeted[0]), 0.0);
    assert_eq!(scale(familiar, targeted[1]), 0.5);
    let gap_count = alternate_count(familiar, &targeted, 16_384);
    assert!(
        gap_count > 4_000 && gap_count < 8_000,
        "target gap alternate count {gap_count}"
    );
}

#[test]
fn fear_pain_and_critical_needs_suppress_exploration_at_the_same_logits() {
    let candidates = pair();
    let baseline = alternate_count(warm(), &candidates, 16_384);
    for state in [
        State {
            fear: 0.75,
            ..warm()
        },
        State {
            pain: 0.75,
            ..warm()
        },
        State {
            hunger: 0.9125,
            ..warm()
        },
        State {
            fatigue: 0.9125,
            ..warm()
        },
        State {
            temperature_stress: 0.9125,
            ..warm()
        },
    ] {
        assert!((urgency(state) - 0.75).abs() < 1e-6);
        let suppressed = alternate_count(state, &candidates, 16_384);
        assert!(
            suppressed + 1_000 < baseline,
            "urgency did not suppress: {suppressed}/{baseline}"
        );
    }
    for state in [
        State {
            fear: 1.0,
            ..warm()
        },
        State {
            pain: 1.0,
            ..warm()
        },
        State {
            hunger: 1.0,
            ..warm()
        },
        State {
            fatigue: 1.0,
            ..warm()
        },
        State {
            temperature_stress: 1.0,
            ..warm()
        },
    ] {
        assert_eq!(alternate_count(state, &candidates, 1024), 0);
    }
    assert_eq!(
        urgency(State {
            hunger: 0.65,
            fatigue: 0.65,
            temperature_stress: 0.65,
            ..warm()
        }),
        0.0
    );
}

#[test]
fn perturbations_are_finite_bounded_and_never_rescue_an_extreme_loser() {
    let mut state = warm();
    let mut candidates = pair();
    candidates[1].logit = -2.001;
    for tick in 0..16_384 {
        state.tick = tick;
        for candidate in candidates {
            let draw = noise(state, candidate);
            assert!(draw.is_finite() && draw > -2.0 && draw < 2.0);
            let change = score(state, candidate) - candidate.logit;
            assert!(change.abs() <= 1.000_001);
        }
        assert_eq!(select(state, &candidates, None), Some(0));
    }
}

#[test]
fn absolute_calibration_is_offset_stable_and_does_not_inflate_for_logit_scale() {
    let mut state = warm();
    let mut candidates = pair();
    candidates[1].logit = -0.25;
    let mut offset_disagreements = 0;
    for tick in 0..4096 {
        state.tick = tick;
        let shifted = candidates.map(|candidate| Candidate {
            logit: candidate.logit + 64.0,
            ..candidate
        });
        offset_disagreements +=
            usize::from(select(state, &candidates, None) != select(state, &shifted, None));
        let enlarged = candidates.map(|candidate| Candidate {
            logit: candidate.logit * 100.0,
            ..candidate
        });
        for (original, scaled) in candidates.iter().zip(enlarged) {
            assert_eq!(scale(state, *original), scale(state, scaled));
        }
        assert_eq!(select(state, &enlarged, None), Some(0));
    }
    assert!(
        offset_disagreements <= 1,
        "unexpected common-offset sensitivity: {offset_disagreements}"
    );
}

#[test]
fn single_legal_and_nonfinite_candidates_fail_closed_without_inventing_actions() {
    let mut state = warm();
    let mut candidates = pair();
    candidates[0].index = 7;
    candidates[1].index = 19;
    for tick in 0..4096 {
        state.tick = tick;
        assert!(matches!(select(state, &candidates, None), Some(7 | 19)));
        assert_eq!(select(state, &candidates[..1], None), Some(7));
    }
    candidates[0].logit = f32::NAN;
    assert_eq!(select(state, &candidates, None), Some(19));
    candidates[1].logit = f32::INFINITY;
    assert_eq!(select(state, &candidates, None), None);
    candidates[0].logit = f32::NEG_INFINITY;
    assert_eq!(select(state, &candidates, None), None);
    assert_eq!(select(state, &[], None), None);
}

#[test]
fn shared_candidate_scores_survive_channel_filtering_and_unrelated_loser_addition() {
    let mut state = warm();
    let candidates = pair();
    let unrelated = Candidate {
        index: 2,
        identity: [9001, 0, 0, 0],
        logit: -100.0,
        gap_match: 1.0,
        motor_slot: Some(3),
    };
    for tick in 0..4096 {
        state.tick = tick;
        let winner = select(state, &candidates, None);
        let extended = [candidates[0], candidates[1], unrelated];
        assert_eq!(select(state, &extended, None), winner);
        assert_eq!(select(state, &extended, Some(0)), winner);
        assert_eq!(select(state, &extended, Some(3)), Some(2));
        assert_eq!(select(state, &extended, Some(5)), None);
    }
}

#[test]
fn all_durable_identity_words_change_draws_while_candidate_indexes_do_not() {
    let state = warm();
    let candidate = pair()[0];
    let original = identity_hash(state, candidate);
    for changed in [
        State {
            organism_id: state.organism_id ^ (1 << 40),
            ..state
        },
        State {
            organism_id: state.organism_id ^ 1,
            ..state
        },
        State {
            tick: state.tick ^ (1 << 40),
            ..state
        },
        State {
            tick: state.tick ^ 1,
            ..state
        },
    ] {
        assert_ne!(identity_hash(changed, candidate), original);
    }
    for word in 0..4 {
        let mut changed = candidate;
        changed.identity[word] ^= 1;
        assert_ne!(identity_hash(state, changed), original);
    }
    assert_eq!(
        identity_hash(
            state,
            Candidate {
                index: candidate.index + 1,
                ..candidate
            }
        ),
        original
    );
}

#[test]
fn equal_logit_32_candidate_sweep_covers_every_index_without_cap_pileup_bias() {
    let candidates: Vec<_> = (0..32)
        .map(|index| Candidate {
            index,
            identity: [100 + index, 0, 0, 0],
            logit: 0.0,
            gap_match: 0.0,
            motor_slot: Some(0),
        })
        .collect();
    let mut counts = [0_u32; 32];
    let mut state = warm();
    for tick in 0..65_536 {
        state.tick = tick;
        let selected = select(state, &candidates, None).unwrap();
        counts[selected as usize] += 1;
    }
    for (index, count) in counts.iter().enumerate() {
        assert!(
            (1_650..=2_450).contains(count),
            "candidate {index} skew/coverage failure: {counts:?}"
        );
    }
}
