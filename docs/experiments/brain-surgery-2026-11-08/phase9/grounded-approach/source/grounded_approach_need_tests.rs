//! Six prescribed physical intervals from eight fixed slots; two blocked slots unavailable.
//! This is not a neural selection or learned policy.
//! No fake neural provenance envelope is constructed to satisfy store admission.
use super::*;
use alife_core::*;
use serde_json::{json, Value};
use std::io::Write;

const SEED: u64 = 921_412;
const SUBJECT: OrganismId = OrganismId(1);
const TARGET_POSITION: Vec3f = Vec3f::new(0.8, 0.0, 0.0);
const CASES: [&str; 4] = [
    "peer_approach",
    "toy_approach",
    "peer_passive",
    "peer_blocked",
];

struct Setup {
    world: HeadlessWorld,
    actor: WorldEntityId,
    target: WorldEntityId,
    frame: PerceptionFrame,
    candidate: ActionCandidate,
    before: BiochemistryState,
    pressure: f32,
    case: &'static str,
}

// Fixture initialization only. These are freshly constructed public-genome
// concentrations, not loaded life/checkpoint state. Change the chemical source,
// then rederive the canonical projection; never change just the drive view.
fn initialize_loneliness(phenotype: &CreaturePhenotype, pressure: f32) -> BiochemistryState {
    let initial = BiochemistryState::new_with_age(phenotype, Tick(600), Tick(600)).unwrap();
    let chemistry = &phenotype.chemistry.biochemical;
    let receptor = chemistry
        .receptors()
        .iter()
        .find(|receptor| {
            receptor.target == BiochemicalTargetLocus::Drive(BiochemicalDriveChannel::Loneliness)
        })
        .unwrap();
    assert_eq!(receptor.threshold, 0.0);
    assert_eq!(receptor.gain, 1.0);
    assert!(!receptor.digital);
    let index = chemistry
        .species()
        .iter()
        .position(|species| species.id == receptor.source)
        .unwrap();
    let mut fresh_wire = serde_json::to_value(initial).unwrap();
    fresh_wire["graph_state"]["concentrations"][index] = json!(pressure);
    let state: BiochemicalGraphState =
        serde_json::from_value(fresh_wire["graph_state"].clone()).unwrap();
    fresh_wire["homeostasis"] =
        serde_json::to_value(state.derive_homeostasis(chemistry).unwrap()).unwrap();
    let biology: BiochemistryState = serde_json::from_value(fresh_wire).unwrap();
    biology.validate_against(phenotype).unwrap();
    assert_eq!(
        biology.homeostasis,
        biology.graph_state().derive_homeostasis(chemistry).unwrap()
    );
    assert_eq!(
        biology.homeostasis.drives.loneliness.to_bits(),
        pressure.to_bits()
    );
    let original = initial.homeostasis.drives.to_array();
    let changed = biology.homeostasis.drives.to_array();
    for index in 0..11 {
        if index != 4 {
            assert_eq!(original[index].to_bits(), changed[index].to_bits());
        }
    }
    assert_eq!(biology.body, initial.body);
    assert_eq!(biology.development, initial.development);
    biology
}

