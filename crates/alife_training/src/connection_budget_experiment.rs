//! Disposable, offline connection selection. This is a replay intervention,
//! not topology admission, native continuation, synaptogenesis or a GPU path.
use crate::{
    cpu::{CpuCanonicalHost, CpuFoundationTrainer},
    AdamWConfig, StageTrainableMask, TrainingReplayEvaluation, TrainingSequence,
};
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, time::Instant};

#[cfg(test)]
mod full_mask_assay;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn identity(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn shuffle_key(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LobeBudgetCapacity {
    pub lobe: u16,
    pub name: String,
    pub neurons: u32,
    pub available_recurrent_connections: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedLobeBudget {
    pub lobe: u16,
    pub requested: usize,
    pub selected: usize,
    pub constrained_by_existing_pool: bool,
}

/// Private mask and phenotype binding prevent accidental reinterpretation of a
/// plan as a new native phenotype or application to a different founder.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConnectionBudgetPlan {
    phenotype: PhenotypeHash,
    source_weights_identity: String,
    selected: Vec<bool>,
    pub topology_identity: String,
    pub budgets: Vec<ResolvedLobeBudget>,
}

/// Forward-only compact index view; cannot update or export a native founder.
pub struct DisposableCompactReplay {
    view: crate::cpu::DetachedRecurrentReplay,
    topology_identity: String,
}

/// Only constructs a fresh public built-in founder. There is deliberately no
/// checkpoint, filesystem source, accepted founder or existing-individual input.
pub struct DisposableBudgetFounder {
    trainer: CpuFoundationTrainer,
    host: CpuCanonicalHost,
    seed: u64,
}

impl DisposableBudgetFounder {
    fn source_weights_identity(&self) -> Result<String> {
        identity(&(
            "disposable-budget-source-weights-v1",
            self.trainer.phenotype().phenotype_hash(),
            self.trainer
                .weights()
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
        ))
    }

    fn validate_plan(&self, plan: &ConnectionBudgetPlan) -> Result<()> {
        if plan.phenotype != self.trainer.phenotype().phenotype_hash()
            || plan.selected.len() != self.trainer.weights().len()
            || plan.source_weights_identity != self.source_weights_identity()?
            || plan.topology_identity
                != identity(&(
                    "disposable-connection-mask-v1",
                    plan.phenotype,
                    &plan.selected,
                ))?
        {
            return Err("plan/source phenotype, weights or mask identity mismatch".into());
        }
        Ok(())
    }

    pub fn compact_view(&self, plan: &ConnectionBudgetPlan) -> Result<DisposableCompactReplay> {
        self.validate_plan(plan)?;
        Ok(DisposableCompactReplay {
            view: self.trainer.detached_recurrent_replay(&plan.selected)?,
            topology_identity: plan.topology_identity.clone(),
        })
    }

    /// Phenotype-compatible input is evaluated counterfactually under this exact
    /// source's weights. No world ownership, native continuation or gradients are claimed.
    pub fn evaluate_compact(
        &self,
        view: &DisposableCompactReplay,
        sequence: &TrainingSequence,
    ) -> Result<TrainingReplayEvaluation> {
        sequence.validate_for(self.trainer.phenotype())?;
        if sequence
            .ticks
            .iter()
            .any(|t| !t.structural_synapses.is_empty())
        {
            return Err("unbudgeted structural edges".into());
        }
        Ok(view.view.evaluate(&self.trainer, sequence)?)
    }
    pub fn new(seed: u64) -> Result<Self> {
        if seed == 0 {
            return Err("experiment seed must be nonzero".into());
        }
        let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
        config.brain_class = BrainScaleTier::Standard2048;
        let capacity = BrainCapacityClass::n2048();
        let builtin = FoundationWeightAsset::builtin_n2048_v1(config.sensor_profile)?;
        let first = create_canonical_new_game_with_n2048_candidate(&config, &builtin)?;
        let record = first
            .world
            .organism_registry()
            .iter()
            .next()
            .ok_or("no founder")?;
        let genome = record.phenotype().brain_genome.clone();
        let development = foundation_construction_development(
            &genome,
            &capacity,
            &record.phenotype().development_state_at(Tick::ZERO)?,
        )?;
        let p =
            PhenotypeCompiler::compile(&genome, &capacity, &development, config.sensor_profile)?;
        let source = FoundationWeightAsset::from_trained_weights(
            &p,
            p.synapses().iter().map(|s| s.genetic_weight()).collect(),
            TrainingStageManifest::new(1, 1, 1),
        )?;
        let game = create_canonical_new_game_with_n2048_candidate(&config, &source)?;
        let (p, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            source.clone(),
        )?;
        // Native trainers require a nonempty mask. This wrapper exposes no
        // update method and run() verifies the entire optimizer is unchanged.
        let mask = StageTrainableMask::recurrent_only(&p)?;
        let trainer = CpuFoundationTrainer::new(p, source, mask, AdamWConfig::default())?;
        let host = CpuCanonicalHost::new_cohort(game, &trainer, (seed ^ (seed >> 32)) as u32)?;
        Ok(Self {
            trainer,
            host,
            seed,
        })
    }

    pub fn capacities(&self) -> Vec<LobeBudgetCapacity> {
        let p = self.trainer.phenotype();
        p.lobe_layout()
            .enabled_regions()
            .map(|r| LobeBudgetCapacity {
                lobe: r.kind.raw(),
                name: format!("{:?}", r.kind),
                neurons: r.len,
                available_recurrent_connections: p
                    .synapses()
                    .iter()
                    .filter(|s| {
                        s.kind() == CompiledSynapseKind::Recurrent && r.contains_neuron(s.target())
                    })
                    .count(),
            })
            .collect()
    }

    /// Exact per-lobe counts. Missing, extra, duplicate and infeasible budgets
    /// fail before selection. Scores are per compiled synapse, not teacher data.
    pub fn exact_plan(
        &self,
        budgets: &[(u16, usize)],
        scores: Option<&[f64]>,
    ) -> Result<ConnectionBudgetPlan> {
        let p = self.trainer.phenotype();
        if scores.is_some_and(|s| s.len() != p.synapses().len() || s.iter().any(|v| !v.is_finite()))
        {
            return Err("scores must be finite and match the compiled connection pool".into());
        }
        let requested: BTreeMap<_, _> = budgets.iter().copied().collect();
        let capacities = self.capacities();
        if requested.len() != budgets.len()
            || requested.len() != capacities.len()
            || capacities.iter().any(|c| {
                requested
                    .get(&c.lobe)
                    .is_none_or(|n| *n > c.available_recurrent_connections)
            })
        {
            return Err("provide exactly one feasible budget per enabled lobe".into());
        }
        let mut selected = vec![false; p.synapses().len()];
        for c in &capacities {
            let region = p
                .lobe_layout()
                .enabled_regions()
                .find(|r| r.kind.raw() == c.lobe)
                .unwrap();
            let mut ids = p
                .synapses()
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.kind() == CompiledSynapseKind::Recurrent && region.contains_neuron(s.target())
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            ids.sort_unstable_by(|&a, &b| {
                scores
                    .map_or(std::cmp::Ordering::Equal, |s| s[b].total_cmp(&s[a]))
                    .then_with(|| {
                        shuffle_key(self.seed ^ a as u64).cmp(&shuffle_key(self.seed ^ b as u64))
                    })
                    .then(a.cmp(&b))
            });
            for &i in ids.iter().take(requested[&c.lobe]) {
                selected[i] = true;
            }
        }
        let topology_identity = identity(&(
            "disposable-connection-mask-v1",
            p.phenotype_hash(),
            &selected,
        ))?;
        Ok(ConnectionBudgetPlan {
            phenotype: p.phenotype_hash(),
            source_weights_identity: self.source_weights_identity()?,
            selected,
            topology_identity,
            budgets: capacities
                .iter()
                .map(|c| ResolvedLobeBudget {
                    lobe: c.lobe,
                    requested: requested[&c.lobe],
                    selected: requested[&c.lobe],
                    constrained_by_existing_pool: false,
                })
                .collect(),
        })
    }

    /// Fan-in here means a lobe's average budget, not a per-neuron degree.
    /// Saturation is explicit in every receipt; exact_plan never caps silently.
    pub fn capped_fan_in_plan(
        &self,
        fan_in: u32,
        scores: Option<&[f64]>,
    ) -> Result<ConnectionBudgetPlan> {
        if ![0, 16, 32, 48, 64].contains(&fan_in) {
            return Err("fan-in must be 0/16/32/48/64".into());
        }
        let capacities = self.capacities();
        let budgets = capacities
            .iter()
            .map(|c| {
                (
                    c.lobe,
                    (fan_in as usize * c.neurons as usize).min(c.available_recurrent_connections),
                )
            })
            .collect::<Vec<_>>();
        let mut plan = self.exact_plan(&budgets, scores)?;
        for (b, c) in plan.budgets.iter_mut().zip(capacities) {
            b.requested = fan_in as usize * c.neurons as usize;
            b.constrained_by_existing_pool = b.requested > b.selected;
        }
        Ok(plan)
    }

    /// The selected subgraph runs through the actual native CPU replay engine.
    /// Cancellation changes only detached offsets. It does not remove iteration
    /// work, alter the founder, train a connection, or bypass topology admission.
    pub fn intervene(
        &self,
        plan: &ConnectionBudgetPlan,
        sequence: &TrainingSequence,
    ) -> Result<TrainingSequence> {
        let p = self.trainer.phenotype();
        sequence.validate_for(p)?;
        self.validate_plan(plan)?;
        if sequence
            .ticks
            .iter()
            .any(|t| !t.structural_synapses.is_empty())
        {
            return Err("plan/source identity mismatch or unbudgeted structural edges".into());
        }
        let mut result = sequence.clone();
        for tick in &mut result.ticks {
            for (i, s) in p.synapses().iter().enumerate() {
                if s.kind() == CompiledSynapseKind::Recurrent && !plan.selected[i] {
                    tick.effective_weight_offsets[i] = -self.trainer.weights()[i];
                }
            }
        }
        result.validate_for(p)?;
        Ok(result)
    }

    /// Bounded real-frame comparison of complete replay alternatives. World
    /// collection remains baseline-only; views are not training/optimizer inputs.
    pub fn run_compact(mut self, ticks: usize) -> Result<serde_json::Value> {
        if !(4..=256).contains(&ticks) {
            return Err("ticks must be 4..256".into());
        }
        let started = Instant::now();
        let before = self.trainer.checkpoint();
        let source_identity = identity(&before)?;
        let collected_at = Instant::now();
        let mut rows = Vec::new();
        let mut scores = vec![0.; self.trainer.weights().len()];
        let mut warmup = 0;
        for step in 0..ticks {
            for d in self.host.step(&self.trainer, 1.)?.decisions {
                if step < ticks / 2 {
                    warmup += 1;
                    for (i, s) in self.trainer.phenotype().synapses().iter().enumerate() {
                        scores[i] += (f64::from(d.initial.activations[s.source() as usize])
                            * f64::from(d.initial.activations[s.target() as usize])
                            * f64::from(s.genetic_weight()))
                        .abs();
                    }
                } else {
                    rows.push(TrainingSequence {
                        phenotype_hash: self.trainer.phenotype().phenotype_hash(),
                        initial: d.initial,
                        ticks: vec![d.replay_tick],
                        burn_in_ticks: 0,
                        memory_candidate_gain: self
                            .trainer
                            .phenotype()
                            .candidate_decoder()
                            .memory_channel()
                            .map_or(0., |m| m.max_candidate_gain()),
                    });
                }
            }
        }
        let collection_seconds = collected_at.elapsed().as_secs_f64();
        if warmup == 0 || rows.is_empty() {
            return Err("no awake warmup or held-out frames".into());
        }
        let baseline = rows
            .iter()
            .map(|q| self.trainer.evaluate_replay(q))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut results = Vec::new();
        for fan_in in [0, 16, 64] {
            let plan = self.capped_fan_in_plan(fan_in, Some(&scores))?;
            let view = self.compact_view(&plan)?;
            let selected = plan.selected.iter().filter(|v| **v).count();
            let mut pairs = Vec::new();
            for pair in 0..7 {
                let mut seconds = [0.; 2];
                let mut outputs = [None, None];
                for which in if pair % 2 == 0 { [0, 1] } else { [1, 0] } {
                    let at = Instant::now();
                    let output = if which == 0 {
                        rows.iter()
                            .map(|q| {
                                let cancelled = self.intervene(&plan, q)?;
                                Ok(self.trainer.evaluate_replay(&cancelled)?)
                            })
                            .collect::<Result<Vec<_>>>()?
                    } else {
                        // Include view construction/source binding in every pair.
                        let prepared = self.compact_view(&plan)?;
                        rows.iter()
                            .map(|q| self.evaluate_compact(&prepared, q))
                            .collect::<Result<Vec<_>>>()?
                    };
                    std::hint::black_box(&output);
                    seconds[which] = at.elapsed().as_secs_f64();
                    outputs[which] = Some(output);
                }
                let a = outputs[0].take().unwrap();
                let b = outputs[1].take().unwrap();
                if evaluation_bits(&a) != evaluation_bits(&b) {
                    return Err("compact/cancellation full-state bit mismatch".into());
                }
                pairs.push(serde_json::json!({"pair":pair,"first":if pair%2==0 {"cancellation"} else {"compact"},"cancellation_seconds":seconds[0],"compact_seconds":seconds[1]}));
            }
            results.push(serde_json::json!({"requested_average_fan_in":fan_in,"lobe_budgets":plan.budgets,"selected_recurrent_entries":selected,
                "view_cadence_entries_step0":selected,"topology_identity":view.topology_identity,"source_weights_identity":plan.source_weights_identity,"pairs":pairs}));
        }
        let after = rows
            .iter()
            .map(|q| self.trainer.evaluate_replay(q))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if identity(&self.trainer.checkpoint())? != source_identity
            || evaluation_bits(&baseline) != evaluation_bits(&after)
        {
            return Err("retained baseline changed".into());
        }
        Ok(
            serde_json::json!({"experiment":"disposable-bound-compact-forward-replay","backend":"native-cpu","seed":self.seed,"world_ticks":ticks,"warmup_decisions":warmup,"held_out_replay_rows":rows.len(),
            "source_checkpoint_identity":source_identity,"source_weights_identity":self.source_weights_identity()?,"frame_identity":identity(&rows)?,"baseline_retained":true,"optimizer_updates":0,
            "collection_seconds":collection_seconds,"experiment_total_seconds":started.elapsed().as_secs_f64(),"trials":results,
            "limits":["real canonical baseline frames; detached counterfactual evaluation, not experimental world rollout or capability",
                "source phenotype and exact weights are bound; neither checkpoint hash nor phenotype proves world ownership",
                "paired scopes include all replay preparation: cancellation validation/hash/clone versus compact graph construction/binding plus forward",
                "selection, collection and controls are outside pair scopes but inside experiment total; source construction is inside CLI total",
                "warm pairs alternate order; first output retained during second call; output drops outside pair timing",
                "no training, gradients, optimizer changes, accepted checkpoint, topology migration or GPU"]}),
        )
    }

    /// Collect actual canonical-world frames; rank on the first half, evaluate
    /// interventions only on the later half. All world actions remain baseline.
    pub fn run(mut self, ticks: usize) -> Result<serde_json::Value> {
        if !(4..=256).contains(&ticks) {
            return Err("ticks must be 4..256".into());
        }
        let before = self.trainer.checkpoint();
        let source_identity = identity(&before)?;
        let collection_start = Instant::now();
        let mut scores = vec![0_f64; self.trainer.phenotype().synapses().len()];
        let mut rows = Vec::new();
        let mut warmup_rows = 0;
        for step in 0..ticks {
            let collected = self.host.step(&self.trainer, 1.)?;
            for d in collected.decisions {
                if step < ticks / 2 {
                    warmup_rows += 1;
                    for (i, s) in self.trainer.phenotype().synapses().iter().enumerate() {
                        scores[i] += (f64::from(d.initial.activations[s.source() as usize])
                            * f64::from(d.initial.activations[s.target() as usize])
                            * f64::from(s.genetic_weight()))
                        .abs();
                    }
                } else {
                    rows.push(TrainingSequence {
                        phenotype_hash: self.trainer.phenotype().phenotype_hash(),
                        initial: d.initial,
                        ticks: vec![d.replay_tick],
                        burn_in_ticks: 0,
                        memory_candidate_gain: self
                            .trainer
                            .phenotype()
                            .candidate_decoder()
                            .memory_channel()
                            .map_or(0., |m| m.max_candidate_gain()),
                    });
                }
            }
        }
        let collection_seconds = collection_start.elapsed().as_secs_f64();
        if warmup_rows == 0 || rows.is_empty() {
            return Err("no awake decisions in warmup or held-out collection".into());
        }
        let frame_identity = identity(&rows)?;
        let started = Instant::now();
        let baseline = rows
            .iter()
            .map(|q| self.trainer.evaluate_replay(q))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let baseline_seconds = started.elapsed().as_secs_f64();
        let baseline_logits = baseline
            .iter()
            .map(|e| &e.candidate_logits)
            .collect::<Vec<_>>();
        let baseline_identity = identity(&baseline_logits)?;
        let mut trials = Vec::new();
        for fan_in in [0, 16, 32, 48, 64] {
            for (mode, signal) in [
                ("seeded-control", None),
                ("past-coactivity", Some(scores.as_slice())),
            ] {
                let plan = self.capped_fan_in_plan(fan_in, signal)?;
                let started = Instant::now();
                let mut max_logit_delta = 0_f32;
                let mut changed_rows = 0;
                let mut state_hasher = Sha256::new();
                for (q, control) in rows.iter().zip(&baseline) {
                    let intervention = self.intervene(&plan, q)?;
                    state_hasher.update(serde_json::to_vec(&intervention)?);
                    let result = self.trainer.evaluate_replay(&intervention)?;
                    let delta = logit_delta(control, &result);
                    max_logit_delta = max_logit_delta.max(delta);
                    changed_rows += usize::from(delta > 0.);
                }
                trials.push(serde_json::json!({"requested_average_fan_in":fan_in, "selection":mode,
                    "topology_identity":plan.topology_identity, "effective_state_identity":format!("{:x}",state_hasher.finalize()),
                    "lobe_budgets":plan.budgets, "changed_replay_rows":changed_rows,
                    "max_candidate_logit_delta":max_logit_delta, "intervention_and_replay_seconds":started.elapsed().as_secs_f64()}));
            }
        }
        let baseline_after = rows
            .iter()
            .map(|q| self.trainer.evaluate_replay(q))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if self.trainer.checkpoint() != before || baseline_after != baseline {
            return Err("experiment changed retained baseline".into());
        }
        Ok(
            serde_json::json!({"schema_version":1,"experiment":"disposable-public-founder-connection-budget-replay",
            "backend":"native-cpu", "hardware":self.trainer.hardware_receipt(), "seed":self.seed,
            "world_ticks":ticks,"warmup_decisions":warmup_rows,"held_out_replay_rows":rows.len(),
            "source_state_identity":source_identity,"baseline_frame_identity":frame_identity,"baseline_logits_identity":baseline_identity,
            "selection_signal_identity":identity(&scores)?,
            "baseline_retained":true,"optimizer_updates":0,"world_policy":"baseline-only",
            "collection_seconds":collection_seconds,"baseline_replay_seconds":baseline_seconds,
            "capacity":self.capacities(),"trials":trials,
            "limits":["selection and reactivation only within the compiled recurrent pool; no new coordinates",
                "fan-in is an average per lobe; infeasible requests are explicitly capped",
                "route gating and cadence still apply; selected counts are capacity, not every-step firing counts",
                "detached replay from baseline states; no experimental world rollout or learning claim",
                "zeroed connections are still scanned; single-pass timings include intervention cloning/hashing and do not establish sparse throughput gains",
                "no checkpoint export, accepted-brain migration or GPU execution"]}),
        )
    }
}

fn evaluation_bits(values: &[TrainingReplayEvaluation]) -> Vec<Vec<u32>> {
    values
        .iter()
        .flat_map(|v| {
            [
                &v.candidate_logits,
                &v.final_activations,
                &v.final_activity_ema,
                &v.final_metabolic_load,
            ]
        })
        .flat_map(|rows| rows.iter().map(|r| r.iter().map(|v| v.to_bits()).collect()))
        .collect()
}

fn logit_delta(a: &TrainingReplayEvaluation, b: &TrainingReplayEvaluation) -> f32 {
    a.candidate_logits
        .iter()
        .flatten()
        .zip(b.candidate_logits.iter().flatten())
        .map(|(a, b)| (a - b).abs())
        .fold(0., f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_budgets(e: &DisposableBudgetFounder) -> Vec<(u16, usize)> {
        e.capacities()
            .iter()
            .map(|c| (c.lobe, c.available_recurrent_connections.min(16)))
            .collect()
    }

    fn real_row(e: &mut DisposableBudgetFounder) -> TrainingSequence {
        let d = (0..8)
            .find_map(|_| {
                e.host
                    .step(&e.trainer, 1.)
                    .unwrap()
                    .decisions
                    .into_iter()
                    .next()
            })
            .expect("awake canonical decision");
        TrainingSequence {
            phenotype_hash: e.trainer.phenotype().phenotype_hash(),
            initial: d.initial,
            ticks: vec![d.replay_tick],
            burn_in_ticks: 0,
            memory_candidate_gain: e
                .trainer
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .map_or(0., |m| m.max_candidate_gain()),
        }
    }

    #[test]
    fn compact_real_frames_match_zero_full_and_sixteen_cancellation_bits() {
        for seed in [70501, 70502] {
            let mut e = DisposableBudgetFounder::new(seed).unwrap();
            let before = e.trainer.checkpoint();
            let rows = (0..4).map(|_| real_row(&mut e)).collect::<Vec<_>>();
            let original = rows.clone();
            let baseline = rows
                .iter()
                .map(|q| e.trainer.evaluate_replay(q).unwrap())
                .collect::<Vec<_>>();
            let mut effects = Vec::new();
            for fan_in in [0, 16, 64] {
                let plan = e.capped_fan_in_plan(fan_in, None).unwrap();
                let view = e.compact_view(&plan).unwrap();
                let a = rows
                    .iter()
                    .map(|q| {
                        e.trainer
                            .evaluate_replay(&e.intervene(&plan, q).unwrap())
                            .unwrap()
                    })
                    .collect::<Vec<_>>();
                let b = rows
                    .iter()
                    .map(|q| e.evaluate_compact(&view, q).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(evaluation_bits(&a), evaluation_bits(&b));
                effects.push(evaluation_bits(&b));
                if fan_in == 64 {
                    assert_eq!(evaluation_bits(&baseline), evaluation_bits(&b));
                }
            }
            assert_ne!(
                effects[0], effects[2],
                "falsifier: all-zero recurrent view must change this fixture"
            );
            assert_eq!(rows, original);
            assert_eq!(before, e.trainer.checkpoint());
            assert_eq!(
                evaluation_bits(&baseline),
                evaluation_bits(
                    &rows
                        .iter()
                        .map(|q| e.trainer.evaluate_replay(q).unwrap())
                        .collect::<Vec<_>>()
                )
            );
        }
    }

    #[test]
    fn compact_and_cancellation_reject_changed_weights_at_same_phenotype() {
        let mut e = DisposableBudgetFounder::new(70503).unwrap();
        let q = real_row(&mut e);
        let plan = e.capped_fan_in_plan(16, None).unwrap();
        let view = e.compact_view(&plan).unwrap();
        let phenotype = e.trainer.phenotype().phenotype_hash();
        let mut foreign = e.trainer.checkpoint();
        foreign.weights[17] += 0.001;
        e.trainer.restore_checkpoint(foreign).unwrap();
        assert_eq!(phenotype, e.trainer.phenotype().phenotype_hash());
        assert!(e.intervene(&plan, &q).is_err());
        assert!(e.compact_view(&plan).is_err());
        assert!(e.evaluate_compact(&view, &q).is_err());
    }

    #[test]
    fn deterministic_exact_budgets_and_adaptive_reselection() {
        let e = DisposableBudgetFounder::new(70401).unwrap();
        let budgets = small_budgets(&e);
        let a = e.exact_plan(&budgets, None).unwrap();
        assert_eq!(a, e.exact_plan(&budgets, None).unwrap());
        let p = e.trainer.phenotype();
        for b in &a.budgets {
            let region = p
                .lobe_layout()
                .enabled_regions()
                .find(|r| r.kind.raw() == b.lobe)
                .unwrap();
            assert_eq!(
                a.selected
                    .iter()
                    .enumerate()
                    .filter(|(i, selected)| **selected
                        && region.contains_neuron(p.synapses()[*i].target()))
                    .count(),
                b.selected
            );
        }
        // Prefer currently absent pool entries: this must exchange connections,
        // while keeping every lobe's exact connection count.
        let scores = a
            .selected
            .iter()
            .map(|selected| if *selected { 0. } else { 1. })
            .collect::<Vec<_>>();
        let adaptive = e.exact_plan(&budgets, Some(&scores)).unwrap();
        assert_eq!(a.budgets, adaptive.budgets);
        assert_ne!(a.topology_identity, adaptive.topology_identity);
        assert!(a
            .selected
            .iter()
            .zip(&adaptive.selected)
            .any(|(a, b)| !a && *b));
        assert_eq!(adaptive, e.exact_plan(&budgets, Some(&scores)).unwrap());
    }

    #[test]
    fn all_requested_fan_ins_resolve_exactly_and_report_constraints() {
        let e = DisposableBudgetFounder::new(70402).unwrap();
        for fan_in in [0, 16, 32, 48, 64] {
            let plan = e.capped_fan_in_plan(fan_in, None).unwrap();
            for (b, capacity) in plan.budgets.iter().zip(e.capacities()) {
                assert_eq!(b.requested, fan_in as usize * capacity.neurons as usize);
                assert_eq!(
                    b.selected,
                    b.requested.min(capacity.available_recurrent_connections)
                );
                assert_eq!(b.constrained_by_existing_pool, b.requested > b.selected);
            }
            assert_eq!(
                plan.selected.iter().filter(|v| **v).count(),
                plan.budgets.iter().map(|b| b.selected).sum::<usize>()
            );
        }
    }

    #[test]
    fn intervention_preserves_baseline_and_decoder_state() {
        let mut e = DisposableBudgetFounder::new(70403).unwrap();
        let q = real_row(&mut e);
        let source = q.clone();
        let checkpoint = e.trainer.checkpoint();
        let control = e.trainer.evaluate_replay(&q).unwrap();
        let plan = e.capped_fan_in_plan(0, None).unwrap();
        let changed = e.intervene(&plan, &q).unwrap();
        assert_eq!(q, source);
        assert_eq!(q.initial, changed.initial);
        for (i, s) in e.trainer.phenotype().synapses().iter().enumerate() {
            if s.kind() == CompiledSynapseKind::Recurrent {
                assert_eq!(
                    e.trainer.weights()[i] + changed.ticks[0].effective_weight_offsets[i],
                    0.
                );
            } else {
                assert_eq!(
                    q.ticks[0].effective_weight_offsets[i].to_bits(),
                    changed.ticks[0].effective_weight_offsets[i].to_bits()
                );
            }
        }
        let changed_evaluation = e.trainer.evaluate_replay(&changed).unwrap();
        assert_ne!(
            control.final_activations,
            changed_evaluation.final_activations
        );
        assert_eq!(e.trainer.checkpoint(), checkpoint);
        assert_eq!(e.trainer.evaluate_replay(&q).unwrap(), control);
        assert_ne!(identity(&q).unwrap(), identity(&changed).unwrap());
        // Selecting the entire pool retains all effective acquired offsets.
        let full = e
            .exact_plan(
                &e.capacities()
                    .iter()
                    .map(|c| (c.lobe, c.available_recurrent_connections))
                    .collect::<Vec<_>>(),
                None,
            )
            .unwrap();
        assert_eq!(e.intervene(&full, &q).unwrap(), q);
    }

    #[test]
    fn invalid_budgets_scores_and_input_are_rejected() {
        assert!(DisposableBudgetFounder::new(0).is_err());
        let e = DisposableBudgetFounder::new(70404).unwrap();
        let mut budgets = small_budgets(&e);
        assert!(e.exact_plan(&[], None).is_err());
        budgets.push(budgets[0]);
        assert!(e.exact_plan(&budgets, None).is_err());
        budgets.pop();
        budgets[0].1 = usize::MAX;
        assert!(e.exact_plan(&budgets, None).is_err());
        let budgets = small_budgets(&e);
        assert!(e.exact_plan(&budgets, Some(&[])).is_err());
        let scores = vec![f64::NAN; e.trainer.weights().len()];
        assert!(e.exact_plan(&budgets, Some(&scores)).is_err());
        assert!(e.capped_fan_in_plan(65, None).is_err());
        assert!(e.run(3).is_err());
    }

    #[test]
    fn identity_isolates_founders_and_unbudgeted_structural_edges() {
        let a = DisposableBudgetFounder::new(70405).unwrap();
        let mut b = DisposableBudgetFounder::new(70406).unwrap();
        let mut q = real_row(&mut b);
        let plan = a.capped_fan_in_plan(16, None).unwrap();
        assert!(b.intervene(&plan, &q).is_err());
        assert_ne!(
            a.trainer.checkpoint().phenotype_hash,
            b.trainer.checkpoint().phenotype_hash
        );
        let plan = b.capped_fan_in_plan(16, None).unwrap();
        q.ticks[0]
            .structural_synapses
            .push(crate::TrainingFrozenSynapse {
                source: 0,
                target: 0,
                route: 0,
                cadence: 0,
                effective_weight: 0.1,
            });
        assert!(b.intervene(&plan, &q).is_err());
    }

    #[test]
    fn real_world_trial_retains_controls_and_repeats_identities() {
        let mut first = DisposableBudgetFounder::new(70407).unwrap().run(4).unwrap();
        let mut second = DisposableBudgetFounder::new(70407).unwrap().run(4).unwrap();
        for receipt in [&mut first, &mut second] {
            assert_eq!(receipt["baseline_retained"], true);
            assert_eq!(receipt["optimizer_updates"], 0);
            let object = receipt.as_object_mut().unwrap();
            object.remove("collection_seconds");
            object.remove("baseline_replay_seconds");
            for trial in object.get_mut("trials").unwrap().as_array_mut().unwrap() {
                trial
                    .as_object_mut()
                    .unwrap()
                    .remove("intervention_and_replay_seconds");
            }
        }
        assert_eq!(first, second);
    }
}
