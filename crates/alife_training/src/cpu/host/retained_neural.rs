//! Explicit retained-neural newborn projection. Global actor/Adam coordinates
//! never change. The native fixed-slot compiler authenticates the live mapping.
use super::*;
use alife_gpu_backend::GpuLiveTopologyCheckpointV1;
use alife_world::persistence::PortableDualWeightBankV1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuRetainedNeuralInheritance {
    pub topology: GpuLiveTopologyCheckpointV1,
    pub lifetime: PortableDualWeightBankV1,
    pub fast: PortableDualWeightBankV1,
    pub predictor: GroundedSuccessorPredictor,
}

impl CpuRetainedNeuralInheritance {
    /// Deterministic forward check against the native fixed-slot layout and
    /// native V11 dendritic/structural CPU contract. This is not a GPU run or a
    /// gameplay score. It catches reordered mutable banks and dropped edges.
    pub fn forward_mapping_probe(
        &self,
        founder: &CpuFoundationTrainer,
    ) -> Result<serde_json::Value, CpuHostError> {
        let p = founder.phenotype();
        let map = self.canonical_to_live(p)?;
        let bank = |w: &PortableDualWeightBankV1| {
            if w.active_bank == 0 {
                w.bank_0_bits.clone()
            } else {
                w.bank_1_bits.clone()
            }
        };
        let lifetime = bank(&self.lifetime);
        let fast = bank(&self.fast);
        let offsets = map
            .iter()
            .map(|&i| {
                f32::from_bits(lifetime[i])
                    + f32::from_bits(self.topology.alpha_bits[i]) * f32::from_bits(fast[i])
            })
            .collect::<Vec<_>>();
        let n = p.neuron_count() as usize;
        let initial = crate::TrainingInitialState {
            activations: (0..n).map(|i| 0.03 + (i % 11) as f32 * 0.007).collect(),
            activity_ema: vec![0.02; n],
            metabolic_load: vec![0.01; n],
            dendrites: self.topology.v11_checkpoint.dendritic_branches.clone(),
        };
        let structural = self
            .topology
            .v11_checkpoint
            .sparse_spans
            .iter()
            .flat_map(|s| &s.edges)
            .map(|e| crate::TrainingFrozenSynapse {
                source: e.source,
                target: e.target,
                route: e.route,
                cadence: u32::from(p.projections()[e.route as usize].update_cadence().raw()),
                effective_weight: e.weight,
            })
            .collect::<Vec<_>>();
        let tick = crate::TrainingReplayTick {
            encoded_inputs: (0..n).map(|i| (i % 7) as f32 * 0.013).collect(),
            projection_gain: 0.85,
            local_threshold_shift: 0.017,
            microstep_count: u32::from(p.microstep_count()),
            enabled_routes: vec![true; p.projections().len()],
            effective_weight_offsets: offsets,
            structural_synapses: structural,
            candidates: vec![crate::TrainingReplayCandidate {
                family: CandidateActionFamily::Idle,
                decoder_inputs: [0.; 54],
                innate_bias: 0.,
            }],
        };
        let q = crate::TrainingSequence {
            phenotype_hash: p.phenotype_hash(),
            initial: initial.clone(),
            ticks: vec![tick.clone(); 2],
            burn_in_ticks: 0,
            memory_candidate_gain: p
                .candidate_decoder()
                .memory_channel()
                .map_or(0., |m| m.max_candidate_gain()),
        };
        let cpu = founder.evaluate_replay(&q)?;
        let mut a = initial.activations;
        let mut m = initial.metabolic_load;
        let mut ema = initial.activity_ema;
        let mut maximum = 0f32;
        let t = &self.topology;
        let base_live = map
            .iter()
            .copied()
            .filter(|&i| i < t.recurrent_synapse_count as usize)
            .collect::<std::collections::BTreeSet<_>>();
        for time in 0..2 {
            for step in 0..p.microstep_count() as usize {
                let mut sums = vec![0.; n];
                for (target, sum) in sums.iter_mut().enumerate() {
                    for i in
                        t.target_offsets[target] as usize..t.target_offsets[target + 1] as usize
                    {
                        if !base_live.contains(&i) {
                            continue;
                        }
                        let route = t.route_indices[i] as usize;
                        if crate::cpu::replay::route_fires(
                            p.projections()[route].update_cadence().raw(),
                            step,
                        ) {
                            let effective = (f32::from_bits(t.genetic_weight_bits[i])
                                + f32::from_bits(lifetime[i]))
                                + f32::from_bits(t.alpha_bits[i]) * f32::from_bits(fast[i]);
                            *sum += a[t.source_indices[i] as usize] * effective;
                        }
                    }
                }
                let mut v11 = self.topology.v11_checkpoint.clone();
                for span in &mut v11.sparse_spans {
                    span.edges.retain(|e| {
                        crate::cpu::replay::route_fires(
                            p.projections()[e.route as usize].update_cadence().raw(),
                            step,
                        )
                    });
                }
                v11.sparse_spans.retain(|s| !s.edges.is_empty());
                // Native restore binds spans to structural connections, so use
                // its original checkpoint and apply its dendritic contract here.
                alife_core::apply_dendritic_conjunctions(&a, &mut sums, &v11.dendritic_branches)
                    .map_err(|_| {
                        CpuHostError::State("native dendritic reference rejected retained state")
                    })?;
                for span in &v11.sparse_spans {
                    for e in &span.edges {
                        sums[e.target as usize] += a[e.source as usize] * e.weight;
                    }
                }
                let mut next = vec![0.; n];
                let mut next_m = vec![0.; n];
                for i in 0..n {
                    let d = &p.neuron_dynamics()[i];
                    let x = d.bias() + tick.encoded_inputs[i] + tick.projection_gain * sums[i]
                        - d.homeostatic_gain() * m[i]
                        - tick.local_threshold_shift;
                    let activated = match d.activation().raw() {
                        0 => x,
                        1 => x.max(0.),
                        2 => x.tanh(),
                        3 => 1. / (1. + (-x).exp()),
                        _ => return Err(CpuHostError::State("unknown native activation")),
                    };
                    next[i] = (1. - d.leak()) * a[i] + d.leak() * activated;
                    next_m[i] = (d.metabolic_decay() * m[i]
                        + (1. - d.metabolic_decay()) * next[i] * next[i])
                        .clamp(0., 1.);
                    ema[i] = (d.activity_ema_decay() * ema[i]
                        + (1. - d.activity_ema_decay()) * next[i].abs())
                    .clamp(0., 1.);
                }
                a = next;
                m = next_m;
            }
            for (actual, expected) in cpu.final_activations[time]
                .iter()
                .zip(&a)
                .chain(cpu.final_metabolic_load[time].iter().zip(&m))
                .chain(cpu.final_activity_ema[time].iter().zip(&ema))
            {
                let error = (actual - expected).abs();
                maximum = maximum.max(error);
                if error > 2e-6 + 2e-5 * actual.abs().max(expected.abs()) {
                    return Err(CpuHostError::State(
                        "retained mapping differs from native fixed-slot scalar reference",
                    ));
                }
            }
        }
        Ok(
            serde_json::json!({"reference":"native-fixed-slot-and-V11-scalar-contract","gpu_executed":false,"ticks":2,"microsteps_per_tick":p.microstep_count(),"max_absolute_error":maximum,"absolute_tolerance":2e-6,"relative_tolerance":2e-5,"passed":true}),
        )
    }

