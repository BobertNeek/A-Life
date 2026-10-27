//! Runnable CPU evaluation, not a trained GPU policy qualification. Preparation
//! executes scripted world actions; only the measured window uses native choices.
use super::*;
use serde_json::json;

fn food(world: &mut HeadlessWorld, label: &str, distance: f32) -> WorldEntityId {
    let body = world.entity(world.organism_entity_ids()[0].1).unwrap();
    let position = Vec3f::new(
        body.position.x + distance * body.body_yaw.cos(),
        body.position.y,
        body.position.z + distance * body.body_yaw.sin(),
    );
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: label.into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition: 1.0,
            hazard_pain: 0.0,
            radius: 0.2,
            token_id: None,
        })
        .unwrap()
}

fn scripted(world: &mut HeadlessWorld, action: ActionId, target: Option<WorldEntityId>) -> bool {
    let (id, body) = world.organism_entity_ids()[0];
    let homeostasis = world
        .organism_registry()
        .get(id)
        .unwrap()
        .biochemistry()
        .homeostasis;
    let frame = world
        .perception_frame(
            id,
            world.tick(),
            SensorProfile::GroundedTerrainVisionV1,
            homeostasis,
        )
        .unwrap();
    let command = frame
        .candidates()
        .iter()
        .find(|candidate| candidate.action_id == action && candidate.target.entity == target)
        .unwrap_or_else(|| panic!("setup lacks legal {action:?} {target:?}"))
        .to_command(id, Confidence::new(1.0).unwrap())
        .unwrap();
    let receipt = world
        .apply_registered_command(&command, body, Tick(world.tick().raw() + 1))
        .unwrap();
    world.try_advance_tick().unwrap();
    receipt.action_result.execution.physical.contact == PhysicalContactKind::Consumed
}