fn setups(pressure: f32) -> Vec<Setup> {
    let genome = CreatureGenome::early_mammal_founder(
        SEED,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let before = initialize_loneliness(&phenotype, pressure);
    let mut peer = HeadlessScenarioBuilder::new(SEED)
        .agent("subject", SUBJECT, Vec3f::ZERO)
        .build()
        .unwrap();
    peer.tick = Tick(600);
    let actor = peer.entity_id("subject").unwrap();
    let target = peer
        .spawn_social_agent("target", OrganismId(2), TARGET_POSITION, 0.0)
        .unwrap();
    peer.set_grounded_physical_properties(
        target,
        GroundedPhysicalProperties::deterministic_for_kind(WorldObjectKind::ActivityToy, 2),
    )
    .unwrap();
    peer.register_organism_record(
        crate::WorldOrganismRecord::new(SUBJECT, actor, genome, phenotype, before, Tick::ZERO)
            .unwrap(),
    )
    .unwrap();
    let mut result = Vec::new();
    for case in CASES {
        // Each arm is an isolated fork of the same unadvanced body/scene.
        let mut world = peer.clone();
        if case == "toy_approach" {
            let object = world.objects.get_mut(&target.raw()).unwrap();
            object.kind = WorldObjectKind::ActivityToy;
            object.organism_id = None;
            // All pose/radius/material/chemical observables remain matched.
        }
        if case == "peer_blocked" {
            for (label, position) in [
                ("cage_east", Vec3f::new(0.4, 0.0, 0.0)),
                ("cage_west", Vec3f::new(-0.4, 0.0, 0.0)),
                ("cage_north", Vec3f::new(0.0, 0.0, 0.4)),
                ("cage_south", Vec3f::new(0.0, 0.0, -0.4)),
            ] {
                let obstacle = world
                    .insert_object(SpawnSpec {
                        label,
                        kind: WorldObjectKind::Obstacle,
                        organism_id: None,
                        position,
                        nutrition: 0.0,
                        hazard_pain: 0.0,
                        token_id: None,
                        social_affinity: 0.0,
                        teacher_channel: None,
                    })
                    .unwrap();
                let obstacle = world.objects.get_mut(&obstacle.raw()).unwrap();
                obstacle.radius = 0.2;
                obstacle.optical_opacity = 0.0;
            }
        }
        world.validate_organism_bindings().unwrap();
        let frame = world
            .perception_frame(
                SUBJECT,
                Tick(600),
                SensorProfile::GroundedObjectSlotsV1,
                before.homeostasis,
            )
            .unwrap();
        println!("cold candidate diagnostics pressure={pressure} case={case} target={} slots={} candidates={:?}", target.raw(), frame.grounded_object_slots().len(), frame.candidates().iter().map(|candidate| (candidate.action_id.raw(), candidate.kind, candidate.family, candidate.target.entity.map(WorldEntityId::raw))).collect::<Vec<_>>());
        let candidate = frame
            .candidates()
            .iter()
            .find(|candidate| {
                candidate.action_id == HeadlessActionIds::APPROACH
                    && candidate.target.entity == Some(target)
            })
            .copied();
        let Some(candidate) = candidate else {
            // Pre-execution revision after the preserved zero-interval failure:
            // retain geometry and admission. Never force attention or a target.
            assert_eq!(case, "peer_blocked", "unexpected missing candidate");
            assert_eq!(frame.grounded_object_slots().len(), 5);
            println!("unavailable fixed slot pressure={pressure} case={case} reason=peer omitted by four-object candidate budget; actual_intervals=0");
            continue;
        };
        assert_eq!(candidate.family, CandidateActionFamily::Approach);
        assert!(matches!(
            candidate.observation,
            CandidateObservationRef::ObjectSlot(_)
        ));
        assert!(!world.body_contacts_at(actor, Vec3f::ZERO).1);
        assert_eq!(frame.homeostasis(), &before.homeostasis);
        result.push(Setup {
            world,
            actor,
            target,
            frame,
            candidate,
            before,
            pressure,
            case,
        });
    }
    assert_eq!(
        result.len(),
        3,
        "exactly the three unchanged eligible slots"
    );
    result
}

fn execute(setup: Setup) -> Value {
    let Setup {
        mut world,
        actor,
        target,
        frame,
        candidate,
        before,
        pressure,
        case,
    } = setup;
    let fork = format!("grounded-approach-need-v1/pressure-{pressure}/{case}");
    let start = world.entity(actor).unwrap().position;
    let initial_touch = world.body_contacts_at(actor, start).1;
    let initial_yaw = world.entity(actor).unwrap().body_yaw;
    let query = NeedConsequenceQueryV1::from_frame(&frame, &candidate).unwrap();
    let store = NeedConsequenceStoreV1::new(SUBJECT, 2).unwrap();
    let mut operation = json!({"kind": "passive_world_tick", "selected_command_count": 0});
    let after;
    let physical_displacement;
    let operation_succeeded;
    let body_event;
    if case == "peer_passive" {
        world.try_advance_tick().unwrap();
        after = *world
            .organism_registry()
            .get(SUBJECT)
            .unwrap()
            .biochemistry();
        physical_displacement = subtract(world.entity(actor).unwrap().position, start);
        operation_succeeded = true;
        // No action receipt exists for passive biology. This is a verified
        // zero ambient-input control, not a fabricated receipt or action source.
        body_event =
            json!({"source": "no-action control, no contact/mating/care", "social_contact": 0.0});
    } else {
        // Selection is prescribed by this experiment. Confidence=1 states that
        // prescription only; it is not a learned-policy or causal confidence.
        let command = candidate
            .to_command(SUBJECT, Confidence::new(1.0).unwrap())
            .unwrap();
        assert_eq!(command.action_id, HeadlessActionIds::APPROACH);
        assert_eq!(command.intensity.raw(), 1.0);
        let receipt = world
            .apply_registered_command(&command, actor, Tick(601))
            .unwrap();
        assert_eq!(receipt.biology_before, before);
        assert_eq!(receipt.action_result.command, command);
        after = receipt.biology_after;
        physical_displacement = receipt.action_result.execution.physical.displacement;
        operation_succeeded = receipt.action_result.execution.succeeded;
        body_event = serde_json::to_value(receipt.action_result.body_event).unwrap();
        operation = json!({
            "kind": "prescribed_registered_Approach",
            "selected_command_count": 1,
            "primitive": command.action_id.raw(),
            "action_kind": format!("{:?}", command.kind),
            "family": format!("{:?}", candidate.family),
            "target_entity": command.target_entity.unwrap().raw(),
            "command_intensity": command.intensity.raw(),
            "prescription_confidence_not_neural": command.confidence.raw(),
            "receipt_succeeded": operation_succeeded,
            "receipt_failure": format!("{:?}", receipt.action_result.execution.failure),
            "receipt_phase": format!("{:?}", receipt.action_result.phase),
            "physical": receipt.action_result.execution.physical,
        });
        world.try_advance_tick().unwrap();
        assert_eq!(
            *world
                .organism_registry()
                .get(SUBJECT)
                .unwrap()
                .biochemistry(),
            after,
            "world finalization must not run the already committed biology twice"
        );
    }
    assert_eq!(world.tick(), Tick(601));
    assert_eq!(before.tick, Tick(600));
    assert_eq!(after.tick, Tick(601));
    assert_eq!(after.development.age_ticks, Tick(601));
    let end = world.entity(actor).unwrap().position;
    assert_eq!(physical_displacement, subtract(end, start));
    let final_touch = world.body_contacts_at(actor, end).1;
    let next_frame = world
        .perception_frame(
            SUBJECT,
            Tick(601),
            SensorProfile::GroundedObjectSlotsV1,
            after.homeostasis,
        )
        .unwrap();
    let next_candidate = next_frame.candidates().iter().find(|candidate| {
        candidate.action_id == HeadlessActionIds::APPROACH
            && candidate.target.entity == Some(target)
    });
    let same_source = store.recall(&query, Tick(601), 1, 2).unwrap();
    assert!(matches!(
        same_source.evidence,
        NeedConsequenceEvidenceV1::Unknown
    ));
    let next_lookup = next_candidate.map(|candidate| {
        let next_query = NeedConsequenceQueryV1::from_frame(&next_frame, candidate).unwrap();
        let recall = store.recall(&next_query, Tick(601), 1, 2).unwrap();
        assert!(matches!(
            recall.evidence,
            NeedConsequenceEvidenceV1::Unknown
        ));
        json!({"evidence": "Unknown", "records_inspected": recall.inspected_records,
            "same_exact_context_key": query == next_query,
            "same_tracked_target": query.tracked_target() == next_query.tracked_target(),
            "same_family": query.action_family() == next_query.action_family()})
    });
    let before_drives = before.homeostasis.drives.to_array();
    let after_drives = after.homeostasis.drives.to_array();
    let delta: [f32; 11] = std::array::from_fn(|i| after_drives[i] - before_drives[i]);
    let condition_met = match case {
        "peer_approach" => {
            operation_succeeded && !initial_touch && final_touch && distance(start, end) > 0.0
        }
        "toy_approach" => {
            operation_succeeded && !initial_touch && !final_touch && distance(start, end) > 0.0
        }
        "peer_passive" => !initial_touch && !final_touch && start == end,
        "peer_blocked" => !operation_succeeded && !initial_touch && !final_touch && start == end,
        _ => unreachable!(),
    };
    let mut row = json!({
        "fixture_fork_namespace": fork,
        "world_binding_limit": "namespace is external fixture ownership, not a runtime world/life epoch API",
        "world_seed": SEED, "organism": SUBJECT.raw(), "actor_entity": actor.raw(),
        "genome_id": before.source_genome_id, "source_sequence": null,
        "source_origin": "prescribed physical interval, no neural decision or sealed neural patch",
        "source_frame_digest": frame.frame_digest().0, "next_frame_digest": next_frame.frame_digest().0,
        "source_tracked_target": format!("{:?}", query.tracked_target()),
        "case": case, "initial_loneliness": pressure, "intended_condition_met": condition_met,
        "ticks": [before.tick.raw(), after.tick.raw()], "world_ticks_per_second": WORLD_TICKS_PER_SECOND,
        "biology_updates": 1, "development_before": before.development, "development_after": after.development,
        "cadence": before.cadence, "start": start, "end": end,
        "initial_yaw": initial_yaw, "final_yaw": world.entity(actor).unwrap().body_yaw,
        "target_start": TARGET_POSITION, "target_end": world.entity(target).unwrap().position,
        "per_segment_displacements": [physical_displacement], "travel": distance(start, end),
        "initial_peer_contact": initial_touch, "final_peer_contact": final_touch,
        "operation": operation, "body_event": body_event,
    });
    let biology_and_evidence = json!({
        "source_before_drives": before_drives, "final_after_drives": after_drives,
        "raw_eleven_deltas": delta, "body_energy_before": before.body.energy, "body_energy_after": after.body.energy,
        "biochemical_work": after.biochemical_work(),
        "sensor_confidence": candidate.sensor_confidence.raw(),
        "measured_store_confidence": null, "causal_confidence": null,
        "store_admission": "not attempted: authentic neural decision required; no fabricated envelope",
        "store_records": store.len(),
        "exact_same_source_lookup": {"evidence": "Unknown", "records_inspected": same_source.inspected_records},
        "actual_next_source_lookup": next_lookup,
        "interpretation": "joint physical interval observation only; not scalar reward, policy, or learned self-care",
    });
    row.as_object_mut()
        .unwrap()
        .extend(biology_and_evidence.as_object().unwrap().clone());
    row
}

#[test]
fn grounded_approach_fixture_preflight_no_intervals() {
    for pressure in [0.0_f32, 0.15] {
        for setup in setups(pressure) {
            println!("preflight pressure={} case={} candidate={} observed={:?} target={} radius={} body_radius={} sensor_confidence={}",
                pressure, setup.case, setup.candidate.action_id.raw(), setup.candidate.observation,
                setup.target.raw(), setup.world.entity(setup.target).unwrap().radius,
                setup.world.locomotion_body_radius(), setup.candidate.sensor_confidence.raw());
        }
    }
}

#[test]
#[ignore = "explicit six-world-interval revision; requires fresh output directory and must not rerun"]
fn grounded_approach_need_six_prescribed_intervals() {
    let directory = std::env::var_os("ALIFE_GROUNDED_APPROACH_RUN_DIR")
        .expect("explicit run directory required");
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let mut started = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(directory.join("started.json"))
        .expect("never rerun this budget: started marker already exists");
    writeln!(
        started,
        "{{\"budget\":8,\"status\":\"started\",\"source\":\"frozen before execution\"}}"
    )
    .unwrap();
    started.sync_all().unwrap();
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(directory.join("outcomes.jsonl"))
        .unwrap();
    let mut count = 0;
    for pressure in [0.0_f32, 0.15] {
        for setup in setups(pressure) {
            // Persist intent before every interval. An interrupted outcome is
            // uncertain, not permission to replay or reclaim its budget slot.
            writeln!(
                started,
                "{{\"interval\":{},\"pressure\":{},\"case\":\"{}\"}}",
                count + 1,
                pressure,
                setup.case
            )
            .unwrap();
            started.sync_all().unwrap();
            let row = execute(setup);
            writeln!(output, "{row}").unwrap();
            output.sync_all().unwrap();
            println!("grounded_approach_outcome {row}");
            count += 1;
        }
    }
    assert_eq!(count, 6);
    writeln!(
        started,
        "{{\"completed_intervals\":6,\"unspent_closed_slots\":2,\"budget_remaining\":0}}"
    )
    .unwrap();
    started.sync_all().unwrap();
}
