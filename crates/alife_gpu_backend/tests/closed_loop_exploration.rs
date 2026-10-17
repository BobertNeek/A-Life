//! CPU/structural qualification of bounded GPU game arbitration.
//!
//! The mathematical fixtures do not execute neural computation and are not
//! hardware parity or gameplay acceptance. Naga validation and exact helper
//! contracts bind their inputs/arithmetic to the actual production shader.

#[path = "support/exploration_reference.rs"]
mod reference;

use alife_gpu_backend::{
    CLOSED_LOOP_DECODE_WGSL, CLOSED_LOOP_MEMORY_CONTEXT_WGSL, GPU_ACTIVE_DISPATCH_ROW_WORDS,
    GPU_CLOSED_LOOP_LAYOUT_VERSION, GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
    GPU_PENDING_ELIGIBILITY_WORDS, GPU_PERCEPTION_DISPATCH_ROW_WORDS,
};
use reference::{
    alternate_count, identity_hash, noise, pair, scale, score, select, urgency, Candidate, State,
};

fn compact(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap())
        .flat_map(str::split_whitespace)
        .collect()
}

fn function_body<'a>(source: &'a str, name: &str) -> &'a str {
    let signature = format!("fn {name}(");
    let start = source.find(&signature).expect("required WGSL function");
    let open = start + source[start..].find('{').unwrap();
    let mut depth = 1;
    for (offset, byte) in source.as_bytes()[open + 1..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return &source[open + 1..open + 1 + offset];
        }
    }
    panic!("unclosed WGSL function {name}")
}

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