    /// Returns original actor coordinate -> exact native live coordinate.
    /// Added native structural synapses use the frozen sparse path. Reject any
    /// learned or plastic added coordinate that that path cannot represent.
    pub fn canonical_to_live(&self, p: &BrainPhenotype) -> Result<Vec<usize>, CpuHostError> {
        let bad = || {
            CpuHostError::State("retained neural layout or mutable added edge is not representable by the CPU sparse path")
        };
        let t = &self.topology;
        t.validate_for_capacity(&BrainCapacityClass::n2048())?;
        let native =
            GpuLiveTopologyCheckpointV1::recompile_for_phenotype(p, t.v11_checkpoint.clone())?;
        if serde_json::to_vec(t)? != serde_json::to_vec(&native)? {
            return Err(CpuHostError::State(
                "retained topology differs from native recompile",
            ));
        }
        self.lifetime.validate()?;
        self.fast.validate()?;
        if self.lifetime.layer_raw != alife_world::persistence::GPU_BRAIN_WEIGHT_LAYER_LIFETIME
            || self.fast.layer_raw != alife_world::persistence::GPU_BRAIN_WEIGHT_LAYER_FAST
            || self.lifetime.phenotype_hash != p.phenotype_hash()
            || self.fast.phenotype_hash != p.phenotype_hash()
            || self.lifetime.synapse_count != t.total_synapse_count
            || self.fast.synapse_count != t.total_synapse_count
            || self.lifetime.active_generation != self.fast.active_generation
            || self.lifetime.active_bank != self.fast.active_bank
        {
            return Err(CpuHostError::State("retained bank identity mismatch"));
        }
        let mut live = BTreeMap::new();
        for target in 0..p.neuron_count() as usize {
            for i in t.target_offsets[target] as usize..t.target_offsets[target + 1] as usize {
                if live
                    .insert((t.source_indices[i], target as u32, t.route_indices[i]), i)
                    .is_some()
                {
                    return Err(CpuHostError::Preparation(format!(
                        "duplicate live coordinate: source {} target {target} route {}",
                        t.source_indices[i], t.route_indices[i]
                    )));
                }
            }
        }
        let upload = alife_gpu_backend::GpuPhenotypeUpload::try_from(p)
            .map_err(|e| CpuHostError::Preparation(e.to_string()))?;
        let base_recurrent = upload.source_indices.len();
        let mut map = Vec::with_capacity(p.synapses().len());
        for (canonical, s) in p.synapses().iter().enumerate() {
            let i = match s.kind() {
                CompiledSynapseKind::Recurrent => live
                    .remove(&(s.source(), s.target(), u32::from(s.route_index())))
                    .ok_or_else(bad)?,
                CompiledSynapseKind::Decoder(_) => {
                    t.recurrent_synapse_count as usize
                        + upload.canonical_to_local_synapse[canonical] as usize
                        - base_recurrent
                }
            };
            if t.genetic_weight_bits.get(i).copied() != Some(s.genetic_weight().to_bits())
                || t.alpha_bits.get(i).copied() != Some(s.alpha().to_bits())
            {
                return Err(CpuHostError::Preparation(format!("retained mapping bits differ at canonical {} live {i}: genetic {} vs {}, alpha {} vs {}",map.len(),t.genetic_weight_bits[i],s.genetic_weight().to_bits(),t.alpha_bits[i],s.alpha().to_bits())));
            }
            map.push(i);
        }
        if map
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != p.synapses().len()
        {
            return Err(bad());
        }
        let sparse = t
            .v11_checkpoint
            .sparse_spans
            .iter()
            .flat_map(|s| &s.edges)
            .map(|e| ((e.source, e.target, e.route), e.weight.to_bits()))
            .collect::<BTreeMap<_, _>>();
        if live.len() != sparse.len() {
            return Err(bad());
        }
        for (key, i) in live {
            if sparse.get(&key).copied() != Some(t.genetic_weight_bits[i])
                || f32::from_bits(t.alpha_bits[i]) != 0.
                || [
                    &self.lifetime.bank_0_bits,
                    &self.lifetime.bank_1_bits,
                    &self.fast.bank_0_bits,
                    &self.fast.bank_1_bits,
                ]
                .iter()
                .any(|bank| f32::from_bits(bank[i]) != 0.)
            {
                return Err(bad());
            }
        }
        Ok(map)
    }
}

