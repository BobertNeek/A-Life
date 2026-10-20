//! Canonical biological ATP policy with CPU-owned dispatch identity.
use alife_core::*;
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CpuActivityState {
    pub last_world_tick: Option<u64>,
    pub remaining_atp_q16: u32,
    pub source_frame_digest: [u64; 4],
}
impl Default for CpuActivityState {
    fn default() -> Self {
        Self {
            last_world_tick: None,
            remaining_atp_q16: BRAIN_ATP_Q16_MAX,
            source_frame_digest: [0; 4],
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct CpuActivityAnchor {
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub sequence_cursor: u64,
    pub previous_dispatch_generation: u64,
    pub source_frame_digest: [u64; 4],
    pub frame_digest: [u64; 4],
}
#[derive(Debug, Clone)]
pub struct CpuActivityBudget {
    pub microsteps: u8,
    pub enabled_routes: Vec<bool>,
    pub work: BrainWorkCounters,
    pub available_atp_q16: u32,
    pub debit_atp_q16: u32,
    pub affordable: bool,
}
/// Evaluate the same native ATP buckets, mandatory-route rules and Q24/Q16
/// affordability arithmetic. The shared policy's hardware pressure terms are
/// neutral; this helper emits CPU budget data, never GPU/hardware evidence.
pub fn cpu_activity_budget(
    p: &BrainPhenotype,
    anchor: CpuActivityAnchor,
    canonical_atp: f32,
    candidates: u32,
    memory_records: u32,
) -> Result<CpuActivityBudget, ScaffoldContractError> {
    if !canonical_atp.is_finite() || !(0.0..=1.0).contains(&canonical_atp) {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    let capacity = BrainCapacityClass::production_for_id(p.brain_class_id())?;
    if candidates > u32::from(capacity.execution().max_candidates())
        || memory_records > u32::from(capacity.execution().max_candidates())
    {
        return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
    }
    let policy = BrainActivityPolicyV1::production_v1();
    let available = (f64::from(canonical_atp) * f64::from(BRAIN_ATP_Q16_MAX)).floor() as u32;
    let identity = BrainDispatchIdentity {
        organism_id_raw: anchor.organism_id.raw(),
        tick: anchor.tick.raw(),
        class_id_raw: p.brain_class_id().raw(),
        handle_slot: 0,
        handle_generation: 1,
        sequence_cursor: anchor.sequence_cursor,
        dispatch_generation: anchor
            .previous_dispatch_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::BrainActivitySequenceMismatch)?,
        frame_digest: anchor.frame_digest,
    };
    // These zero hardware terms are exactly the native look-ahead affordability
    // calculation. No GPU adapter, telemetry sample or hardware receipt is created.
    let pressure = GpuPressureSample::try_new(
        &policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: anchor.previous_dispatch_generation,
            source_frame_digest: anchor.source_frame_digest,
            completed_gpu_time_ns: 0,
            queue_depth: 0,
            logical_heap_used: 0,
            logical_heap_capacity: 1,
            brain_atp_remaining_q16: available,
            brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
        },
    )?;
    let decision =
        NeuralThrottleDecision::derive(&policy, p, capacity.execution(), identity, pressure)?;
    let work = alife_gpu_backend::derive_executed_work(
        p,
        decision.microsteps,
        &decision.enabled_route_ids,
        candidates,
        memory_records,
    )?;
    let debit = policy
        .cost
        .q24_to_atp_q16_round_half_up(policy.cost.neural_cost_q24(&work)?)?;
    let enabled_routes = p
        .projections()
        .iter()
        .map(|r| {
            decision
                .enabled_route_ids
                .binary_search(&r.route_index())
                .is_ok()
        })
        .collect();
    Ok(CpuActivityBudget {
        microsteps: decision.microsteps,
        enabled_routes,
        work,
        available_atp_q16: available,
        debit_atp_q16: debit,
        affordable: available >= debit,
    })
}