fn calls_in(block: &naga::Block, output: &mut Vec<naga::Handle<naga::Function>>) {
    for statement in block.iter() {
        match statement {
            naga::Statement::Call { function, .. } => output.push(*function),
            naga::Statement::Block(block) => calls_in(block, output),
            naga::Statement::If { accept, reject, .. } => {
                calls_in(accept, output);
                calls_in(reject, output);
            }
            naga::Statement::Switch { cases, .. } => {
                for case in cases {
                    calls_in(&case.body, output);
                }
            }
            naga::Statement::Loop {
                body, continuing, ..
            } => {
                calls_in(body, output);
                calls_in(continuing, output);
            }
            _ => {}
        }
    }
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
fn production_decode_validates_without_new_passes_bindings_layout_or_readback() {
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
    assert_eq!(GPU_ACTIVE_DISPATCH_ROW_WORDS, 332);
    assert_eq!(GPU_PERCEPTION_DISPATCH_ROW_WORDS, 272);
    assert_eq!(GPU_CLOSED_LOOP_LAYOUT_VERSION, 4);
    assert_eq!(GPU_PENDING_ELIGIBILITY_WORDS, 36);
    assert_eq!(GPU_CLOSED_LOOP_TICK_READBACK_BYTES, 80);
}

#[test]
fn exact_production_hash_uses_full_durable_identity_without_packed_state() {
    assert_eq!(
        compact(function_body(
            CLOSED_LOOP_DECODE_WGSL,
            "game_exploration_mix"
        )),
        compact(
            r#"
        var value = input;
        value = (value ^ (value >> 16u)) * 0x7feb352du;
        value = (value ^ (value >> 15u)) * 0x846ca68bu;
        return value ^ (value >> 16u);
    "#
        )
    );
    assert_eq!(
        compact(function_body(
            CLOSED_LOOP_DECODE_WGSL,
            "game_exploration_noise"
        )),
        compact(
            r#"
        var value = 0x6d2b79f5u;
        value = game_exploration_mix(value ^ state.organism_id.x);
        value = game_exploration_mix(value ^ state.organism_id.y);
        value = game_exploration_mix(value ^ state.tick.x);
        value = game_exploration_mix(value ^ state.tick.y);
        value = game_exploration_mix(value ^ identity.x);
        value = game_exploration_mix(value ^ identity.y);
        value = game_exploration_mix(value ^ identity.z);
        value = game_exploration_mix(value ^ identity.w);
        let uniform = (f32(value >> 9u) + 0.5) / 8388608.0;
        return GAME_EXPLORATION_NOISE_LIMIT * (2.0 * uniform - 1.0);
    "#
        )
    );
}

#[test]
fn production_scale_reads_only_actual_novelty_gap_and_urgency_evidence() {
    let state = compact(function_body(
        CLOSED_LOOP_DECODE_WGSL,
        "game_exploration_state",
    ));
    for required in [
        "load_learning_header(row_base+GAME_EXPLORATION_LEARNING_HEADER_OFFSET)",
        "frame_payload_words[pending+12u]",
        "frame_payload_words[pending+13u]",
        "vec2<u32>(header.tick_lo,header.tick_hi)",
        "header.sensory_offset+41u",
        "header.sensory_offset+55u",
        "header.sensory_offset+56u",
        "header.sensory_offset+57u",
        "header.sensory_offset+58u",
        "header.sensory_offset+60u",
        "header.sensory_offset+62u",
        "max(max(fear,pain),critical_need)",
    ] {
        assert!(
            state.contains(required),
            "missing actual state source {required}"
        );
    }
    let scale = compact(function_body(
        CLOSED_LOOP_DECODE_WGSL,
        "game_candidate_exploration_scale",
    ));
    assert_eq!(
        scale,
        compact(
            r#"
        var gap = 0.0;
        if (stride >= GAME_EXPLORATION_COGNITIVE_LANE_END) {
            gap = bitcast<f32>(frame_payload_words[candidate.feature_offset + GAME_EXPLORATION_GAP_LANE]);
            if (!game_exploration_unit(gap)) { return bitcast<f32>(INVALID_LOGIT_BITS); }
        }
        let evidence = max(state.novelty, gap);
        return GAME_EXPLORATION_MAX_SCALE * evidence * (0.6 + 0.4 * state.curiosity) * (1.0 - state.urgency);
    "#
        ),
        "the only gap load must remain dominated by the 54-lane guard"
    );
    assert!(scale.contains("if(stride>=GAME_EXPLORATION_COGNITIVE_LANE_END)"));
    assert!(scale.contains("candidate.feature_offset+GAME_EXPLORATION_GAP_LANE"));
    assert!(
        scale.contains("if(!game_exploration_unit(gap)){returnbitcast<f32>(INVALID_LOGIT_BITS);}")
    );
    assert!(scale.contains("letevidence=max(state.novelty,gap);"));
    assert!(scale.contains(
        "returnGAME_EXPLORATION_MAX_SCALE*evidence*(0.6+0.4*state.curiosity)*(1.0-state.urgency);"
    ));
    assert!(
        !scale.contains("51u"),
        "remembered outcome magnitude is not prediction error"
    );
    let source = compact(CLOSED_LOOP_DECODE_WGSL);
    for constant in [
        "GAME_EXPLORATION_MAX_SCALE:f32=0.5;",
        "GAME_EXPLORATION_NOISE_LIMIT:f32=2.0;",
        "GAME_EXPLORATION_CRITICAL_NEED:f32=0.65;",
        "GAME_EXPLORATION_LEARNING_HEADER_OFFSET:u32=272u;",
        "GAME_EXPLORATION_COGNITIVE_LANE_END:u32=54u;",
        "GAME_EXPLORATION_GAP_LANE:u32=50u;",
    ] {
        assert!(
            source.contains(constant),
            "calibration/ABI contract changed: {constant}"
        );
    }
    let memory = compact(CLOSED_LOOP_MEMORY_CONTEXT_WGSL);
    assert!(memory.contains("cognitive.schema_version==COGNITIVE_PROJECTION_SCHEMA_VERSION"));
    assert!(memory.contains(
        "header.decoder_learning_input_offset+gid.x*input_stride+COGNITIVE_LANE_START+channel"
    ));
    assert!(memory.contains("=bitcast<u32>(sample);"));
}

#[test]
fn representative_and_motor_channels_call_the_same_scoring_helper_and_preserve_raw_receipt() {
    let module = module();
    for name in ["select_candidate", "decode_speech_payload"] {
        let entry = module
            .entry_points
            .iter()
            .find(|entry| entry.name == name)
            .unwrap();
        let mut calls = Vec::new();
        calls_in(&entry.function.body, &mut calls);
        let names: Vec<_> = calls
            .iter()
            .map(|handle| module.functions[*handle].name.as_deref().unwrap())
            .collect();
        assert_eq!(
            names
                .iter()
                .filter(|name| **name == "game_candidate_arbitration_score")
                .count(),
            1
        );
        assert!(names.contains(&"game_exploration_enabled"));
        assert!(names.contains(&"game_exploration_state_valid"));
        let body = compact(function_body(CLOSED_LOOP_DECODE_WGSL, name));
        assert!(body.contains("if(bits==INVALID_LOGIT_BITS){continue;}"));
        assert!(body.contains("if(!finite_decode(logit)){continue;}"));
        assert!(body.contains("if(!finite_decode(score)){"));
        assert!(body.contains("CONTRACT_INVALID_DIAGNOSTIC_BIT"));
    }
    let select = compact(function_body(CLOSED_LOOP_DECODE_WGSL, "select_candidate"));
    assert!(select.contains("selected_logit=logit;"));
    assert!(select.contains("bitcast<u32>(selected_logit)"));
    let motor = compact(function_body(
        CLOSED_LOOP_DECODE_WGSL,
        "decode_speech_payload",
    ));
    assert!(motor.contains("if(joint_mode&&(enabled_channels&(1u<<slot))==0u){continue;}"));
    assert!(motor.contains("motor_candidates[slot]=selection.candidate_index;"));
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

#[test]
fn production_invalid_state_guard_checks_payload_identity_ticks_and_normalized_evidence() {
    let guard = compact(function_body(
        CLOSED_LOOP_DECODE_WGSL,
        "game_exploration_state_valid",
    ));
    for required in [
        "span_within(learning.outcome_offset,36u+header.candidate_count*4u,count)",
        "span_within(header.sensory_offset,77u,count)",
        "frame_payload_words[pending+12u]==0u&&frame_payload_words[pending+13u]==0u",
        "frame_payload_words[pending+16u]!=header.tick_lo",
        "frame_payload_words[pending+17u]!=header.tick_hi",
        "array<u32,7>(41u,55u,56u,57u,58u,60u,62u)",
        "game_exploration_unit(bitcast<f32>",
    ] {
        assert!(
            guard.contains(required),
            "missing fail-closed source guard {required}"
        );
    }
    assert_eq!(
        compact(function_body(
            CLOSED_LOOP_DECODE_WGSL,
            "game_exploration_unit"
        )),
        "returnfinite_decode(value)&&value>=0.0&&value<=1.0;"
    );
}
