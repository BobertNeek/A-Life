//! Experimental action-objective budgets; neural execution and biology unchanged.
use super::*;

fn class(e: &Episode, i: usize) -> usize {
    if i == 0 {
        0
    } else if e.speech[i].is_some() {
        1
    } else {
        2
    }
}
pub(super) fn gradients(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
) -> Result<(Vec<f32>, Value)> {
    let mut counts = [0usize; 3];
    for e in episodes {
        for i in 0..e.rows.len() {
            counts[class(e, i)] += 1;
        }
    }
    let budgets = [0.8f32, 0.1, 0.1];
    let mut total = vec![0.; actor.weights().len()];
    let mut losses = [0f64; 3];
    let mut per_class = [
        vec![0f32; actor.weights().len()],
        vec![0.; actor.weights().len()],
        vec![0.; actor.weights().len()],
    ];
    for e in episodes {
        let q = sequence(actor, e, e.rows.len(), 0);
        let eval = actor.evaluate_replay(&q)?;
        let mut adjoints = vec![];
        for (i, row) in e.rows.iter().enumerate() {
            let k = class(e, i);
            let target = ImitationExample {
                candidate_count: row.action.candidate_count,
                representative_mask: row.action.representative_mask,
                motor_masks: row.action.motor_masks,
                forced_slots: row.action.forced_slots.clone(),
                target: e.labels[i].clone(),
            };
            let o = cpu_imitation_objective(&eval.candidate_logits[i], &target, 1., 1.)?;
            let scale = budgets[k] / counts[k] as f32;
            losses[k] += f64::from(o.loss * scale);
            adjoints.push(
                o.logit_gradients
                    .into_iter()
                    .map(|g| g * scale)
                    .collect::<Vec<_>>(),
            );
        }
        // Separate gradients are retained for diagnosing conflicts; their sum is
        // exactly the same linear native action backward objective.
        for (k, dst) in per_class.iter_mut().enumerate() {
            if !(0..e.rows.len()).any(|i| class(e, i) == k) {
                continue;
            }
            let a = adjoints
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    if class(e, i) == k {
                        a.clone()
                    } else {
                        vec![0.; a.len()]
                    }
                })
                .collect::<Vec<_>>();
            let p = actor.probe_replay_gradients(&q, &a)?;
            for (dst, src) in dst.iter_mut().zip(p.gradients) {
                *dst += src;
            }
        }
    }
    for g in &per_class {
        for (dst, src) in total.iter_mut().zip(g) {
            *dst += src;
        }
    }
    let norm = |g: &[f32]| g.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    let dot = |a: &[f32], b: &[f32]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| f64::from(*a) * f64::from(*b))
            .sum::<f64>()
    };
    Ok((
        total.clone(),
        json!({"row_counts_first_request_narration_other":counts,"budgets":budgets,"weighted_action_losses":losses,"gradient_norms":per_class.iter().map(|g|norm(g)).collect::<Vec<_>>(),"request_dot_total":dot(&per_class[0],&total),"request_dot_narration":dot(&per_class[0],&per_class[1]),"request_dot_other":dot(&per_class[0],&per_class[2]),"first_request_labels_balanced_across_three_real_forks":true,"no_speech_receives_same_offline_intention_labels":true}),
    ))
}
pub(super) fn held_out_place(world: &mut HeadlessWorld, s: Scene, layout: usize) -> Result<()> {
    let b = world
        .entity(world.organism_entity_ids()[0].1)
        .ok_or("no body")?
        .clone();
    let side = if layout == 0 { -1. } else { 1. };
    let d = if layout == 0 { 0.41 } else { 0.73 };
    for (id, bearing, distance) in [(s.food, side * 0.34, d), (s.toy, -side * 0.76, d + 0.03)] {
        if world.entity(id).is_some_and(|e| e.carried_by.is_some()) {
            return Err("cannot move held fixture".into());
        }
        world.editor_move_object(
            id,
            Vec3f::new(
                b.position.x + distance * (b.body_yaw + bearing).cos(),
                b.position.y,
                b.position.z + distance * (b.body_yaw + bearing).sin(),
            ),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_first_actions_keep_native_labels_and_masked_adam_history() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let mut actor = new_actor_profile(seed, &asset, true).unwrap();
        let config = LanguagePilotConfig {
            seed,
            epochs: 1,
            rounds: 1,
            decisions: 24,
            llamacpp_translation: false,
            experimental_word_bridge: true,
        };
        let (episodes, _) = collect_factorial(
            &actor,
            &asset,
            config,
            Arm::Grounded,
            0,
            &mut std::io::sink(),
            true,
        )
        .unwrap();
        assert_eq!(episodes.len(), 6);
        for e in episodes.as_chunks::<3>().0 {
            let masks = e
                .iter()
                .map(|e| e.labels[0].representative.unwrap())
                .collect::<Vec<_>>();
            assert_ne!(masks[0], masks[1]);
            assert_ne!(masks[1], masks[2]);
            assert_eq!(e[0].rows[0].initial, e[1].rows[0].initial);
            assert_eq!(e[1].rows[0].initial, e[2].rows[0].initial);
        }
        let (g, r) = gradients(&actor, &episodes).unwrap();
        assert_eq!(r["row_counts_first_request_narration_other"][0], 6);
        assert!(g.iter().all(|x| x.is_finite()));
        assert!(g.iter().any(|x| *x != 0.));
        let before = actor.checkpoint();
        actor.apply_gradients(&g).unwrap();
        preserve_frozen(&before, &actor.checkpoint()).unwrap();
        assert_eq!(actor.optimizer_step(), before.optimizer_step + 1);
        let e = &episodes[0];
        let q = sequence(&actor, e, 1, 0);
        let tape = actor.replay_activation_tape(&q).unwrap();
        let eval = actor.evaluate_replay(&q).unwrap();
        assert_eq!(tape.last().unwrap(), &eval.final_activations[0]);
    }
}
