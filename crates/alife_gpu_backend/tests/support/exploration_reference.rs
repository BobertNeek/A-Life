//! Test-only reference arithmetic for bounded game exploration.
//! Integration tests check it against the WGSL inputs and arithmetic.

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub organism_id: u64,
    pub tick: u64,
    pub novelty: f32,
    pub curiosity: f32,
    pub hunger: f32,
    pub fatigue: f32,
    pub temperature_stress: f32,
    pub fear: f32,
    pub pain: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub index: u32,
    pub identity: [u32; 4],
    pub logit: f32,
    pub gap_match: f32,
    pub motor_slot: Option<usize>,
}

pub fn mix(mut value: u32) -> u32 {
    value = (value ^ (value >> 16)).wrapping_mul(0x7feb_352d);
    value = (value ^ (value >> 15)).wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

pub fn identity_hash(state: State, candidate: Candidate) -> u32 {
    let mut value = 0x6d2b_79f5;
    for word in [
        state.organism_id as u32,
        (state.organism_id >> 32) as u32,
        state.tick as u32,
        (state.tick >> 32) as u32,
        candidate.identity[0],
        candidate.identity[1],
        candidate.identity[2],
        candidate.identity[3],
    ] {
        value = mix(value ^ word);
    }
    value
}

pub fn uniform(state: State, candidate: Candidate) -> f32 {
    ((identity_hash(state, candidate) >> 9) as f32 + 0.5) / 8_388_608.0
}

pub fn noise(state: State, candidate: Candidate) -> f32 {
    2.0 * (2.0 * uniform(state, candidate) - 1.0)
}

pub fn urgency(state: State) -> f32 {
    let critical = ((state
        .hunger
        .max(state.fatigue)
        .max(state.temperature_stress)
        - 0.65)
        / 0.35)
        .clamp(0.0, 1.0);
    state.fear.max(state.pain).max(critical).clamp(0.0, 1.0)
}

pub fn scale(state: State, candidate: Candidate) -> f32 {
    0.5 * state.novelty.max(candidate.gap_match).clamp(0.0, 1.0)
        * (0.6 + 0.4 * state.curiosity.clamp(0.0, 1.0))
        * (1.0 - urgency(state))
}

pub fn score(state: State, candidate: Candidate) -> f32 {
    let scale = scale(state, candidate);
    if scale == 0.0 {
        candidate.logit
    } else {
        candidate.logit + scale * noise(state, candidate)
    }
}

pub fn select(state: State, candidates: &[Candidate], motor_slot: Option<usize>) -> Option<u32> {
    let mut best: Option<(u32, f32)> = None;
    for candidate in candidates {
        if !candidate.logit.is_finite()
            || motor_slot.is_some_and(|slot| candidate.motor_slot != Some(slot))
        {
            continue;
        }
        let value = score(state, *candidate);
        if best.is_none_or(|(index, previous)| {
            value > previous || (value == previous && candidate.index < index)
        }) {
            best = Some((candidate.index, value));
        }
    }
    best.map(|(index, _)| index)
}

pub fn pair() -> [Candidate; 2] {
    [
        Candidate {
            index: 0,
            identity: [4, 0, 0, 0],
            logit: 0.0,
            gap_match: 0.0,
            motor_slot: Some(0),
        },
        Candidate {
            index: 1,
            identity: [101, 0, 0, 0],
            logit: -0.2,
            gap_match: 0.0,
            motor_slot: Some(0),
        },
    ]
}

pub fn alternate_count(mut state: State, candidates: &[Candidate], ticks: u64) -> usize {
    (0..ticks)
        .filter(|tick| {
            state.tick = *tick;
            select(state, candidates, None) == Some(1)
        })
        .count()
}
