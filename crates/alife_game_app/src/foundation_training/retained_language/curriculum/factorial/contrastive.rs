//! Offline paired command ranking through the native recurrent/action graph.
//! All rows are actual first decisions from matched real-world forks.
use super::*;
fn single(actor: &CpuFoundationTrainer, e: &Episode) -> TrainingSequence {
    sequence(actor, e, 1, 0)
}
fn requested(e: &Episode) -> Result<usize> {
    let mask = e.labels[0]
        .representative
        .ok_or("paired request needs a legal label")?;
    if mask.count_ones() != 1 {
        return Err("paired request label must be one native candidate".into());
    }
    Ok(mask.trailing_zeros() as usize)
}
fn softplus(x: f32) -> f32 {
    x.max(0.) + (-x.abs()).exp().ln_1p()
}
pub(super) fn gradients(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
) -> Result<(Vec<f32>, Value)> {
    if episodes.is_empty() || !episodes.len().is_multiple_of(3) {
        return Err("paired commands need complete triples".into());
    }
    let mut g = vec![0f32; actor.weights().len()];
    let mut records = vec![];
    let mut loss = 0f64;
    let scale = 1. / episodes.len() as f32; // three pairs per real-world triple
    for triple in episodes.chunks_exact(3) {
        let qs = triple.iter().map(|e| single(actor, e)).collect::<Vec<_>>();
        let evals = qs
            .iter()
            .map(|q| actor.evaluate_replay(q))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            if qs[a].initial != qs[b].initial
                || qs[a].ticks[0].candidates != qs[b].ticks[0].candidates
            {
                return Err("command pairs lost matched native boundary/candidates".into());
            }
            let ia = requested(&triple[a])?;
            let ib = requested(&triple[b])?;
            if ia == ib {
                return Err("command pair has indistinguishable native labels".into());
            }
            let la = &evals[a].candidate_logits[0];
            let lb = &evals[b].candidate_logits[0];
            let margin = (la[ia] - la[ib]) - (lb[ia] - lb[ib]);
            let error = 1. - margin;
            let d = -(1. / (1. + (-error).exp())) * scale;
            loss += f64::from(softplus(error) * scale);
            let mut adj = vec![0.; la.len()];
            adj[ia] = d;
            adj[ib] = -d;
            let ga = actor.probe_replay_gradients(&qs[a], &[adj.clone()])?;
            for v in &mut adj {
                *v = -*v;
            }
            let gb = actor.probe_replay_gradients(&qs[b], &[adj])?;
            // Pair before accumulation: identical no-speech inputs produce exact
            // opposite adjoints and zero extra gradient, including shared bias.
            for (dst, (a, b)) in g.iter_mut().zip(ga.gradients.into_iter().zip(gb.gradients)) {
                *dst += a + b;
            }
            records.push(json!({"command_pair":[a,b],"native_requested_candidates":[ia,ib],"paired_logit_margin":margin,"target_margin":1.,"world_initial_sha256":identity(&qs[a].initial)?}));
        }
    }
    let gradient_l2 = g.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>().sqrt();
    Ok((
        g,
        json!({"native_pair_gradient_l2":gradient_l2,"mean_paired_softplus_loss":loss,"pairs":records,"cue_independent_preferences_cancel_algebraically":true,"coefficient":2.}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_native_gradient_matches_finite_difference_and_silent_null() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let config = LanguagePilotConfig {
            seed,
            epochs: 1,
            rounds: 1,
            decisions: 4,
            llamacpp_translation: false,
            experimental_word_bridge: true,
        };
        let (heard, _) = collect_factorial(
            &actor,
            &asset,
            config,
            Arm::Grounded,
            0,
            &mut std::io::sink(),
            true,
        )
        .unwrap();
        let (g, _) = gradients(&actor, &heard).unwrap();
        let id = g
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
            .unwrap()
            .0;
        assert!(g[id].abs() > 1e-5);
        let mut losses = vec![];
        let epsilon = 0.001;
        for d in [-epsilon, epsilon] {
            let mut cp = actor.checkpoint();
            cp.weights[id] += d;
            let changed =
                CpuFoundationTrainer::from_checkpoint(actor.phenotype().clone(), asset.clone(), cp)
                    .unwrap();
            losses.push(
                gradients(&changed, &heard).unwrap().1["mean_paired_softplus_loss"]
                    .as_f64()
                    .unwrap(),
            );
        }
        let numeric = (losses[1] - losses[0]) / (2. * f64::from(epsilon));
        assert!(
            (numeric - f64::from(g[id])).abs() < 0.003,
            "analytic {} numeric {numeric}",
            g[id]
        );
        let (silent, _) = collect_factorial(
            &actor,
            &asset,
            config,
            Arm::NoSpeech,
            0,
            &mut std::io::sink(),
            true,
        )
        .unwrap();
        let (g, receipt) = gradients(&actor, &silent).unwrap();
        assert!(
            g.iter().all(|x| *x == 0.),
            "identical inputs must have zero paired-language gradient"
        );
        assert!(receipt["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["paired_logit_margin"] == 0.));
    }
}