impl CpuCanonicalHost {
    /// Install only into an undispatched fresh body. Personal memory, activation,
    /// chemistry and eligibility remain newborn state. Learned topology,
    /// predictor and active somatic weights are retained, while both original
    /// live banks remain byte-exact inheritance evidence in every checkpoint.
    pub fn inherit_retained_neural(
        &mut self,
        founder: &CpuFoundationTrainer,
        source: &CpuRetainedNeuralInheritance,
    ) -> Result<(), CpuHostError> {
        self.validate_founder(founder)?;
        let p = founder.phenotype();
        let map = source.canonical_to_live(p)?;
        let mut next = self.clone();
        if next.sampling_counter != 0 {
            return Err(CpuHostError::State(
                "retained neural inheritance requires an undispatched newborn",
            ));
        }
        for r in next.residents.values_mut() {
            let mut neural = r.neural.checkpoint()?;
            if r.next_sequence != 1
                || neural.dispatch_generation != 0
                || neural.last_decision_tick.is_some()
                || r.inherited_neural_source.is_some()
                || neural
                    .fast
                    .iter()
                    .chain(&neural.lifetime)
                    .chain(&neural.eligibility)
                    .any(|v| *v != 0.)
            {
                return Err(CpuHostError::State(
                    "retained neural target already contains learned or dispatched state",
                ));
            }
            fn bank(w: &PortableDualWeightBankV1) -> &[u32] {
                if w.active_bank == 0 {
                    &w.bank_0_bits
                } else {
                    &w.bank_1_bits
                }
            }
            neural.lifetime = map
                .iter()
                .map(|&i| f32::from_bits(bank(&source.lifetime)[i]))
                .collect();
            neural.fast = map
                .iter()
                .map(|&i| f32::from_bits(bank(&source.fast)[i]))
                .collect();
            neural.weight_generation = source.lifetime.active_generation;
            neural.neural.dendrites = source.topology.v11_checkpoint.dendritic_branches.clone();
            r.neural = CpuOrganismState::from_checkpoint(p, neural)?;
            r.v11 = GpuV11CausalState::restore(source.topology.v11_checkpoint.clone())?;
            r.predictor = source.predictor.clone();
            r.inherited_neural_source = Some(source.clone());
            r.sealed_authority = None;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }
}