#[test]
fn native_need_switching_reports_matched_biology_and_movement_sensory_controls() {
    let (game, trainer, _) =
        new_game_with_profile_fixture(None, SensorProfile::GroundedTerrainVisionV1);
    let founder_before = trainer.checkpoint();
    let mut first_private = None;
    let mut matched = std::collections::BTreeMap::new();
    let mut needs = std::collections::BTreeMap::new();
    for (condition, fed, rested) in [
        ("hungry_tired", false, false),
        ("fed_tired", true, false),
        ("fed_rested", true, true),
    ] {
        for (layout, distance) in [("reachable", 0.5), ("distant", 4.0), ("rearward", -4.0)] {
            let (mut host, target) =
                CpuCanonicalHost::new_prepared_cohort(game.clone(), &trainer, 42, |world| {
                    let (id, body) = world.organism_entity_ids()[0];
                    // Matched empty scene: no teacher, obstacle or competing target.
                    world.configure_ecology(EcologyState::default()).unwrap();
                    for object in world.object_snapshots() {
                        if object.id != body {
                            world
                                .editor_remove_object(object.id)
                                .expect("remove fixture scenery");
                        }
                    }
                    for _ in 0..7200 {
                        if world
                            .organism_registry()
                            .get(id)
                            .unwrap()
                            .biochemistry()
                            .homeostasis
                            .drives
                            .fatigue
                            >= 0.28
                        {
                            break;
                        }
                        world.try_advance_tick().expect("ordinary fixture aging");
                    }
                    let aged = world
                        .organism_registry()
                        .get(id)
                        .unwrap()
                        .biochemistry()
                        .homeostasis;
                    assert!(
                        aged.drives.fatigue >= 0.28,
                        "ordinary preparation reached {aged:?}"
                    );
                    let mut setup_meals = 0;
                    for meal_index in 0..3 {
                        let meal = food(world, &format!("setup-meal-{meal_index}"), 0.5);
                        for _ in 0..2 {
                            setup_meals += usize::from(scripted(
                                world,
                                if fed {
                                    HeadlessActionIds::EAT
                                } else {
                                    HeadlessActionIds::LOOK_CENTER
                                },
                                fed.then_some(meal),
                            ));
                        }
                        world.editor_remove_object(meal).expect("remove setup meal");
                    }
                    assert_eq!(setup_meals, if fed { 3 } else { 0 });
                    // Equal elapsed time and stationary pose; the full physiological
                    // differences are measured, not called a single-drive ablation.
                    for _ in 0..32 {
                        scripted(
                            world,
                            if rested {
                                ActionKind::Rest.canonical_id()
                            } else {
                                HeadlessActionIds::LOOK_CENTER
                            },
                            None,
                        );
                    }
                    scripted(world, HeadlessActionIds::LOOK_CENTER, None);
                    Ok::<_, ScaffoldContractError>(food(world, "probe-food", distance))
                })
                .unwrap();
            let before = host.checkpoint(&trainer).unwrap();
            let private = serde_json::to_vec(&before.residents).unwrap();
            if let Some(expected) = &first_private {
                assert_eq!(&private, expected);
            } else {
                first_private = Some(private);
            }
            let id = host.world().organism_entity_ids()[0].0;
            let body = host
                .world()
                .entity(host.world().organism_entity_ids()[0].1)
                .unwrap()
                .clone();
            let biology = *host
                .world()
                .organism_registry()
                .get(id)
                .unwrap()
                .biochemistry();
            assert!(!biology.body.sleeping);
            assert!(biology.body.health >= 0.95);
            needs.insert(condition, biology.homeostasis);
            let mut perception_world = host.world().clone();
            let frame = perception_world
                .perception_frame(
                    id,
                    host.world().tick(),
                    trainer.phenotype().sensor_profile(),
                    biology.homeostasis,
                )
                .unwrap();
            let target_available = frame
                .candidates()
                .iter()
                .any(|c| c.target.entity == Some(target));
            let eat_available = frame
                .candidates()
                .iter()
                .any(|c| c.action_id == HeadlessActionIds::EAT && c.target.entity == Some(target));
            assert_eq!(target_available, layout != "rearward");
            // A distant visible target may be legal to attempt to eat. Check
            // physical access through real execution on a separate world copy.
            let mut motor_control = host.world().clone();
            let direct_consumption = eat_available
                && (0..2)
                    .any(|_| scripted(&mut motor_control, HeadlessActionIds::EAT, Some(target)));
            assert_eq!(direct_consumption, layout == "reachable");
            let controls = (
                host.world().tick(),
                body.position,
                body.body_yaw,
                frame.candidates().to_vec(),
            );
            if let Some(expected) = matched.get(layout) {
                assert_eq!(&controls, expected);
            } else {
                matched.insert(layout, controls);
            }
            let mut repeat = CpuCanonicalHost::from_checkpoint(before, &trainer).unwrap();
            let mut actions = std::collections::BTreeMap::<String, usize>::new();
            let mut meals = 0;
            let mut rest = 0;
            let mut decisions = 0;
            let mut first_logits = Vec::new();
            for _ in 0..8 {
                let step = host.step(&trainer, 1.0).unwrap();
                let repeated = repeat.step(&trainer, 1.0).unwrap();
                assert!(
                    serde_json::to_vec(&step.decisions).unwrap()
                        == serde_json::to_vec(&repeated.decisions).unwrap(),
                    "checkpoint restore changed native decision evidence"
                );
                for row in step.decisions {
                    if decisions == 0 {
                        let replay = trainer
                            .evaluate_replay(&alife_training::TrainingSequence {
                                phenotype_hash: trainer.phenotype().phenotype_hash(),
                                initial: row.initial.clone(),
                                ticks: vec![row.replay_tick.clone()],
                                burn_in_ticks: 0,
                                memory_candidate_gain: trainer
                                    .phenotype()
                                    .candidate_decoder()
                                    .memory_channel()
                                    .map_or(0.0, |m| m.max_candidate_gain()),
                            })
                            .unwrap();
                        assert_eq!(replay.final_activations[0], row.final_features);
                        assert_eq!(
                            replay.candidate_logits[0].len(),
                            row.patch.pre_action().perception().candidates().len()
                        );
                        first_logits = row.patch.pre_action().perception().candidates().iter().zip(&replay.candidate_logits[0])
                            .map(|(candidate, logit)| json!({"action":candidate.kind,"target":candidate.target.entity,"logit":logit})).collect();
                    }
                    decisions += 1;
                    row.patch.validate_contract().unwrap();
                    assert_eq!(
                        row.patch.decision().policy_backend(),
                        PolicyBackend::NeuralClosedLoopCpu
                    );
                    assert_eq!(row.organism_id, id);
                    *actions
                        .entry(format!(
                            "{:?}",
                            row.patch.pre_action().perception().candidates()
                                [row.action.representative as usize]
                                .kind
                        ))
                        .or_default() += 1;
                    meals += usize::from(
                        row.patch.outcome().physical.contact == PhysicalContactKind::Consumed,
                    );
                    rest += usize::from(row.patch.selected_bundle().is_some_and(|b| {
                        b.channels
                            .iter()
                            .any(|c| c.primitive == ActionKind::Rest.canonical_id())
                    }));
                }
            }
            assert!(
                decisions > 0,
                "comparison window executed no native decisions"
            );
            assert_eq!(
                serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
                serde_json::to_vec(&repeat.checkpoint(&trainer).unwrap()).unwrap()
            );
            let after = host
                .world()
                .organism_registry()
                .get(id)
                .unwrap()
                .biochemistry();
            println!(
                "NEED_SWITCHING {}",
                json!({"backend":"native-cpu","policy":"procedural-untrained-fixture","condition":condition,"layout":layout,"initial_homeostasis":biology.homeostasis,"initial_energy":biology.body.energy,"decisions":decisions,"representative_actions":actions,"first_candidate_logits":first_logits,"rest_bundles":rest,"consumptions":meals,"final_homeostasis":after.homeostasis,"target_available":target_available,"eat_available":eat_available,"direct_consumption_control":direct_consumption,"movement":host.world().entity(body.id).unwrap().position != body.position})
            );
        }
    }
    assert!(needs["hungry_tired"].drives.hunger > needs["fed_tired"].drives.hunger + 0.1);
    assert!(
        needs["fed_tired"].drives.fatigue > needs["fed_tired"].drives.hunger + 0.05,
        "prepared fed/fatigued condition must retain measurable fatigue above residual hunger"
    );
    assert!(needs["fed_tired"].drives.fatigue > needs["fed_rested"].drives.fatigue + 0.03);
    assert_eq!(trainer.checkpoint(), founder_before);
}
